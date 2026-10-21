// TODO Reviewed ??-??-2026
//! Converts inline links and autolinks into reference-style links.

use std::path::PathBuf;

use anyhow::Result;

use crate::common::Scope;
use crate::common::replace_text_file_atomically;
use crate::common::walk_parsed_markdown_files;
use crate::links::InlineLinkConversionStats;
use crate::links::convert_inline_links_to_reference_definitions;

/// Rewrites matching Markdown files in place so inline links become reference-style links.
///
/// Existing reference definitions are preserved and reused. New definitions are appended
/// to the end of each updated file.
pub fn execute(directories: Vec<PathBuf>) -> Result<()> {
    tracing::info!(
        directory_count = directories.len(),
        "Starting inline-link conversion workflow"
    );

    let scope = Scope::default();
    let mut processed_files = 0_usize;
    let mut total_stats = InlineLinkConversionStats::default();

    walk_parsed_markdown_files(
        &directories,
        &scope,
        |path, doc| {
            if !doc.source.contains("http") && !doc.source.contains('[') {
                return Ok(());
            }
            let conversion = convert_inline_links_to_reference_definitions(doc);
            if conversion.markdown != doc.source {
                replace_text_file_atomically(path, &conversion.markdown)?;
            }
            let stats = conversion.stats;
            processed_files += 1;
            total_stats.existing_reference_definitions += stats.existing_reference_definitions;
            total_stats.reused_reference_definitions += stats.reused_reference_definitions;
            total_stats.appended_reference_definitions += stats.appended_reference_definitions;
            total_stats.preserved_issue_links += stats.preserved_issue_links;

            tracing::info!(
                path = %path.display(),
                existing_reference_definitions = stats.existing_reference_definitions,
                reused_reference_definitions = stats.reused_reference_definitions,
                appended_reference_definitions = stats.appended_reference_definitions,
                preserved_issue_links = stats.preserved_issue_links,
                "Converted inline links in file"
            );
            Ok(())
        },
        crate::common::warn_and_continue_on_parse_failure,
    )?;

    tracing::info!(
        processed_files,
        existing_reference_definitions = total_stats.existing_reference_definitions,
        reused_reference_definitions = total_stats.reused_reference_definitions,
        appended_reference_definitions = total_stats.appended_reference_definitions,
        preserved_issue_links = total_stats.preserved_issue_links,
        "Completed inline-link conversion workflow"
    );

    println!("Inline links converted successfully.");
    Ok(())
}
