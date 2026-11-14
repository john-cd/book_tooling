//! Parsers for custom `{{ ... }}` directives.
//!
//! TODO Reviewed ??-??-2026.

use winnow::ModalResult;
use winnow::Parser as _;
use winnow::ascii::space0;
use winnow::ascii::space1;
use winnow::combinator::alt;
use winnow::combinator::delimited;
use winnow::combinator::opt;
use winnow::combinator::preceded;
use winnow::combinator::repeat;
use winnow::error::StrContext::Expected;
use winnow::error::StrContext::Label;
use winnow::error::StrContextValue::Description;
use winnow::token::take_until;
use winnow::token::take_while;

use crate::DirectiveData;
use crate::ast::DestinationKind;
use crate::ast::Element;

/// Consumes a custom `{{ ... }}` directive and wraps its arguments in a Markdown AST node.
///
/// # Errors
/// Backtracks when the input does not start with a supported directive and valid arguments.
pub fn parse_directive_element<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    alt((
        parse_link_directive,
        parse_badge_directive,
        parse_crate_block_directive,
        parse_badge_set_directive,
        parse_category_block_directive,
        parse_example_block_directive,
    ))
    .map(|directive: DirectiveData<'a>| Element::CustomDirective(directive))
    .context(Label("custom directive"))
    .context(Expected(Description(
        "a custom directive starting with {{#",
    )))
    .parse_next(input)
}

/// Parses the destination keyword of a link or badge directive, such as `web` or `crates.io`.
fn parse_destination_kind(input: &mut &str) -> ModalResult<DestinationKind> {
    alt((
        "cat".value(DestinationKind::Category),
        "crates.io".value(DestinationKind::CratesIo),
        "crate".value(DestinationKind::Crate),
        "docs".value(DestinationKind::Docs),
        "github".value(DestinationKind::GithubRepo),
        "lib.rs".value(DestinationKind::LibRs),
        "web".value(DestinationKind::Web),
    ))
    .parse_next(input)
}
/// Consumes the colon or whitespace after a destination keyword and returns the trimmed name before
/// `}}`.
///
/// # Errors
/// Backtracks if the separator, closing delimiter, or a nonblank name is missing.
fn parse_link_or_badge_name<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    alt((
        preceded(':', preceded(space0, take_until(1.., "}}"))),
        preceded(space1, take_until(1.., "}}")),
    ))
    .verify(|name: &str| !name.trim().is_empty())
    .map(str::trim)
    .parse_next(input)
}
/// Parses a link directive such as `{{web example.com}}`.
fn parse_link_directive<'a>(input: &mut &'a str) -> ModalResult<DirectiveData<'a>> {
    delimited(
        "{{",
        (parse_destination_kind, parse_link_or_badge_name).map(|(kind, name)| {
            DirectiveData::Link {
                kind,
                name: name.trim(),
            }
        }),
        "}}",
    )
    .parse_next(input)
}

/// Parses a badge directive such as `{{!crate serde}}`.
fn parse_badge_directive<'a>(input: &mut &'a str) -> ModalResult<DirectiveData<'a>> {
    delimited(
        "{{!",
        (parse_destination_kind, parse_link_or_badge_name).map(|(kind, name)| {
            DirectiveData::Badge {
                kind,
                name: name.trim(),
            }
        }),
        "}}",
    )
    .parse_next(input)
}

/// Parses a `{{#crate ...}}` block directive.
fn parse_crate_block_directive<'a>(input: &mut &'a str) -> ModalResult<DirectiveData<'a>> {
    // {{#crate: crt cat1 cat-2}}
    delimited(
        "{{#crate",
        (
            opt(":"),
            space0,
            take_while(1.., |c: char| c != ' ' && c != '}'),
            repeat(
                0..,
                preceded(" ", take_while(1.., |c: char| c != ' ' && c != '}')),
            ),
        )
            .map(
                |(_, _, name, cats): (_, _, &str, Vec<&str>)| DirectiveData::CrateBlock {
                    crate_name: name.trim(),
                    additional_categories: cats
                        .into_iter()
                        .flat_map(split_category_arguments)
                        .collect(),
                },
            ),
        "}}",
    )
    .parse_next(input)
}

/// Parses `{{#badges ...}}` into a crate name, an index-entry flag, and optional category
/// arguments. A trailing `=` on the crate name suppresses its index entry.
///
/// # Errors
/// Backtracks for an empty or malformed crate argument or a missing closing delimiter.
fn parse_badge_set_directive<'a>(input: &mut &'a str) -> ModalResult<DirectiveData<'a>> {
    // {{#badges: crt cat1 cat-2="Title with spaces"}}
    delimited(
        "{{#badges",
        (opt(":"), space0, take_until(1.., "}}")).verify_map(|(_, _, args): (_, _, &str)| {
            let mut arguments = split_category_arguments(args.trim()).into_iter();
            let crate_argument = arguments.next()?;
            let (crate_name, include_crate_index) = match crate_argument.strip_suffix('=') {
                Some(crate_name) => (crate_name, false),
                None if crate_argument.contains('=') => return None,
                None => (crate_argument, true),
            };

            (!crate_name.is_empty()).then(|| DirectiveData::BadgeSet {
                crate_name,
                include_crate_index,
                additional_categories: arguments.collect(),
            })
        }),
        "}}",
    )
    .parse_next(input)
}

/// Parses a `{{#categories ...}}` block directive.
fn parse_category_block_directive<'a>(input: &mut &'a str) -> ModalResult<DirectiveData<'a>> {
    delimited(
        "{{#categories",
        (opt(":"), space0, take_until(1.., "}}"))
            .map(|(_, _, categories): (_, _, &str)| split_category_arguments(categories))
            .verify(|categories: &Vec<&str>| !categories.is_empty())
            .map(|categories| DirectiveData::CategoryBlock { categories }),
        "}}",
    )
    .parse_next(input)
}

/// Splits comma- and whitespace-separated category arguments into borrowed slices, keeping
/// separators inside quoted titles.
fn split_category_arguments(input: &str) -> Vec<&str> {
    let mut categories = Vec::new();
    let mut category_start = None;
    let mut in_quotes = false;

    for (index, character) in input.char_indices() {
        match character {
            '"' => in_quotes = !in_quotes,
            ',' | ' ' | '\t' if !in_quotes => {
                if let Some(start) = category_start.take() {
                    let category = input.get(start..index).unwrap_or_default().trim();
                    if !category.is_empty() {
                        categories.push(category);
                    }
                }
            }
            _ => {
                category_start.get_or_insert(index);
            }
        }
    }

    if let Some(start) = category_start {
        let category = input.get(start..).unwrap_or_default().trim();
        if !category.is_empty() {
            categories.push(category);
        }
    }

    categories
}

/// Consumes `{{#example ...}}` and returns the trimmed example name without resolving it.
fn parse_example_block_directive<'a>(input: &mut &'a str) -> ModalResult<DirectiveData<'a>> {
    // Nested directives: {{#example {{#include ...}}}}
    // Actually, commonmark-style directives usually aren't that complex but let's allow
    // the content to contain {{ if we can.
    delimited("{{#example ", take_until(1.., "}}"), "}}")
        .verify(|name: &str| !name.trim().is_empty())
        .map(|name: &str| DirectiveData::ExampleBlock { name: name.trim() })
        .parse_next(input)
}

/// Tests for recognition and round-tripping of custom directives.
#[cfg(test)]
mod tests {
    use super::*;

    /// Categories directive parses comma separated titles.
    #[test]
    fn categories_directive_parses_comma_separated_titles() {
        let mut input = "{{#categories filesystem=Filesystem,api-bindings=\"API bindings\"}}";
        let Element::CustomDirective(DirectiveData::CategoryBlock { categories }) =
            parse_directive_element(&mut input).expect("valid categories directive")
        else {
            panic!("expected a categories directive");
        };

        assert_eq!(
            categories,
            ["filesystem=Filesystem", "api-bindings=\"API bindings\""]
        );
        assert!(input.is_empty());
    }

    /// Crate directive keeps additional categories separate.
    #[test]
    fn crate_directive_keeps_additional_categories_separate() {
        let mut input = "{{#crate same-file filesystem=Filesystem,api-bindings=API}}";
        assert_eq!(
            parse_directive_element(&mut input).expect("valid crate directive"),
            Element::CustomDirective(DirectiveData::CrateBlock {
                crate_name: "same-file",
                additional_categories: vec!["filesystem=Filesystem", "api-bindings=API"],
            })
        );
        assert!(input.is_empty());
    }

    /// Badge set directive keeps quoted category titles with spaces together.
    #[test]
    fn badge_set_directive_keeps_quoted_titles_together() {
        let mut input = "{{#badges ndarray data-structures=\"Data structures\" science}}";
        assert_eq!(
            parse_directive_element(&mut input).expect("valid badge set directive"),
            Element::CustomDirective(DirectiveData::BadgeSet {
                crate_name: "ndarray",
                include_crate_index: true,
                additional_categories: vec!["data-structures=\"Data structures\"", "science"],
            })
        );
        assert!(input.is_empty());
    }

    /// Badge set directive keeps additional categories separate.
    #[test]
    fn badge_set_directive_keeps_additional_categories_separate() {
        let mut input = "{{#badges ndarray data-structures science=\"Science\"}}";
        assert_eq!(
            parse_directive_element(&mut input).expect("valid badge set directive"),
            Element::CustomDirective(DirectiveData::BadgeSet {
                crate_name: "ndarray",
                include_crate_index: true,
                additional_categories: vec!["data-structures", "science=\"Science\""],
            })
        );
        assert!(input.is_empty());
    }

    /// Badge set directive can omit the crate title index term without omitting categories.
    #[test]
    fn badge_set_directive_omits_crate_index_term() {
        let mut input = "{{#badges ndarray= data-structures=\"Data structures\"}}";
        assert_eq!(
            parse_directive_element(&mut input).expect("valid unindexed badge set directive"),
            Element::CustomDirective(DirectiveData::BadgeSet {
                crate_name: "ndarray",
                include_crate_index: false,
                additional_categories: vec!["data-structures=\"Data structures\""],
            })
        );
        assert!(input.is_empty());
    }

    /// Checks that category directives keep percent-encoded names and require a nonblank payload.
    #[test]
    fn category_links_and_badges_parse_encoded_names_and_reject_empty_names() {
        assert_eq!(
            parse_directive_element(&mut "{{cat: api-bindings%2Fffi}}")
                .expect("parse colon-separated category link"),
            Element::CustomDirective(DirectiveData::Link {
                kind: DestinationKind::Category,
                name: "api-bindings%2Fffi",
            })
        );
        assert_eq!(
            parse_directive_element(&mut "{{!cat C++ / FFI}}")
                .expect("parse category badge with punctuation"),
            Element::CustomDirective(DirectiveData::Badge {
                kind: DestinationKind::Category,
                name: "C++ / FFI",
            })
        );

        let mut empty_name = "{{cat }}";
        parse_directive_element(&mut empty_name).expect_err("input should be rejected");
        let mut invalid_destination = "{{catalog accessibility}}";
        parse_directive_element(&mut invalid_destination).expect_err("input should be rejected");
    }

    /// Checks that the display form of every custom directive parses back to the same variant.
    #[test]
    fn every_directive_variant_round_trips_through_display() {
        let sources = [
            "{{cat accessibility}}",
            "{{!crates.io serde}}",
            "{{#crate serde filesystem api-bindings}}",
            "{{#badges serde}}",
            "{{#badges serde=}}",
            "{{#badges serde filesystem api-bindings}}",
            "{{#categories filesystem=Filesystem,api-bindings=\"API bindings\"}}",
            "{{#example ../../crates/language/examples/constants_and_statics/constants.rs:example}}",
        ];

        for source in sources {
            let mut original_input = source;
            let original = parse_directive_element(&mut original_input)
                .expect("parse supported directive variant");
            assert!(original_input.is_empty());

            let formatted = original.to_string();
            assert_eq!(formatted, source);
            let mut formatted_input = formatted.as_str();
            let reparsed = parse_directive_element(&mut formatted_input)
                .expect("parse canonical directive formatting");
            assert!(formatted_input.is_empty());
            assert_eq!(reparsed, original);
        }
    }

    /// Checks that empty directives and native mdBook helpers are outside the custom grammar.
    #[test]
    fn empty_payloads_and_native_helpers_are_not_custom_directives() {
        for source in [
            "{{cat }}",
            "{{!docs }}",
            "{{#crate }}",
            "{{#badges }}",
            "{{#badges =}}",
            "{{#categories }}",
            "{{#example }}",
            "{{#include refs.incl.md}}",
            "{{#rustdoc_include snippet.rs}}",
            "{{#playground code}}",
        ] {
            let mut input = source;
            assert!(
                parse_directive_element(&mut input).is_err(),
                "source: {source}"
            );
        }
    }
}
