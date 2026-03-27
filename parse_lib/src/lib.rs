#![allow(missing_docs)]
//! Parsers for Markdown and related elements.

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
pub mod parse_parts;
pub mod parse_urls;
pub mod parse_wikilinks;

pub use ast::*;
pub use errors::*;
pub use parse_document::parse_document;
