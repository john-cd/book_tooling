//! AST types used by `parse_lib`.
//!
//! This module groups the main document model, individual element payload types,
//! directive-related enums, helper types such as index anchors, and reconstruction helpers.

/// Directive-related AST types.
pub mod directive;
/// Single-document AST container.
pub mod document;
/// Multi-document AST container.
pub mod documents;
/// Element payload types and the top-level [`Element`] enum.
pub mod element;
/// Index-anchor helper types.
pub mod indexanchor;
/// Utilities for reconstructing Markdown from AST nodes.
pub mod reconstruct;

pub use directive::*;
pub use document::*;
pub use documents::*;
pub use element::*;
pub use indexanchor::*;
pub use reconstruct::*;
