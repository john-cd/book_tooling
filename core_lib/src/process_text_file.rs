use std::borrow::Cow;
use std::fs;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use memmap2::Mmap;
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
    // Read the file into memory.
    let mut file = File::open(filepath)?;
    let size = file.metadata()?.len() as usize;
    let mut original = String::with_capacity(size);
    file.read_to_string(&mut original)?;
    drop(file); // Close the file early.

    if !original.is_empty() && should_process(&original) {
        let updated: Cow<'_, str> = update_contents(&original);
        // TODO
        // if  updated.is_empty() {
        //
        //     return Ok(());
        // }
        if updated != original {
            let temp_filepath = filepath.with_extension(".tmp");
            let mut temp_file = File::create(&temp_filepath)?;
            temp_file.write_all(updated.as_bytes())?;
            // Renames a file or directory to a new name, replacing the original file if it already exists.
            std::fs::rename(&temp_filepath, filepath)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
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
