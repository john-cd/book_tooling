//! Reference-definition analysis and cleanup helpers.
//!
//! The module is split by responsibility:
//!
//! - `occurrences` finds definitions and uses, with positions, in one file;
//! - `labels` answers "which labels are unused?" for a single document;
//! - `scope` resolves `{{#include}}` scopes across the files of a book;
//! - `report` holds the audit records and the JSON report;
//! - `delete` removes unused definitions without touching other bytes;
//! - `audit` ties them together for the `refdefs` command.
//!
//! TODO Reviewed ??-??-2026.

mod audit;
mod delete;
mod labels;
mod occurrences;
mod report;
mod scope;

pub use audit::audit;
pub use delete::delete_unused_refdefs;
pub use delete::remove_proven_unused_refdefs;
pub use labels::find_unused_refdefs;
pub use report::ReferenceDefinitionAudit;
pub use report::ReferenceUseAudit;
pub use report::ReferenceUseKind;
pub use report::ScopedRefDefAnalysis;
pub use report::write_audit_report;
pub use scope::find_unused_refdefs_in_scopes;

/// Normalize a Markdown reference label by collapsing whitespace and folding case.
#[must_use]
pub fn normalize_reference_label(label: &str) -> String {
    label
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
