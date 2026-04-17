use std::fs;
use std::io::Read;
use std::io::Write;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::Path;
use std::sync::Arc;
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
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        listener
            .set_nonblocking(true)
            .expect("set listener nonblocking");

        let address = listener.local_addr().expect("get local address");
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_signal = Arc::clone(&shutdown);

        let handle = thread::spawn(move || {
            while !shutdown_signal.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let _ = respond_ok(&mut stream);
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

impl Drop for TestHttpServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(self.address);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn respond_ok(stream: &mut TcpStream) -> std::io::Result<()> {
    let mut buffer = [0_u8; 1024];
    let _ = stream.read(&mut buffer)?;
    stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
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
