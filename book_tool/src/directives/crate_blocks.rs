//! Process crate block directives, for example `{{#crate crt}}`.
//!
//! Crate blocks combine multiple crate and categories badges.
//!
//! TODO Reviewed ??-??-2026

use parse_lib::ast::DirectiveData;

use crate::common::DestinationKind;
use crate::common::render_badge;
use crate::common::render_badge_with_text;

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
