//! Helpers to run table-driven tests.
//!
//! TODO Reviewed ??-??-2026

use std::fs::File;
use std::io::Write as _;
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
///
/// # Errors
///
/// Returns an error if a fixture cannot be created, `func_to_test` fails, the file is missing
/// afterwards, or its contents differ from the expected ones.
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
        anyhow::ensure!(
            file_path.exists(),
            "{} should exist after the function under test runs",
            file_path.display()
        );
        if let Some(expected) = expected_final_contents {
            let final_contents = std::fs::read_to_string(&file_path)?;
            anyhow::ensure!(
                &final_contents == expected,
                "unexpected contents in {}",
                file_path.display()
            );
        }
    }
    temp_dir.close()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cases whose function rewrites the file as expected pass.
    #[test]
    fn passes_when_the_function_produces_the_expected_contents() {
        let cases = [TestCase {
            file_name_and_ext: "a.txt",
            original_contents: Some("old"),
            expected_final_contents: Some("new"),
        }];
        test_with(&cases, |dir| {
            std::fs::write(dir.join("a.txt"), "new")?;
            Ok(())
        })
        .expect("expected contents");
    }

    /// Unexpected final contents are reported as an error naming the file.
    #[test]
    fn fails_when_contents_differ() {
        let cases = [TestCase {
            file_name_and_ext: "a.txt",
            original_contents: Some("old"),
            expected_final_contents: Some("new"),
        }];
        let err = test_with(&cases, |_dir| Ok(())).expect_err("contents unchanged");
        assert!(err.to_string().contains("a.txt"), "{err}");
    }

    /// A file removed by the function under test is reported as missing.
    #[test]
    fn fails_when_the_file_disappears() {
        let cases = [TestCase {
            file_name_and_ext: "gone.txt",
            original_contents: None,
            expected_final_contents: None,
        }];
        let err = test_with(&cases, |dir| {
            std::fs::remove_file(dir.join("gone.txt"))?;
            Ok(())
        })
        .expect_err("file removed");
        assert!(err.to_string().contains("should exist"), "{err}");
    }

    /// Errors from the function under test are propagated.
    #[test]
    fn propagates_function_errors() {
        let cases = [TestCase {
            file_name_and_ext: "a.txt",
            original_contents: None,
            expected_final_contents: None,
        }];
        let err = test_with(&cases, |_dir| anyhow::bail!("boom")).expect_err("fails");
        assert_eq!(err.to_string(), "boom");
    }

    /// Without expectations only existence is checked, and an empty case list succeeds.
    #[test]
    fn unspecified_expectations_are_not_checked() {
        let cases = [TestCase {
            file_name_and_ext: "a.txt",
            original_contents: Some("anything"),
            expected_final_contents: None,
        }];
        test_with(&cases, |_dir| Ok(())).expect("existence only");
        test_with(&[], |_dir| Ok(())).expect("no cases");
    }

    /// Each case gets the function run against the shared directory once.
    #[test]
    fn runs_the_function_once_per_case() {
        let cases = [
            TestCase {
                file_name_and_ext: "one.txt",
                original_contents: Some("1"),
                expected_final_contents: Some("1"),
            },
            TestCase {
                file_name_and_ext: "two.txt",
                original_contents: Some("2"),
                expected_final_contents: Some("2"),
            },
        ];
        let calls = std::cell::Cell::new(0_usize);
        test_with(&cases, |_dir| {
            calls.set(calls.get() + 1);
            Ok(())
        })
        .expect("passes");
        assert_eq!(calls.get(), 2);
    }
}
