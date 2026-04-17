//! # parse_lib
//!
//! A high-performance, non-destructive Markdown parser and transformation library.
//!
//! This crate provides tools to parse Markdown into a rich AST, traverse and modify that AST
//! using the Visitor pattern, and reconstruct Markdown or render it to HTML.
//!
//! It is specifically designed to handle custom directives (e.g., `{{#crate name}}`) and
//! wikilinks (`[[page | display]]`) while maintaining faithful reconstruction of the original
//! document formatting.

// --- Core & Traits ---
pub mod ast;
pub mod errors;
pub mod visit;

// --- Extractors ---
pub mod directive_extractor;
pub mod html_comment_extractor;
pub mod link_extractor;
pub mod plain_text_extractor;

// --- Renderers ---
pub mod html_renderer;
pub mod markdown_renderer;

// --- Parsers ---
pub mod markdown;
pub mod parse_directives;
pub mod parse_document;
pub mod parse_hidden;
mod parse_parts;
pub mod parse_urls;
pub mod parse_wikilinks;

pub use ast::*;
pub use errors::*;
pub use parse_document::parse_document;
