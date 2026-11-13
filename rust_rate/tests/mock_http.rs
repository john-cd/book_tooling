//! Tests the GitHub crawler and crates.io client against a local HTTP server.
//!
//! TODO Reviewed ??-??-2026.

use std::io::Read as _;
use std::io::Write as _;
use std::net::TcpListener;
use std::net::TcpStream;
use std::process::Command;
use std::process::Output;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use rust_rate::crates_io::fetch_summary;
use rust_rate::crawler::CrawlerLimits;
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
        Self::spawn_with_handler(move |listener, recorded| {
            for response in responses {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                Self::record_request(&mut stream, &recorded);
                let _ignored = stream.write_all(response.as_bytes());
            }
        })
    }

    /// Starts a server whose handler controls the accepted connection.
    fn spawn_with_handler<F>(handler: F) -> std::io::Result<Self>
    where
        F: FnOnce(TcpListener, Arc<Mutex<Vec<String>>>) + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let url = format!("http://{}", listener.local_addr()?);
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&requests);
        thread::spawn(move || handler(listener, recorded));
        Ok(Self { url, requests })
    }

    /// Records the first line from an accepted HTTP request.
    fn record_request(stream: &mut TcpStream, recorded: &Mutex<Vec<String>>) {
        let mut buffer = [0_u8; 4096];
        let read = stream.read(&mut buffer).unwrap_or_default();
        let request = String::from_utf8_lossy(buffer.get(..read).unwrap_or_default());
        if let Ok(mut lines) = recorded.lock() {
            lines.push(request.lines().next().unwrap_or_default().to_owned());
        }
    }

    /// Request lines received so far.
    fn requests(&self) -> Vec<String> {
        self.requests.lock().map_or_default(|lines| lines.clone())
    }
}

/// Creates a crawler with short deadlines and explicit body ceilings for local tests.
fn crawler_with_limits(
    api_base: &str,
    request_timeout: Duration,
    max_search_response_bytes: usize,
    max_content_response_bytes: usize,
) -> anyhow::Result<GithubCrawler> {
    GithubCrawler::new_with_limits(
        None,
        CrawlerLimits {
            request_timeout,
            connect_timeout: Duration::from_secs(1),
            max_search_response_bytes,
            max_content_response_bytes,
        },
    )
    .map(|crawler| crawler.with_api_base(api_base))
}

/// Runs the CLI with short request deadlines against a local API root.
fn run_stats_cli(
    api_base: &str,
    request_timeout_ms: u64,
    max_search_response_bytes: usize,
) -> std::io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_rust_rate"))
        .args([
            "stats",
            "anyhow",
            "--limit",
            "0",
            "--api-base",
            api_base,
            "--token-env",
            "RUST_RATE_TEST_TOKEN_NOT_SET",
            "--request-timeout-ms",
            &request_timeout_ms.to_string(),
            "--connect-timeout-ms",
            "1000",
            "--max-search-response-bytes",
            &max_search_response_bytes.to_string(),
            "--max-content-response-bytes",
            "64",
        ])
        .env_remove("RUST_RATE_TEST_TOKEN_NOT_SET")
        .output()
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

/// Search follows pagination until a short page.
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

/// Search reports rate limiting with the reset time.
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

/// Search reports other http errors by status.
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

/// The request deadline cancels a search whose server never sends response headers.
#[tokio::test]
async fn search_request_deadline_bounds_a_stalled_header_response() {
    let server = MockServer::spawn_with_handler(|listener, recorded| {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        MockServer::record_request(&mut stream, &recorded);
        thread::sleep(Duration::from_millis(400));
        let _ignored = stream.write_all(response("200 OK", "", r#"{"items":[]}"#).as_bytes());
    })
    .expect("start stalled-header server");
    let crawler = crawler_with_limits(&server.url, Duration::from_millis(100), 1024, 1024)
        .expect("build bounded crawler");
    let started = Instant::now();

    let error = crawler
        .search_cargo_tomls("anyhow")
        .await
        .expect_err("the stalled response should time out");

    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(format!("{error:#}").contains("timed out"), "{error:#}");
}

/// The request deadline cancels a content response whose body stops making progress.
#[tokio::test]
async fn content_request_deadline_bounds_a_stalled_body() {
    let server = MockServer::spawn_with_handler(|listener, recorded| {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        MockServer::record_request(&mut stream, &recorded);
        let _header_write_result = stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\na");
        thread::sleep(Duration::from_millis(400));
        let _body_write_result = stream.write_all(b"123456789");
    })
    .expect("start stalled-body server");
    let crawler = crawler_with_limits(&server.url, Duration::from_millis(100), 1024, 1024)
        .expect("build bounded crawler");
    let started = Instant::now();

    let error = crawler
        .download_content(&format!("{}/file", server.url))
        .await
        .expect_err("the stalled body should time out");

    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(format!("{error:#}").contains("timed out"), "{error:#}");
}

/// Rejects a search body that exceeds its configured limit without parsing it.
#[tokio::test]
async fn search_response_byte_limit_rejects_an_oversized_body() {
    let server = MockServer::spawn(vec![response("200 OK", "", r#"{"items":[]}"#)])
        .expect("start oversized search server");
    let crawler = crawler_with_limits(&server.url, Duration::from_secs(1), 8, 1024)
        .expect("build bounded crawler");

    let error = crawler
        .search_cargo_tomls("anyhow")
        .await
        .expect_err("the oversized search response should fail");

    assert!(
        error.to_string().contains("exceeding the configured limit"),
        "{error:#}"
    );
}

/// Stops reading a chunked search response as soon as it crosses the configured limit.
#[tokio::test]
async fn search_response_byte_limit_rejects_oversized_chunked_body() {
    let body = r#"{"items":[]}"#;
    let response = format!(
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:X}\r\n{body}\r\n0\r\n\r\n",
        body.len()
    );
    let server = MockServer::spawn(vec![response]).expect("start chunked search server");
    let crawler = crawler_with_limits(&server.url, Duration::from_secs(1), 8, 1024)
        .expect("build bounded crawler");

    let error = crawler
        .search_cargo_tomls("anyhow")
        .await
        .expect_err("the oversized chunked response should fail");

    assert!(
        error.to_string().contains("exceeds the configured limit"),
        "{error:#}"
    );
}

/// Rejects a content response whose declared length exceeds its configured limit.
#[tokio::test]
async fn content_response_byte_limit_rejects_an_oversized_body() {
    let server = MockServer::spawn(vec![response("200 OK", "", "123456789")])
        .expect("start oversized content server");
    let crawler = crawler_with_limits(&server.url, Duration::from_secs(1), 1024, 8)
        .expect("build bounded crawler");

    let error = crawler
        .download_content(&format!("{}/file", server.url))
        .await
        .expect_err("the oversized content response should fail");

    let message = error.to_string();
    assert!(message.contains("declares 9 bytes"), "{message}");
    assert!(message.contains("limit of 8 bytes"), "{message}");
}

/// The CLI exits nonzero when a local GitHub server stalls past the request deadline.
#[test]
fn cli_exits_nonzero_when_search_response_headers_stall() {
    let server = MockServer::spawn_with_handler(|listener, recorded| {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        MockServer::record_request(&mut stream, &recorded);
        thread::sleep(Duration::from_millis(400));
        let _ignored = stream.write_all(response("200 OK", "", r#"{"items":[]}"#).as_bytes());
    })
    .expect("start stalled-header server");
    let started = Instant::now();

    let output = run_stats_cli(&server.url, 100, 1024).expect("run rust_rate CLI");

    assert!(!output.status.success());
    assert!(started.elapsed() < Duration::from_secs(3));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("timed out"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The CLI exits nonzero when a local GitHub search response exceeds the byte ceiling.
#[test]
fn cli_exits_nonzero_when_search_response_exceeds_its_byte_limit() {
    let server = MockServer::spawn(vec![response("200 OK", "", r#"{"items":[]}"#)])
        .expect("start oversized search server");

    let output = run_stats_cli(&server.url, 1000, 8).expect("run rust_rate CLI");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("exceeding the configured limit"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Download rejects error statuses instead of returning the body.
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

/// Crates io summary decodes the crate record.
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

/// Crates io failures are reported.
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
