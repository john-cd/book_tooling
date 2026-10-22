//! Shared utilities for file-based command workflows.
//!
//! TODO Reviewed ??-??-2026

use std::borrow::Cow;
use std::io::Write as _;
use std::path::Path;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::bail;
use tempfile::NamedTempFile;

/// Markdown rendering helpers shared by directive processing and other workflows.
mod templates;
/// Directory traversal helpers shared by file-oriented commands.
mod walk;

pub use templates::*;
pub use walk::*;

/// Writes a parse-failure diagnostic that remains visible even when tracing is disabled.
pub fn warn_and_continue_on_parse_failure(path: &Path, error: &anyhow::Error) {
    eprintln!(
        "Failed to process Markdown file {}: {error:#}",
        path.display()
    );
    tracing::warn!(
        path = %path.display(),
        error = %error,
        "Failed to parse Markdown file; skipping file and continuing"
    );
}

/// Reads a UTF-8 text file, optionally transforms it, and writes changes back atomically.
///
/// The file is only rewritten when `should_process` returns `true` and the transformed
/// contents differ from the original text.
///
/// # Errors
///
/// Returns an error if the file cannot be read, transformed or atomically replaced.
pub fn process_text_file<P, C, T>(path: P, should_process: C, transform: T) -> Result<()>
where
    P: AsRef<Path>,
    C: Fn(&str) -> bool,
    T: FnOnce(&str) -> Cow<'_, str>,
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

    if transformed == content {
        tracing::debug!(path = %path.display(), "Transformation produced no file changes");
    } else {
        tracing::info!(
            path = %path.display(),
            old_bytes = content.len(),
            new_bytes = transformed.len(),
            "Writing updated file contents"
        );
        replace_text_file_atomically(path, transformed.as_ref())?;
    }

    Ok(())
}

/// Replaces a regular text file only after the full replacement has been staged beside it.
///
/// Symbolic links and non-file destinations are rejected. A unique temporary file is removed
/// automatically if writing or replacement fails, and the source permissions are retained.
///
/// # Errors
///
/// Returns an error if the target is a symlink, or if the temporary file cannot be written or persisted over the target.
pub fn replace_text_file_atomically(path: &Path, contents: &str) -> Result<()> {
    let permissions = match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!("Refusing to replace symbolic link: {}", path.display());
        }
        Ok(metadata) if !metadata.is_file() => {
            bail!("Refusing to replace non-file path: {}", path.display());
        }
        Ok(metadata) => Some(metadata.permissions()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(error)
                .with_context(|| format!("Failed to inspect file: {}", path.display()));
        }
    };

    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut staged = NamedTempFile::new_in(parent)
        .with_context(|| format!("Failed to stage replacement for {}", path.display()))?;
    staged
        .write_all(contents.as_bytes())
        .with_context(|| format!("Failed to write staged replacement for {}", path.display()))?;
    staged
        .flush()
        .with_context(|| format!("Failed to flush staged replacement for {}", path.display()))?;
    if let Some(permissions) = permissions {
        staged
            .as_file()
            .set_permissions(permissions)
            .with_context(|| format!("Failed to preserve permissions for {}", path.display()))?;
    }
    staged
        .persist(path)
        .map(|_| ())
        .map_err(|error| error.error)
        .with_context(|| format!("Failed to replace {}", path.display()))
}

/// Reads a UTF-8 Markdown file, parses it, and writes a transformed version atomically.
///
/// Parse failures are delegated to `on_parse_error`, leave the original file unchanged, and
/// are returned to the caller as errors.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed, if processing fails, or if the file cannot be rewritten.
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
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;
    if !should_process(&content) {
        return Ok(());
    }

    tracing::debug!(path = %path.display(), "Parsing Markdown document");
    let doc = match parse_lib::parse_document("file", None, &content) {
        Ok(doc) => doc,
        Err(error) => {
            on_parse_error(path, &error);
            return Err(error)
                .with_context(|| format!("Failed to parse Markdown file: {}", path.display()));
        }
    };
    tracing::debug!(path = %path.display(), "Parsed Markdown document successfully");
    let transformed = transform(path, &doc);
    if transformed != content {
        replace_text_file_atomically(path, transformed.as_ref())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn atomic_replace_handles_existing_and_new_files_without_clobbering_neighbors() -> Result<()> {
        let temp = tempdir()?;
        let file = temp.path().join("chapter.md");
        let neighbor = temp.path().join("chapter.tmp");
        std::fs::write(&file, "old source")?;
        std::fs::write(&neighbor, "user-owned temporary-looking file")?;

        replace_text_file_atomically(&file, "new source")?;
        assert_eq!(std::fs::read_to_string(&file)?, "new source");
        assert_eq!(
            std::fs::read_to_string(&neighbor)?,
            "user-owned temporary-looking file"
        );

        let new_file = temp.path().join("new.md");
        replace_text_file_atomically(&new_file, "created atomically")?;
        assert_eq!(std::fs::read_to_string(new_file)?, "created atomically");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn atomic_replace_preserves_permissions() -> Result<()> {
        use std::os::unix::fs::PermissionsExt as _;

        let temp = tempdir()?;
        let file = temp.path().join("chapter.md");
        std::fs::write(&file, "old")?;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o640))?;

        replace_text_file_atomically(&file, "new")?;

        assert_eq!(std::fs::metadata(file)?.permissions().mode() & 0o777, 0o640);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn atomic_replace_rejects_symlinks_without_touching_the_target() -> Result<()> {
        use std::os::unix::fs::symlink;

        let temp = tempdir()?;
        let target = temp.path().join("target.md");
        let link = temp.path().join("chapter.md");
        std::fs::write(&target, "target source")?;
        symlink(&target, &link)?;

        let error = replace_text_file_atomically(&link, "replacement")
            .expect_err("symlink destinations must be rejected");

        assert!(error.to_string().contains("symbolic link"));
        assert_eq!(std::fs::read_to_string(target)?, "target source");
        assert!(std::fs::symlink_metadata(link)?.file_type().is_symlink());
        Ok(())
    }
}
