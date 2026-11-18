//! Reference-definition analysis and cleanup helpers.
//! The module is split by responsibility:
//! - `occurrences` finds definitions and uses, with positions, in one file;
//! - `labels` answers "which labels are unused?" for a single document;
//! - `scope` resolves `{{#include}}` scopes across the files of a book;
//! - `report` holds the audit records and the JSON report;
//! - `delete` removes unused definitions without touching other bytes;
//! - `audit` ties them together for the `refdefs` command.
//!
//! TODO Reviewed ??-??-2026.

use icu_casemap::CaseMapper;

/// Implements audit behavior for the refdefs component, keeping the related rules and data
/// transformations at one boundary. The parent module calls these routines to provide normalize
/// reference label behavior.
mod audit;
/// Implements delete behavior for the refdefs component, keeping the related rules and data
/// transformations at one boundary. The parent module calls these routines to provide normalize
/// reference label behavior.
mod delete;
/// Implements labels behavior for the refdefs component, keeping the related rules and data
/// transformations at one boundary. The parent module calls these routines to provide normalize
/// reference label behavior.
mod labels;
/// Implements occurrences behavior for the refdefs component, keeping the related rules and data
/// transformations at one boundary. The parent module calls these routines to provide normalize
/// reference label behavior.
mod occurrences;
/// Implements report behavior for the refdefs component, keeping the related rules and data
/// transformations at one boundary. The parent module calls these routines to provide normalize
/// reference label behavior.
mod report;
/// Implements scope behavior for the refdefs component, keeping the related rules and data
/// transformations at one boundary. The parent module calls these routines to provide normalize
/// reference label behavior.
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

/// Returns the CommonMark-normalized reference-label key for `label`.
///
/// Unicode case folding runs before leading/trailing spaces, tabs, and line endings are removed
/// and internal runs are collapsed to one ASCII space.
#[must_use]
pub fn normalize_reference_label(label: &str) -> String {
    let mut normalized = String::new();
    let mut pending_space = false;
    let case_folded = CaseMapper::new().fold_string(label);

    for character in case_folded.chars() {
        if matches!(character, ' ' | '\t' | '\r' | '\n') {
            pending_space = !normalized.is_empty();
        } else {
            if pending_space {
                normalized.push(' ');
                pending_space = false;
            }
            normalized.push(character);
        }
    }

    normalized
}
