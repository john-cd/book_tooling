//! Shared utilities for file-based command workflows.

use std::borrow::Cow;
use std::io::Write;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
/// Markdown rendering helpers shared by directive processing and other workflows.
pub mod templates;
/// Directory traversal helpers shared by file-oriented commands.
pub mod walk;

/// Reads a UTF-8 text file, optionally transforms it, and writes changes back atomically.
///
/// The file is only rewritten when `should_process` returns `true` and the transformed
/// contents differ from the original text.
pub fn process_text_file<P, C, T>(path: P, should_process: C, transform: T) -> Result<()>
where
    P: AsRef<Path>,
    C: Fn(&str) -> bool,
    T: FnOnce(&str) -> Cow<str>,
{
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    if !should_process(&content) {
        return Ok(());
    }

    let transformed = transform(&content);

    if transformed != content {
        let temp_path = path.with_extension("tmp");
        {
            let mut file = std::fs::File::create(&temp_path).with_context(|| {
                format!("Failed to create temporary file: {}", temp_path.display())
            })?;
            file.write_all(transformed.as_bytes()).with_context(|| {
                format!("Failed to write to temporary file: {}", temp_path.display())
            })?;
        }
        std::fs::rename(&temp_path, path)
            .with_context(|| format!("Failed to rename temporary file to: {}", path.display()))?;
    }

    Ok(())
}
