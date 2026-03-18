use winnow::ModalResult;
use winnow::Parser;
use winnow::error::StrContext::*;
use winnow::error::StrContextValue::*;
use winnow::token::take_till;

use crate::ast::Element;

/// Parses any characters until the next recognized element or end of input.
pub fn parse_plain_text<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    take_till(1.., ['<', '[', '(', '`', '{', '#', '\n', '\r'])
        .map(|content| Element::Text(crate::TextData { content }))
        .context(Label("plain text"))
        .context(Expected(Description(
            "text without special characters like <, [, (, `, {, #, or newlines",
        )))
        .parse_next(input)
}

/// Fallback parser that consumes exactly one character.
pub fn parse_fallback_char<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    winnow::token::any.take()
        .map(|content| Element::Text(crate::TextData { content }))
        .parse_next(input)
}
