//! Parses custom directives.

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

/// Parses a custom directive element.
pub fn parse_directive_element<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    alt((
        parse_link_directive,
        parse_badge_directive,
        parse_crate_block_directive,
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
                    additional_categories: cats,
                },
            ),
        "}}",
    )
    .parse_next(input)
}

fn parse_example_block_directive<'a>(input: &mut &'a str) -> ModalResult<DirectiveData<'a>> {
    delimited("{{#example ", take_until(1.., "}}"), "}}")
        .map(|name: &str| DirectiveData::ExampleBlock { name: name.trim() })
        .parse_next(input)
}
