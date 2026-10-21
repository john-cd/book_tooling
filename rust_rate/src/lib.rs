// TODO Reviewed ??-??-2026
//! GitHub- and crates.io-based Rust crate analysis utilities.
//!
//! The crate is organized around three main concerns:
//! - [`crawler`]: GitHub API access for discovering Cargo manifests and downloading content.
//! - [`stats`]: aggregation of crate usage and package category statistics.
//! - [`patterns`]: source analysis for crate-specific usage patterns.
//! - [`rust_rate`]: a small crates.io summary workflow.
//! - [`templates`] and [`utils`]: formatting and test helpers.

/// GitHub API crawling utilities.
pub mod crawler;
/// Source-pattern analysis utilities.
pub mod patterns;
/// crates.io summary workflow utilities.
pub mod rust_rate;
/// Cargo.toml statistics aggregation utilities.
pub mod stats;
/// Markdown link and badge rendering helpers.
pub mod templates;
/// Test helpers and other shared support utilities.
pub mod utils;
