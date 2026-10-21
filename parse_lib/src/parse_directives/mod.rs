// TODO Reviewed ??-??-2026
//! Parsers for custom `{{ ... }}` directives.

use winnow::ModalResult;
use winnow::Parser;
use winnow::ascii::space0;
use winnow::combinator::alt;
use winnow::combinator::delimited;
use winnow::combinator::opt;
use winnow::combinator::preceded;
use winnow::combinator::repeat;
use winnow::error::StrContext::*;
use winnow::error::StrContextValue::*;
use winnow::token::take_until;
use winnow::token::take_while;

use crate::DirectiveData;
use crate::ast::DestinationKind;
use crate::ast::Element;

/// Parses one custom directive and wraps it as [`Element::CustomDirective`].
pub fn parse_directive_element<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    alt((
        parse_link_directive,
        parse_badge_directive,
        parse_crate_block_directive,
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

fn parse_destination_kind(input: &mut &str) -> ModalResult<DestinationKind> {
    alt((
        "cat".value(DestinationKind::Category),
        "crate".value(DestinationKind::Crate),
        "docs".value(DestinationKind::Docs),
        "github".value(DestinationKind::GithubRepo),
        "lib.rs".value(DestinationKind::LibRs),
        "crates.io".value(DestinationKind::CratesIo),
        "web".value(DestinationKind::Web),
    ))
    .parse_next(input)
}

fn parse_link_directive<'a>(input: &mut &'a str) -> ModalResult<DirectiveData<'a>> {
    delimited(
        "{{",
        (
            parse_destination_kind,
            preceded(space0, take_until(1.., "}}")),
        )
            .map(|(kind, name)| DirectiveData::Link {
                kind,
                name: name.trim(),
            }),
        "}}",
    )
    .parse_next(input)
}

fn parse_badge_directive<'a>(input: &mut &'a str) -> ModalResult<DirectiveData<'a>> {
    delimited(
        "{{!",
        (
            parse_destination_kind,
            preceded(space0, take_until(1.., "}}")),
        )
            .map(|(kind, name)| DirectiveData::Badge {
                kind,
                name: name.trim(),
            }),
        "}}",
    )
    .parse_next(input)
}

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

fn parse_category_block_directive<'a>(input: &mut &'a str) -> ModalResult<DirectiveData<'a>> {
    delimited(
        "{{#categories",
        (opt(":"), space0, take_until(1.., "}}")).map(|(_, _, categories): (_, _, &str)| {
            DirectiveData::CategoryBlock {
                categories: split_category_arguments(categories),
            }
        }),
        "}}",
    )
    .parse_next(input)
}

fn split_category_arguments(input: &str) -> Vec<&str> {
    let mut categories = Vec::new();
    let mut start = None;
    let mut in_quotes = false;

    for (index, character) in input.char_indices() {
        match character {
            '"' => in_quotes = !in_quotes,
            ',' | ' ' | '\t' if !in_quotes => {
                if let Some(start) = start.take() {
                    let category = input[start..index].trim();
                    if !category.is_empty() {
                        categories.push(category);
                    }
                }
            }
            _ => {
                start.get_or_insert(index);
            }
        }
    }

    if let Some(start) = start {
        let category = input[start..].trim();
        if !category.is_empty() {
            categories.push(category);
        }
    }

    categories
}

fn parse_example_block_directive<'a>(input: &mut &'a str) -> ModalResult<DirectiveData<'a>> {
    // Nested directives: {{#example {{#include ...}}}}
    // Actually, commonmark-style directives usually aren't that complex but let's allow
    // the content to contain {{ if we can.
    delimited("{{#example ", take_until(1.., "}}"), "}}")
        .map(|name: &str| DirectiveData::ExampleBlock { name: name.trim() })
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
