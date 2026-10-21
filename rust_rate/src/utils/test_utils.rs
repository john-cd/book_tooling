// TODO Reviewed ??-??-2026
use std::fs::File;
use std::io::Write;
use std::path::Path;

use anyhow::Result;

/// A test case for file-modifying code.
#[derive(Debug)]
pub struct TestCase {
    /// Relative path of the file to create inside the temporary directory.
    pub file_name_and_ext: &'static str,
    /// Optional original file contents to seed before running the test.
    pub original_contents: Option<&'static str>,
    /// Optional expected file contents after the function under test runs.
    pub expected_final_contents: Option<&'static str>,
}

/// Runs a file-oriented test workflow in a temporary directory.
///
/// Each [`TestCase`] is created inside the temporary directory before `func_to_test`
/// is invoked. After the function runs, the helper validates the resulting file contents.
pub fn test_with<F>(test_cases: &[TestCase], func_to_test: F) -> Result<()>
where
    F: Fn(&Path) -> Result<()>,
{
    let temp_dir = tempfile::tempdir()?;
    let temp_dir_path = temp_dir.path();

    for TestCase {
        file_name_and_ext,
        original_contents,
        expected_final_contents,
    } in test_cases
    {
        // Create a test file and write to it.
        let file_path = temp_dir_path.join(file_name_and_ext);
        let mut file = File::create(&file_path)?;
        if let Some(contents) = original_contents {
            file.write_all(contents.as_bytes())?;
        }
        file.flush()?;
        drop(file);
        // Call the function to test.
        func_to_test(temp_dir_path)?;
        // Check that the file contains the expected contents afterwards.
        assert!(file_path.exists());
        if let Some(expected) = expected_final_contents {
            let final_contents = std::fs::read_to_string(file_path)?;
            assert_eq!(&final_contents, expected);
        }
    }
    temp_dir.close()?;
    Ok(())
}
