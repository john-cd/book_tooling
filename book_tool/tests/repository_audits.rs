//! Process-level behavior for repository audit commands.

use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
use std::process::Output;

use tempfile::tempdir;

fn run(command: &str, root: &Path) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_book_tool"))
        .arg(command)
        .arg("--root")
        .arg(root)
        .output()
}

fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn config_drift_reports_success_and_preserves_failure_status() -> io::Result<()> {
    let clean = tempdir()?;
    let clean_output = run("config-drift", clean.path())?;
    assert!(
        clean_output.status.success(),
        "{}",
        output_text(&clean_output)
    );
    assert!(output_text(&clean_output).contains("Configuration paths and pins are consistent:"));

    fs::write(
        clean.path().join(".editorconfig"),
        "[*]\ncharset = latin1\n",
    )?;
    let failed_output = run("config-drift", clean.path())?;
    assert_eq!(failed_output.status.code(), Some(1_i32));
    assert!(
        output_text(&failed_output).contains("ERROR: .editorconfig [*] must set charset=utf-8")
    );
    Ok(())
}

#[test]
fn orphan_sources_reports_unresolved_modules_and_unreachable_files() -> io::Result<()> {
    let temp = tempdir()?;
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"audit-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )?;
    let src = temp.path().join("src");
    fs::create_dir_all(&src)?;
    fs::write(src.join("lib.rs"), "mod missing;\nmod live;\n")?;
    fs::write(src.join("live.rs"), "// reachable\n")?;
    fs::write(src.join("orphan.rs"), "// not declared\n")?;

    let output = run("orphan-sources", temp.path())?;
    assert_eq!(
        output.status.code(),
        Some(1_i32),
        "{}",
        output_text(&output)
    );
    let text = output_text(&output);
    assert!(text.contains("orphan source check failed:"), "{text}");
    assert!(text.contains("declares module `missing`"), "{text}");
    assert!(
        text.contains("orphan source src\\orphan.rs")
            || text.contains("orphan source src/orphan.rs"),
        "{text}"
    );
    Ok(())
}

#[test]
fn orphan_sources_accepts_declared_flat_and_nested_modules() -> io::Result<()> {
    let temp = tempdir()?;
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"audit-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )?;
    let src = temp.path().join("src");
    fs::create_dir_all(src.join("nested"))?;
    fs::write(src.join("lib.rs"), "mod flat;\npub mod nested;\n")?;
    fs::write(src.join("flat.rs"), "// flat module\n")?;
    fs::write(src.join("nested/mod.rs"), "// nested module\n")?;

    let output = run("orphan-sources", temp.path())?;
    assert!(output.status.success(), "{}", output_text(&output));
    assert!(output_text(&output).contains("No orphan Rust source files found"));
    Ok(())
}
