//! Extracts HTML comments from Markdown files into an audit file.

use std::path::PathBuf;

use anyhow::Result;
use parse_lib::visit::Visitor;

use crate::common::walk::Scope;
use crate::common::walk::walk_parsed_markdown_files;

/// Scans the provided directories and writes all discovered HTML comments to `output`.
///
/// Output is grouped by source file and overwrites the destination file if it already exists.
pub fn execute(directories: Vec<PathBuf>, output: PathBuf) -> Result<()> {
    let scope = Scope::default();
    let mut all_comments_output = String::new();

    walk_parsed_markdown_files(
        &directories,
        &scope,
        |path, doc| {
            let mut extractor = parse_lib::html_comment_extractor::HtmlCommentExtractor::new();
            extractor.visit_document(doc);

            for comment in extractor.comments {
                all_comments_output += &format!("File: {}\n<!--{}-->\n\n", path.display(), comment);
            }

            Ok(())
        },
        |_path, _error| {},
    )?;

    std::fs::write(&output, all_comments_output)?;
    println!(
        "HTML comments extracted successfully to {}.",
        output.display()
    );
    Ok(())
}
