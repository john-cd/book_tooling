//! Module for extracting HTML comments from Markdown files.
//!
// Note on old code: This module was moved to `book_tool`.
// Uses `core_lib` and `parse_lib` for iterating files and extracting HTML comments.

use anyhow::Result;
use parse_lib::visit::Visitor;
use std::path::PathBuf;

// TODO: The `core_lib` crate has been merged. Update these imports to point to the
// correct new modules inside `crate::common`. The module names `walk` and `scope`
// are best guesses based on the functions used below.
use crate::common::scope::Scope;
use crate::common::walk::walk_directory_and_process_files;

/// Executes the comment extraction process.
///
/// # Arguments
///
/// * `directories` - A list of directory paths to scan.
/// * `output` - The file path where extracted comments should be written.
pub fn execute(directories: Vec<PathBuf>, output: PathBuf) -> Result<()> {
    let scope = Scope::default();
    let mut all_comments_output = String::new();

    for dir in &directories {
        walk_directory_and_process_files(dir, &scope, |path| {
            let content = std::fs::read_to_string(path)?;
            let filename = path.file_name().unwrap_or_default().to_string_lossy();
            if let Ok(doc) = parse_lib::parse_document(&filename, Some(path), &content) {
                let mut extractor = parse_lib::html_comment_extractor::HtmlCommentExtractor::new();
                extractor.visit_document(&doc);

                for comment in extractor.comments {
                    all_comments_output.push_str(&format!("File: {}\n<!--{}-->\n\n", path.display(), comment));
                }
            }
            Ok(())
        })?;
    }

    std::fs::write(&output, all_comments_output)?;
    println!("HTML comments extracted successfully to {}.", output.display());
    Ok(())
}
