use std::collections::VecDeque;
use std::fs;
use std::io::Read;
use std::io::Write;
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
                        let _ = respond(&mut stream, &response);
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
    "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
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
        let _ = TcpStream::connect(self.address);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn write_markdown_file(root: &Path, name: &str, content: &str) -> std::path::PathBuf {
    let path = root.join(name);
    fs::write(&path, content).expect("write markdown file");
    path
}

#[test]
fn inline_links_execute_converts_links_to_reference_definitions() {
    let temp = tempdir().expect("create temp dir");
    let markdown = "See [Rust](https://www.rust-lang.org/).\n";
    let file = write_markdown_file(temp.path(), "doc.md", markdown);

    book_tool::inline_links::execute(vec![temp.path().to_path_buf()]).expect("run inline-links");

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

#[test]
fn directives_execute_expands_example_directives() {
    let temp = tempdir().expect("create temp dir");
    let file = write_markdown_file(temp.path(), "doc.md", "{{#example demo}}\n");

    book_tool::directives::execute(vec![temp.path().to_path_buf()]).expect("run directives");

    let updated = fs::read_to_string(file).expect("read transformed markdown");
    assert_eq!(updated, "{{#include examples/demo.md}}\n");
}

#[tokio::test]
async fn link_check_execute_skips_deleted_unused_reference_definitions() {
    let server = TestHttpServer::spawn();
    let temp = tempdir().expect("create temp dir");
    let markdown = format!(
        "[Used][used]\n\n[used]: {}\n[unused]: http://127.0.0.1:9/unused\n",
        server.url("/ok")
    );
    let file = write_markdown_file(temp.path(), "doc.md", &markdown);

    book_tool::link_check::execute(vec![temp.path().to_path_buf()], true, true)
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

#[tokio::test]
async fn link_check_execute_retries_retry_after_responses() {
    let server = TestHttpServer::spawn_with_responses(vec![retry_after_response(0), ok_response()]);
    let temp = tempdir().expect("create temp dir");
    let markdown = format!("[retry me]({})\n", server.url("/rate-limited"));
    write_markdown_file(temp.path(), "doc.md", &markdown);

    book_tool::link_check::execute(vec![temp.path().to_path_buf()], false, false)
        .await
        .expect("run link-check with retry");
}

#[tokio::test]
async fn link_check_execute_retries_service_unavailable_without_retry_after() {
    let server = TestHttpServer::spawn_with_responses(vec![
        service_unavailable_response_with_headers(&[]),
        ok_response(),
    ]);
    let temp = tempdir().expect("create temp dir");
    let markdown = format!("[retry me]({})\n", server.url("/service-unavailable"));
    write_markdown_file(temp.path(), "doc.md", &markdown);

    book_tool::link_check::execute(vec![temp.path().to_path_buf()], false, false)
        .await
        .expect("run link-check with missing retry-after");
}

#[tokio::test]
async fn link_check_execute_retries_invalid_retry_after_responses() {
    let server = TestHttpServer::spawn_with_responses(vec![
        invalid_retry_after_response("soon"),
        ok_response(),
    ]);
    let temp = tempdir().expect("create temp dir");
    let markdown = format!("[retry me]({})\n", server.url("/invalid-retry-after"));
    write_markdown_file(temp.path(), "doc.md", &markdown);

    book_tool::link_check::execute(vec![temp.path().to_path_buf()], false, false)
        .await
        .expect("run link-check with invalid retry-after");
}

#[tokio::test]
async fn link_check_execute_fails_after_max_retry_exhaustion() {
    let server = TestHttpServer::spawn_with_responses(vec![retry_after_response(0); 8]);
    let temp = tempdir().expect("create temp dir");
    let markdown = format!("[retry me]({})\n", server.url("/always-rate-limited"));
    write_markdown_file(temp.path(), "doc.md", &markdown);

    let error = book_tool::link_check::execute(vec![temp.path().to_path_buf()], false, false)
        .await
        .expect_err("link-check should fail after exhausting retries");

    assert!(
        error.to_string().contains("broken links"),
        "link-check error was: {error}"
    );
}

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

#[test]
fn inline_links_execute_continues_after_parse_failure() {
    let temp = tempdir().expect("create temp dir");
    let valid_file = write_markdown_file(
        temp.path(),
        "valid.md",
        "See [Rust](https://www.rust-lang.org/).\n",
    );
    let invalid_file = write_markdown_file(
        temp.path(),
        "invalid.md",
        "<div class=\"hidden\">broken\n\nSee [Rust](https://www.rust-lang.org/).\n",
    );

    book_tool::inline_links::execute(vec![temp.path().to_path_buf()])
        .expect("run inline-links with parse failure present");

    let updated_valid = fs::read_to_string(valid_file).expect("read transformed markdown");
    assert!(
        updated_valid.contains("[Rust][1]"),
        "updated content was:\n{updated_valid}"
    );

    let unchanged_invalid = fs::read_to_string(invalid_file).expect("read invalid markdown");
    assert!(
        unchanged_invalid.contains("<div class=\"hidden\">broken"),
        "invalid content was unexpectedly rewritten:\n{unchanged_invalid}"
    );
}

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
