//! Tests for configuring the `directives` subcommand through a TOML file.
//!
//! TODO Reviewed ??-??-2026.

use std::fs;
use std::process::Command;

use tempfile::tempdir;

/// Checks that configuration controls expansion families and which file extensions are processed.
#[test]
fn directives_cli_uses_configured_features_and_file_extensions() {
    let temp = tempdir().expect("create temporary book directory");
    let chapter = temp.path().join("chapter.md");
    let skipped = temp.path().join("draft.markdown");
    let excluded = temp.path().join("refs.incl.md");
    let source = "{{docs serde}} {{#example demo}} {{#include refs.incl.md}}\n";
    fs::write(&chapter, source).expect("write chapter");
    fs::write(&skipped, "{{docs skipped}}\n").expect("write non-configured extension");
    fs::write(&excluded, "{{docs excluded}}\n").expect("write excluded include file");
    let config = temp.path().join("config.toml");
    fs::write(
        &config,
        concat!(
            "process-crate-block-directives = false\n",
            "process-example-directives = true\n",
            "process-link-and-badge-directives = true\n",
            "process-wikilinks = false\n",
            "included-extensions = [\"md\"]\n",
            "excluded-filenames = [\"refs.incl.md\", \"SUMMARY.md\"]\n"
        ),
    )
    .expect("write directive configuration");

    let enabled = Command::new(env!("CARGO_BIN_EXE_book_tool"))
        .args(["directives", temp.path().to_str().expect("UTF-8 temp path")])
        .current_dir(temp.path())
        .output()
        .expect("run directives CLI with configuration");
    assert!(
        enabled.status.success(),
        "directives stderr: {}",
        String::from_utf8_lossy(&enabled.stderr)
    );
    assert_eq!(
        fs::read_to_string(&chapter).expect("read configured chapter"),
        "[serde][c~serde~docs]\u{2197}{{hi:serde}} {{#include examples/demo.md}} {{#include refs.incl.md}}\n"
    );
    assert_eq!(
        fs::read_to_string(skipped).expect("read skipped extension"),
        "{{docs skipped}}\n"
    );
    assert_eq!(
        fs::read_to_string(excluded).expect("read excluded include file"),
        "{{docs excluded}}\n"
    );

    fs::write(&chapter, source).expect("restore chapter before disabled run");
    fs::write(
        &config,
        concat!(
            "process-crate-block-directives = false\n",
            "process-example-directives = false\n",
            "process-link-and-badge-directives = false\n",
            "process-wikilinks = false\n",
            "included-extensions = [\"md\"]\n",
            "excluded-filenames = [\"refs.incl.md\", \"SUMMARY.md\"]\n"
        ),
    )
    .expect("write disabled directive configuration");
    let disabled = Command::new(env!("CARGO_BIN_EXE_book_tool"))
        .args(["directives", temp.path().to_str().expect("UTF-8 temp path")])
        .current_dir(temp.path())
        .output()
        .expect("run directives CLI with processing disabled");
    assert!(
        disabled.status.success(),
        "directives stderr: {}",
        String::from_utf8_lossy(&disabled.stderr)
    );
    assert_eq!(
        fs::read_to_string(chapter).expect("read disabled chapter"),
        source
    );
}

/// Verifies malformed TOML fails before the CLI modifies source files.
#[test]
fn directives_cli_reports_invalid_config_without_rewriting_chapters() {
    let temp = tempdir().expect("create temporary book directory");
    let chapter = temp.path().join("chapter.md");
    let source = "{{docs serde}}\n";
    fs::write(&chapter, source).expect("write chapter");
    fs::write(
        temp.path().join("config.toml"),
        "process-example-directives = [\n",
    )
    .expect("write malformed configuration");

    let output = Command::new(env!("CARGO_BIN_EXE_book_tool"))
        .args(["directives", temp.path().to_str().expect("UTF-8 temp path")])
        .current_dir(temp.path())
        .output()
        .expect("run directives CLI with malformed configuration");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("config.toml"),
        "directives stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(chapter).expect("read unchanged chapter"),
        source
    );
}

/// Verifies `--config` selects a configuration file outside the working directory.
#[test]
fn directives_cli_reads_the_configuration_named_by_the_config_flag() {
    let book = tempdir().expect("create temporary book directory");
    let elsewhere = tempdir().expect("create temporary configuration directory");
    let chapter = book.path().join("chapter.md");
    fs::write(&chapter, "{{docs serde}}\n").expect("write chapter");
    let config = elsewhere.path().join("custom.toml");
    fs::write(&config, "process-link-and-badge-directives = false\n")
        .expect("write directive configuration");

    let output = Command::new(env!("CARGO_BIN_EXE_book_tool"))
        .args(["directives", "--config"])
        .arg(&config)
        .arg(book.path())
        .current_dir(elsewhere.path())
        .output()
        .expect("run directives CLI with --config");

    assert!(
        output.status.success(),
        "directives stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(chapter).expect("read chapter"),
        "{{docs serde}}\n"
    );

    let missing = Command::new(env!("CARGO_BIN_EXE_book_tool"))
        .args(["directives", "--config"])
        .arg(elsewhere.path().join("missing.toml"))
        .arg(book.path())
        .output()
        .expect("run directives CLI with a missing --config");
    assert!(
        !missing.status.success(),
        "a missing explicit configuration must be an error"
    );
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("missing.toml"),
        "stderr: {}",
        String::from_utf8_lossy(&missing.stderr)
    );
}
