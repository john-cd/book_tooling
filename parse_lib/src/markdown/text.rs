//! Parsers for plain text and for the fallback single character.
//!
//! TODO Reviewed ??-??-2026

use winnow::ModalResult;
use winnow::Parser as _;
use winnow::combinator::alt;
use winnow::token::take_till;

use crate::ast::Element;

/// Characters that can begin other inline or block syntax, and therefore end a plain-text run.
const SPECIAL_CHARS: [char; 16] = [
    '<', '[', '(', '`', '{', '!', '#', '-', '|', ']', ')', '}', '*', '_', '~', '\\',
];

/// Parses any characters until the next recognized element or end of input.
///
/// An underscore between two alphanumeric characters (`snake_case`, `fs_extra`) is intraword
/// and part of the text: as in `CommonMark`, it can neither open nor close emphasis.
///
/// # Errors
///
/// Returns a backtrack error if the input is empty or starts with a character that begins other syntax.
pub fn parse_plain_text<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    let source: &'a str = input;
    // take_till with 1.. ensures we consume at least one non-special character.
    // This takes advantage of highly optimized SIMD operations in winnow.
    let mut consumed = take_till(1.., SPECIAL_CHARS).parse_next(input)?.len();

    while let Some(rest) = source.get(consumed..)
        && let Some(after_underscore) = rest.strip_prefix('_')
        && source
            .get(..consumed)
            .and_then(|before| before.chars().next_back())
            .is_some_and(char::is_alphanumeric)
        && after_underscore
            .chars()
            .next()
            .is_some_and(char::is_alphanumeric)
    {
        consumed += 1 + after_underscore
            .find(SPECIAL_CHARS)
            .unwrap_or(after_underscore.len());
    }

    let content = source.get(..consumed).unwrap_or(source);
    *input = source.get(consumed..).unwrap_or_default();
    Ok(Element::Text(crate::ast::TextData { content }))
}

/// Fallback parser that handles escaped characters and consumes exactly one character if not escaped.
///
/// # Errors
///
/// Returns a backtrack error if the input is empty.
pub fn parse_fallback_char<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    alt((
        winnow::combinator::seq!(_: "\\", winnow::token::any).take(),
        winnow::token::any.take(),
    ))
    .map(|content| Element::Text(crate::ast::TextData { content }))
    .parse_next(input)
}
