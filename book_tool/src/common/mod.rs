//! Shared utilities for file-based command workflows.
//!
//! TODO Reviewed ??-??-2026.

use std::io::Write as _;
use std::path::Path;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::bail;
use tempfile::NamedTempFile;

/// Directory traversal helpers shared by file-oriented commands.
mod walk;

pub use book_templates::*;
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

/// Replaces a regular text file only after the full replacement has been staged beside it.
///
/// Symbolic links and non-file destinations are rejected. A unique temporary file is removed
/// automatically if writing or replacement fails, and the source permissions are retained.
///
/// # Errors
///
/// Returns an error if the target is a symlink, or if the temporary file cannot be written or
/// persisted over the target.
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

/// Tests for atomic replacement and file content preservation.
#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    /// Replacing an existing or new file leaves look-alike neighbouring files untouched.
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

    /// Replacement keeps the original file mode.
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

    /// Symbolic-link destinations are refused and their targets stay unchanged.
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
