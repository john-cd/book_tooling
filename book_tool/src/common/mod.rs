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
    tracing::debug!(path = %path.display(), "Reading text file");
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    if !should_process(&content) {
        tracing::debug!(
            path = %path.display(),
            "Skipping file because the fast-path predicate returned false"
        );
        return Ok(());
    }

    tracing::debug!(path = %path.display(), "Transforming text file");
    let transformed = transform(&content);

    if transformed != content {
        tracing::info!(
            path = %path.display(),
            old_bytes = content.len(),
            new_bytes = transformed.len(),
            "Writing updated file contents"
        );
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
    } else {
        tracing::debug!(path = %path.display(), "Transformation produced no file changes");
    }

    Ok(())
}

/// Reads a UTF-8 Markdown file, parses it, and writes a transformed version atomically.
///
/// Parse failures are delegated to `on_parse_error` and leave the original file unchanged.
pub fn process_parsed_markdown_file<P, C, T, E>(
    path: P,
    should_process: C,
    transform: T,
    on_parse_error: E,
) -> Result<()>
where
    P: AsRef<Path>,
    C: Fn(&str) -> bool,
    T: for<'input> FnOnce(&Path, &parse_lib::Document<'input>) -> Cow<'input, str>,
    E: FnOnce(&Path, &anyhow::Error),
{
    let path = path.as_ref();

    process_text_file(path, should_process, |content| {
        tracing::debug!(path = %path.display(), "Parsing Markdown document");
        match parse_lib::parse_document("file", None, content) {
            Ok(doc) => {
                tracing::debug!(path = %path.display(), "Parsed Markdown document successfully");
                transform(path, &doc)
            }
            Err(error) => {
                on_parse_error(path, &error);
                Cow::Borrowed(content)
            }
        }
    })
}
