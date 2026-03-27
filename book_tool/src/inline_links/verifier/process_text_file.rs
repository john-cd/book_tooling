use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use tempfile::NamedTempFile;

/// Atomically process a text file.
///
/// It reads the file, applies a predicate, and if the predicate matches,
/// it applies a transformation and writes the result back to the file atomically.
pub fn process_text_file<P, Pred, Trans>(
    filepath: P,
    should_process: Pred,
    transform: Trans,
) -> Result<()>
where
    P: AsRef<Path>,
    Pred: FnOnce(&str) -> bool,
    Trans: FnOnce(&str) -> Cow<'_, str>,
{
    let filepath = filepath.as_ref();

    // Read the file into memory.
    let mut file = File::open(filepath)
        .with_context(|| format!("Failed to open file: {}", filepath.display()))?;
    let size = file.metadata()
        .with_context(|| format!("Failed to read metadata for: {}", filepath.display()))?.len() as usize;
    let mut original = String::with_capacity(size);
    file.read_to_string(&mut original)
        .with_context(|| format!("Failed to read contents of: {}", filepath.display()))?;
    drop(file); // Close the file early.

    if !original.is_empty() && should_process(&original) {
        let updated: Cow<'_, str> = transform(&original);

        if updated != original {
            let parent = filepath.parent().unwrap_or_else(|| Path::new("."));
            let mut temp_file = NamedTempFile::new_in(parent)
                .with_context(|| format!("Failed to create temporary file in: {}", parent.display()))?;
            temp_file.write_all(updated.as_bytes())
                .with_context(|| format!("Failed to write to temporary file for: {}", filepath.display()))?;
            temp_file.persist(filepath)
                .with_context(|| format!("Failed to persist temporary file to: {}", filepath.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_process_text_file_updates_content() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "original content").unwrap();

        process_text_file(
            &file_path,
            |_| true,
            |s| Cow::Owned(s.replace("original", "updated")),
        )
        .unwrap();

        let updated_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(updated_content, "updated content");
    }

    #[test]
    fn test_process_text_file_skips_if_should_not_process() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "original content").unwrap();

        process_text_file(
            &file_path,
            |_| false,
            |s| Cow::Owned(s.replace("original", "updated")),
        )
        .unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "original content");
    }

    #[test]
    fn test_process_text_file_to_empty() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "original content").unwrap();

        process_text_file(&file_path, |_| true, |_| Cow::Owned("".to_string())).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "");
    }
}
