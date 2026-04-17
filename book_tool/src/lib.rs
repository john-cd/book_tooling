//! Command-oriented Markdown tooling built on top of `parse_lib`.
//!
//! `book_tool` exposes reusable modules behind the CLI commands defined in
//! `main.rs`. Each public module corresponds to one user-facing workflow or a
//! supporting utility layer.
//!
//! - [`common`]: shared file traversal, atomic file updates, and template helpers.
//! - [`directives`]: expands custom `{{ ... }}` directives and wikilinks.
//! - [`extract_comments`]: aggregates HTML comments from Markdown files.
//! - [`inline_links`]: converts inline links and autolinks into reference-style links.
//! - [`link_check`]: verifies external links and optionally removes unused refdefs.
//! - [`links`]: lower-level link conversion, extraction, and asynchronous verification utilities.
//! - [`refdefs`]: reference-definition analysis and cleanup helpers.
//! - [`search`]: plain-text search across Markdown documents.

/// Shared helpers used by multiple command modules.
pub mod common;
/// Directive and wikilink expansion workflows.
pub mod directives;
/// HTML comment extraction workflows.
pub mod extract_comments;
/// Inline-link to reference-link conversion workflows.
pub mod inline_links;
/// External link verification workflows.
pub mod link_check;
/// Link conversion, extraction, and validation utilities.
pub mod links;
/// Reference-definition analysis and cleanup helpers.
pub mod refdefs;
/// Plain-text Markdown search workflows.
pub mod search;
