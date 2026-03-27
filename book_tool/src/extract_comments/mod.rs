// Note on old code: This module was moved to `book_tool`.
// Uses `core_lib` and `parse_lib` for iterating files and extracting HTML comments.

use anyhow::Result;
use parse_lib::visit::Visitor;
use std::path::PathBuf;

pub fn execute(directories: Vec<PathBuf>, output: PathBuf) -> Result<()> {
    let scope = core_lib::Scope::default();
    let mut all_comments_output = String::new();

    for dir in &directories {
        core_lib::walk_directory_and_process_files(dir, &scope, |path| {
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
