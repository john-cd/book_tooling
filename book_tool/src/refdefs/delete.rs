//! Removal of unused reference definitions.
//!
//! TODO Reviewed ??-??-2026

use std::collections::HashSet;
use std::hash::BuildHasher;
use std::ops::Range;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::bail;
use parse_lib::Document;
use parse_lib::ReferenceDefinitionData;
use parse_lib::parse_document;
use parse_lib::visit::Visitable as _;
use parse_lib::visit::Visitor;

use super::normalize_reference_label;
use super::report::ScopedRefDefAnalysis;
use crate::common::replace_text_file_atomically;

/// Remove definitions identified as unused by a complete include-scope analysis.
///
/// # Errors
///
/// Returns an error if a file cannot be read, parsed or rewritten.
pub fn remove_proven_unused_refdefs(analysis: &ScopedRefDefAnalysis) -> Result<usize> {
    let mut replacements = Vec::new();
    for (path, unused_labels) in &analysis.unused_by_file {
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let document = parse_document("file", Some(path), &source)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        let replacement = delete_unused_refdefs(&document, unused_labels)?;
        if replacement != source {
            replacements.push((path.clone(), replacement));
        }
    }
    replacements.sort_by(|left, right| left.0.cmp(&right.0));
    let changed_files = replacements.len();
    for (path, replacement) in replacements {
        replace_text_file_atomically(&path, &replacement)?;
    }
    Ok(changed_files)
}

/// Remove only proven-unused reference-definition source ranges.
///
/// # Errors
///
/// Returns an error if a definition cannot be located in the source.
pub fn delete_unused_refdefs<S: BuildHasher>(
    doc: &Document<'_>,
    unused_labels: &HashSet<String, S>,
) -> Result<String> {
    /// Collects source ranges for reference definitions proven unused.
    struct RemovalCollector<'a, S> {
        /// Original Markdown text that owns the borrowed AST fragments.
        source: &'a str,
        /// Normalized labels that may be removed.
        unused: &'a HashSet<String, S>,
        /// Whole-line ranges to remove after traversal completes.
        ranges: Vec<Range<usize>>,
        /// First source-location error encountered during traversal.
        error: Option<anyhow::Error>,
    }

    impl<S> RemovalCollector<'_, S> {
        /// Locates a borrowed fragment in the source document.
        ///
        /// # Errors
        ///
        /// Returns an error if `fragment` does not belong to the source document.
        fn source_range(&self, fragment: &str) -> Result<Range<usize>> {
            parse_lib::source_span(self.source, fragment)
                .context("Reference definition does not belong to its source document")
        }
    }

    impl<'a, S: BuildHasher> Visitor<'a> for RemovalCollector<'_, S> {
        /// Records the whole-line source range of every definition whose label is in the unused
        /// set.
        fn visit_reference_definition(&mut self, definition: &'a ReferenceDefinitionData<'a>) {
            if !self.unused.iter().any(|label| {
                normalize_reference_label(label) == normalize_reference_label(definition.label)
            }) {
                return;
            }

            let mut end = match self.source_range(definition.label) {
                Ok(range) => range.end,
                Err(error) => {
                    self.error = Some(error);
                    return;
                }
            };
            for value in [Some(definition.url), definition.title]
                .into_iter()
                .flatten()
            {
                match self.source_range(value) {
                    Ok(range) => end = end.max(range.end),
                    Err(error) => {
                        self.error = Some(error);
                        return;
                    }
                }
            }
            let Ok(label_range) = self.source_range(definition.label) else {
                self.error = Some(anyhow::anyhow!(
                    "Reference definition label is outside its source document"
                ));
                return;
            };
            let line_start = self
                .source
                .get(..label_range.start)
                .unwrap_or_default()
                .rfind('\n')
                .map_or(0, |newline| newline + 1);
            let line_end = self
                .source
                .get(end..)
                .unwrap_or_default()
                .find('\n')
                .map_or(self.source.len(), |newline| end + newline + 1);
            self.ranges.push(line_start..line_end);
        }
    }

    let mut collector = RemovalCollector {
        source: doc.source,
        unused: unused_labels,
        ranges: Vec::new(),
        error: None,
    };
    doc.accept(&mut collector);
    if let Some(error) = collector.error {
        return Err(error);
    }
    collector.ranges.sort_by_key(|range| range.start);

    let mut output = String::with_capacity(doc.source.len());
    let mut cursor = 0;
    for range in collector.ranges {
        if range.start < cursor || range.end > doc.source.len() {
            bail!("Reference definition removal ranges overlap or exceed source bounds");
        }
        output.push_str(doc.source.get(cursor..range.start).unwrap_or_default());
        cursor = range.end;
    }
    output.push_str(doc.source.get(cursor..).unwrap_or_default());
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::super::find_unused_refdefs;
    use super::*;

    /// Removes definitions identified as unused from the parsed document.
    #[test]
    fn test_delete_unused_refdefs() {
        let content = "# Title\n\n[Used][used]\n\n[used]: https://example.com\n[unused]: https://unused.com\n";
        let doc = parse_document("test.md", None, content).expect("parse failed");
        let unused = find_unused_refdefs(&doc);
        let cleaned = delete_unused_refdefs(&doc, &unused).expect("delete unused definitions");
        assert!(
            cleaned.contains("[used]: https://example.com"),
            "Cleaned content should contain [used] refdef, got:\n{cleaned}"
        );
        assert!(!cleaned.contains("[unused]: https://unused.com"));
    }

    /// Preserves CRLF and unrelated Markdown bytes while removing an unused definition.
    #[test]
    fn deleting_definitions_preserves_unrelated_source_bytes() -> Result<()> {
        let content = "[Rust](https://rust-lang.org/ \"Rust title\")\r\n\r\n[unused]: https://example.com/unused \"title\"\r\n";
        let doc = parse_document("test.md", None, content)?;
        let unused = find_unused_refdefs(&doc);
        let cleaned = delete_unused_refdefs(&doc, &unused)?;

        assert_eq!(
            cleaned,
            "[Rust](https://rust-lang.org/ \"Rust title\")\r\n\r\n"
        );
        Ok(())
    }
}
