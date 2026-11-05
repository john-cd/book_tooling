//! Template system for rendering Markdown badges and links.
//!
//! This module provides a centralized way to render common Markdown elements used
//! across the book tooling, such as crate badges and external links.
//!
//! TODO Reviewed ??-??-2026

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
    render_badge_with_text(kind, name, name, name)
}

/// Renders a badge with independent image alt text and index/title text.
#[must_use]
pub fn render_badge_with_text(
    kind: DestinationKind,
    name: &str,
    alt_text: &str,
    title: &str,
) -> String {
    if kind == DestinationKind::Category {
        return format!("[![{alt_text}][cat~{name}~badge]][cat~{name}]{{{{hi:{title}}}}}");
    }

    let prefix = kind.prefix();
    let label = kind.label();
    format!(
        "[![{alt_text}][{prefix}~{name}~{label}~badge]][{prefix}~{name}~{label}]{{{{hi:{title}}}}}"
    )
}

/// Renders a Markdown badge without the trailing index entry, so several badges can share one or
/// a category badge can omit its title index entry.
///
/// Example: `render_badge_without_index(DestinationKind::CratesIo, "anyhow", "anyhow~crates.io")`
/// produces: `[![anyhow~crates.io][p~anyhow~crates.io~badge]][p~anyhow~crates.io]`.
#[must_use]
pub fn render_badge_without_index(kind: DestinationKind, name: &str, alt_text: &str) -> String {
    if kind == DestinationKind::Category {
        return format!("[![{alt_text}][cat~{name}~badge]][cat~{name}]");
    }

    let prefix = kind.prefix();
    let label = kind.label();
    format!("[![{alt_text}][{prefix}~{name}~{label}~badge]][{prefix}~{name}~{label}]")
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

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;

    /// Verifies every destination matches the shared Markdown output contract.
    #[test]
    fn template_outputs_match_shared_destination_fixtures() -> Result<(), Box<dyn std::error::Error>>
    {
        for line in include_str!("../../test_fixtures/template_output.tsv").lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut fields = line.split('\t');
            let kind_name = fields
                .next()
                .ok_or_else(|| io::Error::other("fixture is missing a destination kind"))?;
            let kind = match kind_name {
                "category" => DestinationKind::Category,
                "crate" => DestinationKind::Crate,
                "docs" => DestinationKind::Docs,
                "github-repo" => DestinationKind::GithubRepo,
                "lib-rs" => DestinationKind::LibRs,
                "crates-io" => DestinationKind::CratesIo,
                "web" => DestinationKind::Web,
                other => {
                    return Err(
                        io::Error::other(format!("unknown destination kind: {other}")).into(),
                    );
                }
            };
            let name = fields
                .next()
                .ok_or_else(|| io::Error::other("fixture is missing a destination name"))?;
            let expected_badge = fields
                .next()
                .ok_or_else(|| io::Error::other("fixture is missing a badge expectation"))?;
            let expected_link = fields
                .next()
                .ok_or_else(|| io::Error::other("fixture is missing a link expectation"))?;
            if fields.next().is_some() {
                return Err(io::Error::other("fixture contains extra fields").into());
            }

            assert_eq!(
                render_badge(kind, name),
                expected_badge,
                "kind: {kind_name}"
            );
            assert_eq!(render_link(kind, name), expected_link, "kind: {kind_name}");
        }
        Ok(())
    }

    /// Verifies independent badge text, pre-encoded category slugs, and empty values are stable.
    #[test]
    fn category_badge_preserves_encoded_slug_and_separate_punctuation_text() {
        assert_eq!(
            render_badge_with_text(
                DestinationKind::Category,
                "api%2Fbindings",
                "C++ / FFI",
                "API bindings (C++)",
            ),
            "[![C++ / FFI][cat~api%2Fbindings~badge]][cat~api%2Fbindings]{{hi:API bindings (C++)}}"
        );
        assert_eq!(
            render_badge(DestinationKind::Category, ""),
            "[![][cat~~badge]][cat~]{{hi:}}"
        );
        assert_eq!(
            render_link(DestinationKind::Category, ""),
            "[][cat~]\u{2197}{{hi:}}"
        );
    }

    /// Renders an unindexed category badge with its category reference definitions.
    #[test]
    fn category_badge_without_index_uses_category_reference_definitions() {
        assert_eq!(
            render_badge_without_index(DestinationKind::Category, "science", "cat~science"),
            "[![cat~science][cat~science~badge]][cat~science]"
        );
    }
}
