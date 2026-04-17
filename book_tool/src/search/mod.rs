use std::path::PathBuf;

use anyhow::Result;
use parse_lib::plain_text_extractor::PlainTextExtractor;
use parse_lib::visit::Visitor;

use crate::common::walk::Scope;
use crate::common::walk::walk_directory_and_process_files;

pub fn execute(query: String, directories: Vec<PathBuf>) -> Result<()> {
    let scope = Scope::default();
    let query_lower = query.to_lowercase();
    let mut found_matches_global = false;

    for dir in &directories {
        let mut found_matches = false;
        walk_directory_and_process_files(dir, &scope, |path| {
            let content = std::fs::read_to_string(path)?;
            let filename = path.file_name().unwrap_or_default().to_string_lossy();
            if let Ok(doc) = parse_lib::parse_document(&filename, Some(path), &content) {
                let mut extractor = PlainTextExtractor::new();
                extractor.visit_document(&doc);

                let plain_text = extractor.text.to_lowercase();
                if let Some(index) = plain_text.find(&query_lower) {
                    found_matches = true;

                    // Extract a small snippet surrounding the match for context
                    let start = index.saturating_sub(30);
                    let end = (index + query.len() + 30).min(extractor.text.len());
                    let snippet = &extractor.text[start..end].replace('\n', " ");

                    println!("📄 {}: ...{}...", path.display(), snippet.trim());
                }
            }
            Ok(())
        })?;
        if found_matches {
            found_matches_global = true;
        }
    }

    if !found_matches_global {
        println!("No matches found for '{}'.", query);
    }
    Ok(())
}
