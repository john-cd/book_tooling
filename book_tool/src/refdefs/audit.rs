//! The `refdefs` command: audit, report and optionally clean up reference definitions.
//!
//! TODO Reviewed ??-??-2026.

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
/// Returns an error if the roots cannot be analyzed, the report cannot be written, or cleanup
/// fails.
pub fn audit(
    directories: &[PathBuf],
    report_path: &Path,
    delete_unused: bool,
    additional_use_directories: Vec<PathBuf>,
    protected_definition_files: Vec<PathBuf>,
) -> Result<()> {
    let analysis = find_unused_refdefs_in_scopes(directories)?;
    let mut analysis = analysis;
    let additional_uses = collect_uses_outside_scoped_roots(&additional_use_directories)?;
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
        anyhow::ensure!(
            collect_uses_outside_scoped_roots(&additional_use_directories)? == additional_uses,
            "Supplementary references changed after analysis. Re-run the audit."
        );
        let changed_files = remove_proven_unused_refdefs(&analysis)?;
        println!("Removed proven-unused definitions from {changed_files} file(s).");
    }
    Ok(())
}

/// Tests for audit scopes, protected definitions, and deletion safeguards.
#[cfg(test)]
mod tests {
    use super::*;

    /// Reads the generated audit report as JSON.
    ///
    /// # Errors
    /// Propagates filesystem, UTF-8 decoding, and JSON parsing errors.
    fn read_report(path: &Path) -> Result<serde_json::Value> {
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }
    /// A definition file reached through a chain of includes is attributed to the root chapter.
    #[test]
    fn provenance_follows_chained_includes() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        std::fs::write(
            root.join("chapter.md"),
            "{{#include middle.incl.md}}\n[Used]\n",
        )?;
        std::fs::write(root.join("middle.incl.md"), "{{#include refs.incl.md}}\n")?;
        std::fs::write(
            root.join("refs.incl.md"),
            "[used]: https://a\n[spare]: https://b\n",
        )?;
        let report = root.join("report.json");

        audit(&[root.to_path_buf()], &report, false, vec![], vec![])?;

        let value = read_report(&report)?;
        let entries = value.as_array().expect("array");
        let used = entries
            .iter()
            .find(|entry| entry["label"] == "used")
            .expect("used");
        let spare = entries
            .iter()
            .find(|entry| entry["label"] == "spare")
            .expect("spare");
        assert_eq!(used["proven_unused"], false);
        assert_eq!(spare["proven_unused"], true);
        let included_by = used["included_by"].as_array().expect("included_by");
        assert!(
            included_by.iter().any(|entry| entry
                .as_str()
                .is_some_and(|text| text.ends_with("chapter.md"))),
            "{included_by:?}"
        );
        Ok(())
    }

    /// Uses in supplementary roots and protected files suppress cleanup.
    #[test]
    fn additional_uses_and_protected_files_prevent_proven_unused() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let book = dir.path().join("book");
        let other = dir.path().join("other");
        std::fs::create_dir_all(&book)?;
        std::fs::create_dir_all(&other)?;
        std::fs::write(book.join("chapter.md"), "{{#include refs.incl.md}}\n")?;
        std::fs::write(book.join("refs.incl.md"), "[ext]: https://a\n")?;
        std::fs::write(other.join("note.md"), "[Ext]\n")?;
        let report = dir.path().join("report.json");

        audit(
            std::slice::from_ref(&book),
            &report,
            false,
            vec![other],
            vec![],
        )?;
        let value = read_report(&report)?;
        assert_eq!(value[0]["proven_unused"], false);
        assert_eq!(
            value[0]["uses_outside_scoped_roots"]
                .as_array()
                .map(Vec::len),
            Some(1)
        );

        std::fs::write(book.join("refs.incl.md"), "[lonely]: https://a\n")?;
        audit(
            std::slice::from_ref(&book),
            &report,
            false,
            vec![],
            vec![book.join("refs.incl.md")],
        )?;
        let protected_report = read_report(&report)?;
        assert_eq!(protected_report[0]["protected"], true);
        assert_eq!(protected_report[0]["proven_unused"], false);
        Ok(())
    }

    /// Checks that an audit reports a configured protected file that does not exist.
    #[test]
    fn missing_protected_file_is_an_error() -> Result<()> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("c.md"), "x\n")?;
        let err = audit(
            &[dir.path().to_path_buf()],
            &dir.path().join("r.json"),
            false,
            vec![],
            vec![dir.path().join("gone.md")],
        )
        .expect_err("unresolvable protected file");
        assert!(
            format!("{err:#}").contains("protected definition file"),
            "{err:#}"
        );
        Ok(())
    }
}
