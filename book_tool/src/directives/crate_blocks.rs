//! Process crate block directives, for example `{{#crate crt}}`.
//!
//! Crate blocks combine multiple crate and categories badges.
//!
//! TODO Reviewed ??-??-2026

use parse_lib::ast::DirectiveData;

use crate::common::DestinationKind;
use crate::common::render_badge;
use crate::common::render_badge_with_text;
use crate::common::render_badge_without_index;

/// Renders a crate block based on the provided `DirectiveData`.
pub(super) fn render_crate_block(directive: &DirectiveData<'_>) -> String {
    if let DirectiveData::CrateBlock {
        crate_name,
        additional_categories,
    } = directive
    {
        let mut badges = vec![render_badge(DestinationKind::Docs, crate_name)];

        for cat in additional_categories {
            badges.push(render_category_badge(cat));
        }

        badges.join(" ")
    } else {
        String::new()
    }
}

/// Renders a badge set: `docs.rs`, `crates.io`, repository and `lib.rs` badges sharing one
/// index entry, followed by the category badges.
pub(super) fn render_badge_set(directive: &DirectiveData<'_>) -> String {
    let DirectiveData::BadgeSet {
        crate_name,
        additional_categories,
    } = directive
    else {
        return String::new();
    };

    let crate_badges = [
        (DestinationKind::Docs, (*crate_name).to_owned()),
        (DestinationKind::CratesIo, format!("{crate_name}~crates.io")),
        (DestinationKind::GithubRepo, format!("{crate_name}~repo")),
        (DestinationKind::LibRs, format!("{crate_name}~lib.rs")),
    ]
    .map(|(kind, alt_text)| render_badge_without_index(kind, crate_name, &alt_text))
    .join(" ");

    let mut parts = vec![format!("{crate_badges}{{{{hi:{crate_name}}}}}")];
    parts.extend(
        additional_categories
            .iter()
            .map(|category| render_category_badge(category)),
    );
    parts.join(" ")
}

/// Renders a `{{#categories ...}}` directive as a space-separated list of category badges.
pub(super) fn render_category_block(directive: &DirectiveData<'_>) -> String {
    if let DirectiveData::CategoryBlock { categories } = directive {
        categories
            .iter()
            .map(|category| render_category_badge(category))
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        String::new()
    }
}

/// Renders one `name` or `name=Title` category entry as a badge.
fn render_category_badge(category: &str) -> String {
    let (name, title) = category.split_once('=').unwrap_or((category, category));
    let title = title.trim_matches('"');
    render_badge_with_text(
        DestinationKind::Category,
        name,
        &format!("cat~{name}"),
        title,
    )
}
