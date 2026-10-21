// TODO Reviewed ??-??-2026
//! Parses Markdown ATX Headings (with {# anchor} mdbook extension).
//!
//! See <https://spec.commonmark.org/0.31.2/#atx-headings>
//!

use winnow::ModalResult;
use winnow::Parser;
use winnow::ascii::line_ending;
use winnow::ascii::space0;
use winnow::ascii::space1;
use winnow::combinator::alt;
use winnow::combinator::cut_err;
use winnow::combinator::delimited;
use winnow::combinator::eof;
use winnow::combinator::opt;
use winnow::combinator::preceded;
use winnow::combinator::repeat;
use winnow::combinator::seq;
use winnow::combinator::terminated;
use winnow::error::StrContext::*;
use winnow::error::StrContextValue::*;
use winnow::stream::Stream;
use winnow::token::take_while;

use crate::ast::*;

// Identation followed by opening sequence of 1–6 unescaped # characters
fn parse_opening_hashes(input: &mut &str) -> ModalResult<u8> {
    preceded(
        // Parse indentation. Up to three spaces of indentation are allowed.
        take_while(0..=3, ' '),
        // Opening sequence of 1–6 unescaped # characters
        repeat(1.., "#")
            .verify_map(|count: usize| if count <= 6 { Some(count as u8) } else { None }),
    )
    .parse_next(input)
}

/// Parse Attribute Block.
/// This is a simplified parser for attributes focusing on the ID.
///
/// <https://rust-lang.github.io/mdBook/format/markdown.html#heading-attributes>
fn parse_attribute_block<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    let attributes_content_chars = |c: char| {
        c != '{' && c != '}' && c != '<' && c != '>' && c != '\\' && c != '\n' && c != '\r'
    };

    let parse_attributes_content = delimited(
        space0,
        preceded("#", take_while(1.., attributes_content_chars)),
        space0,
    )
    .verify_map(|s: &str| {
        let s = s.trim();
        if s.is_empty() {
            None // No ID specified.
        } else {
            Some(s) // Return the ID.
        }
    });

    terminated(
        delimited("{", parse_attributes_content, cut_err("}")),
        space0,
    )
    .context(Label("attribute block"))
    .context(Expected(Description(
        r"attribute block with ID within {# }. No { } < > \ or newlines inside.",
    )))
    .parse_next(input)
}

// Helper to parse content that doesn't contain a closing hash sequence or attribute block start.
fn parse_heading_content<'s>(
    input: &mut &'s str,
) -> ModalResult<(Option<&'s str>, Option<&'s str>)> {
    // Take characters until we hit a newline or a potential attribute block start '{' or #.
    let mut content =
        take_while(0.., |c| c != '\n' && c != '\r' && c != '{' && c != '#').map(|content: &str| {
            let cnt = content.trim();
            if !cnt.is_empty() { Some(cnt) } else { None }
        });

    // Optional closing sequence of any number of unescaped # characters.
    // The optional closing sequence of #s must be preceded by spaces or tabs and may be followed by spaces or tabs only.
    let mut opt_closing_hashes = opt(terminated(take_while(1.., |c| c == '#'), space0));

    seq!(
        // The opening sequence of # characters must be followed by spaces or tabs.
        _: opt(space1),
        content, // Content of the heading.
        _: opt_closing_hashes,
        opt(parse_attribute_block),
        _: alt((line_ending, eof))
    )
    .context(Label("heading content"))
    .context(Expected(Description(
        " a new line or heading content followed by an optional attribute block",
    )))
    .parse_next(input)
}

/// Parses a Markdown ATX heading.
pub fn parse_atx_heading<'s>(input: &mut &'s str) -> ModalResult<Element<'s>> {
    let start = input.checkpoint();
    let mut hashes = parse_opening_hashes;
    let mut content = parse_heading_content;

    match hashes.parse_next(input) {
        Ok(level) => {
            match content.parse_next(input) {
                Ok((content, id)) => Ok(Element::Heading(HeadingData { level, content, id })),
                Err(_) => {
                    input.reset(&start);
                    // Return a backtrack error instead of a cut error.
                    Err(winnow::error::ErrMode::Backtrack(
                        winnow::error::ContextError::default(),
                    ))
                }
            }
        }
        Err(e) => {
            input.reset(&start);
            Err(e)
        }
    }
}
