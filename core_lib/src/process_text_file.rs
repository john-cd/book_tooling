use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;

use anyhow::Result;

/// Read a text file in memory, and, if its contents should be processed, update its contents on disk.
///
/// This function performs an in-place update by writing to a temporary file and then renaming it,
/// ensuring atomicity and avoiding partial writes.
///
/// # Arguments
///
/// * `filepath` - The path to the text file to process.
/// * `should_process` - A closure that returns true if the file should be processed, given its contents.
///   This is used as a quick check before applying more expensive transformations.
/// * `update_contents` - A closure that returns the updated contents of the file, given its contents.
///   It returns a `Cow<str>` to avoid allocations when no changes are needed.
///
/// # Errors
///
/// Returns an error if the file cannot be read, written, or renamed.
pub fn process_text_file<P, F>(filepath: &Path, should_process: P, update_contents: F) -> Result<()>
where
    P: Fn(&str) -> bool,
    F: Fn(&str) -> Cow<str>,
{
    // Read the file into memory.
    // TODO: Consider using memory mapping for very large files.
    let mut file = File::open(filepath)?;
    let size = file.metadata()?.len() as usize;
    let mut original = String::with_capacity(size);
    file.read_to_string(&mut original)?;
    drop(file); // Close the file early.

    if !original.is_empty() && should_process(&original) {
        let updated: Cow<'_, str> = update_contents(&original);
        // TODO: Handle the case where the updated content is empty, if that's a valid outcome.
        // if  updated.is_empty() {
        //
        //     return Ok(());
        // }
        if updated != original {
            let temp_filepath = filepath.with_extension(".tmp");
            // TODO: Ensure the temporary file name is unique to avoid collisions during parallel processing.
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
    use std::borrow::Cow;

    use super::*;
    use crate::utils::test_utils::TestCase;
    use crate::utils::test_utils::test_with;

    #[test]
    fn test_process_text_file_updates_content() -> Result<()> {
        let test_cases = vec![TestCase {
            file_name_and_ext: "test.md",
            original_contents: Some("original content"),
            expected_final_contents: Some("updated content"),
        }];

        test_with(&test_cases, |dir_path: &std::path::Path| {
            let file_path = dir_path.join("test.md");
            process_text_file(
                &file_path,
                |s| s.contains("original"),
                |_| Cow::Borrowed("updated content"),
            )
        })
    }

    #[test]
    fn test_process_text_file_skips_if_should_not_process() -> Result<()> {
        let test_cases = vec![TestCase {
            file_name_and_ext: "test.md",
            original_contents: Some("original content"),
            expected_final_contents: Some("original content"),
        }];

        test_with(&test_cases, |dir_path: &std::path::Path| {
            let file_path = dir_path.join("test.md");
            process_text_file(
                &file_path,
                |s| s.contains("other"),
                |_| Cow::Borrowed("updated content"),
            )
        })
    }
}
