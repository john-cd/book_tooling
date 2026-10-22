//! The `refdefs` command: audit, report and optionally clean up reference definitions.
//!
//! TODO Reviewed ??-??-2026

use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;

use super::delete::remove_proven_unused_refdefs;
use super::normalize_reference_label;
use super::occurrences::collect_uses_outside_scoped_roots;
use super::report::write_audit_report;
use super::scope::find_unused_refdefs_in_scopes;
use super::scope::path_identity;

/// Analyze reference definitions, save the report, and optionally remove proven-unused entries.
///
/// # Errors
///
/// Returns an error if the roots cannot be analyzed, the report cannot be written, or cleanup fails.
pub fn audit(
    directories: &[PathBuf],
    report_path: &Path,
    delete_unused: bool,
    additional_use_directories: Vec<PathBuf>,
    protected_definition_files: Vec<PathBuf>,
) -> Result<()> {
    let analysis = find_unused_refdefs_in_scopes(directories)?;
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
