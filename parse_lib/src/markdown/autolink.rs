use winnow::Parser;
use winnow::combinator::alt;
use winnow::error::StrContext::*;
use winnow::error::StrContextValue::*;
use winnow::prelude::*;
use winnow::token::take_while;

use super::super::parse_parts::link_destination::parse_angle_brackets;
use crate::ast::*;
use crate::parse_urls::parse_naked_url;

/// Parses an autolink (`<url>` or a naked URL).
///
/// <https://www.w3schools.com/tags//ref_urlencode.asp>
pub fn parse_autolink<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    let mut autolink_p = alt((
        parse_angle_brackets,
        take_while(1.., |c: char| {
            !c.is_ascii_control() && c != ' ' && c != '<' && c != '>'
        })
        .verify(|s: &str| s.starts_with("http://") || s.starts_with("https://")),
    ))
    .map(|url| Element::Autolink(AutolinkData { url }))
    .context(Label("autolink"))
    .context(Expected(Description(
        "an autolink starting with < and ending with >; or a naked http(s) URL",
    )));

    autolink_p.parse_next(input)
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
        assert!(parse_autolink.parse_peek("<http://example.com").is_err());
    }

    #[test]
    fn test_parse_autolink_invalid_delimited_missing_opening_tag() {
        // If it doesn't start with '<', it might try parse_naked_url.
        // "http://example.com>" -> parse_naked_url might match "http://example.com" and leave ">".
    }

    #[test]
    fn test_parse_autolink_invalid_naked_no_protocol() {
        assert!(parse_autolink(&mut "example.com").is_err());
    }

    #[test]
    fn test_parse_autolink_empty_string() {
        assert!(parse_autolink(&mut "").is_err());
    }
}
