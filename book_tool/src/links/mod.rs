//! Low-level link extraction and verification helpers.

/// Asynchronous URL verification utilities.
pub mod asynchro;
/// AST-based URL extraction helpers.
pub mod link;

pub use asynchro::*;
pub use link::*;
