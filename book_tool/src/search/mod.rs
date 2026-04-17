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
    tracing::info!(
        query = %query,
        directory_count = directories.len(),
        "Starting plain-text search workflow"
    );

    let scope = Scope::default();
    let query_lower = query.to_lowercase();
    let mut matched_files = 0_usize;

    for dir in &directories {
        walk_parsed_markdown_files(
            [dir],
            &scope,
            |path, doc| {
                let mut extractor = PlainTextExtractor::new();
                extractor.visit_document(doc);

                let plain_text = extractor.text.to_lowercase();
                if let Some(index) = plain_text.find(&query_lower) {
                    matched_files += 1;
                    tracing::info!(
                        path = %path.display(),
                        query = %query,
                        "Search query matched file"
                    );

                    // Extract a small snippet surrounding the match for context
                    let start = index.saturating_sub(30);
                    let end = (index + query.len() + 30).min(extractor.text.len());
                    let snippet = &extractor.text[start..end].replace('\n', " ");

                    println!("📄 {}: ...{}...", path.display(), snippet.trim());
                }

                Ok(())
            },
            |path, error| tracing::warn!("Failed to parse {}: {}", path.display(), error),
        )?;
    }

    if matched_files == 0 {
        tracing::info!(query = %query, "No search matches found");
        println!("No matches found for '{}'.", query);
    } else {
        tracing::info!(query = %query, matched_files, "Completed plain-text search workflow");
    }
    Ok(())
}
