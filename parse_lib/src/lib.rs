//! Parsers for Markdown and related elements.
//!
//! The parsers do not cover all corner cases of the CommonMark spec, just what we need.
// [finish](https://github.com/john-cd/rust_howto/issues/1425)

pub mod ast;
pub mod errors;
pub mod markdown;
pub mod parse_directives;
pub mod parse_document;
pub mod parse_hidden;
pub mod parse_parts;
pub mod parse_urls;
pub mod parse_wikilinks;
pub mod visit;

pub use ast::*;
pub use errors::*;
pub use parse_document::parse_document;
