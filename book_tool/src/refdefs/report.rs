//! Audit records and the deterministic JSON report.
//!
//! TODO Reviewed ??-??-2026.

use std::collections::BTreeMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use serde::Serialize;

use crate::common::replace_text_file_atomically;

/// The kind of Markdown construct that uses a reference definition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceUseKind {
    /// A reference-style link.
    Link,
    /// A reference-style image.
    Image,
}

/// One use of a reference label by a reference-style link or image.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct ReferenceUseAudit {
    /// File that contains the use.
    pub path: PathBuf,
    /// 1-based line of the use.
    pub line: usize,
    /// 1-based column of the use.
    pub column: usize,
    /// Label as written in the source.
    pub label: String,
    /// Whether the use is a link or an image.
    pub kind: ReferenceUseKind,
}

/// One reference definition together with where it is defined and used.
#[derive(Debug, Clone, Serialize)]
pub struct ReferenceDefinitionAudit {
    /// File that contains the definition.
    pub path: PathBuf,
    /// 1-based line of the definition.
    pub line: usize,
    /// 1-based column of the definition.
    pub column: usize,
    /// Label as written in the source.
    pub label: String,
    /// Label normalized for case-insensitive matching.
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

/// Unused definitions proven unused within every chapter/include scope in the scanned roots.
#[derive(Debug, Default)]
pub struct ScopedRefDefAnalysis {
    /// Canonical roots used to discover the Markdown inputs.
    pub(super) roots: Vec<PathBuf>,
    /// Contents read by the scope analysis, including chapters without definitions.
    pub(super) sources: BTreeMap<PathBuf, String>,
    /// Labels of unused definitions, grouped by the file that defines them.
    pub unused_by_file: BTreeMap<PathBuf, HashSet<String>>,
    /// Audit entries for every definition in the analyzed scopes.
    pub definitions: Vec<ReferenceDefinitionAudit>,
}

/// Write a deterministic JSON report containing every definition and its scoped uses.
///
/// # Errors
///
/// Returns an error if the report cannot be serialized or written.
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

/// Tests for audit report ordering and JSON serialization.
#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a reference-use fixture at the given path and line with the supplied kind.
    fn usage(path: &str, line: usize, kind: ReferenceUseKind) -> ReferenceUseAudit {
        ReferenceUseAudit {
            path: PathBuf::from(path),
            line,
            column: 3,
            label: "Serde".to_owned(),
            kind,
        }
    }

    /// Builds a definition fixture with reference uses for testing report serialization.
    fn definition() -> ReferenceDefinitionAudit {
        ReferenceDefinitionAudit {
            path: PathBuf::from("refs.md"),
            line: 1,
            column: 2,
            label: "Serde".to_owned(),
            normalized_label: "serde".to_owned(),
            included_by: vec![PathBuf::from("a.md")],
            uses: vec![usage("a.md", 4, ReferenceUseKind::Link)],
            uses_outside_scoped_roots: vec![usage("b.md", 9, ReferenceUseKind::Image)],
            ambiguous: false,
            protected: true,
            proven_unused: false,
        }
    }

    /// Checks field ordering and the serialized spelling of reference-use kinds.
    #[test]
    fn definition_serializes_with_stable_field_order_and_snake_case_kinds() {
        let json = serde_json::to_string(&definition()).expect("serializes");
        assert_eq!(
            json,
            concat!(
                r#"{"path":"refs.md","line":1,"column":2,"label":"Serde","#,
                r#""normalized_label":"serde","included_by":["a.md"],"#,
                r#""uses":[{"path":"a.md","line":4,"column":3,"label":"Serde","kind":"link"}],"#,
                r#""uses_outside_scoped_roots":[{"path":"b.md","line":9,"column":3,"label":"Serde","kind":"image"}],"#,
                r#""ambiguous":false,"protected":true,"proven_unused":false}"#
            )
        );
    }

    /// Checks the order of link and image uses when sorting a report.
    #[test]
    fn use_kinds_order_links_before_images() {
        assert!(ReferenceUseKind::Link < ReferenceUseKind::Image);
    }

    /// Checks the report formatting and preservation of input order.
    #[test]
    fn report_is_pretty_printed_array_with_trailing_newline_in_input_order() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("report.json");
        let mut second = definition();
        second.label = "Zed".to_owned();
        let analysis = ScopedRefDefAnalysis {
            definitions: vec![definition(), second],
            ..ScopedRefDefAnalysis::default()
        };
        write_audit_report(&path, &analysis).expect("writes");

        let text = std::fs::read_to_string(&path).expect("reads");
        assert!(text.ends_with("]\n"), "{text}");
        assert!(text.contains("\n    \"path\""), "pretty printed: {text}");
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        let labels: Vec<_> = value
            .as_array()
            .expect("array")
            .iter()
            .map(|entry| entry["label"].as_str().expect("label"))
            .collect();
        assert_eq!(labels, ["Serde", "Zed"]);
    }

    /// Checks that an audit with no findings writes an empty JSON array.
    #[test]
    fn empty_analysis_writes_an_empty_array() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("report.json");
        write_audit_report(&path, &ScopedRefDefAnalysis::default()).expect("writes");
        assert_eq!(std::fs::read_to_string(path).expect("reads"), "[]\n");
    }

    /// Checks that a report write failure identifies its output path.
    #[test]
    fn unwritable_destination_reports_the_path() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("missing-dir").join("report.json");
        let err = write_audit_report(&path, &ScopedRefDefAnalysis::default())
            .expect_err("parent directory is missing");
        assert!(format!("{err:#}").contains("report.json"), "{err:#}");
    }
}
