//! Parses a link label, which is enclosed in square brackets `[]`.
//!
//! Simplified from spec: <https://spec.commonmark.org/0.31.2/#link-label>
//!
//! - A link label begins with a left bracket ([) and ends with the first right bracket (]) that is not backslash-escaped.
//! - Between these brackets there must be at least one character that is not a space, tab, or line ending.
//! - Unescaped square bracket characters are not allowed inside the opening and closing square brackets of link labels.
//! - A link label can have at most 999 characters inside the square brackets.

use winnow::ModalResult;
use winnow::Parser;
use winnow::combinator::alt;
use winnow::combinator::delimited;
use winnow::combinator::repeat;
use winnow::error::StrContext::*;
use winnow::error::StrContextValue::*;
use winnow::token::one_of;
use winnow::token::take_while;

/// Parses link label.
///
/// Example: "[my label]" -> "my label".
pub fn parse_link_label<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    // According to spec, link labels can contain escaped brackets.
    // However, they cannot contain unescaped brackets.
    let label_content = repeat::<_, _, (), _, _>(
        1..,
        alt((
            take_while(1.., |c: char| {
                c != '[' && c != ']' && c != '\\' && c != '\n' && c != '\r'
            })
            .void(),
            ('\\', one_of(['[', ']', '\\'])).void(),
        )),
    )
    .take()
    .verify(|s: &str| !s.trim().is_empty() && s.len() <= 999);

    let mut label_content = repeat::<_, _, (), _, _>(1.., alt((parse_escaped, parse_normal)))
        .take()
        .verify(|s: &str| {
            let len = s.chars().count();
            (1..=999).contains(&len) && !s.trim().is_empty()
        });

    delimited("[", label_content, "]")
        .context(Label("link label"))
        .context(Expected(Description(
            "[my label]. Max 999 chars. No newline allowed.",
        )));

    parser.parse_next(input)
}
