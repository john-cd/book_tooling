//! Extracts HTML comments from Markdown files into an audit file.
//!
//! TODO Reviewed ??-??-2026

use std::fmt::Write as _;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use parse_lib::visit::Visitor as _;

use crate::common::Scope;
use crate::common::replace_text_file_atomically;
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

    walk_parsed_markdown_files(
        directories,
        scope,
        |path, doc| {
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
