//! Plain-text search across Markdown documents.
//!
//! TODO Reviewed ??-??-2026.

use std::ops::Range;
use std::path::PathBuf;

use anyhow::Result;
use parse_lib::plain_text_extractor::PlainTextExtractor;
use parse_lib::visit::Visitor as _;

use crate::common::Scope;
use crate::common::walk_parsed_markdown_files;
use crate::common::warn_and_continue_on_parse_failure;

/// Number of bytes of context shown on each side of a match.
const SNIPPET_CONTEXT_BYTES: usize = 30;

/// Returns the byte range of the first case-insensitive occurrence of `query` in `text`.
///
/// Characters are compared after Unicode lowercasing, but the range indexes `text` itself, so it
/// stays valid when lowercasing changes a character's UTF-8 length (for example `İ`).
fn find_case_insensitive(text: &str, query: &str) -> Option<Range<usize>> {
    let folded_query: Vec<char> = query.chars().flat_map(char::to_lowercase).collect();
    if folded_query.is_empty() {
        return None;
    }

    text.char_indices().find_map(|(start, _)| {
        let mut matched = 0_usize;
        let mut end = start;
        for character in text.get(start..)?.chars() {
            for lowered in character.to_lowercase() {
                if folded_query.get(matched) != Some(&lowered) {
                    return None;
                }
                matched += 1;
            }
            end += character.len_utf8();
            if matched == folded_query.len() {
                return Some(start..end);
            }
        }
        None
    })
}

/// Returns a one-line snippet of `text` around the first case-insensitive match of `query`.
fn find_snippet(text: &str, query: &str) -> Option<String> {
    let matched = find_case_insensitive(text, query)?;
    let mut start = matched.start.saturating_sub(SNIPPET_CONTEXT_BYTES);
    while !text.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = (matched.end + SNIPPET_CONTEXT_BYTES).min(text.len());
    while !text.is_char_boundary(end) {
        end += 1;
    }
    Some(text.get(start..end)?.replace('\n', " "))
}

/// Searches Markdown files using plain-text extraction and prints contextual snippets.
///
/// The search is case-insensitive and ignores most Markdown formatting by operating on
/// text extracted from the parsed document rather than the raw source.
///
/// # Errors
///
/// Returns an error if a directory cannot be traversed or a file cannot be read.
pub fn execute(query: &str, directories: &[PathBuf], scope: &Scope) -> Result<()> {
    tracing::info!(
        query = %query,
        directory_count = directories.len(),
        "Starting plain-text search workflow"
    );

    let mut matched_files = 0_usize;

    walk_parsed_markdown_files(
        directories,
        scope,
        |path, doc| {
            let mut extractor = PlainTextExtractor::new();
            extractor.visit_document(doc);

            if let Some(snippet) = find_snippet(&extractor.text, query) {
                matched_files += 1;
                tracing::info!(
                    path = %path.display(),
                    query = %query,
                    "Search query matched file"
                );
                println!("📄 {}: ...{}...", path.display(), snippet.trim());
            }

            Ok(())
        },
        warn_and_continue_on_parse_failure,
    )?;

    if matched_files == 0 {
        tracing::info!(query = %query, "No search matches found");
        println!("No matches found for '{query}'.");
    } else {
        tracing::info!(query = %query, matched_files, "Completed plain-text search workflow");
    }
    Ok(())
}

/// Tests for plain-text matching and document context.
#[cfg(test)]
mod tests {
    use super::*;

    /// Matching ignores case and returns offsets into the original text.
    #[test]
    fn matching_is_case_insensitive_and_reports_original_offsets() {
        assert_eq!(find_case_insensitive("Hello World", "WORLD"), Some(6..11));
        assert_eq!(find_case_insensitive("Hello World", "xyz"), None);
        assert_eq!(find_case_insensitive("Hello", ""), None);
    }

    /// Offsets stay correct when lowercasing changes a character's byte length.
    #[test]
    fn offsets_stay_valid_when_lowercasing_changes_byte_length() {
        // `İ` (2 bytes) lowercases to `i̇` (3 bytes), which shifts every later offset in a
        // lowercased copy.
        let text = "\u{130}\u{130}\u{130} needle here";
        let range = find_case_insensitive(text, "NEEDLE").expect("needle should match");
        assert_eq!(text.get(range), Some("needle"));
    }

    /// Snippet edges never split a multi-byte character.
    #[test]
    fn snippets_are_cut_on_character_boundaries() {
        let text = format!("{}needle{}", "\u{e9}".repeat(40), "\u{fc}".repeat(40));
        let snippet = find_snippet(&text, "needle").expect("needle should match");
        assert!(snippet.contains("needle"));
        assert!(snippet.starts_with('\u{e9}') && snippet.ends_with('\u{fc}'));
    }

    /// Snippets are flattened to a single line.
    #[test]
    fn snippets_replace_newlines_with_spaces() {
        let snippet = find_snippet("one\ntwo needle\nthree", "needle").expect("match");
        assert_eq!(snippet, "one two needle three");
    }
}
