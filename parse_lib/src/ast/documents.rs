//! A collection of parsed Markdown documents.
//!
//! TODO Reviewed ??-??-2026.

use super::Document;

/// A collection of parsed Markdown documents.
#[derive(Debug, PartialEq, Eq)]
pub struct Documents<'s> {
    /// Documents in traversal or input order.
    pub documents: Vec<Document<'s>>,
}

impl<'s> Documents<'s> {
    /// Creates a new document collection.
    #[must_use]
    pub fn new(documents: Vec<Document<'s>>) -> Self {
        Self { documents }
    }
}

// impl std::fmt::Display for Documents<'_> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         for doc in &self.documents {
//             write!(f, "{doc}")?;
//         }
//         Ok(())
//     }
// }
