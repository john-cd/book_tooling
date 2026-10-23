//! Markdown-specific parsing modules.
//!
//! This module contains various parsers for standard Markdown elements like
//! links, images, headings, and code blocks.
//!
//! TODO Reviewed ??-??-2026

pub use crate::parse_parts::inline_link::parse_inline_link;
pub use crate::parse_parts::reference_definition::parse_link_reference_definition;

/// Parser for `<https://...>` autolinks.
mod autolink;

/// Parsers for code spans and fenced code blocks.
mod code;
mod headings;

/// Parsers for inline and reference-style images.
mod images;

/// Parsers for reference-style links.
mod links;

/// Parsers for plain text and fallback characters.
mod text;

pub use autolink::*;
pub use code::*;
pub use headings::*;
pub use images::*;
pub use links::*;
pub use text::*;
