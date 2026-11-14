//! Parser for autolinks, delimited (`<https://...>`) or naked.
//!
//! TODO Reviewed ??-??-2026.

use winnow::Parser as _;
use winnow::combinator::alt;
use winnow::error::StrContext::Expected;
use winnow::error::StrContext::Label;
use winnow::error::StrContextValue::Description;
use winnow::prelude::*;

use super::super::parse_parts::link_destination::parse_angle_brackets;
use crate::ast::AutolinkData;
use crate::ast::Element;
use crate::parse_urls::parse_naked_url;

/// See <https://www.w3schools.com/tags//ref_urlencode.asp>.
/// # Errors
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

/// Tests for Markdown autolink parsing.
#[cfg(test)]
mod tests {
    use super::*;

    /// A naked `http://` URL is returned as an autolink.
    #[test]
    fn test_parse_autolink_naked_http() {
        assert_eq!(
            parse_autolink.parse_peek("http://example.com"),
            Ok((
                "",
                Element::Autolink(AutolinkData {
                    url: "http://example.com"
                })
            ))
        );
    }

    /// `<http://...>` yields the URL without the angle brackets.
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

    /// `<https://...>` with a path yields the full URL.
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

    /// Text after the closing `>` is left unconsumed.
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

    /// `<http://...` without a closing `>` is rejected.
    #[test]
    fn test_parse_autolink_invalid_delimited_missing_closing_tag() {
        parse_autolink
            .parse_peek("<http://example.com")
            .expect_err("input should be rejected");
    }

    /// Without a leading `<` the naked-URL parser takes over and stops before the stray `>`.
    #[test]
    fn test_parse_autolink_invalid_delimited_missing_opening_tag() {
        // Without a leading `<` the naked-URL parser takes over and stops before the stray `>`.
        assert_eq!(
            parse_autolink.parse_peek("http://example.com>"),
            Ok((
                ">",
                Element::Autolink(AutolinkData {
                    url: "http://example.com"
                })
            ))
        );
    }

    /// A bare domain without `http://` or `https://` is not an autolink.
    #[test]
    fn test_parse_autolink_invalid_naked_no_protocol() {
        parse_autolink(&mut "example.com").expect_err("input should be rejected");
    }

    /// Empty input is rejected.
    #[test]
    fn test_parse_autolink_empty_string() {
        parse_autolink(&mut "").expect_err("input should be rejected");
    }
}
