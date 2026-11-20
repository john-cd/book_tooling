//! Extracts HTML comments from Markdown files into an audit file.
//!
//! TODO Reviewed ??-??-2026.

use std::fmt::Write as _;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use parse_lib::visit::Visitor as _;

use crate::common::Scope;
use crate::common::ensure_distinct_report;
use crate::common::replace_text_file_atomically;
use crate::common::report_identity;
use crate::common::walk_parsed_markdown_files;
use crate::common::warn_and_continue_on_parse_failure;

/// Scans the provided directories and writes all discovered HTML comments to `output`.
///
/// Output is grouped by source file and overwrites the destination file if it already exists.
///
/// # Errors
///
/// Returns an error if a directory cannot be processed or the output file cannot be written.
pub fn execute(directories: &[PathBuf], output: &Path, scope: &Scope) -> Result<()> {
    tracing::info!(
        directory_count = directories.len(),
        output = %output.display(),
        "Starting HTML comment extraction workflow"
    );

    let mut all_comments_output = String::new();
    let mut extracted_comments = 0_usize;
    let mut files_with_comments = 0_usize;

    let destination = report_identity(output)?;
    walk_parsed_markdown_files(
        directories,
        scope,
        |path, doc| {
            ensure_distinct_report(output, destination.as_ref(), path)?;
            let mut extractor = parse_lib::html_comment_extractor::HtmlCommentExtractor::new();
            extractor.visit_document(doc);
            let comment_count = extractor.comments.len();

            if comment_count > 0 {
                files_with_comments += 1;
                extracted_comments += comment_count;
                tracing::info!(
                    path = %path.display(),
                    comment_count,
                    "Extracted HTML comments from file"
                );
            } else {
                tracing::debug!(path = %path.display(), "No HTML comments found in file");
            }

            for comment in extractor.comments {
                let _unused = write!(
                    all_comments_output,
                    "File: {}\n<!--{}-->\n\n",
                    path.display(),
                    comment
                );
            }

            Ok(())
        },
        warn_and_continue_on_parse_failure,
    )?;

    drop(destination);
    replace_text_file_atomically(output, &all_comments_output)?;
    tracing::info!(
        output = %output.display(),
        extracted_comments,
        files_with_comments,
        "Completed HTML comment extraction workflow"
    );
    println!(
        "HTML comments extracted successfully to {}.",
        output.display()
    );
    Ok(())
}

/// Tests for comment extraction and output file handling.
#[cfg(test)]
mod tests {
    use super::*;

    /// Checks that the output groups comments by source file and replaces existing content.
    #[test]
    fn writes_comments_grouped_by_file_and_overwrites_output() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let book = dir.path().join("book");
        std::fs::create_dir_all(&book)?;
        std::fs::write(
            book.join("a.md"),
            "text <!-- first --> more\n\n<!-- second -->\n",
        )?;
        std::fs::write(book.join("b.md"), "no comments here\n")?;
        let output = dir.path().join("comments.txt");
        std::fs::write(&output, "stale")?;

        execute(std::slice::from_ref(&book), &output, &Scope::default())?;

        let written = std::fs::read_to_string(&output)?;
        assert!(!written.contains("stale"));
        assert!(written.contains("a.md\n<!-- first -->\n\n"), "{written}");
        assert!(written.contains("<!-- second -->\n\n"), "{written}");
        assert!(!written.contains("b.md"), "{written}");
        Ok(())
    }

    /// Checks that a scan with no comments creates an empty output file.
    #[test]
    fn no_comments_yields_an_empty_output_file() -> Result<()> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("a.md"), "plain\n")?;
        let output = dir.path().join("out").with_extension("txt");
        execute(&[dir.path().to_path_buf()], &output, &Scope::default())?;
        assert_eq!(std::fs::read_to_string(output)?, "");
        Ok(())
    }

    /// Checks that comment extraction reports a missing source directory.
    #[test]
    fn missing_input_directory_is_an_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        let result = execute(
            &[dir.path().join("absent")],
            &dir.path().join("out.txt"),
            &Scope::default(),
        );
        result.expect_err("missing directory is reported");
    }
}
