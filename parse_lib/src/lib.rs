//! # `parse_lib`
//!
//! A high-performance, non-destructive Markdown parser and transformation library.
//!
//! This crate provides tools to parse Markdown into a rich AST, traverse and modify that AST
//! using the Visitor pattern, and reconstruct Markdown or render it to HTML.
//!
//! It is specifically designed to handle custom directives (e.g., `{{#crate name}}`) and
//! wikilinks (`[[page | display]]`) while maintaining faithful reconstruction of the original
//! document formatting.
//!
//! The public API is organized into a few layers:
//! - [`ast`]: reusable data structures representing parsed Markdown.
//! - [`visit`]: traits and visitors for traversing or re-rendering documents.
//! - Extractors such as [`directive_extractor`], [`html_comment_extractor`], [`link_extractor`],
//!   and [`plain_text_extractor`].
//! - Renderers such as [`html_renderer`] and [`markdown_renderer`].
//! - Parser entry points in [`markdown`], [`parse_directives`], [`parse_hidden`], [`parse_urls`],
//!   [`parse_wikilinks`], and [`parse_document()`].
//!
//! TODO Reviewed ??-??-2026.

// --- Core & Traits ---
/// Abstract syntax tree types for Markdown documents and custom extensions.
pub mod ast;

/// Error types used by parsing entry points.
pub mod errors;

/// Maps AST fragments back to byte ranges of their source.
pub mod source_span;

/// Visitor traits and helper visitors for traversing parsed documents.
pub mod visit;

// --- Extractors ---
/// Extracts custom directives from parsed documents.
pub mod directive_extractor;

/// Extracts HTML comments from parsed documents.
pub mod html_comment_extractor;

/// Extracts external links from parsed documents.
pub mod link_extractor;

/// Extracts plain text from parsed documents.
pub mod plain_text_extractor;

// --- Renderers ---
/// Renders parsed documents as HTML.
pub mod html_renderer;

/// Renders parsed documents back to Markdown.
pub mod markdown_renderer;

// --- Parsers ---
/// Parsers for standalone Markdown elements such as links, images, and headings.
pub mod markdown;

/// Parsers for custom `{{ ... }}` directives.
pub mod parse_directives;

/// The main document parser entry point.
pub mod parse_document;

/// Parsers for hidden HTML blocks used by mdBook-style content.
pub mod parse_hidden;

/// Reusable parsers for the pieces of links and link destinations.
mod parse_parts;

/// Parsers and URL classifiers for naked URLs and autolinks.
pub mod parse_urls;

/// Parsers for `[[wikilink]]` syntax.
pub mod parse_wikilinks;

pub use ast::*;
pub use errors::*;
pub use parse_document::parse_document;
pub use source_span::source_span;
