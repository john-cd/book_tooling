use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[test]
fn test_inline_links_cli() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.md");
    fs::write(&file_path, "# Test\n\n[Rust](https://rust-lang.org)\n").unwrap();

    let status = Command::new("cargo")
        .args([
            "run",
            "-p",
            "inline_links",
            "--",
            dir.path().to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run inline_links");

    assert!(status.success());

    let content = fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("[Rust][Rust]"));
    assert!(content.contains("[Rust]: https://rust-lang.org"));
}

#[test]
fn test_link_checker_cli() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.md");
    fs::write(
        &file_path,
        "# Test\n\n[used]: https://rust-lang.org\n[unused]: https://example.com\n[link][used]\n",
    )
    .unwrap();

    // link_checker deletes unused refdefs by default
    let status = Command::new("cargo")
        .args([
            "run",
            "-p",
            "external_link_checker",
            "--",
            dir.path().to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run link_checker");

    assert!(status.success());

    let content = fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("[used]: https://rust-lang.org"));
}

#[test]
fn test_process_directives_cli() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.md");
    fs::write(&file_path, "[[page|display]]\n").unwrap();

    let status = Command::new("cargo")
        .args([
            "run",
            "-p",
            "process_directives",
            "--",
            dir.path().to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run process_directives");

    assert!(status.success());

    let content = fs::read_to_string(&file_path).unwrap();
    // According to wikilinks.rs: replacement = "[display][p~page]{hi:page}"
    assert!(content.contains("[display][p~page]{hi:page}"));
}
