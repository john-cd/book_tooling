//! The model for link / badge / crate block / example block directives.
//!
//! TODO Reviewed ??-??-2026

use std::fmt::Display;

/// Parsed representation of a custom `{{ ... }}` directive.
#[derive(Debug, PartialEq, Eq)]
pub enum DirectiveData<'a> {
    /// Insert a Markdown link.
    ///
    /// - category: {{cat xyz}}
    /// - internal page: {{crate xyz}}
    /// - `docs.rs` link: {{docs xyz}}
    /// - Github link: {{github xyz}}
    /// - `lib.rs` link: {{lib.rs xyz}}
    /// - `crates.io` link: {{crates.io xyz}}
    /// - Website for the crate: {{web xyz}}
    Link {
        /// Destination family for the link.
        kind: DestinationKind,
        /// Directive payload, usually a crate, category, or repository name.
        name: &'a str,
    },

    /// Insert a Badge,
    /// e.g. `[![some_crate][c~some_crate~docs~badge]][c~some_crate~docs]`.
    ///
    /// `{{!cat xyz}}
    /// {{!crate xyz}}
    /// {{!docs xyz}}
    /// {{!github xyz}}
    /// {{!lib.rs xyz}}
    /// {{!crates.io xyz}}
    /// {{!web xyz}}`.
    Badge {
        /// Destination family for the badge link.
        kind: DestinationKind,
        /// Directive payload, usually a crate, category, or repository name.
        name: &'a str,
    },

    /// Insert a Crate block e.g., multiple badges for a given crate:
    /// `{{#crate crt}}`.
    ///
    /// With optional additional categories, for example
    /// `{{#crate: crt cat1 cat-2 cat-2-2 cat3::sub-cat-3 }}`.
    CrateBlock {
        /// Primary crate displayed by the badge block.
        crate_name: &'a str,
        /// Additional category badges to include in the block.
        additional_categories: Vec<&'a str>,
    },

    /// Insert the full badge set for a crate: `docs.rs`, `crates.io`, GitHub repository and
    /// `lib.rs` badges, an optional shared index entry, then optional category badges, for
    /// example `{{#badges crt cat1 cat-2=Title}}` or `{{#badges crt=}}`.
    BadgeSet {
        /// Crate displayed by the badge set.
        crate_name: &'a str,
        /// Whether to add the crate name to the book index.
        include_crate_index: bool,
        /// Additional category badges to include in the set.
        additional_categories: Vec<&'a str>,
    },

    /// Insert badges for one or more categories.
    ///
    /// A category may use `slug=Title` to preserve its displayed title, for
    /// example `{{#categories filesystem=Filesystem,api-bindings="API bindings"}}`.
    CategoryBlock {
        /// Category slugs, optionally paired with their display titles.
        categories: Vec<&'a str>,
    },

    /// Insert a fenced code block with an `include` directive pointing to a new code example (.rs)
    /// file, for example `{{#example some_example}}`.
    ExampleBlock {
        /// Example name to resolve to an included file.
        name: &'a str,
    },
}

/// Formats a directive using canonical, parseable double-brace syntax.
///
/// Formatting normalizes spacing and does not preserve the original source bytes.
impl Display for DirectiveData<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DirectiveData::Link { kind, name } => write!(f, "{{{{{kind} {name}}}}}"),
            DirectiveData::Badge { kind, name } => write!(f, "{{{{!{kind} {name}}}}}"),
            DirectiveData::CrateBlock {
                crate_name,
                additional_categories,
            } => {
                if additional_categories.is_empty() {
                    write!(f, "{{{{#crate {crate_name}}}}}")
                } else {
                    let categories = &additional_categories.join(" ");
                    write!(f, "{{{{#crate {crate_name} {categories}}}}}")
                }
            }
            DirectiveData::BadgeSet {
                crate_name,
                include_crate_index,
                additional_categories,
            } => {
                let crate_index = if *include_crate_index { "" } else { "=" };
                if additional_categories.is_empty() {
                    write!(f, "{{{{#badges {crate_name}{crate_index}}}}}")
                } else {
                    let categories = &additional_categories.join(" ");
                    write!(f, "{{{{#badges {crate_name}{crate_index} {categories}}}}}")
                }
            }
            DirectiveData::CategoryBlock { categories } => {
                write!(f, "{{{{#categories {}}}}}", categories.join(","))
            }
            DirectiveData::ExampleBlock { name } => write!(f, "{{{{#example {name}}}}}"),
        }
    }
}

/// Destination family used by link and badge directives.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DestinationKind {
    /// Link to or badge for a `crates.io` category page.
    Category,
    /// Link to or badge for an internal crate page.
    Crate,
    /// Link to or badge for a `docs.rs` crate page.
    Docs,
    /// Link to or badge for a GitHub repo.
    GithubRepo,
    /// Link to or badge for a `lib.rs` crate page.
    LibRs,
    /// Link to or badge for a `crates.io` crate page.
    CratesIo,
    /// Link to or badge for another website.
    Web,
}

impl Display for DestinationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Category => write!(f, "cat"),
            Self::Crate => write!(f, "crate"),
            Self::Docs => write!(f, "docs"),
            Self::GithubRepo => write!(f, "github"),
            Self::LibRs => write!(f, "lib.rs"),
            Self::CratesIo => write!(f, "crates.io"),
            Self::Web => write!(f, "web"),
        }
    }
}
