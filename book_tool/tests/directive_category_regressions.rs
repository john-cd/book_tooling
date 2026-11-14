//! Regression tests for category directives processed through the `book_tool` CLI.
//!
//! TODO Reviewed ??-??-2026.

use std::fs;
use std::process::Command;

use tempfile::tempdir;

/// Creates a command for the compiled `book_tool` binary.
fn book_tool_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_book_tool"))
}

/// Checks that category expansion preserves encoded slugs and leaves disabled directives intact.
#[test]
fn category_cli_renders_encoded_punctuation_and_preserves_disabled_directives() {
    let temp = tempdir().expect("create temporary book directory");
    let chapter = temp.path().join("chapter.md");
    let config = temp.path().join("config.toml");
    let source = concat!(
        "{{cat: api-bindings%2Fffi}} {{!cat api-bindings%2Fffi}}\n",
        "{{#include refs.incl.md}}\n"
    );
    fs::write(&chapter, source).expect("write copied real-book chapter excerpt");
    fs::write(
        &config,
        concat!(
            "process-crate-block-directives = false\n",
            "process-example-directives = false\n",
            "process-link-and-badge-directives = true\n",
            "process-wikilinks = false\n",
            "included-extensions = [\"md\"]\n",
            "excluded-filenames = [\"refs.incl.md\", \"SUMMARY.md\"]\n"
        ),
    )
    .expect("write enabled directive configuration");

    let enabled = book_tool_command()
        .args(["directives", temp.path().to_str().expect("UTF-8 temp path")])
        .current_dir(temp.path())
        .output()
        .expect("run directives CLI with category rendering enabled");
    assert!(
        enabled.status.success(),
        "directives stderr: {}",
        String::from_utf8_lossy(&enabled.stderr)
    );
    assert_eq!(
        fs::read_to_string(&chapter).expect("read rendered chapter"),
        concat!(
            "[api-bindings%2Fffi][cat~api-bindings%2Fffi]↗{{hi:api-bindings%2Fffi}} ",
            "[![api-bindings%2Fffi][cat~api-bindings%2Fffi~badge]]",
            "[cat~api-bindings%2Fffi]{{hi:api-bindings%2Fffi}}\n",
            "{{#include refs.incl.md}}\n"
        )
    );

    fs::write(&chapter, source).expect("restore original chapter before disabled run");
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

    let disabled = book_tool_command()
        .args(["directives", temp.path().to_str().expect("UTF-8 temp path")])
        .current_dir(temp.path())
        .output()
        .expect("run directives CLI with category rendering disabled");
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
