//! End-to-end tests of `rust_rate stats` against a local mock of the GitHub API.
//!
//! TODO Reviewed ??-??-2026.

#![expect(
    clippy::expect_used,
    reason = "Helpers in this test file fail loudly when test setup breaks."
)]

use std::io::BufRead as _;
use std::io::BufReader;
use std::io::Write as _;
use std::net::TcpListener;
use std::net::TcpStream;
use std::process::Command;
use std::process::Output;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

/// A canned HTTP response: status line text, extra headers, and body.
struct Reply {
    /// Status code and reason, such as `200 OK`.
    status: &'static str,
    /// Additional header lines without line endings.
    headers: Vec<String>,
    /// Response body.
    body: String,
}

/// A local HTTP server that answers GitHub API requests and records request targets.
struct MockGithub {
    /// Base URL of the server, such as `http://127.0.0.1:4000`.
    base: String,
    /// Request targets (path and query) in arrival order.
    requests: Arc<Mutex<Vec<String>>>,
}

impl MockGithub {
    /// Starts a server on an ephemeral port; `route` maps `(base, target)` to a reply.
    fn start(route: impl Fn(&str, &str) -> Reply + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
        let base = format!("http://{}", listener.local_addr().expect("local address"));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let route = Arc::new(route);
        let (served_base, served_requests) = (base.clone(), Arc::clone(&requests));
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (conn_base, conn_requests, conn_route) = (
                    served_base.clone(),
                    Arc::clone(&served_requests),
                    Arc::clone(&route),
                );
                thread::spawn(move || {
                    serve(stream, &conn_base, &conn_requests, conn_route.as_ref());
                });
            }
        });
        Self { base, requests }
    }

    /// Returns the request targets seen so far.
    fn seen(&self) -> Vec<String> {
        self.requests.lock().expect("request log").clone()
    }
}

/// Reads one request from `stream`, records its target, and writes the routed reply.
fn serve(
    mut stream: TcpStream,
    base: &str,
    requests: &Mutex<Vec<String>>,
    route: &(dyn Fn(&str, &str) -> Reply + Send + Sync),
) {
    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).unwrap_or(0) == 0 {
        return;
    }
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).unwrap_or(0) == 0 || header.trim().is_empty() {
            break;
        }
    }
    let target = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_owned();
    requests.lock().expect("request log").push(target.clone());
    let reply = route(base, &target);
    let mut response = format!(
        "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        reply.status,
        reply.body.len()
    );
    for header in reply.headers {
        response.push_str(&header);
        response.push_str("\r\n");
    }
    response.push_str("\r\n");
    response.push_str(&reply.body);
    // The client may already have hung up; there is nobody to report that to.
    let _unused = stream.write_all(response.as_bytes());
}

/// Builds a `200 OK` reply with `body`.
fn ok(body: impl Into<String>) -> Reply {
    Reply {
        status: "200 OK",
        headers: Vec::new(),
        body: body.into(),
    }
}

/// Builds a code-search reply whose items point at `{base}/raw/{name}` for each name.
fn search_reply(base: &str, names: &[&str]) -> Reply {
    let items: Vec<String> = names
        .iter()
        .map(|name| {
            format!(
                r#"{{"path":"{name}","url":"{base}/raw/{name}","repository":{{"full_name":"o/{name}"}}}}"#
            )
        })
        .collect();
    ok(format!(r#"{{"items":[{}]}}"#, items.join(",")))
}

/// Runs `rust_rate stats anyhow` against the mock server.
fn run_stats(base: &str, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rust_rate"))
        .args(["stats", "anyhow", "--api-base", base])
        .args(extra)
        .env("RUST_LOG", "info")
        .env_remove("GITHUB_TOKEN")
        .output()
        .expect("run rust_rate")
}

/// Routes the happy-path scenario: two manifests and one Rust source.
fn happy_route(base: &str, target: &str) -> Reply {
    if target.starts_with("/search/code") && target.contains("extension:rs") {
        search_reply(base, &["src.rs"])
    } else if target.starts_with("/search/code") {
        search_reply(base, &["one.toml", "two.toml"])
    } else if target == "/raw/one.toml" {
        ok("[dependencies]\nanyhow = \"1\"\nserde = \"1\"\n")
    } else if target == "/raw/two.toml" {
        ok("[dependencies]\nanyhow = \"1\"\n")
    } else if target == "/raw/src.rs" {
        ok("use anyhow::Result;\nfn main() -> anyhow::Result<()> {\n    Ok(())\n}\n")
    } else {
        Reply {
            status: "404 Not Found",
            headers: Vec::new(),
            body: String::new(),
        }
    }
}

/// Manifests feed the usage summary and sources feed the pattern analysis.
#[test]
fn stats_summarizes_manifests_and_analyzes_downloaded_sources() {
    let server = MockGithub::start(happy_route);
    let output = run_stats(&server.base, &[]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "stderr: {stderr}");
    assert!(stdout.contains("anyhow: 2 usages"), "stdout: {stdout}");
    assert!(stdout.contains("serde: 1 usages"), "stdout: {stdout}");
    // Diagnostics go through `tracing`, whose default subscriber writes to stdout.
    assert!(stdout.contains("Import of anyhow"), "stdout: {stdout}");
    let seen = server.seen();
    assert!(seen.contains(&"/raw/src.rs".to_owned()), "{seen:?}");
    assert!(
        !seen.contains(&"/raw/src.rs/Cargo.toml".to_owned()),
        "{seen:?}"
    );
}

/// `--limit` caps how many manifests are downloaded.
#[test]
fn stats_limit_caps_downloads() {
    let server = MockGithub::start(happy_route);
    let output = run_stats(&server.base, &["--limit", "1"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("anyhow: 1 usages"), "stdout: {stdout}");
    let seen = server.seen();
    assert!(seen.contains(&"/raw/one.toml".to_owned()), "{seen:?}");
    assert!(!seen.contains(&"/raw/two.toml".to_owned()), "{seen:?}");
}

/// A failed download is skipped and the summary still prints.
#[test]
fn stats_skips_downloads_that_fail() {
    let server = MockGithub::start(|base, target| {
        if target == "/raw/two.toml" {
            Reply {
                status: "500 Internal Server Error",
                headers: Vec::new(),
                body: String::new(),
            }
        } else {
            happy_route(base, target)
        }
    });
    let output = run_stats(&server.base, &[]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("anyhow: 1 usages"), "stdout: {stdout}");
}

/// Rate limiting fails the command and says when the quota resets.
#[test]
fn stats_reports_rate_limiting() {
    let server = MockGithub::start(|_base, _target| Reply {
        status: "403 Forbidden",
        headers: vec!["x-ratelimit-reset: 1700000000".to_owned()],
        body: String::new(),
    });
    let output = run_stats(&server.base, &[]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("rate limit"), "stderr: {stderr}");
    assert!(stderr.contains("1700000000"), "stderr: {stderr}");
}

/// An invalid crate name is rejected before any network access.
#[test]
fn stats_rejects_invalid_crate_names_without_requests() {
    let server = MockGithub::start(happy_route);
    let output = Command::new(env!("CARGO_BIN_EXE_rust_rate"))
        .args(["stats", "not a crate!", "--api-base", &server.base])
        .output()
        .expect("run rust_rate");

    assert!(!output.status.success());
    assert!(server.seen().is_empty());
}
