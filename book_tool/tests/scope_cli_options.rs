//! Tests of the `--extension` and `--exclude` options shared by file-oriented subcommands.
//!
//! TODO Reviewed ??-??-2026

#![expect(
    clippy::expect_used,
    reason = "Helpers in this test file fail loudly when test setup breaks."
)]

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::tempdir;

/// Runs `book_tool search needle <dir> <extra...>` and returns its stdout.
fn search(dir: &Path, extra: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_book_tool"))
        .args(["search", "needle", dir.to_str().expect("UTF-8 temp path")])
        .args(extra)
        .output()
        .expect("run search CLI");
    assert!(
        output.status.success(),
        "search stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("UTF-8 stdout")
}

/// Creates `a.md`, `c.md` and `b.txt`, each containing the word `needle`.
fn book_with_mixed_files() -> tempfile::TempDir {
    let temp = tempdir().expect("create temporary book directory");
    for name in ["a.md", "c.md", "b.txt"] {
        fs::write(temp.path().join(name), "a needle in text\n").expect("write file");
    }
    temp
}

/// Without options only Markdown files are searched.
#[test]
fn default_scope_searches_markdown_only() {
    let temp = book_with_mixed_files();
    let stdout = search(temp.path(), &[]);
    assert!(
        stdout.contains("a.md") && stdout.contains("c.md"),
        "{stdout}"
    );
    assert!(!stdout.contains("b.txt"), "{stdout}");
}

/// `--extension` replaces the default extension list.
#[test]
fn extension_option_selects_other_file_types() {
    let temp = book_with_mixed_files();
    let stdout = search(temp.path(), &["--extension", "txt"]);
    assert!(stdout.contains("b.txt"), "{stdout}");
    assert!(!stdout.contains("a.md"), "{stdout}");
}

/// `--exclude` skips the named file and keeps the default exclusions.
#[test]
fn exclude_option_skips_named_files() {
    let temp = book_with_mixed_files();
    let stdout = search(temp.path(), &["--exclude", "c.md"]);
    assert!(stdout.contains("a.md"), "{stdout}");
    assert!(!stdout.contains("c.md"), "{stdout}");
}
