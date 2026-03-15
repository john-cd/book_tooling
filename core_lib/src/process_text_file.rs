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
    let path = filepath.as_ref();
    let file =
        fs::File::open(path).with_context(|| format!("Could not open file {}", path.display()))?;
    let metadata = file.metadata()?;

    if metadata.len() > 1024 * 1024 {
        // Use memory mapping for files larger than 1MB
        let mmap = unsafe { Mmap::map(&file)? };
        let content = std::str::from_utf8(&mmap)
            .with_context(|| format!("File {} is not valid UTF-8", path.display()))?;

        if should_process(content) {
            let updated_content = transform(content);
            if updated_content != content {
                write_atomically(path, &updated_content)?;
            }
        }
    } else {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Could not read file {}", path.display()))?;

        if should_process(&content) {
            let updated_content = transform(&content);

            if updated_content != content {
                write_atomically(path, &updated_content)?;
            }
        }
    }

    Ok(())
}

fn write_atomically(path: &Path, updated_content: &str) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut temp_file = NamedTempFile::new_in(parent)
        .with_context(|| format!("Could not create temporary file in {}", parent.display()))?;

    std::io::Write::write_all(&mut temp_file, updated_content.as_bytes())
        .with_context(|| format!("Could not write to temporary file for {}", path.display()))?;

    temp_file
        .persist(path)
        .with_context(|| format!("Could not atomically update file {}", path.display()))?;
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
