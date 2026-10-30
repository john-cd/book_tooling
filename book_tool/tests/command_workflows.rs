//! End-to-end tests of the `book_tool` subcommands.
//!
//! TODO Reviewed ??-??-2026

#![expect(
    clippy::expect_used,
    reason = "Helpers in this test file fail loudly when test setup breaks."
)]

use std::collections::VecDeque;
use std::fs;
use std::io::Read as _;
use std::io::Write as _;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use book_tool::common::Scope;
use tempfile::tempdir;

struct TestHttpServer {
    address: SocketAddr,
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl TestHttpServer {
    fn spawn() -> Self {
        Self::spawn_with_responses(Vec::new())
    }

    fn spawn_with_responses(responses: Vec<String>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        listener
            .set_nonblocking(true)
            .expect("set listener nonblocking");

        let address = listener.local_addr().expect("get local address");
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_signal = Arc::clone(&shutdown);
        let responses = Arc::new(Mutex::new(VecDeque::from(responses)));
        let response_queue = Arc::clone(&responses);

        let handle = thread::spawn(move || {
            while !shutdown_signal.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let response = response_queue
                            .lock()
                            .expect("lock response queue")
                            .pop_front()
                            .unwrap_or_else(ok_response);
                        drop(respond(&mut stream, &response));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            address,
            shutdown,
            handle: Some(handle),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.address, path)
    }
}

fn book_tool_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_book_tool"))
}

fn respond(stream: &mut TcpStream, response: &str) -> std::io::Result<()> {
    let mut buffer = [0_u8; 1024];
    let _ = stream.read(&mut buffer)?;
    stream.write_all(response.as_bytes())
}

fn ok_response() -> String {
    "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned()
}

fn status_response(status_line: &str, headers: &[&str]) -> String {
    let mut response = format!("HTTP/1.1 {status_line}\r\n");
    for header in headers {
        response.push_str(header);
        response.push_str("\r\n");
    }
    response.push_str("Content-Length: 0\r\nConnection: close\r\n\r\n");
    response
}

fn retry_after_response(seconds: u64) -> String {
    format!(
        "HTTP/1.1 429 Too Many Requests\r\nRetry-After: {seconds}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
}

fn invalid_retry_after_response(value: &str) -> String {
    status_response("429 Too Many Requests", &[&format!("Retry-After: {value}")])
}

fn service_unavailable_response_with_headers(headers: &[&str]) -> String {
    status_response("503 Service Unavailable", headers)
}

impl Drop for TestHttpServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        drop(TcpStream::connect(self.address));
        if let Some(handle) = self.handle.take() {
            drop(handle.join());
        }
    }
}

fn write_markdown_file(root: &Path, name: &str, content: &str) -> std::path::PathBuf {
    let path = root.join(name);
    fs::write(&path, content).expect("write markdown file");
    path
}

/// Inline links execute converts links to reference definitions.
#[test]
fn inline_links_execute_converts_links_to_reference_definitions() {
    let temp = tempdir().expect("create temp dir");
    let markdown = "See [Rust](https://www.rust-lang.org/).\n";
    let file = write_markdown_file(temp.path(), "doc.md", markdown);

    book_tool::inline_links::execute(&[temp.path().to_path_buf()], &Scope::default())
        .expect("run inline-links");

    let updated = fs::read_to_string(file).expect("read transformed markdown");
    assert!(
        updated.contains("[Rust][1]"),
        "updated content was:\n{updated}"
    );
    assert!(
        updated.contains("[1]: https://www.rust-lang.org/"),
        "updated content was:\n{updated}"
    );
}

/// Directives execute expands example directives.
#[test]
fn directives_execute_expands_example_directives() {
    let temp = tempdir().expect("create temp dir");
    let file = write_markdown_file(temp.path(), "doc.md", "{{#example demo}}\n");

    book_tool::directives::execute(vec![temp.path().to_path_buf()], None).expect("run directives");

    let updated = fs::read_to_string(file).expect("read transformed markdown");
    assert_eq!(updated, "{{#include examples/demo.md}}\n");
}

/// Link check execute skips deleted unused reference definitions.
#[tokio::test]
async fn link_check_execute_skips_deleted_unused_reference_definitions() {
    let server = TestHttpServer::spawn();
    let temp = tempdir().expect("create temp dir");
    let markdown = format!(
        "[Used][used]\n\n[used]: {}\n[unused]: http://127.0.0.1:9/unused\n",
        server.url("/ok")
    );
    let file = write_markdown_file(temp.path(), "doc.md", &markdown);

    book_tool::link_check::execute(
        vec![temp.path().to_path_buf()],
        true,
        true,
        &Scope::default(),
    )
    .await
    .expect("run link-check");

    let updated = fs::read_to_string(file).expect("read transformed markdown");
    assert!(
        updated.contains("[used]: http://"),
        "updated content was:\n{updated}"
    );
    assert!(
        !updated.contains("[unused]: http://127.0.0.1:9/unused"),
        "updated content was:\n{updated}"
    );
}

/// Link check execute retries retry after responses.
#[tokio::test]
async fn link_check_execute_retries_retry_after_responses() {
    let server = TestHttpServer::spawn_with_responses(vec![retry_after_response(0), ok_response()]);
    let temp = tempdir().expect("create temp dir");
    let markdown = format!("[retry me]({})\n", server.url("/rate-limited"));
    write_markdown_file(temp.path(), "doc.md", &markdown);

    book_tool::link_check::execute(
        vec![temp.path().to_path_buf()],
        false,
        false,
        &Scope::default(),
    )
    .await
    .expect("run link-check with retry");
}

/// Link check execute retries service unavailable without retry after.
#[tokio::test]
async fn link_check_execute_retries_service_unavailable_without_retry_after() {
    let server = TestHttpServer::spawn_with_responses(vec![
        service_unavailable_response_with_headers(&[]),
        ok_response(),
    ]);
    let temp = tempdir().expect("create temp dir");
    let markdown = format!("[retry me]({})\n", server.url("/service-unavailable"));
    write_markdown_file(temp.path(), "doc.md", &markdown);

    book_tool::link_check::execute(
        vec![temp.path().to_path_buf()],
        false,
        false,
        &Scope::default(),
    )
    .await
    .expect("run link-check with missing retry-after");
}

/// Link check execute retries invalid retry after responses.
#[tokio::test]
async fn link_check_execute_retries_invalid_retry_after_responses() {
    let server = TestHttpServer::spawn_with_responses(vec![
        invalid_retry_after_response("soon"),
        ok_response(),
    ]);
    let temp = tempdir().expect("create temp dir");
    let markdown = format!("[retry me]({})\n", server.url("/invalid-retry-after"));
    write_markdown_file(temp.path(), "doc.md", &markdown);

    book_tool::link_check::execute(
        vec![temp.path().to_path_buf()],
        false,
        false,
        &Scope::default(),
    )
    .await
    .expect("run link-check with invalid retry-after");
}

/// A server that rejects `HEAD` with 405 is retried with `GET` and counts as healthy.
#[tokio::test]
async fn link_check_execute_falls_back_to_get_when_head_is_not_allowed() {
    let server = TestHttpServer::spawn_with_responses(vec![
        status_response("405 Method Not Allowed", &[]),
        ok_response(),
    ]);
    let temp = tempdir().expect("create temp dir");
    let markdown = format!("[head-hostile]({})\n", server.url("/no-head"));
    write_markdown_file(temp.path(), "doc.md", &markdown);

    book_tool::link_check::execute(
        vec![temp.path().to_path_buf()],
        false,
        false,
        &Scope::default(),
    )
    .await
    .expect("GET fallback should succeed");
}

/// Redirects are followed to their final destination.
#[tokio::test]
async fn link_check_execute_follows_redirects() {
    let server = TestHttpServer::spawn_with_responses(vec![
        status_response("302 Found", &["Location: /final"]),
        ok_response(),
    ]);
    let temp = tempdir().expect("create temp dir");
    let markdown = format!("[moved]({})\n", server.url("/old"));
    write_markdown_file(temp.path(), "doc.md", &markdown);

    book_tool::link_check::execute(
        vec![temp.path().to_path_buf()],
        false,
        false,
        &Scope::default(),
    )
    .await
    .expect("redirected link should verify");
}

/// Link check execute fails after max retry exhaustion.
#[tokio::test]
async fn link_check_execute_fails_after_max_retry_exhaustion() {
    let server = TestHttpServer::spawn_with_responses(vec![retry_after_response(0); 8]);
    let temp = tempdir().expect("create temp dir");
    let markdown = format!("[retry me]({})\n", server.url("/always-rate-limited"));
    write_markdown_file(temp.path(), "doc.md", &markdown);

    let error = book_tool::link_check::execute(
        vec![temp.path().to_path_buf()],
        false,
        false,
        &Scope::default(),
    )
    .await
    .expect_err("link-check should fail after exhausting retries");

    assert!(
        error.to_string().contains("broken links"),
        "link-check error was: {error}"
    );
}

/// Link check cli rejects delete without refdefs.
#[test]
fn link_check_cli_rejects_delete_without_refdefs() {
    let temp = tempdir().expect("create temp dir");
    write_markdown_file(temp.path(), "doc.md", "[ok](https://example.com)\n");

    let output = book_tool_command()
        .args([
            "link-check",
            temp.path().to_str().expect("utf-8 temp path"),
            "--delete",
        ])
        .output()
        .expect("run link-check command");

    assert!(
        !output.status.success(),
        "link-check unexpectedly succeeded: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(stderr.contains("refdefs"), "link-check stderr: {stderr}");
}

/// Directives cli fails for invalid config.
#[test]
fn directives_cli_fails_for_invalid_config() {
    let temp = tempdir().expect("create temp dir");
    write_markdown_file(temp.path(), "doc.md", "{{#example demo}}\n");
    fs::write(
        temp.path().join("config.toml"),
        "process-example-directives = [\n",
    )
    .expect("write invalid config");

    let output = book_tool_command()
        .args(["directives", temp.path().to_str().expect("utf-8 temp path")])
        .current_dir(temp.path())
        .output()
        .expect("run directives command with invalid config");

    assert!(
        !output.status.success(),
        "directives unexpectedly succeeded: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(stderr.contains("config"), "directives stderr: {stderr}");
}

/// Inline links reports failed input and leaves it unchanged.
#[test]
fn inline_links_reports_failed_input_and_leaves_it_unchanged() {
    let temp = tempdir().expect("create temp dir");
    let valid_file = write_markdown_file(
        temp.path(),
        "valid.md",
        "See [Rust](https://www.rust-lang.org/).\n",
    );
    let invalid_file = temp.path().join("invalid.md");
    fs::write(&invalid_file, [0xff, 0xfe, 0x00]).expect("write invalid UTF-8 Markdown");

    let error = book_tool::inline_links::execute(&[temp.path().to_path_buf()], &Scope::default())
        .expect_err("incomplete inline-link processing must fail");
    assert!(error.to_string().contains("incomplete"));

    let updated_valid = fs::read_to_string(valid_file).expect("read transformed markdown");
    assert!(
        updated_valid.contains("[Rust][1]"),
        "updated content was:\n{updated_valid}"
    );

    assert_eq!(
        fs::read(invalid_file).expect("read invalid Markdown"),
        [0xff, 0xfe, 0x00]
    );
}

/// Directives cli reports mixed corpus failures and preserves bad input.
#[test]
fn directives_cli_reports_mixed_corpus_failures_and_preserves_bad_input() {
    let temp = tempdir().expect("create temp dir");
    let valid = write_markdown_file(temp.path(), "valid.md", "{{#example demo}}\n");
    let invalid = temp.path().join("invalid.md");
    let invalid_source = [0xff, 0xfe, 0x00];
    fs::write(&invalid, invalid_source).expect("write invalid UTF-8 Markdown");

    let output = book_tool_command()
        .args(["directives", temp.path().to_str().expect("utf-8 temp path")])
        .current_dir(temp.path())
        .output()
        .expect("run directives command");

    assert!(
        !output.status.success(),
        "directives command should fail on an incomplete corpus"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid.md"), "stderr: {stderr}");
    assert!(
        stderr.contains("scanned 2, processed 1, failed 1"),
        "stderr: {stderr}"
    );
    assert_eq!(
        fs::read(invalid).expect("read invalid Markdown"),
        invalid_source
    );
    assert_eq!(
        fs::read_to_string(valid).expect("read transformed Markdown"),
        "{{#include examples/demo.md}}\n"
    );
}

/// Search cli reports plain text matches.
#[test]
fn search_cli_reports_plain_text_matches() {
    let temp = tempdir().expect("create temp dir");
    let file = write_markdown_file(
        temp.path(),
        "search.md",
        "# Title\n\nThis paragraph hides a **special needle** in markdown.\n",
    );

    let output = book_tool_command()
        .args([
            "search",
            "special needle",
            temp.path().to_str().expect("utf-8 temp path"),
        ])
        .output()
        .expect("run search command");

    assert!(
        output.status.success(),
        "search stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(&file.display().to_string()),
        "search stdout: {stdout}"
    );
    assert!(
        stdout.to_lowercase().contains("special needle"),
        "search stdout: {stdout}"
    );
}

/// Extract comments cli writes output file.
#[test]
fn extract_comments_cli_writes_output_file() {
    let temp = tempdir().expect("create temp dir");
    let source = write_markdown_file(
        temp.path(),
        "comments.md",
        "Intro\n<!-- keep this comment -->\n\nBody\n",
    );
    let output_file = temp.path().join("comments.out.md");

    let output = book_tool_command()
        .args([
            "extract-comments",
            temp.path().to_str().expect("utf-8 temp path"),
            "--output",
            output_file.to_str().expect("utf-8 output path"),
        ])
        .output()
        .expect("run extract-comments command");

    assert!(
        output.status.success(),
        "extract-comments stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let extracted = fs::read_to_string(output_file).expect("read extracted comments");
    assert!(
        extracted.contains(&source.display().to_string()),
        "extract-comments output: {extracted}"
    );
    assert!(
        extracted.contains("<!-- keep this comment -->"),
        "extract-comments output: {extracted}"
    );
}

/// Refdefs audit reports uses before idempotent cleanup.
#[test]
fn refdefs_audit_reports_uses_before_idempotent_cleanup() {
    let temp = tempdir().expect("create temp dir");
    let chapter = write_markdown_file(
        temp.path(),
        "chapter.md",
        "{{#include refs.incl.md}}\n[Full][SERDE]\n[ SeRdE ][]\n[SERDE]\n![crate][serDe]\n![SeRdE][]\n![serde]\n[![crate][sErDe]][SERDE]\n`[serde]` <!-- [serde] -->\n```md\n[serde]\n```\n[serde](https://example.com)\n",
    );
    let definitions = write_markdown_file(
        temp.path(),
        "refs.incl.md",
        "[serde]: https://docs.rs/serde\n[unused]: https://example.com/unused\n",
    );
    let chapter_before = fs::read(&chapter).expect("read original chapter");
    let report = temp.path().join("reference-audit.json");
    let root = temp.path().to_str().expect("utf-8 source path");
    let report_path = report.to_str().expect("utf-8 report path");

    let first = book_tool_command()
        .args(["refdefs-audit", root, "--report", report_path, "--delete"])
        .output()
        .expect("run reference audit and cleanup");
    assert!(
        first.status.success(),
        "refdefs-audit stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );

    let first_report: serde_json::Value =
        serde_json::from_slice(&fs::read(&report).expect("read pre-cleanup report"))
            .expect("parse pre-cleanup report");
    let records = first_report.as_array().expect("report is a JSON array");
    let serde_record = records
        .iter()
        .find(|record| record["label"] == "serde")
        .expect("report contains the shared definition");
    assert_eq!(serde_record["proven_unused"], false);
    assert_eq!(
        serde_record["uses"].as_array().expect("uses array").len(),
        8
    );
    assert!(
        serde_record["uses"]
            .as_array()
            .expect("uses array")
            .iter()
            .any(|usage| usage["kind"] == "image")
    );
    assert!(
        serde_record["uses_outside_scoped_roots"]
            .as_array()
            .expect("outside-scope use array")
            .is_empty()
    );
    let unused_record = records
        .iter()
        .find(|record| record["label"] == "unused")
        .expect("report contains the unused definition");
    assert_eq!(unused_record["proven_unused"], true);
    assert!(
        unused_record["uses"]
            .as_array()
            .expect("uses array")
            .is_empty()
    );

    let cleaned = fs::read_to_string(&definitions).expect("read cleaned definitions");
    assert!(cleaned.contains("[serde]: https://docs.rs/serde"));
    assert!(!cleaned.contains("[unused]:"));
    assert_eq!(
        fs::read(&chapter).expect("read chapter after cleanup"),
        chapter_before
    );

    let second = book_tool_command()
        .args(["refdefs-audit", root, "--report", report_path, "--delete"])
        .output()
        .expect("run idempotent audit");
    assert!(
        second.status.success(),
        "second refdefs-audit stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    let second_report = fs::read(&report).expect("read idempotent report");
    let second_report_json: serde_json::Value =
        serde_json::from_slice(&second_report).expect("parse idempotent report");
    assert!(
        second_report_json
            .as_array()
            .expect("report is a JSON array")
            .iter()
            .all(|record| record["proven_unused"] == false)
    );

    let third = book_tool_command()
        .args(["refdefs-audit", root, "--report", report_path, "--delete"])
        .output()
        .expect("repeat idempotent audit");
    assert!(
        third.status.success(),
        "third refdefs-audit stderr: {}",
        String::from_utf8_lossy(&third.stderr)
    );
    assert_eq!(
        fs::read(&report).expect("read stable second-run report"),
        second_report
    );
}

/// Refdefs audit preserves definitions used in additional sources.
#[test]
fn refdefs_audit_preserves_definitions_used_in_additional_sources() {
    let temp = tempdir().expect("create temp dir");
    let published = temp.path().join("published");
    let drafts = temp.path().join("drafts");
    fs::create_dir_all(&published).expect("create published source root");
    fs::create_dir_all(&drafts).expect("create draft source root");
    let definitions = write_markdown_file(
        &published,
        "refs.incl.md",
        "[serde]: https://docs.rs/serde\n[unused]: https://example.com/unused\n",
    );
    let generated = write_markdown_file(
        &published,
        "generated-refs.md",
        "[reserved]: https://example.com/generated\n",
    );
    write_markdown_file(
        &published,
        "chapter.md",
        "{{#include refs.incl.md}}\n{{#include generated-refs.md}}\n",
    );
    write_markdown_file(
        &drafts,
        "draft.md",
        "[Issue note with nested [serde][SERDE] reference](https://example.com)\n",
    );
    let report = temp.path().join("reference-audit.json");

    let output = book_tool_command()
        .args([
            "refdefs-audit",
            published.to_str().expect("utf-8 published path"),
            "--report",
            report.to_str().expect("utf-8 report path"),
            "--also-scan-uses-in",
            drafts.to_str().expect("utf-8 drafts path"),
            "--preserve-definitions-in",
            generated
                .to_str()
                .expect("utf-8 generated definitions path"),
            "--delete",
        ])
        .output()
        .expect("run reference audit with additional source root");
    assert!(
        output.status.success(),
        "refdefs-audit stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(report).expect("read audit report"))
            .expect("parse audit report");
    let records = report.as_array().expect("report is a JSON array");
    let serde_record = records
        .iter()
        .find(|record| record["label"] == "serde")
        .expect("report contains serde definition");
    assert_eq!(serde_record["proven_unused"], false);
    let outside_uses = serde_record["uses_outside_scoped_roots"]
        .as_array()
        .expect("outside-scope use array");
    assert_eq!(outside_uses.len(), 1);
    assert_eq!(outside_uses[0]["line"], 1_u64);
    assert!(
        outside_uses[0]["path"]
            .as_str()
            .expect("input should parse")
            .ends_with("draft.md")
    );

    let unused_record = records
        .iter()
        .find(|record| record["label"] == "unused")
        .expect("report contains unused definition");
    assert_eq!(unused_record["proven_unused"], true);
    let reserved_record = records
        .iter()
        .find(|record| record["label"] == "reserved")
        .expect("report contains protected definition");
    assert_eq!(reserved_record["protected"], true);
    assert_eq!(reserved_record["proven_unused"], false);
    let cleaned = fs::read_to_string(definitions).expect("read cleaned definitions");
    assert!(cleaned.contains("[serde]: https://docs.rs/serde"));
    assert!(!cleaned.contains("[unused]:"));
    assert_eq!(
        fs::read_to_string(generated).expect("read protected generated definitions"),
        "[reserved]: https://example.com/generated\n"
    );
}

/// Verifies the link-check executable resolves included references and prunes only unused labels.
#[test]
fn link_check_cli_checks_local_urls_and_cleans_shared_references_deterministically() {
    let server = TestHttpServer::spawn();
    let temp = tempdir().expect("create temporary book directory");
    let chapter = write_markdown_file(
        temp.path(),
        "chapter.md",
        &format!(
            "{{{{#include refs.incl.md}}}}\n[Book source][ SeRdE ]\n[Direct]({})\n",
            server.url("/book")
        ),
    );
    let references = write_markdown_file(
        temp.path(),
        "refs.incl.md",
        &format!(
            "[serde]: {}\n[unused]: http://127.0.0.1:9/unused\n",
            server.url("/book")
        ),
    );

    let output = book_tool_command()
        .args([
            "link-check",
            temp.path().to_str().expect("UTF-8 temp path"),
            "--refdefs",
            "--delete",
        ])
        .env("RUST_LOG", "off")
        .output()
        .expect("run link-check with a local HTTP fixture");

    assert!(
        output.status.success(),
        "link-check stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout
            .matches("Found 1 unique external links. Verifying...")
            .count(),
        1,
        "link-check stdout: {stdout}"
    );
    let canonical_references = references
        .canonicalize()
        .expect("canonicalize shared references");
    assert!(
        stdout.contains(&format!(
            "Unused reference definitions in {}: unused",
            canonical_references.display()
        )),
        "link-check stdout: {stdout}"
    );
    assert!(stdout.ends_with("All links verified successfully.\n"));
    assert_eq!(
        fs::read_to_string(&chapter).expect("read chapter after cleanup"),
        format!(
            "{{{{#include refs.incl.md}}}}\n[Book source][ SeRdE ]\n[Direct]({})\n",
            server.url("/book")
        )
    );
    let cleaned = fs::read_to_string(references).expect("read shared definitions after cleanup");
    assert!(cleaned.contains("[serde]:"));
    assert!(!cleaned.contains("[unused]:"));
}

/// Verifies broken local HTTP links produce a nonzero executable status without source edits.
#[test]
fn link_check_cli_returns_failure_for_broken_urls_and_keeps_the_chapter_unchanged() {
    let not_found = status_response("404 Not Found", &[]);
    let server = TestHttpServer::spawn_with_responses(vec![not_found.clone(), not_found]);
    let temp = tempdir().expect("create temporary book directory");
    let source = format!("[Missing]({})\n", server.url("/missing"));
    let chapter = write_markdown_file(temp.path(), "chapter.md", &source);

    let output = book_tool_command()
        .args(["link-check", temp.path().to_str().expect("UTF-8 temp path")])
        .env("RUST_LOG", "off")
        .output()
        .expect("run link-check against a 404 fixture");

    assert!(
        !output.status.success(),
        "link-check unexpectedly succeeded"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Found 1 unique external links. Verifying...\n"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Found 1 broken links."),
        "link-check stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let broken_line = format!("BROKEN: {} (HTTP 404 Not Found)", server.url("/missing"));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(&broken_line),
        "link-check stderr should name the URL and status: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(chapter).expect("read unchanged chapter"),
        source
    );
}
