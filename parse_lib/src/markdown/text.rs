use winnow::ModalResult;
use winnow::Parser;
use winnow::stream::Offset;
use winnow::stream::Stream;
use winnow::token::any;

use crate::ast::Element;

/// Parses any characters until the next recognized element or end of input.
pub fn parse_plain_text<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    if input.is_empty() {
        return Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::default(),
        ));
    }

    let start = input.checkpoint();
    let _: char = any.parse_next(input)?;
    let offset = input.offset_from(&start);
    input.reset(&start);
    let content = input.next_slice(offset);

    // TODO: optimize by taking more than one char if it doesn't contain special chars.

    Ok(Element::Text(crate::TextData { content }))
}
