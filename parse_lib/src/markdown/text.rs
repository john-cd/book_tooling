// TODO Reviewed ??-??-2026
use winnow::ModalResult;
use winnow::Parser as _;
use winnow::combinator::alt;
use winnow::token::take_till;

use crate::ast::Element;

/// Parses any characters until the next recognized element or end of input.
///
/// # Errors
///
/// Returns a backtrack error if the input is empty or starts with a character that begins other syntax.
pub fn parse_plain_text<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    // Consume until the next special character.
    // take_till with 1.. ensures we consume at least one non-special character.
    // This takes advantage of highly optimized SIMD operations in winnow.
    let content = take_till(
        1..,
        [
            '<', '[', '(', '`', '{', '!', '#', '-', '|', ']', ')', '}', '*', '_', '~', '\\',
        ],
    )
    .parse_next(input)?;

    // Special check for * or _ or ~ if they are part of a formatting tag
    // This is a bit simplified, but helps avoid over-consuming.
    if input.starts_with('*') || input.starts_with('_') || input.starts_with('~') {
        // If the consumed content ends with something that could be part of the tag,
        // we might have overshot. But take_till stops *at* the character.
    }

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
