//! Audit records and the deterministic JSON report.
//!
//! TODO Reviewed ??-??-2026

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
