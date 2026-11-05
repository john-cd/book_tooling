//! Exit codes and diagnostics of `book_tool` for missing inputs, unreadable files, and bad
//! arguments.
//!
//! TODO Reviewed ??-??-2026

#![expect(
    clippy::expect_used,
    reason = "Helpers in this test file fail loudly when test setup breaks."
)]

use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Output;

use tempfile::tempdir;

/// Runs `book_tool` with `args` and returns its output.
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_book_tool"))
        .args(args)
        .output()
        .expect("run book_tool")
}

/// Converts a path to UTF-8 text for use as an argument.
fn text(path: &Path) -> &str {
    path.to_str().expect("UTF-8 temp path")
}

/// Returns stderr as text.
fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Every directory-based subcommand fails with a non-zero status for a missing directory.
#[test]
fn missing_directories_fail_with_a_diagnostic_naming_the_path() {
    let temp = tempdir().expect("create temporary directory");
    let missing = temp.path().join("no-such-dir");
    let missing = text(&missing);
    let out = temp.path().join("out.txt");
    let report = temp.path().join("report.json");
    let cases: Vec<Vec<&str>> = vec![
        vec!["directives", missing],
        vec!["inline-links", missing],
        vec!["search", "needle", missing],
        vec!["extract-comments", missing, "--output", text(&out)],
        vec!["refdefs-audit", missing, "--report", text(&report)],
        vec!["link-check", missing],
    ];
    for args in cases {
        let output = run(&args);
        assert!(
            !output.status.success(),
            "{args:?} must fail, stdout: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(
            stderr(&output).contains("no-such-dir"),
            "{args:?} stderr must name the directory: {}",
            stderr(&output)
        );
    }
}

/// A Markdown "file" that cannot be read as text does not crash the process.
#[test]
fn unreadable_markdown_input_is_reported_and_left_unchanged() {
    let temp = tempdir().expect("create temporary directory");
    let invalid = temp.path().join("binary.md");
    let bytes = [0xff_u8, 0xfe, 0x00, 0x80];
    fs::write(&invalid, bytes).expect("write invalid UTF-8");

    let output = run(&["directives", text(temp.path())]);

    assert!(
        !output.status.success(),
        "invalid UTF-8 must fail, stderr: {}",
        stderr(&output)
    );
    assert!(stderr(&output).contains("binary.md"), "{}", stderr(&output));
    assert_eq!(fs::read(&invalid).expect("read input"), bytes);
}

/// A directory with a Markdown extension is not treated as a file.
#[test]
fn directory_named_like_a_markdown_file_is_ignored() {
    let temp = tempdir().expect("create temporary directory");
    fs::create_dir_all(temp.path().join("folder.md")).expect("create directory");
    fs::write(temp.path().join("real.md"), "a needle\n").expect("write file");

    let output = run(&["search", "needle", text(temp.path())]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("real.md"),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

/// The report destination of `refdefs-audit` must be writable.
#[test]
fn unwritable_report_destination_fails() {
    let temp = tempdir().expect("create temporary directory");
    fs::write(temp.path().join("chapter.md"), "text\n").expect("write chapter");
    let report = temp.path().join("missing-dir").join("report.json");

    let output = run(&[
        "refdefs-audit",
        text(temp.path()),
        "--report",
        text(&report),
    ]);

    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("report.json"),
        "{}",
        stderr(&output)
    );
}

/// Malformed command lines exit with clap's usage status and print usage help.
#[test]
fn malformed_arguments_exit_with_usage_errors() {
    let temp = tempdir().expect("create temporary directory");
    let dir = text(temp.path());
    let cases: Vec<Vec<&str>> = vec![
        vec![],
        vec!["no-such-command"],
        vec!["directives"],
        vec!["search"],
        vec!["search", "needle"],
        vec!["refdefs-audit", dir],
        vec!["extract-comments", dir],
        vec!["link-check", dir, "--delete"],
        vec!["inline-links", dir, "--no-such-flag"],
    ];
    for args in cases {
        let output = run(&args);
        assert_eq!(
            output.status.code(),
            Some(2_i32),
            "{args:?} must be a usage error, stderr: {}",
            stderr(&output)
        );
        assert!(
            stderr(&output).contains("Usage") || stderr(&output).contains("usage"),
            "{args:?} stderr must include usage: {}",
            stderr(&output)
        );
    }
}

/// `--help` succeeds and lists every subcommand.
#[test]
fn help_lists_all_subcommands() {
    let output = run(&["--help"]);
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    for command in [
        "directives",
        "link-check",
        "refdefs-audit",
        "inline-links",
        "search",
        "extract-comments",
    ] {
        assert!(help.contains(command), "help is missing {command}: {help}");
    }
}
