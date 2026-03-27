use winnow::ModalResult;
use winnow::Parser;
use winnow::token::take_till;

use crate::ast::Element;

/// Parses any characters until the next recognized element or end of input.
pub fn parse_plain_text<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    // Consume until the next special character.
    // take_till with 1.. ensures we consume at least one non-special character.
    // This takes advantage of highly optimized SIMD operations in winnow.
    let content = take_till(1.., [
        '<', '[', '(', '`', '{', '!', '#', '-', '|', ']', ')', '}', '*', '_', '~'
    ])
    .parse_next(input)?;

    Ok(Element::Text(crate::TextData { content }))
}

/// Fallback parser that consumes exactly one character.
pub fn parse_fallback_char<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    winnow::token::any
        .take()
        .map(|content| Element::Text(crate::TextData { content }))
        .parse_next(input)
}
