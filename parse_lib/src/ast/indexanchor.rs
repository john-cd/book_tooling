// TODO Reviewed ??-??-2026
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
