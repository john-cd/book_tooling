//! Parses link titles.
//!
//! <https://spec.commonmark.org/0.31.2/#link-title>
//!
//! A link title consists of either
//! - a sequence of zero or more characters enclosed in double-quotes ("),
//!   including a " character only if it is backslash-escaped, or
//! - a sequence of zero or more characters enclosed in single-quotes ('),
//!   including a ' character only if it is backslash-escaped, or
//! - a sequence of zero or more characters enclosed in parentheses (()),
//!   including a ) character only if it is backslash-escaped.

use winnow::ModalResult;
use winnow::Parser;
use winnow::combinator::alt;
use winnow::combinator::cut_err;
use winnow::combinator::delimited;
use winnow::combinator::repeat;
use winnow::error::ErrMode;
use winnow::error::StrContext::*;
use winnow::error::StrContextValue::*;
use winnow::token::one_of;
use winnow::token::take_while;

/// Parses a character that is allowed in a link title (not control or the delimiter).
fn is_allowed_normal_char(c: char, delimiter: char) -> bool {
    c != delimiter && c != '\\' && c != '\n' && c != '\r'
}

/// Helper to parse the content of a title based on the delimiter used.
fn parse_title_content<'s>(input: &mut &'s str, delimiter: char) -> ModalResult<&'s str> {
    repeat::<_, _, (), _, _>(
        0..,
        alt((
            take_while(1.., move |c: char| is_allowed_normal_char(c, delimiter)).void(),
            ('\\', one_of(['"', '\'', '(', ')', '\\'])).void(),
        )),
    )
    .take()
    .parse_next(input)
    .map_err(ErrMode::Backtrack)
}

/// Parses a link title enclosed in double quotes.
fn parse_double_quoted_title<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    delimited(
        '"',
        |i: &mut &'s str| parse_title_content(i, '"'),
        cut_err('"'),
    )
    .context(Label("double-quoted link title"))
    .parse_next(input)
}

/// Parses a link title enclosed in single quotes.
fn parse_single_quoted_title<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    delimited(
        '\'',
        |i: &mut &'s str| parse_title_content(i, '\''),
        cut_err('\''),
    )
    .context(Label("single-quoted link title"))
    .parse_next(input)
}

/// Parses a link title enclosed in parentheses.
fn parse_parenthesized_title<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    delimited(
        '(',
        |i: &mut &'s str| parse_title_content(i, ')'),
        cut_err(')'),
    )
    .context(Label("parenthesized link title"))
    .parse_next(input)
}

/// The main parser for a link title.
pub fn parse_link_title<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    alt((
        parse_double_quoted_title,
        parse_single_quoted_title,
        parse_parenthesized_title,
    ))
    .context(Label("link title"))
    .context(Expected(Description(
        "a sequence of characters enclosed in \", ' or ()",
    )))
    .parse_next(input)
}
