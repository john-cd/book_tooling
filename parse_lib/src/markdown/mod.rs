//! Markdown-specific parsing modules.
//!
//! This module contains various parsers for standard Markdown elements like
//! links, images, headings, and code blocks.
//!
//! TODO Reviewed ??-??-2026

pub use crate::parse_parts::inline_link::parse_inline_link;
pub use crate::parse_parts::reference_definition::parse_link_reference_definition;

mod autolink;
mod code;
mod headings;
mod images;
mod links;
mod text;

pub use autolink::*;
pub use code::*;
pub use headings::*;
pub use images::*;
pub use links::*;
pub use text::*;
