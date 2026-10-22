//! Tests the GitHub crawler and crates.io client against a local HTTP server.
//!
//! TODO Reviewed ??-??-2026

use std::io::Read as _;
use std::io::Write as _;
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use rust_rate::crates_io::fetch_summary;
use rust_rate::crawler::GithubCrawler;

/// Serves canned responses in order and records the request lines it receives.
struct MockServer {
    /// Base URL, for example `http://127.0.0.1:41234`.
    url: String,
    /// First line of every request received so far.
    requests: Arc<Mutex<Vec<String>>>,
}

impl MockServer {
    /// Starts a server that answers consecutive connections with `responses`.
    fn spawn(responses: Vec<String>) -> std::io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let url = format!("http://{}", listener.local_addr()?);
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&requests);
        thread::spawn(move || {
            for response in responses {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut buffer = [0_u8; 4096];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(buffer.get(..read).unwrap_or_default());
                if let Ok(mut lines) = recorded.lock() {
                    lines.push(request.lines().next().unwrap_or_default().to_owned());
                }
                let _ignored = stream.write_all(response.as_bytes());
            }
        });
        Ok(Self { url, requests })
    }

    /// Request lines received so far.
    fn requests(&self) -> Vec<String> {
        self.requests
            .lock()
            .map(|lines| lines.clone())
            .unwrap_or_default()
    }
}

/// Builds a complete HTTP/1.1 response with a JSON or plain body.
fn response(status: &str, extra_headers: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\n{extra_headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

/// Builds a code-search page holding `count` items.
fn search_page(count: usize) -> String {
    let items = (0..count)
        .map(|index| {
            format!(
                r#"{{"repository":{{"full_name":"o/r{index}"}},"path":"Cargo.toml","url":"http://example.invalid/{index}"}}"#
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    response("200 OK", "", &format!(r#"{{"items":[{items}]}}"#))
}

#[tokio::test]
async fn search_follows_pagination_until_a_short_page() {
    let server =
        MockServer::spawn(vec![search_page(100), search_page(3)]).expect("start mock server");
    let crawler = GithubCrawler::new(None)
        .expect("build crawler")
        .with_api_base(&server.url);

    let items = crawler.search_cargo_tomls("anyhow").await.expect("search");

    assert_eq!(items.len(), 103);
    let requests = server.requests();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].contains("page=1"), "{requests:?}");
    assert!(requests[1].contains("page=2"), "{requests:?}");
}

#[tokio::test]
async fn search_reports_rate_limiting_with_the_reset_time() {
    let server = MockServer::spawn(vec![response(
        "429 Too Many Requests",
        "x-ratelimit-reset: 1700000000\r\n",
        "{}",
    )])
    .expect("start mock server");
    let crawler = GithubCrawler::new(None)
        .expect("build crawler")
        .with_api_base(&server.url);

    let error = crawler
        .search_cargo_tomls("anyhow")
        .await
        .expect_err("rate limit must fail");

    let message = error.to_string();
    assert!(message.contains("rate limit"), "{message}");
    assert!(message.contains("1700000000"), "{message}");
}

#[tokio::test]
async fn search_reports_other_http_errors_by_status() {
    let server = MockServer::spawn(vec![response("422 Unprocessable Entity", "", "{}")])
        .expect("start mock server");
    let crawler = GithubCrawler::new(None)
        .expect("build crawler")
        .with_api_base(&server.url);

    let error = crawler
        .search_cargo_tomls("anyhow")
        .await
        .expect_err("HTTP 422 must fail");

    assert!(error.to_string().contains("422"), "{error}");
}

#[tokio::test]
async fn download_rejects_error_statuses_instead_of_returning_the_body() {
    let server = MockServer::spawn(vec![
        response("200 OK", "", "[package]\nname = \"x\"\n"),
        response("404 Not Found", "", "Not Found"),
    ])
    .expect("start mock server");
    let crawler = GithubCrawler::new(None).expect("build crawler");

    let content = crawler
        .download_content(&format!("{}/ok", server.url))
        .await
        .expect("download succeeds");
    let error = crawler
        .download_content(&format!("{}/missing", server.url))
        .await
        .expect_err("a 404 must not be returned as content");

    assert!(content.contains("name = \"x\""));
    assert!(error.to_string().contains("404"), "{error}");
}

#[tokio::test]
async fn crates_io_summary_decodes_the_crate_record() {
    let body = r#"{"crate":{"name":"anyhow","description":"Flexible errors","repository":null,"downloads":42,"recent_downloads":7}}"#;
    let server = MockServer::spawn(vec![response("200 OK", "", body)]).expect("start mock server");

    let summary = fetch_summary(&server.url, "anyhow")
        .await
        .expect("fetch summary");

    assert_eq!(summary.name, "anyhow");
    assert_eq!(summary.description.as_deref(), Some("Flexible errors"));
    assert_eq!(summary.repository, None);
    assert_eq!((summary.downloads, summary.recent_downloads), (42, 7));
    assert_eq!(server.requests(), ["GET /crates/anyhow HTTP/1.1"]);
}

#[tokio::test]
async fn crates_io_failures_are_reported() {
    let server = MockServer::spawn(vec![
        response("404 Not Found", "", "{}"),
        response("200 OK", "", "{}"),
    ])
    .expect("start mock server");

    let missing = fetch_summary(&server.url, "nope")
        .await
        .expect_err("HTTP 404 must fail");
    let malformed = fetch_summary(&server.url, "empty")
        .await
        .expect_err("a response without crate data must fail");
    let invalid = fetch_summary(&server.url, "../etc")
        .await
        .expect_err("invalid names must be rejected before any request");

    assert!(missing.to_string().contains("404"), "{missing}");
    assert!(malformed.to_string().contains("crate data"), "{malformed}");
    assert!(
        invalid.to_string().contains("valid crate name"),
        "{invalid}"
    );
    assert_eq!(server.requests().len(), 2);
}
