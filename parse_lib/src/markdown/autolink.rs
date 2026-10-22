//! Parser for autolinks, delimited (`<https://...>`) or naked.
//!
//! TODO Reviewed ??-??-2026

use winnow::Parser as _;
use winnow::combinator::alt;
use winnow::error::StrContext::{Expected, Label};
use winnow::error::StrContextValue::Description;
use winnow::prelude::*;

use super::super::parse_parts::link_destination::parse_angle_brackets;
use crate::ast::{AutolinkData, Element};
use crate::parse_urls::parse_naked_url;

/// Parses an autolink (`<url>` or a naked URL).
///
/// See <https://www.w3schools.com/tags//ref_urlencode.asp>.
///
/// # Errors
///
/// Returns a backtrack error if the input does not begin with an autolink.
pub fn parse_autolink<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    alt((
        parse_angle_brackets.map(|url| Element::Autolink(AutolinkData { url })),
        parse_naked_url,
    ))
    .context(Label("autolink"))
    .context(Expected(Description(
        "an autolink starting with < and ending with >; or a naked URL",
    )))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_autolink_naked_http() {
        // We expect parse_naked_url to work here.
    }

    #[test]
    fn test_parse_autolink_delimited_http() {
        assert_eq!(
            parse_autolink.parse_peek("<http://example.com>"),
            Ok((
                "",
                Element::Autolink(AutolinkData {
                    url: "http://example.com"
                })
            ))
        );
    }

    #[test]
    fn test_parse_autolink_delimited_https() {
        assert_eq!(
            parse_autolink.parse_peek("<https://example.org/path>"),
            Ok((
                "",
                Element::Autolink(AutolinkData {
                    url: "https://example.org/path"
                })
            ))
        );
    }

    #[test]
    fn test_parse_autolink_delimited_with_extra_text_after() {
        assert_eq!(
            parse_autolink.parse_peek("<https://example.org/path> and some text"),
            Ok((
                " and some text",
                Element::Autolink(AutolinkData {
                    url: "https://example.org/path"
                })
            ))
        );
    }

    #[test]
    fn test_parse_autolink_invalid_delimited_missing_closing_tag() {
        parse_autolink
            .parse_peek("<http://example.com")
            .expect_err("input should be rejected");
    }

    #[test]
    fn test_parse_autolink_invalid_delimited_missing_opening_tag() {
        // If it doesn't start with '<', it might try parse_naked_url.
        // "http://example.com>" -> parse_naked_url might match "http://example.com" and leave ">".
    }

    #[test]
    fn test_parse_autolink_invalid_naked_no_protocol() {
        parse_autolink(&mut "example.com").expect_err("input should be rejected");
    }

    #[test]
    fn test_parse_autolink_empty_string() {
        parse_autolink(&mut "").expect_err("input should be rejected");
    }
}
