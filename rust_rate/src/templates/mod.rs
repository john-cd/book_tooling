//! Template system for rendering Markdown badges and links.
//!
//! This module provides a centralized way to render common Markdown elements used
//! across the book tooling, such as crate badges and external links.
//!
//! TODO Reviewed ??-??-2026

#[cfg(test)]
mod tests;

/// Crate destinations.
///
/// Represents different types of targets for badges and links.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestinationKind {
    /// A crate category.
    Category,
    /// A crate itself.
    Crate,
    /// Documentation (e.g., docs.rs).
    Docs,
    /// A GitHub repository.
    GithubRepo,
    /// lib.rs page.
    LibRs,
    /// crates.io page.
    CratesIo,
    /// A generic website.
    Web,
}

impl DestinationKind {
    /// Returns the standard label for this destination kind.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Category => "cat",
            Self::Crate => "crate",
            Self::Docs => "docs",
            Self::GithubRepo => "repo",
            Self::LibRs => "lib.rs",
            Self::CratesIo => "crates.io",
            Self::Web => "website",
        }
    }

    /// Returns the prefix used in reference labels for this destination kind.
    #[must_use]
    pub fn prefix(self) -> char {
        match self {
            Self::Crate => 'p',
            Self::Category
            | Self::Docs
            | Self::GithubRepo
            | Self::LibRs
            | Self::CratesIo
            | Self::Web => 'c',
        }
    }
}

/// Renders a Markdown badge for a given destination.
///
/// Example: `render_badge(DestinationKind::CratesIo, "anyhow")`
/// produces: `[![anyhow][p~anyhow~crates.io~badge]][p~anyhow~crates.io]{{hi:anyhow}}`.
#[must_use]
pub fn render_badge(kind: DestinationKind, name: &str) -> String {
    if kind == DestinationKind::Category {
        return format!("[![{name}][cat~{name}~badge]][cat~{name}]{{{{hi:{name}}}}}");
    }

    let prefix = kind.prefix();
    let label = kind.label();
    format!("[![{name}][{prefix}~{name}~{label}~badge]][{prefix}~{name}~{label}]{{{{hi:{name}}}}}")
}

/// Renders a Markdown link for a given destination.
///
/// Example: `render_link(DestinationKind::Docs, "anyhow")`
/// produces: `[anyhow][p~anyhow~docs]↗{{hi:anyhow}}`.
#[must_use]
pub fn render_link(kind: DestinationKind, name: &str) -> String {
    if kind == DestinationKind::Category {
        format!("[{name}][cat~{name}]↗{{{{hi:{name}}}}}")
    } else {
        let prefix = kind.prefix();
        let label = kind.label();
        format!("[{name}][{prefix}~{name}~{label}]↗{{{{hi:{name}}}}}")
    }
}
