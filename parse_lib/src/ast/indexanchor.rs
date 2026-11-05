//! How an index anchor directive renders its text: as is, in italics or hidden.
//!
//! TODO Reviewed ??-??-2026.

use std::fmt::Display;

/// Rendering mode for an index anchor directive.
#[derive(Debug, PartialEq, Eq)]
pub enum IndexAnchorKind {
    /// Indexed text appears as is.
    /// `{{i:<text>}}`.
    Regular,
    /// Indexed text appears in italics.
    /// `{{ii:<text>}}`.
    Italics,
    /// Indexed text is hidden.
    /// `{{hi:<text>}}`.
    Hidden,
}

impl Display for IndexAnchorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Regular => write!(f, "i:"),
            Self::Italics => write!(f, "ii:"),
            Self::Hidden => write!(f, "hi:"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies: display uses directive prefixes.
    #[test]
    fn display_uses_directive_prefixes() {
        assert_eq!(IndexAnchorKind::Regular.to_string(), "i:");
        assert_eq!(IndexAnchorKind::Italics.to_string(), "ii:");
        assert_eq!(IndexAnchorKind::Hidden.to_string(), "hi:");
    }

    /// Verifies: kinds compare by variant.
    #[test]
    fn kinds_compare_by_variant() {
        assert_eq!(IndexAnchorKind::Hidden, IndexAnchorKind::Hidden);
        assert_ne!(IndexAnchorKind::Regular, IndexAnchorKind::Italics);
    }
}
