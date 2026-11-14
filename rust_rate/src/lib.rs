//! GitHub- and crates.io-based Rust crate analysis utilities.
//! The crate provides these modules.
//! - [`crawler`]: GitHub API access for discovering Cargo manifests and downloading content.
//! - [`stats`]: aggregation of crate usage and package category statistics.
//! - [`patterns`]: source analysis for crate-specific usage patterns.
//! - [`crates_io`]: a small crates.io summary workflow.
//! - [`templates`] and [`utils`]: formatting and test helpers.
//!
//! TODO Reviewed ??-??-2026.

/// Retrieves crate metadata and download counts from crates.io.
pub mod crates_io;

/// GitHub API crawling utilities.
pub mod crawler;

/// Source-pattern analysis utilities.
pub mod patterns;

/// Cargo.toml statistics aggregation utilities.
pub mod stats;

/// Markdown link and badge rendering helpers, shared with `book_tool` through `book_templates`.
pub mod templates {
    pub use book_templates::*;
}

/// Test helpers and other shared support utilities.
pub mod utils;
