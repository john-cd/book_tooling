//! Reference-definition analysis and cleanup helpers.

use std::collections::HashMap;
use std::collections::HashSet;
use std::ops::Range;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use parse_lib::visit::Visitable;
use parse_lib::visit::Visitor;
use parse_lib::*;
use walkdir::WalkDir;

/// Normalize a Markdown reference label by collapsing whitespace and folding case.
pub fn normalize_reference_label(label: &str) -> String {
    label
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[derive(Default)]
struct LabelSets {
    used: HashSet<String>,
    definitions: Vec<(String, String)>,
}

impl<'a> Visitor<'a> for LabelSets {
    fn visit_reference_style_link(&mut self, link: &'a ReferenceStyleLinkData<'a>) {
        self.used.insert(normalize_reference_label(link.label));
    }

    fn visit_reference_style_image(&mut self, image: &'a ReferenceStyleImageData<'a>) {
        self.used.insert(normalize_reference_label(image.label));
    }

    fn visit_reference_definition(&mut self, definition: &'a ReferenceDefinitionData<'a>) {
        self.definitions.push((
            definition.label.to_string(),
            normalize_reference_label(definition.label),
        ));
    }
}

struct MarkdownFile {
    path: PathBuf,
    labels: LabelSets,
    includes: Vec<String>,
}

/// Unused definitions proven unused within every chapter/include scope in the scanned roots.
#[derive(Debug, Default)]
pub struct ScopedRefDefAnalysis {
    pub unused_by_file: HashMap<PathBuf, HashSet<String>>,
}

fn path_identity(path: &Path) -> String {
    let identity = path.to_string_lossy().replace('\\', "/");
    #[cfg(windows)]
    {
        identity.to_lowercase()
    }
    #[cfg(not(windows))]
    {
        identity
    }
}

fn active_markdown_includes(source: &str) -> Vec<String> {
    let mut includes = Vec::new();
    let mut fence: Option<(u8, usize)> = None;
    let mut in_html_comment = false;

    for line in source.lines() {
        let trimmed = line.trim_start();
        if let Some((marker, minimum_length)) = fence {
            let run = trimmed
                .as_bytes()
                .iter()
                .take_while(|byte| **byte == marker)
                .count();
            if run >= minimum_length && trimmed[run..].trim().is_empty() {
                fence = None;
            }
            continue;
        }

        let bytes = trimmed.as_bytes();
        if let Some(&marker @ (b'`' | b'~')) = bytes.first() {
            let run = bytes.iter().take_while(|byte| **byte == marker).count();
            if run >= 3 {
                fence = Some((marker, run));
                continue;
            }
        }

        let mut index = 0usize;
        let mut code_span_length = 0usize;
        while index < bytes.len() {
            if in_html_comment {
                if trimmed[index..].starts_with("-->") {
                    in_html_comment = false;
                    index += 3;
                } else {
                    index += trimmed[index..].chars().next().map_or(1, char::len_utf8);
                }
                continue;
            }
            if trimmed[index..].starts_with("<!--") {
                in_html_comment = true;
                index += 4;
                continue;
            }
            if bytes[index] == b'`' {
                let run = bytes[index..]
                    .iter()
                    .take_while(|byte| **byte == b'`')
                    .count();
                code_span_length = if code_span_length == run {
                    0
                } else if code_span_length == 0 {
                    run
                } else {
                    code_span_length
                };
                index += run;
                continue;
            }
            if code_span_length == 0 && trimmed[index..].starts_with("{{#include") {
                let Some(close_offset) = trimmed[index..].find("}}") else {
                    break;
                };
                let directive = &trimmed[index..index + close_offset];
                let token = directive
                    .strip_prefix("{{#include")
                    .unwrap_or_default()
                    .trim();
                let token = token.split_whitespace().next().unwrap_or_default();
                if let Some(path_end) = token.find(".md") {
                    includes.push(token[..path_end + 3].to_string());
                }
                index += close_offset + 2;
                continue;
            }
            index += trimmed[index..].chars().next().map_or(1, char::len_utf8);
        }
    }
    includes
}

/// Analyze reference labels separately for each Markdown file and its recursively included files.
///
/// Definitions are marked unused only when every containing scope parses completely. Missing
/// includes, traversal errors and cycles return an error so callers cannot apply partial cleanup.
pub fn find_unused_refdefs_in_scopes<I, P>(directories: I) -> Result<ScopedRefDefAnalysis>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let roots = directories
        .into_iter()
        .map(|directory| {
            directory.as_ref().canonicalize().with_context(|| {
                format!(
                    "Failed to resolve Markdown root {}",
                    directory.as_ref().display()
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let mut files = HashMap::<String, MarkdownFile>::new();
    for root in &roots {
        for entry in WalkDir::new(root) {
            let entry = entry.with_context(|| format!("Failed to traverse {}", root.display()))?;
            if !entry.file_type().is_file()
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
            let key = path_identity(&path);
            if files.contains_key(&key) {
                continue;
            }
            let source = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read Markdown file {}", path.display()))?;
            let document = parse_document("file", Some(&path), &source)
                .with_context(|| format!("Failed to parse Markdown file {}", path.display()))?;
            let mut labels = LabelSets::default();
            document.accept(&mut labels);

            let mut includes = Vec::new();
            for include in active_markdown_includes(&source) {
                let included_path = path
                    .parent()
                    .context("Expected including Markdown file parent")?
                    .join(&include);
                let canonical_include = included_path.canonicalize().with_context(|| {
                    format!(
                        "Failed to resolve Markdown include {include} from {}",
                        path.display()
                    )
                })?;
                if !roots.iter().any(|root| canonical_include.starts_with(root)) {
                    bail!(
                        "Markdown include {} from {} is outside the scanned roots",
                        canonical_include.display(),
                        path.display()
                    );
                }
                let include_key = path_identity(&canonical_include);
                if !files.contains_key(&include_key) {
                    // The target may be visited later in the WalkDir traversal; retain its key.
                    includes.push(include_key);
                } else {
                    includes.push(include_key);
                }
            }

            files.insert(
                key,
                MarkdownFile {
                    path,
                    labels,
                    includes,
                },
            );
        }
    }

    for (key, file) in &files {
        for include in &file.includes {
            if !files.contains_key(include) {
                bail!(
                    "Markdown include referenced by {} is missing from the scanned source tree",
                    file.path.display()
                );
            }
        }
        if !files.contains_key(key) {
            bail!("Internal Markdown scope index is inconsistent");
        }
    }

    let mut used_definitions = HashSet::<(String, String)>::new();
    let mut duplicate_definitions = HashSet::<(String, String)>::new();
    for root_key in files.keys() {
        let mut active = Vec::new();
        let mut scope_files = Vec::new();
        collect_scope(root_key, &files, &mut active, &mut scope_files)?;

        let used = scope_files
            .iter()
            .flat_map(|key| {
                files
                    .get(key)
                    .into_iter()
                    .flat_map(|file| file.labels.used.iter().cloned())
            })
            .collect::<HashSet<_>>();
        let mut definitions_in_scope = HashMap::<String, Vec<String>>::new();
        for key in &scope_files {
            if let Some(file) = files.get(key) {
                for (_, normalized) in &file.labels.definitions {
                    definitions_in_scope
                        .entry(normalized.clone())
                        .or_default()
                        .push(key.clone());
                }
            }
        }

        for key in &scope_files {
            if let Some(file) = files.get(key) {
                for (_, normalized) in &file.labels.definitions {
                    let owners = definitions_in_scope.get(normalized).map_or(0, Vec::len);
                    if used.contains(normalized) {
                        used_definitions.insert((key.clone(), normalized.clone()));
                    } else if owners > 1 {
                        duplicate_definitions.insert((key.clone(), normalized.clone()));
                    }
                }
            }
        }
    }

    let mut analysis = ScopedRefDefAnalysis::default();
    for (key, file) in files {
        let unused = file
            .labels
            .definitions
            .into_iter()
            .filter_map(|(label, normalized)| {
                let identity = (key.clone(), normalized);
                (!used_definitions.contains(&identity)
                    && !duplicate_definitions.contains(&identity))
                .then_some(label)
            })
            .collect::<HashSet<_>>();
        if !unused.is_empty() {
            analysis.unused_by_file.insert(file.path, unused);
        }
    }
    Ok(analysis)
}

fn collect_scope(
    key: &str,
    files: &HashMap<String, MarkdownFile>,
    active: &mut Vec<String>,
    visited: &mut Vec<String>,
) -> Result<()> {
    if active.iter().any(|ancestor| ancestor == key) {
        let cycle = active
            .iter()
            .cloned()
            .chain(std::iter::once(key.to_string()))
            .collect::<Vec<_>>()
            .join(" -> ");
        bail!("Recursive Markdown include cycle: {cycle}");
    }
    if visited.iter().any(|visited_key| visited_key == key) {
        return Ok(());
    }
    let file = files
        .get(key)
        .with_context(|| format!("Missing Markdown source in include scope: {key}"))?;
    active.push(key.to_string());
    visited.push(key.to_string());
    for include in &file.includes {
        collect_scope(include, files, active, visited)?;
    }
    active.pop();
    Ok(())
}

/// Find labels that are defined but never used in links or images.
pub fn find_unused_refdefs(doc: &Document) -> HashSet<String> {
    let mut defined_labels = HashMap::<String, (String, usize)>::new();

    let mut labels = LabelSets::default();
    doc.accept(&mut labels);
    let used_labels = labels.used;
    for (label, normalized) in labels.definitions {
        let entry = defined_labels
            .entry(normalized)
            .or_insert_with(|| (label.clone(), 0));
        entry.1 += 1;
    }

    defined_labels
        .into_iter()
        .filter_map(|(normalized, (label, count))| {
            (count == 1 && !used_labels.contains(&normalized)).then_some(label)
        })
        .collect()
}

/// Remove only proven-unused reference-definition source ranges.
pub fn delete_unused_refdefs(doc: &Document, unused_labels: &HashSet<String>) -> Result<String> {
    struct RemovalCollector<'a> {
        source: &'a str,
        source_base: usize,
        unused: &'a HashSet<String>,
        ranges: Vec<Range<usize>>,
        error: Option<anyhow::Error>,
    }

    impl RemovalCollector<'_> {
        fn source_range(&self, fragment: &str) -> Result<Range<usize>> {
            let start = (fragment.as_ptr() as usize)
                .checked_sub(self.source_base)
                .context("Reference definition does not belong to its source document")?;
            let end = start
                .checked_add(fragment.len())
                .context("Reference definition source range overflowed")?;
            if end > self.source.len()
                || !self.source.is_char_boundary(start)
                || !self.source.is_char_boundary(end)
            {
                bail!("Reference definition source range is outside its document");
            }
            Ok(start..end)
        }
    }

    impl<'a> Visitor<'a> for RemovalCollector<'_> {
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
            let line_start = self.source[..label_range.start]
                .rfind('\n')
                .map_or(0, |newline| newline + 1);
            let line_end = self.source[end..]
                .find('\n')
                .map_or(self.source.len(), |newline| end + newline + 1);
            self.ranges.push(line_start..line_end);
        }
    }

    let mut collector = RemovalCollector {
        source: doc.source,
        source_base: doc.source.as_ptr() as usize,
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
        output.push_str(&doc.source[cursor..range.start]);
        cursor = range.end;
    }
    output.push_str(&doc.source[cursor..]);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use parse_lib::parse_document;

    use super::*;

    #[test]
    fn test_unused_refdefs() {
        let content = "# Title\n\n[Used][used]\n\n[used]: https://example.com\n[unused]: https://unused.com\n";
        let doc = parse_document("test.md", None, content).expect("parse failed");
        let unused = find_unused_refdefs(&doc);
        assert!(
            unused.contains("unused"),
            "Unused labels should contain 'unused', got: {:?}",
            unused
        );
        assert_eq!(unused.len(), 1);
    }

    #[test]
    fn test_delete_unused_refdefs() {
        let content = "# Title\n\n[Used][used]\n\n[used]: https://example.com\n[unused]: https://unused.com\n";
        let doc = parse_document("test.md", None, content).expect("parse failed");
        let unused = find_unused_refdefs(&doc);
        let cleaned = delete_unused_refdefs(&doc, &unused).expect("delete unused definitions");
        assert!(
            cleaned.contains("[used]: https://example.com"),
            "Cleaned content should contain [used] refdef, got:\n{}",
            cleaned
        );
        assert!(!cleaned.contains("[unused]: https://unused.com"));
    }

    #[test]
    fn labels_match_case_insensitively_and_collapse_whitespace() {
        let content = "[Full][SeRdE] [serde][] [SERDE] ![crate][ SErDe ]\n[  seRde   ]: https://example.com\n";
        let doc = parse_document("test.md", None, content).expect("parse label variants");

        assert!(find_unused_refdefs(&doc).is_empty());
    }

    #[test]
    fn duplicate_labels_are_preserved_as_ambiguous() {
        let content = "[Use][x]\n[x]: https://one.example\n[X]: https://two.example\n";
        let doc = parse_document("test.md", None, content).expect("parse duplicate definitions");

        assert!(find_unused_refdefs(&doc).is_empty());
    }

    #[test]
    fn scoped_analysis_keeps_shared_definitions_used_by_an_including_chapter() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path();
        std::fs::write(
            source.join("chapter.md"),
            "{{#include refs.incl.md}}\n\n[Crate][SeRdE]\n",
        )?;
        std::fs::write(
            source.join("refs.incl.md"),
            "[serde]: https://docs.rs/serde\n[unused]: https://example.com/unused\n",
        )?;

        let analysis = find_unused_refdefs_in_scopes([source])?;
        let refs_path = source.join("refs.incl.md").canonicalize()?;
        let unused = analysis
            .unused_by_file
            .get(&refs_path)
            .expect("one definition is unused");
        assert!(unused.contains("unused"));
        assert!(!unused.contains("serde"));
        Ok(())
    }

    #[test]
    fn scoped_analysis_keeps_chapter_namespaces_separate() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path();
        std::fs::write(source.join("chapter-a.md"), "{{#include a.incl.md}}\n")?;
        std::fs::write(source.join("chapter-b.md"), "[Local][same]\n")?;
        std::fs::write(source.join("a.incl.md"), "[same]: https://one.example\n")?;

        let analysis = find_unused_refdefs_in_scopes([source])?;
        let defs = source.join("a.incl.md").canonicalize()?;
        assert!(analysis.unused_by_file[&defs].contains("same"));
        Ok(())
    }

    #[test]
    fn scoped_analysis_rejects_missing_includes_and_cycles() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path();
        let chapter = source.join("chapter.md");

        std::fs::write(&chapter, "{{#include missing.md}}\n")?;
        assert!(find_unused_refdefs_in_scopes([source]).is_err());

        std::fs::write(source.join("chapter.md"), "{{#include other.md}}\n")?;
        std::fs::write(source.join("other.md"), "{{#include chapter.md}}\n")?;
        let error = find_unused_refdefs_in_scopes([source]).expect_err("cycle is incomplete");
        assert!(error.to_string().contains("cycle"));
        Ok(())
    }

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
