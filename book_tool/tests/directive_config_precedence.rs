//! Precedence of directive configuration sources: `--config`, `./config.toml`, then defaults.
//!
//! The `directives` subcommand has no per-directive override flags; the configuration file is the
//! only override, and an explicit `--config` always wins over the working directory's file.
//!
//! TODO Reviewed ??-??-2026.

#![expect(
    clippy::expect_used,
    reason = "Helpers in this test file fail loudly when test setup breaks."
)]

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::tempdir;

/// Chapter text with one link directive and one wikilink-free example directive.
const SOURCE: &str = "{{docs serde}} {{#example demo}}\n";

/// Runs `book_tool directives` over `book` from `cwd`, adding `config` when given.
fn run_directives(book: &Path, cwd: &Path, config: Option<&Path>) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_book_tool"));
    command.arg("directives");
    if let Some(config) = config {
        command.arg("--config").arg(config);
    }
    let output = command
        .arg(book)
        .current_dir(cwd)
        .output()
        .expect("run directives CLI");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Without any configuration file every directive family is enabled.
#[test]
fn defaults_apply_without_a_configuration_file() {
    let book = tempdir().expect("create book directory");
    let cwd = tempdir().expect("create working directory");
    let chapter = book.path().join("chapter.md");
    fs::write(&chapter, SOURCE).expect("write chapter");

    run_directives(book.path(), cwd.path(), None);

    let rendered = fs::read_to_string(chapter).expect("read chapter");
    assert!(rendered.contains("[serde][c~serde~docs]"), "{rendered}");
    assert!(
        rendered.contains("{{#include examples/demo.md}}"),
        "{rendered}"
    );
}

/// `config.toml` in the working directory is used when `--config` is absent.
#[test]
fn working_directory_config_applies_when_no_flag_is_given() {
    let book = tempdir().expect("create book directory");
    let cwd = tempdir().expect("create working directory");
    let chapter = book.path().join("chapter.md");
    fs::write(&chapter, SOURCE).expect("write chapter");
    fs::write(
        cwd.path().join("config.toml"),
        "process-link-and-badge-directives = false\n",
    )
    .expect("write config");

    run_directives(book.path(), cwd.path(), None);

    let rendered = fs::read_to_string(chapter).expect("read chapter");
    assert!(rendered.contains("{{docs serde}}"), "{rendered}");
    assert!(
        rendered.contains("{{#include examples/demo.md}}"),
        "{rendered}"
    );
}

/// An explicit `--config` takes precedence over the working directory's `config.toml`.
#[test]
fn explicit_config_overrides_the_working_directory_config() {
    let book = tempdir().expect("create book directory");
    let cwd = tempdir().expect("create working directory");
    let chapter = book.path().join("chapter.md");
    fs::write(&chapter, SOURCE).expect("write chapter");
    fs::write(
        cwd.path().join("config.toml"),
        "process-link-and-badge-directives = false\nprocess-example-directives = false\n",
    )
    .expect("write working-directory config");
    let explicit = cwd.path().join("explicit.toml");
    fs::write(&explicit, "process-example-directives = true\n").expect("write explicit config");

    run_directives(book.path(), cwd.path(), Some(&explicit));

    // The explicit file starts from defaults, so nothing from ./config.toml leaks through.
    let rendered = fs::read_to_string(chapter).expect("read chapter");
    assert!(rendered.contains("[serde][c~serde~docs]"), "{rendered}");
    assert!(
        rendered.contains("{{#include examples/demo.md}}"),
        "{rendered}"
    );
}

/// Unknown keys in the configuration are ignored rather than rejected.
#[test]
fn unknown_configuration_keys_are_ignored() {
    let book = tempdir().expect("create book directory");
    let cwd = tempdir().expect("create working directory");
    let chapter = book.path().join("chapter.md");
    fs::write(&chapter, SOURCE).expect("write chapter");
    fs::write(
        cwd.path().join("config.toml"),
        "future-option = true\nprocess-example-directives = false\n",
    )
    .expect("write config");

    run_directives(book.path(), cwd.path(), None);

    let rendered = fs::read_to_string(chapter).expect("read chapter");
    assert!(rendered.contains("{{#example demo}}"), "{rendered}");
}
