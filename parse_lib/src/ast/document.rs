use std::path::Path;

use super::Element;

/// Represents a parsed Markdown document.
///
/// A `Document` consists of a name, an optional filesystem path, and a collection
/// of top-level `Element` nodes that form the AST.
#[derive(Debug, PartialEq, Eq)]
pub struct Document<'s> {
    /// The name of the document (e.g., filename).
    pub name: &'s str,
    /// The physical path to the file on disk, if known.
    pub path: Option<&'s Path>,
    /// The original Markdown source, retained for source-preserving transforms.
    pub source: &'s str,
    /// The sequence of elements that make up the document content.
    pub elements: Vec<Element<'s>>,
}

impl<'s> Document<'s> {
    /// Creates a new `Document` with the given elements.
    pub fn new(
        name: &'s str,
        path: Option<&'s Path>,
        source: &'s str,
        elements: Vec<Element<'s>>,
    ) -> Self {
        Document {
            name,
            path,
            source,
            elements,
        }
    }

    // /// Returns a reference to the elements in the document.
    // pub fn elements(&'s self) -> &'s [Element<'s>] {
    //     &self.elements
    // }

    // /// Returns a mutable reference to the elements in the document.
    // pub fn elements_mut(&mut self) -> &mut Vec<Element<'s>> {
    //     &mut self.elements
    // }

    /// Returns true if the document has no elements.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}
