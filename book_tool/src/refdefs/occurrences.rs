//! Locates reference definitions and uses, with source positions, in Markdown files.
//!
//! TODO Reviewed ??-??-2026.

use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;

use anyhow::Context as _;
use anyhow::Result;
use parse_lib::ReferenceDefinitionData;
use parse_lib::ReferenceStyleImageData;
use parse_lib::ReferenceStyleLinkData;
use parse_lib::parse_document;
use parse_lib::visit::Visitable as _;
use parse_lib::visit::Visitor;
use walkdir::WalkDir;

use super::normalize_reference_label;
use super::report::ReferenceUseAudit;
use super::report::ReferenceUseKind;

/// Where a reference definition appears in a source file.
#[derive(Debug, Clone)]
pub(super) struct DefinitionOccurrence {
    /// Label as written in the source.
    pub(super) label: String,
    /// Label normalized for case-insensitive matching.
    pub(super) normalized_label: String,
    /// 1-based line of the label.
    pub(super) line: usize,
    /// 1-based column of the label.
    pub(super) column: usize,
}

/// Where a reference link or image appears in a source file.
#[derive(Debug, Clone)]
pub(super) struct UseOccurrence {
    /// Label as written in the source.
    pub(super) label: String,
    /// Label normalized for case-insensitive matching.
    pub(super) normalized_label: String,
    /// 1-based line of the label.
    pub(super) line: usize,
    /// 1-based column of the label.
    pub(super) column: usize,
    /// Whether the use is a link or an image.
    pub(super) kind: ReferenceUseKind,
}

/// Every reference definition and use found in one source file.
#[derive(Default)]
pub(super) struct ReferenceOccurrences {
    /// The reference definitions in the file.
    pub(super) definitions: Vec<DefinitionOccurrence>,
    /// The reference links and images in the file.
    pub(super) uses: Vec<UseOccurrence>,
}

/// Visitor that records reference definitions and uses together with their source locations.
struct OccurrenceCollector<'source> {
    /// The source text being scanned.
    source: &'source str,
    /// The reference definitions and uses in the file.
    occurrences: ReferenceOccurrences,
    /// The first error hit while locating a fragment; later recording stops once it is set.
    error: Option<anyhow::Error>,
}

impl OccurrenceCollector<'_> {
    /// Returns the 1-based line and column of `fragment` within the source.
    fn location(&self, fragment: &str) -> Result<(usize, usize)> {
        let start = parse_lib::source_span(self.source, fragment)
            .context("Reference label does not belong to its source document")?
            .start;

        let prefix = self.source.get(..start).unwrap_or_default();
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let column = prefix
            .rsplit('\n')
            .next()
            .map_or(1, |line_prefix| line_prefix.chars().count() + 1);
        Ok((line, column))
    }

    /// Records a reference link or image that uses `label`, unless an earlier error was stored.
    fn record_use(&mut self, label: &str, kind: ReferenceUseKind) {
        if self.error.is_some() {
            return;
        }
        match self.location(label) {
            Ok((line, column)) => self.occurrences.uses.push(UseOccurrence {
                label: label.to_owned(),
                normalized_label: normalize_reference_label(label),
                line,
                column,
                kind,
            }),
            Err(error) => self.error = Some(error),
        }
    }

    /// Records a reference definition of `label`, unless an earlier error was stored.
    fn record_definition(&mut self, label: &str) {
        if self.error.is_some() {
            return;
        }
        match self.location(label) {
            Ok((line, column)) => self.occurrences.definitions.push(DefinitionOccurrence {
                label: label.to_owned(),
                normalized_label: normalize_reference_label(label),
                line,
                column,
            }),
            Err(error) => self.error = Some(error),
        }
    }
}

impl<'a> Visitor<'a> for OccurrenceCollector<'_> {
    /// Records a use of the link's label, with its source position.
    fn visit_reference_style_link(&mut self, link: &'a ReferenceStyleLinkData<'a>) {
        self.record_use(link.label, ReferenceUseKind::Link);
    }

    /// Records a use of the image's label, with its source position.
    fn visit_reference_style_image(&mut self, image: &'a ReferenceStyleImageData<'a>) {
        self.record_use(image.label, ReferenceUseKind::Image);
    }

    /// Records the definition's label with its source position.
    fn visit_reference_definition(&mut self, definition: &'a ReferenceDefinitionData<'a>) {
        self.record_definition(definition.label);
    }
}

/// Returns the UTF-8 length of the character starting at byte `index`, or 1 if there is none.
pub(super) fn next_character_len(text: &str, index: usize) -> usize {
    text.get(index..)
        .and_then(|suffix| suffix.chars().next())
        .map_or(1, char::len_utf8)
}

/// Finds the bracket pair that opens at `opening`, honoring nesting and backslash escapes; returns
/// `(start, end, closing)` byte offsets.
fn bracket_contents(line: &str, opening: usize) -> Option<(usize, usize, usize)> {
    let bytes = line.as_bytes();
    if bytes.get(opening) != Some(&b'[') {
        return None;
    }

    let mut depth = 1_usize;
    let mut index = opening + 1;
    while index < bytes.len() {
        match bytes.get(index).copied()? {
            b'\\' => {
                index += 1;
                if index < bytes.len() {
                    index += next_character_len(line, index);
                }
            }
            b'[' => {
                depth += 1;
                index += 1;
            }
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some((opening + 1, index, index));
                }
                index += 1;
            }
            _ => index += next_character_len(line, index),
        }
    }
    None
}

/// Returns whether the character at `opening` is preceded by an odd number of backslashes.
fn is_escaped(line: &str, opening: usize) -> bool {
    let bytes = line.as_bytes();
    let mut cursor = opening;
    let mut backslashes = 0_usize;
    while cursor > 0 && bytes.get(cursor - 1) == Some(&b'\\') {
        backslashes += 1;
        cursor -= 1;
    }
    (backslashes & 1) == 1
}

/// Returns the first byte offset at or after `index` that is not a space or tab.
fn skip_spaces(line: &str, mut index: usize) -> usize {
    let bytes = line.as_bytes();
    while bytes
        .get(index)
        .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
    {
        index += 1;
    }
    index
}

/// Finds shortcut and collapsed references left as plain text by the lightweight AST parser.
/// Full references are collected by the visitor; this scan also recognizes them so whitespace
/// accepted by mdBook does not hide a use from the cleanup analysis.
#[expect(
    clippy::too_many_lines,
    clippy::excessive_nesting,
    reason = "A single-pass scanner whose branches follow the Markdown reference syntax; splitting it would obscure the state it threads through."
)]
fn implicit_reference_uses(source: &str) -> Vec<(&str, ReferenceUseKind)> {
    let mut uses = Vec::new();
    let mut fence: Option<(u8, usize)> = None;
    let mut inline_code_length = 0_usize;
    let mut in_html_comment = false;

    for raw_line in source.split_inclusive('\n') {
        let without_newline = raw_line.strip_suffix('\n').unwrap_or(raw_line);
        let line = without_newline
            .strip_suffix('\r')
            .unwrap_or(without_newline);
        let trimmed = line.trim_start();
        let trimmed_bytes = trimmed.as_bytes();

        if let Some((marker, minimum_length)) = fence {
            let run = trimmed_bytes
                .iter()
                .take_while(|byte| **byte == marker)
                .count();
            if run >= minimum_length
                && trimmed
                    .get(run..)
                    .is_some_and(|suffix| suffix.trim().is_empty())
            {
                fence = None;
            }
            continue;
        }

        if let Some(&marker @ (b'`' | b'~')) = trimmed_bytes.first() {
            let run = trimmed_bytes
                .iter()
                .take_while(|byte| **byte == marker)
                .count();
            if run >= 3 {
                fence = Some((marker, run));
                continue;
            }
        }

        let bytes = line.as_bytes();
        let mut index = 0_usize;
        while index < bytes.len() {
            if in_html_comment {
                if line
                    .get(index..)
                    .is_some_and(|suffix| suffix.starts_with("-->"))
                {
                    in_html_comment = false;
                    index += 3;
                } else {
                    index += next_character_len(line, index);
                }
                continue;
            }
            if line
                .get(index..)
                .is_some_and(|suffix| suffix.starts_with("<!--"))
            {
                in_html_comment = true;
                index += 4;
                continue;
            }

            if bytes.get(index) == Some(&b'`') {
                let run = bytes
                    .get(index..)
                    .unwrap_or_default()
                    .iter()
                    .take_while(|byte| **byte == b'`')
                    .count();
                inline_code_length = if inline_code_length == run {
                    0
                } else if inline_code_length == 0 {
                    run
                } else {
                    inline_code_length
                };
                index += run;
                continue;
            }

            if inline_code_length == 0
                && bytes.get(index) == Some(&b'[')
                && !is_escaped(line, index)
            {
                if line
                    .get(index..)
                    .is_some_and(|suffix| suffix.starts_with("[["))
                    && let Some(end) = line.get(index..).and_then(|suffix| suffix.find("]]"))
                {
                    index += end + 2;
                    continue;
                }

                if let Some((label_start, label_end, close)) = bracket_contents(line, index) {
                    let after_text = skip_spaces(line, close + 1);
                    let after_text_slice = line.get(after_text..).unwrap_or_default();
                    if after_text_slice.starts_with(':') {
                        index = close + 1;
                        continue;
                    }

                    let image = index > 0
                        && bytes.get(index - 1) == Some(&b'!')
                        && !is_escaped(line, index - 1);
                    let kind = if image {
                        ReferenceUseKind::Image
                    } else {
                        ReferenceUseKind::Link
                    };

                    let visible_text = line.get(label_start..label_end).unwrap_or_default();
                    let (label, next_index) = if after_text_slice.starts_with('[') {
                        if let Some((second_start, second_end, second_close)) =
                            bracket_contents(line, after_text)
                        {
                            let second_label =
                                line.get(second_start..second_end).unwrap_or_default();
                            if second_label.trim().is_empty() {
                                (
                                    line.get(label_start..label_end).unwrap_or_default(),
                                    second_close + 1,
                                )
                            } else {
                                if after_text > close + 1 {
                                    // `[a] [b]` is a full reference for tolerant renderers but
                                    // two shortcut references under CommonMark; keep both
                                    // labels alive so cleanup never deletes a definition that
                                    // either reading uses.
                                    uses.push((
                                        line.get(label_start..label_end).unwrap_or_default(),
                                        kind.clone(),
                                    ));
                                }
                                (second_label, second_close + 1)
                            }
                        } else {
                            (
                                line.get(label_start..label_end).unwrap_or_default(),
                                close + 1,
                            )
                        }
                    } else if after_text_slice.starts_with('(') {
                        ("", close + 1)
                    } else {
                        (
                            line.get(label_start..label_end).unwrap_or_default(),
                            close + 1,
                        )
                    };

                    if !label.trim().is_empty() {
                        uses.push((label, kind));
                    }
                    // The outer bracket range can contain nested references. Inspect its
                    // visible text separately to retain image badges and links in draft notes.
                    uses.extend(implicit_reference_uses(visible_text));
                    index = next_index;
                    continue;
                }
            }

            index += next_character_len(line, index);
        }
    }
    uses
}

/// Parses `source` and collects its reference definitions and uses, including implicit uses.
pub(super) fn collect_reference_occurrences(
    path: &Path,
    source: &str,
) -> Result<ReferenceOccurrences> {
    let document = parse_document("file", Some(path), source)
        .with_context(|| format!("Failed to parse Markdown file {}", path.display()))?;
    let mut collector = OccurrenceCollector {
        source,
        occurrences: ReferenceOccurrences::default(),
        error: None,
    };
    document.accept(&mut collector);
    for (label, kind) in implicit_reference_uses(source) {
        collector.record_use(label, kind);
    }
    let mut seen_uses = HashSet::new();
    collector.occurrences.uses.retain(|usage| {
        seen_uses.insert((
            usage.line,
            usage.column,
            usage.label.clone(),
            usage.kind.clone(),
        ))
    });
    if let Some(error) = collector.error {
        return Err(error)
            .with_context(|| format!("Failed to locate reference in {}", path.display()));
    }
    Ok(collector.occurrences)
}

/// Collects reference uses found in supplementary Markdown roots, grouped by normalized label.
pub(super) fn collect_uses_outside_scoped_roots<I, P>(
    directories: I,
) -> Result<HashMap<String, HashSet<ReferenceUseAudit>>>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut uses_by_label = HashMap::<String, HashSet<ReferenceUseAudit>>::new();
    for directory in directories {
        let root = directory.as_ref().canonicalize().with_context(|| {
            format!(
                "Failed to resolve additional Markdown root {}",
                directory.as_ref().display()
            )
        })?;
        for entry in WalkDir::new(&root) {
            let entry = entry.with_context(|| format!("Failed to traverse {}", root.display()))?;
            if (entry.file_type().is_dir() || entry.file_type().is_symlink())
                || !entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension.to_string_lossy().eq_ignore_ascii_case("md"))
            {
                continue;
            }

            let path = entry.path().canonicalize().with_context(|| {
                format!("Failed to resolve Markdown file {}", entry.path().display())
            })?;
            let source = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read Markdown file {}", path.display()))?;
            let occurrences = collect_reference_occurrences(&path, &source)?;
            for usage in occurrences.uses {
                uses_by_label
                    .entry(usage.normalized_label)
                    .or_default()
                    .insert(ReferenceUseAudit {
                        path: path.clone(),
                        line: usage.line,
                        column: usage.column,
                        label: usage.label,
                        kind: usage.kind,
                    });
            }
        }
    }
    Ok(uses_by_label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_and_collapsed_references_are_uses() {
        let found = implicit_reference_uses("See [serde] and [Tokio][].\n");
        let names: Vec<_> = found.iter().map(|(label, _)| *label).collect();
        assert_eq!(names, ["serde", "Tokio"]);
        assert!(
            found
                .iter()
                .all(|(_, kind)| *kind == ReferenceUseKind::Link)
        );
    }

    #[test]
    fn image_shortcuts_are_image_uses() {
        let found = implicit_reference_uses("![diagram]\n");
        assert_eq!(found, [("diagram", ReferenceUseKind::Image)]);
    }

    #[test]
    fn fenced_code_html_comments_and_inline_code_are_ignored() {
        let source = "```\n[fenced]\n```\n<!-- [commented] -->\n`[inline]`\n[real]\n";
        let names: Vec<_> = implicit_reference_uses(source)
            .into_iter()
            .map(|(label, _)| label)
            .collect();
        assert_eq!(names, ["real"]);
    }

    #[test]
    fn escaped_brackets_are_not_uses() {
        assert!(implicit_reference_uses(r"\[escaped]").is_empty());
    }

    #[test]
    fn crlf_line_endings_do_not_leak_into_labels() {
        let names: Vec<_> = implicit_reference_uses("[one]\r\n[two]\r\n")
            .into_iter()
            .map(|(label, _)| label)
            .collect();
        assert_eq!(names, ["one", "two"]);
    }

    #[test]
    fn occurrences_cover_lists_quotes_and_tables_without_duplicates() {
        let source = "- item [a]\n> quote [b][]\n\n| h |\n|---|\n| [c][C] |\n\n[a]: https://a\n[B]: https://b\n";
        let occurrences = collect_reference_occurrences(Path::new("t.md"), source).expect("parses");
        let mut used: Vec<_> = occurrences
            .uses
            .iter()
            .map(|usage| usage.normalized_label.as_str())
            .collect();
        used.sort_unstable();
        used.dedup();
        assert_eq!(used, ["a", "b", "c"]);
        let defined: Vec<_> = occurrences
            .definitions
            .iter()
            .map(|definition| definition.normalized_label.as_str())
            .collect();
        assert_eq!(defined, ["a", "b"]);
        let mut seen = HashSet::new();
        for usage in &occurrences.uses {
            assert!(
                seen.insert((usage.line, usage.column, usage.label.clone())),
                "duplicate use {usage:?}"
            );
        }
        assert!(
            occurrences
                .uses
                .iter()
                .all(|usage| usage.line >= 1 && usage.column >= 1)
        );
    }

    #[test]
    fn additional_roots_group_uses_by_normalized_label() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("one.md"), "[Foo  Bar]\n").expect("write");
        std::fs::write(dir.path().join("skip.txt"), "[foo bar]\n").expect("write");
        let uses = collect_uses_outside_scoped_roots([dir.path()]).expect("collects");
        assert_eq!(uses.len(), 1);
        assert_eq!(uses.get("foo bar").expect("label grouped").len(), 1);
    }

    #[test]
    fn missing_additional_root_is_an_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        let err =
            collect_uses_outside_scoped_roots([dir.path().join("nope")]).expect_err("missing root");
        assert!(
            format!("{err:#}").contains("additional Markdown root"),
            "{err:#}"
        );
    }

    /// A shortcut reference followed by a space and another bracket keeps both labels in use.
    #[test]
    fn spaced_bracket_pairs_keep_both_labels_in_use() {
        let names: Vec<_> = implicit_reference_uses("a [Used] [other] [text][y]")
            .into_iter()
            .map(|(label, _)| label)
            .collect();
        assert_eq!(names, ["Used", "other", "y"]);
    }
}
