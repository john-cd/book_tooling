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
        include_crate_index,
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

    let crate_badges = if *include_crate_index {
        format!("{crate_badges}{{{{hi:{crate_name}}}}}")
    } else {
        crate_badges
    };
    let mut parts = vec![crate_badges];
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

/// Renders one `name`, `name=Title`, or `name=` category entry as a badge.
fn render_category_badge(category: &str) -> String {
    let (name, title) = category.split_once('=').unwrap_or((category, category));
    let title = title.trim_matches('"');
    let alt_text = format!("cat~{name}");
    if title.is_empty() {
        render_badge_without_index(DestinationKind::Category, name, &alt_text)
    } else {
        render_badge_with_text(DestinationKind::Category, name, &alt_text, title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn non_block() -> DirectiveData<'static> {
        DirectiveData::ExampleBlock { name: "x" }
    }

    #[test]
    fn crate_block_without_categories_is_one_docs_badge() {
        let out = render_crate_block(&DirectiveData::CrateBlock {
            crate_name: "serde",
            additional_categories: vec![],
        });
        assert_eq!(out, render_badge(DestinationKind::Docs, "serde"));
    }

    #[test]
    fn crate_block_appends_category_badges_in_order() {
        let out = render_crate_block(&DirectiveData::CrateBlock {
            crate_name: "serde",
            additional_categories: vec!["a-cat", "b-cat"],
        });
        let first = out.find("cat~a-cat").expect("first category");
        let second = out.find("cat~b-cat").expect("second category");
        assert!(first < second);
    }

    #[test]
    fn mismatched_directives_render_empty() {
        assert_eq!(render_crate_block(&non_block()), "");
        assert_eq!(render_badge_set(&non_block()), "");
        assert_eq!(render_category_block(&non_block()), "");
    }

    #[test]
    fn empty_category_list_renders_empty() {
        let out = render_category_block(&DirectiveData::CategoryBlock { categories: vec![] });
        assert_eq!(out, "");
    }

    #[test]
    fn category_title_forms() {
        let plain = render_category_badge("filesystem");
        assert!(plain.contains("cat~filesystem"), "{plain}");
        assert!(plain.contains("{{hi:filesystem}}"), "{plain}");

        let titled = render_category_badge("api-bindings=\"API bindings\"");
        assert!(titled.contains("{{hi:API bindings}}"), "{titled}");
        assert!(!titled.contains('"'), "{titled}");

        // `slug=` suppresses the index entry.
        let no_index = render_category_badge("filesystem=");
        assert!(no_index.contains("cat~filesystem"), "{no_index}");
        assert!(!no_index.contains("{{hi:"), "{no_index}");
    }

    #[test]
    fn category_block_joins_badges_with_spaces() {
        let out = render_category_block(&DirectiveData::CategoryBlock {
            categories: vec!["one", "two=Two"],
        });
        assert_eq!(
            out,
            format!(
                "{} {}",
                render_category_badge("one"),
                render_category_badge("two=Two")
            )
        );
    }

    #[test]
    fn badge_set_index_flag_controls_crate_index_entry() {
        let with = render_badge_set(&DirectiveData::BadgeSet {
            crate_name: "tokio",
            include_crate_index: true,
            additional_categories: vec![],
        });
        let without = render_badge_set(&DirectiveData::BadgeSet {
            crate_name: "tokio",
            include_crate_index: false,
            additional_categories: vec![],
        });
        assert!(with.ends_with("{{hi:tokio}}"), "{with}");
        assert!(!without.contains("{{hi:tokio}}"), "{without}");
        for kind in ["tokio~crates.io", "tokio~repo", "tokio~lib.rs"] {
            assert!(with.contains(kind), "{with}");
        }
    }
}
