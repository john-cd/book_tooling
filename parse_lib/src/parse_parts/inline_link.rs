//! Parses complete Markdown inline links such as `[text](destination)`.
//!
//! TODO Reviewed ??-??-2026.

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

/// Consumes a single-quoted, double-quoted, or parenthesized Markdown link title. The returned
/// slice excludes the delimiters and preserves backslash escapes.
///
/// # Errors
/// Backtracks if the opening or matching unescaped closing delimiter is missing.
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

/// Consumes a Markdown inline link and returns its text, destination, and optional title. An
/// omitted destination becomes an empty string.
///
/// # Errors
/// Backtracks if the link syntax is incomplete or a title follows a destination without intervening
/// whitespace.
pub fn parse_inline_link<'s>(input: &mut &'s str) -> ModalResult<Element<'s>> {
    let (text, _, _, dest, ws, title, ..) = seq!(
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

    // CommonMark spec dictates that if a title is present, it must be separated from the
    // destination by whitespace.
    if title.is_some() && dest.is_some() && ws.is_empty() {
        return fail.parse_next(input);
    }

    Ok(Element::InlineLink(InlineLinkData {
        text,
        url: dest.unwrap_or(""),
        title,
    }))
}

/// Tests for link destinations, titles, and delimiters.
#[cfg(test)]
mod tests {
    use super::*;

    /// `[text](url)` yields the text and URL with no title.
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

    /// A quoted title after the URL is captured.
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
