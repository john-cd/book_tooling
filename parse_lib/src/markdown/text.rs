use winnow::ModalResult;
use winnow::Parser;
use winnow::stream::Offset;
use winnow::stream::Stream;
use winnow::token::any;
use winnow::token::take_while;

use crate::ast::Element;

/// Parses any characters until the next recognized element or end of input.
pub fn parse_plain_text<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    if input.is_empty() {
        return Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::default(),
        ));
    }

    let first_char = input.chars().next().unwrap();
    if matches!(
        first_char,
        '<' | '[' | '(' | '`' | '{' | '!' | '#' | '-' | '|' | ']' | ')' | '}'
    ) {
        // Just consume one special character as text.
        let start = input.checkpoint();
        let _: char = any.parse_next(input)?;
        let offset = input.offset_from(&start);
        input.reset(&start);
        let content = input.next_slice(offset);
        Ok(Element::Text(crate::TextData { content }))
    } else {
        // Consume until the next special character.
        // This is much faster than one-by-one.
        let content = take_while(1.., |c: char| {
            !matches!(
                c,
                '<' | '[' | '(' | '`' | '{' | '!' | '#' | '-' | '|' | ']' | ')' | '}'
            )
        })
        .parse_next(input)?;
        Ok(Element::Text(crate::TextData { content }))
    }
}

/// Fallback parser that consumes exactly one character.
pub fn parse_fallback_char<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    winnow::token::any
        .take()
        .map(|content| Element::Text(crate::TextData { content }))
        .parse_next(input)
}
