//! AST types used by `parse_lib`.
//!
//! This module groups the main document model, individual element payload types,
//! directive-related enums, helper types such as index anchors, and reconstruction helpers.

/// Directive-related AST types.
mod directive;
/// Single-document AST container.
mod document;
/// Multi-document AST container.
mod documents;
/// Element payload types and the top-level [`Element`] enum.
mod element;
/// Index-anchor helper types.
mod indexanchor;
/// Utilities for reconstructing Markdown from AST nodes.
mod reconstruct;

pub use directive::*;
pub use document::*;
pub use documents::*;
pub use element::*;
pub use indexanchor::*;
pub use reconstruct::*;
