//! Plain-text search across Markdown documents.

use std::path::PathBuf;

use anyhow::Result;
use parse_lib::plain_text_extractor::PlainTextExtractor;
use parse_lib::visit::Visitor;

use crate::common::walk::Scope;
use crate::common::walk::walk_parsed_markdown_files;

/// Searches Markdown files using plain-text extraction and prints contextual snippets.
///
/// The search is case-insensitive and ignores most Markdown formatting by operating on
/// text extracted from the parsed document rather than the raw source.
pub fn execute(query: String, directories: Vec<PathBuf>) -> Result<()> {
    let scope = Scope::default();
    let query_lower = query.to_lowercase();
    let mut found_matches_global = false;

    for dir in &directories {
        let mut found_matches = false;
        walk_parsed_markdown_files(
            [dir],
            &scope,
            |path, doc| {
                let mut extractor = PlainTextExtractor::new();
                extractor.visit_document(doc);

                let plain_text = extractor.text.to_lowercase();
                if let Some(index) = plain_text.find(&query_lower) {
                    found_matches = true;

                    // Extract a small snippet surrounding the match for context
                    let start = index.saturating_sub(30);
                    let end = (index + query.len() + 30).min(extractor.text.len());
                    let snippet = &extractor.text[start..end].replace('\n', " ");

                    println!("📄 {}: ...{}...", path.display(), snippet.trim());
                }

                Ok(())
            },
            |_path, _error| {},
        )?;

        if found_matches {
            found_matches_global = true;
        }
    }

    if !found_matches_global {
        println!("No matches found for '{}'.", query);
    }
    Ok(())
}
