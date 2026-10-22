//! Parses a complete Markdown inline link: `text`.
//!
//! TODO Reviewed ??-??-2026

use winnow::ModalResult;
use winnow::Parser as _;
use winnow::combinator::alt;
use winnow::combinator::delimited;
use winnow::combinator::fail;
use winnow::combinator::opt;
use winnow::combinator::repeat;
use winnow::combinator::seq;
use winnow::token::any;
use winnow::token::take_while;

use super::link_destination::parse_link_destination;
use super::link_text::parse_link_text;
use crate::ast::Element;
use crate::ast::InlineLinkData;

/// Parses an optional link title enclosed in `""`, `''`, or `()`.
/// Allows for backslash-escaped characters inside the title.
pub(crate) fn parse_link_title<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    alt((
        delimited(
            "\"",
            repeat::<_, _, (), _, _>(
                0..,
                alt((
                    take_while(1.., |c| c != '"' && c != '\\').void(),
                    ('\\', any).void(),
                )),
            )
            .take(),
            "\"",
        ),
        delimited(
            "'",
            repeat::<_, _, (), _, _>(
                0..,
                alt((
                    take_while(1.., |c| c != '\'' && c != '\\').void(),
                    ('\\', any).void(),
                )),
            )
            .take(),
            "'",
        ),
        delimited(
            "(",
            repeat::<_, _, (), _, _>(
                0..,
                alt((
                    take_while(1.., |c| c != ')' && c != '\\').void(),
                    ('\\', any).void(),
                )),
            )
            .take(),
            ")",
        ),
    ))
    .parse_next(input)
}

/// Parses a complete inline link: `text`.
///
/// # Errors
///
/// Returns a backtrack error if the input does not begin with an inline link.
pub fn parse_inline_link<'s>(input: &mut &'s str) -> ModalResult<Element<'s>> {
    let (text, _, _, dest, ws, title, _, _) = seq!(
        parse_link_text,
        "(",
        take_while(0.., [' ', '\t', '\n', '\r']),
        opt(parse_link_destination),
        take_while(0.., [' ', '\t', '\n', '\r']),
        opt(parse_link_title),
        take_while(0.., [' ', '\t', '\n', '\r']),
        ")"
    )
    .parse_next(input)?;

    // CommonMark spec dictates that if a title is present, it must be separated from the destination by whitespace.
    if title.is_some() && dest.is_some() && ws.is_empty() {
        return fail.parse_next(input);
    }

    Ok(Element::InlineLink(InlineLinkData {
        text,
        url: dest.unwrap_or(""),
        title,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_inline_link_simple() {
        let mut input = "[my link](https://example.com)";
        assert_eq!(
            parse_inline_link.parse_next(&mut input),
            Ok(Element::InlineLink(InlineLinkData {
                text: "my link",
                url: "https://example.com",
                title: None
            }))
        );
        assert_eq!(input, "");
    }

    #[test]
    fn test_parse_inline_link_with_title() {
        let mut input = "[my link](https://example.com \"A Title\")";
        assert_eq!(
            parse_inline_link.parse_next(&mut input),
            Ok(Element::InlineLink(InlineLinkData {
                text: "my link",
                url: "https://example.com",
                title: Some("A Title")
            }))
        );
        assert_eq!(input, "");
    }
}
