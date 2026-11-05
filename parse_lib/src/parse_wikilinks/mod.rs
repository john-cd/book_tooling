//! Parsers for `[[wikilink]]` syntax.
//!
//! TODO Reviewed ??-??-2026.

/// Winnow parser for wikilinks.
mod parser;

pub use parser::parse_wikilink;
