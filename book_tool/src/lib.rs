//! Note on old code:
//! Many modules (converter, directives, extract_comments, refdefs)
//! have been moved to `book_tool` to centralize the tooling logic.
//! The folder + mod.rs module structure has been kept intact.

pub mod common;
pub mod directives;
pub mod extract_comments;
pub mod inline_links;
pub mod link_check;
pub mod links;
pub mod refdefs;
pub mod search;
