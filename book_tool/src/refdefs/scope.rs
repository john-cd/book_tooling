//! Include-scope analysis across the Markdown files of a book.
//!
//! TODO Reviewed ??-??-2026.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::bail;
use walkdir::WalkDir;

use super::occurrences::ReferenceOccurrences;
use super::occurrences::collect_reference_occurrences;
use super::occurrences::next_character_len;
use super::report::ReferenceDefinitionAudit;
use super::report::ReferenceUseAudit;
use super::report::ScopedRefDefAnalysis;

/// A parsed Markdown file in the scanned roots.
struct MarkdownFile {
    /// Path of the file.
    path: PathBuf,
    /// The reference definitions and uses in the file.
    occurrences: ReferenceOccurrences,
    /// Identities of the files this file includes.
    includes: Vec<String>,
}

/// Normalizes a path to a string usable as a map key; lowercased on Windows, where paths are
/// case-insensitive.
pub(super) fn path_identity(path: &Path) -> String {
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

/// Returns the `{{#include}}` targets in `source` that are not inside code fences or HTML comments.
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
            if run >= minimum_length && trimmed.get(run..).unwrap_or_default().trim().is_empty() {
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

        let mut index = 0_usize;
        let mut code_span_length = 0_usize;
        while index < bytes.len() {
            if in_html_comment {
                if trimmed.get(index..).unwrap_or_default().starts_with("-->") {
                    in_html_comment = false;
                    index += 3;
                } else {
                    index += next_character_len(trimmed, index);
                }
                continue;
            }
            if trimmed.get(index..).unwrap_or_default().starts_with("<!--") {
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
            if code_span_length == 0
                && trimmed
                    .get(index..)
                    .unwrap_or_default()
                    .starts_with("{{#include")
            {
                let Some(close_offset) = trimmed.get(index..).unwrap_or_default().find("}}") else {
                    break;
                };
                let directive = trimmed.get(index..index + close_offset).unwrap_or_default();
                let token = directive
                    .strip_prefix("{{#include")
                    .unwrap_or_default()
                    .trim();
                let token = token.split_whitespace().next().unwrap_or_default();
                if let Some(path_end) = token.find(".md") {
                    includes.push(token.get(..path_end + 3).unwrap_or(token).to_owned());
                }
                index += close_offset + 2;
                continue;
            }
            index += next_character_len(trimmed, index);
        }
    }
    includes
}

/// Analyze reference labels separately for each Markdown file and its recursively included files.
///
/// Definitions are marked unused only when every containing scope parses completely. Missing
/// includes, traversal errors and cycles return an error so callers cannot apply partial cleanup.
///
/// # Errors
///
/// Returns an error if a root cannot be read or parsed, an include is missing, or the includes form
/// a cycle.
#[expect(
    clippy::too_many_lines,
    clippy::excessive_nesting,
    reason = "Builds the include graph and the per-scope analysis in one pass over the scanned roots."
)]
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

    let mut files = BTreeMap::<String, MarkdownFile>::new();
    for root in &roots {
        for entry in WalkDir::new(root) {
            let entry = entry.with_context(|| format!("Failed to traverse {}", root.display()))?;
            if (entry.file_type().is_dir() || entry.file_type().is_symlink())
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
                if !roots
                    .iter()
                    .any(|allowed_root| canonical_include.starts_with(allowed_root))
                {
                    bail!(
                        "Markdown include {} from {} is outside the scanned roots",
                        canonical_include.display(),
                        path.display()
                    );
                }
                let include_key = path_identity(&canonical_include);
                // The target may be visited later in the WalkDir traversal; retain its key either
                // way.
                includes.push(include_key);
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
    let mut roots_by_file = BTreeMap::<String, HashSet<String>>::new();
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

/// Depth-first walk of the include graph from `key`; fails on missing sources and include cycles.
fn collect_scope(
    key: &str,
    files: &BTreeMap<String, MarkdownFile>,
    active: &mut Vec<String>,
    visited: &mut Vec<String>,
) -> Result<()> {
    if active.iter().any(|ancestor| ancestor == key) {
        let cycle = active
            .iter()
            .cloned()
            .chain(std::iter::once(key.to_owned()))
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
    active.push(key.to_owned());
    visited.push(key.to_owned());
    for include in &file.includes {
        collect_scope(include, files, active, visited)?;
    }
    active.pop();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::ReferenceUseKind;
    use super::*;

    /// Keeps a shared definition when an including chapter references it.
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

    /// Reports the source path and line for every reference use.
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

    /// Keeps reference-definition namespaces separate across chapters.
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

    /// Rejects missing includes and cycles rather than returning partial analysis.
    #[test]
    fn scoped_analysis_rejects_missing_includes_and_cycles() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path();
        let chapter = source.join("chapter.md");

        std::fs::write(&chapter, "{{#include missing.md}}\n")?;
        find_unused_refdefs_in_scopes([source]).expect_err("input should be rejected");

        std::fs::write(source.join("chapter.md"), "{{#include other.md}}\n")?;
        std::fs::write(source.join("other.md"), "{{#include chapter.md}}\n")?;
        let error = find_unused_refdefs_in_scopes([source]).expect_err("cycle is incomplete");
        assert!(error.to_string().contains("cycle"));
        Ok(())
    }

    /// Ignores SUMMARY navigation links when building chapter include scopes.
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
}
