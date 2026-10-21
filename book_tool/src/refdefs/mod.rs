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
use serde::Serialize;
use walkdir::WalkDir;

use crate::common::replace_text_file_atomically;

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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceUseKind {
    Link,
    Image,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct ReferenceUseAudit {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub label: String,
    pub kind: ReferenceUseKind,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReferenceDefinitionAudit {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub label: String,
    pub normalized_label: String,
    /// Markdown roots whose include scope contains this definition.
    pub included_by: Vec<PathBuf>,
    /// Every matching reference link or image visible in those scopes.
    pub uses: Vec<ReferenceUseAudit>,
    /// Uses found in supplementary Markdown roots outside the analyzed include scopes.
    pub uses_outside_scoped_roots: Vec<ReferenceUseAudit>,
    /// True when multiple definitions share this label in any containing scope.
    pub ambiguous: bool,
    /// True when cleanup was suppressed because this definition file is protected.
    pub protected: bool,
    /// True only when the definition is unused and unambiguous in every scope.
    pub proven_unused: bool,
}

#[derive(Debug, Clone)]
struct DefinitionOccurrence {
    label: String,
    normalized_label: String,
    line: usize,
    column: usize,
}

#[derive(Debug, Clone)]
struct UseOccurrence {
    label: String,
    normalized_label: String,
    line: usize,
    column: usize,
    kind: ReferenceUseKind,
}

#[derive(Default)]
struct ReferenceOccurrences {
    definitions: Vec<DefinitionOccurrence>,
    uses: Vec<UseOccurrence>,
}

struct OccurrenceCollector<'source> {
    source: &'source str,
    source_base: usize,
    occurrences: ReferenceOccurrences,
    error: Option<anyhow::Error>,
}

impl OccurrenceCollector<'_> {
    fn location(&self, fragment: &str) -> Result<(usize, usize)> {
        let start = (fragment.as_ptr() as usize)
            .checked_sub(self.source_base)
            .context("Reference label does not belong to its source document")?;
        let end = start
            .checked_add(fragment.len())
            .context("Reference label source range overflowed")?;
        if end > self.source.len()
            || !self.source.is_char_boundary(start)
            || !self.source.is_char_boundary(end)
        {
            bail!("Reference label source range is outside its document");
        }

        let prefix = &self.source[..start];
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let column = prefix
            .rsplit('\n')
            .next()
            .map_or(1, |line_prefix| line_prefix.chars().count() + 1);
        Ok((line, column))
    }

    fn record_use(&mut self, label: &str, kind: ReferenceUseKind) {
        if self.error.is_some() {
            return;
        }
        match self.location(label) {
            Ok((line, column)) => self.occurrences.uses.push(UseOccurrence {
                label: label.to_string(),
                normalized_label: normalize_reference_label(label),
                line,
                column,
                kind,
            }),
            Err(error) => self.error = Some(error),
        }
    }

    fn record_definition(&mut self, label: &str) {
        if self.error.is_some() {
            return;
        }
        match self.location(label) {
            Ok((line, column)) => self.occurrences.definitions.push(DefinitionOccurrence {
                label: label.to_string(),
                normalized_label: normalize_reference_label(label),
                line,
                column,
            }),
            Err(error) => self.error = Some(error),
        }
    }
}

impl<'a> Visitor<'a> for OccurrenceCollector<'_> {
    fn visit_reference_style_link(&mut self, link: &'a ReferenceStyleLinkData<'a>) {
        self.record_use(link.label, ReferenceUseKind::Link);
    }

    fn visit_reference_style_image(&mut self, image: &'a ReferenceStyleImageData<'a>) {
        self.record_use(image.label, ReferenceUseKind::Image);
    }

    fn visit_reference_definition(&mut self, definition: &'a ReferenceDefinitionData<'a>) {
        self.record_definition(definition.label);
    }
}

fn next_character_len(text: &str, index: usize) -> usize {
    text.get(index..)
        .and_then(|suffix| suffix.chars().next())
        .map_or(1, char::len_utf8)
}

fn bracket_contents(line: &str, opening: usize) -> Option<(usize, usize, usize)> {
    let bytes = line.as_bytes();
    if bytes.get(opening) != Some(&b'[') {
        return None;
    }

    let mut depth = 1usize;
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

fn is_escaped(line: &str, opening: usize) -> bool {
    let bytes = line.as_bytes();
    let mut cursor = opening;
    let mut backslashes = 0usize;
    while cursor > 0 && bytes.get(cursor - 1) == Some(&b'\\') {
        backslashes += 1;
        cursor -= 1;
    }
    backslashes % 2 == 1
}

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
fn implicit_reference_uses(source: &str) -> Vec<(&str, ReferenceUseKind)> {
    let mut uses = Vec::new();
    let mut fence: Option<(u8, usize)> = None;
    let mut inline_code_length = 0usize;
    let mut in_html_comment = false;

    for raw_line in source.split_inclusive('\n') {
        let line = raw_line
            .strip_suffix('\n')
            .unwrap_or(raw_line)
            .strip_suffix('\r')
            .unwrap_or(raw_line.strip_suffix('\n').unwrap_or(raw_line));
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
        let mut index = 0usize;
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
                {
                    if let Some(end) = line.get(index..).and_then(|suffix| suffix.find("]]")) {
                        index += end + 2;
                        continue;
                    }
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

fn collect_reference_occurrences(path: &Path, source: &str) -> Result<ReferenceOccurrences> {
    let document = parse_document("file", Some(path), source)
        .with_context(|| format!("Failed to parse Markdown file {}", path.display()))?;
    let mut collector = OccurrenceCollector {
        source,
        source_base: source.as_ptr() as usize,
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

fn collect_uses_outside_scoped_roots<I, P>(
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
    occurrences: ReferenceOccurrences,
    includes: Vec<String>,
}

/// Unused definitions proven unused within every chapter/include scope in the scanned roots.
#[derive(Debug, Default)]
pub struct ScopedRefDefAnalysis {
    pub unused_by_file: HashMap<PathBuf, HashSet<String>>,
    pub definitions: Vec<ReferenceDefinitionAudit>,
}

/// Write a deterministic JSON report containing every definition and its scoped uses.
pub fn write_audit_report(path: &Path, analysis: &ScopedRefDefAnalysis) -> Result<()> {
    let mut report = serde_json::to_string_pretty(&analysis.definitions)
        .context("Failed to serialize reference-definition audit")?;
    report.push('\n');
    replace_text_file_atomically(path, &report).with_context(|| {
        format!(
            "Failed to write reference-definition report to {}",
            path.display()
        )
    })
}

/// Remove definitions identified as unused by a complete include-scope analysis.
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

/// Analyze reference definitions, save the report, and optionally remove proven-unused entries.
pub fn audit(
    directories: Vec<PathBuf>,
    report_path: &Path,
    delete_unused: bool,
    additional_use_directories: Vec<PathBuf>,
    protected_definition_files: Vec<PathBuf>,
) -> Result<()> {
    let analysis = find_unused_refdefs_in_scopes(&directories)?;
    let mut analysis = analysis;
    let additional_uses = collect_uses_outside_scoped_roots(additional_use_directories)?;
    let additional_labels = additional_uses.keys().cloned().collect::<HashSet<_>>();
    let protected_definition_files = protected_definition_files
        .into_iter()
        .map(|path| {
            path.canonicalize().with_context(|| {
                format!(
                    "Failed to resolve protected definition file {}",
                    path.display()
                )
            })
        })
        .map(|path| path.map(|path| path_identity(&path)))
        .collect::<Result<HashSet<_>>>()?;
    for unused_labels in analysis.unused_by_file.values_mut() {
        unused_labels
            .retain(|label| !additional_labels.contains(&normalize_reference_label(label)));
    }
    analysis.unused_by_file.retain(|path, labels| {
        !labels.is_empty() && !protected_definition_files.contains(&path_identity(path))
    });
    for definition in &mut analysis.definitions {
        definition.protected =
            protected_definition_files.contains(&path_identity(&definition.path));
        let scoped_uses = definition.uses.iter().cloned().collect::<HashSet<_>>();
        definition.uses_outside_scoped_roots = additional_uses
            .get(&definition.normalized_label)
            .into_iter()
            .flatten()
            .filter(|usage| !scoped_uses.contains(*usage))
            .cloned()
            .collect();
        definition.uses_outside_scoped_roots.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then_with(|| left.line.cmp(&right.line))
                .then_with(|| left.column.cmp(&right.column))
                .then_with(|| left.label.cmp(&right.label))
                .then_with(|| left.kind.cmp(&right.kind))
        });
        definition.proven_unused = analysis
            .unused_by_file
            .get(&definition.path)
            .is_some_and(|labels| labels.contains(&definition.label));
    }
    write_audit_report(report_path, &analysis)?;

    let unused_count = analysis
        .definitions
        .iter()
        .filter(|definition| definition.proven_unused)
        .count();
    println!(
        "Wrote {} definition records to {} ({} proven unused).",
        analysis.definitions.len(),
        report_path.display(),
        unused_count
    );

    if delete_unused {
        let changed_files = remove_proven_unused_refdefs(&analysis)?;
        println!("Removed proven-unused definitions from {changed_files} file(s).");
    }
    Ok(())
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
                || entry
                    .file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case("SUMMARY.md")
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
            let occurrences = collect_reference_occurrences(&path, &source)?;

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
                    occurrences,
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
    let mut ambiguous_definitions = HashSet::<(String, String)>::new();
    let mut roots_by_file = HashMap::<String, HashSet<String>>::new();
    let mut uses_by_definition = HashMap::<(String, String), HashSet<ReferenceUseAudit>>::new();
    for root_key in files.keys() {
        let mut active = Vec::new();
        let mut scope_files = Vec::new();
        collect_scope(root_key, &files, &mut active, &mut scope_files)?;

        let used = scope_files
            .iter()
            .flat_map(|key| {
                files.get(key).into_iter().flat_map(|file| {
                    file.occurrences
                        .uses
                        .iter()
                        .map(|usage| usage.normalized_label.clone())
                })
            })
            .collect::<HashSet<_>>();
        let mut uses_by_label = HashMap::<String, HashSet<ReferenceUseAudit>>::new();
        let mut definitions_in_scope = HashMap::<String, Vec<String>>::new();
        for key in &scope_files {
            if let Some(file) = files.get(key) {
                roots_by_file
                    .entry(key.clone())
                    .or_default()
                    .insert(root_key.clone());
                for usage in &file.occurrences.uses {
                    uses_by_label
                        .entry(usage.normalized_label.clone())
                        .or_default()
                        .insert(ReferenceUseAudit {
                            path: file.path.clone(),
                            line: usage.line,
                            column: usage.column,
                            label: usage.label.clone(),
                            kind: usage.kind.clone(),
                        });
                }
                for definition in &file.occurrences.definitions {
                    definitions_in_scope
                        .entry(definition.normalized_label.clone())
                        .or_default()
                        .push(key.clone());
                }
            }
        }

        for key in &scope_files {
            if let Some(file) = files.get(key) {
                for definition in &file.occurrences.definitions {
                    let identity = (key.clone(), definition.normalized_label.clone());
                    let owners = definitions_in_scope
                        .get(&definition.normalized_label)
                        .map_or(0, Vec::len);
                    if owners > 1 {
                        ambiguous_definitions.insert(identity.clone());
                    }
                    if used.contains(&definition.normalized_label) {
                        used_definitions.insert(identity.clone());
                        if let Some(uses) = uses_by_label.get(&definition.normalized_label) {
                            uses_by_definition
                                .entry(identity)
                                .or_default()
                                .extend(uses.iter().cloned());
                        }
                    } else if owners > 1 {
                        duplicate_definitions.insert(identity);
                    }
                }
            }
        }
    }

    let mut analysis = ScopedRefDefAnalysis::default();
    for (key, file) in &files {
        let unused = file
            .occurrences
            .definitions
            .iter()
            .filter_map(|definition| {
                let identity = (key.clone(), definition.normalized_label.clone());
                (!used_definitions.contains(&identity)
                    && !duplicate_definitions.contains(&identity))
                .then_some(definition.label.clone())
            })
            .collect::<HashSet<_>>();
        if !unused.is_empty() {
            analysis.unused_by_file.insert(file.path.clone(), unused);
        }
    }

    for (key, file) in &files {
        let mut included_by = roots_by_file
            .get(key)
            .into_iter()
            .flatten()
            .filter_map(|root| files.get(root).map(|root_file| root_file.path.clone()))
            .collect::<Vec<_>>();
        included_by.sort();
        for definition in &file.occurrences.definitions {
            let identity = (key.clone(), definition.normalized_label.clone());
            let mut uses = uses_by_definition
                .get(&identity)
                .into_iter()
                .flatten()
                .cloned()
                .collect::<Vec<_>>();
            uses.sort_by(|left, right| {
                left.path
                    .cmp(&right.path)
                    .then_with(|| left.line.cmp(&right.line))
                    .then_with(|| left.column.cmp(&right.column))
                    .then_with(|| left.label.cmp(&right.label))
                    .then_with(|| left.kind.cmp(&right.kind))
            });
            let proven_unused = analysis
                .unused_by_file
                .get(&file.path)
                .is_some_and(|labels| labels.contains(&definition.label));
            analysis.definitions.push(ReferenceDefinitionAudit {
                path: file.path.clone(),
                line: definition.line,
                column: definition.column,
                label: definition.label.clone(),
                normalized_label: definition.normalized_label.clone(),
                included_by: included_by.clone(),
                uses,
                uses_outside_scoped_roots: Vec::new(),
                ambiguous: ambiguous_definitions.contains(&identity),
                protected: false,
                proven_unused,
            });
        }
    }
    analysis.definitions.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.column.cmp(&right.column))
            .then_with(|| left.label.cmp(&right.label))
    });
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
    fn scoped_report_lists_definition_provenance_and_every_reference_use() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path();
        let chapter = source.join("chapter.md");
        let definitions = source.join("refs.incl.md");
        std::fs::write(
            &chapter,
            "{{#include refs.incl.md}}\n[Full][SERDE]\n[ SeRdE ][]\n[SERDE]\n![crate][serDe]\n![SeRdE][]\n![serde]\n[![crate][sErDe]][SERDE]\n",
        )?;
        std::fs::write(
            &definitions,
            "[serde]: https://docs.rs/serde\n[unused]: https://example.com/unused\n",
        )?;

        let analysis = find_unused_refdefs_in_scopes([source])?;
        let definitions_path = definitions.canonicalize()?;
        let chapter_path = chapter.canonicalize()?;
        let used = analysis
            .definitions
            .iter()
            .find(|definition| definition.path == definitions_path && definition.label == "serde")
            .expect("the shared serde definition is reported");
        assert!(!used.proven_unused);
        assert!(!used.ambiguous);
        assert!(used.included_by.contains(&chapter_path));
        assert_eq!(
            used.uses.iter().map(|usage| usage.line).collect::<Vec<_>>(),
            [2, 3, 4, 5, 6, 7, 8, 8],
            "uses: {:?}",
            used.uses
        );
        assert_eq!(
            used.uses
                .iter()
                .filter(|usage| usage.kind == ReferenceUseKind::Image)
                .count(),
            4
        );

        let unused = analysis
            .definitions
            .iter()
            .find(|definition| definition.path == definitions_path && definition.label == "unused")
            .expect("the unused definition is reported");
        assert!(unused.proven_unused);
        assert!(!unused.ambiguous);
        assert!(unused.uses.is_empty());
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
    fn summary_navigation_is_not_treated_as_a_chapter_include_scope() -> Result<()> {
        let temp = tempfile::tempdir()?;
        std::fs::write(
            temp.path().join("SUMMARY.md"),
            "{{#include refs.incl.md}}\n",
        )?;
        std::fs::write(temp.path().join("chapter.md"), "# Chapter\n")?;

        find_unused_refdefs_in_scopes([temp.path()])?;
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
