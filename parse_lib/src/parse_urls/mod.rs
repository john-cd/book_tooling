use winnow::ModalResult;
use winnow::Result;
use winnow::Parser;
use winnow::combinator::alt;
use winnow::combinator::delimited;
use winnow::combinator::recognize;
use winnow::error::ErrMode;
use winnow::error::StrContext::*;
use winnow::error::StrContextValue::*;
use winnow::token::take_while;

use crate::ast::{AutolinkData, Element};

/// Predicate function to check if a character is one of the valid URL characters.
/// Valid chars in URLs are A-Z, a-z, 0-9, -, ., _, ~, :, /, ?, #, [, ], @, !, $, &, ', (, ), *, +, ,, ;, %, =
/// ":" | "/" | "@" | "?" | "#" have special meaning in URLs.
fn is_valid_url_char(c: char) -> bool {
    matches!(c,
        'A'..='Z'
        | 'a'..='z'
        | '0'..='9'
        | '-' | '.' | '_' | '~'
        | ':' | '/' | '?' | '#' | '[' | ']' | '@' | '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '%' | '='
    )
}

/// Parses one or more valid URL characters.
pub(crate) fn parse_url_chars<'s>(input: &mut &'s str) -> Result<&'s str> {
    take_while(1.., is_valid_url_char)
        .verify(|s: &str| !s.is_empty())
        .context(Label("valid URL characters"))
        .context(Expected(Description(
            "alphanumeric, -, ., _, ~, :, /, ?, #, [, ], @, !, $, &, ', (, ), *, +, ,, ;, %, =",
        )))
        .parse_next(input)
}

/// Parses a naked URL (e.g. `https://example.com`) directly from text.
pub fn parse_naked_url<'s>(input: &mut &'s str) -> ModalResult<Element<'s>> {
    let url_text = recognize((
        alt(("https://", "http://")),
        parse_url_chars,
    ))
    .context(Label("naked URL"))
    .parse_next(input)
    .map_err(ErrMode::Backtrack)?;

    Ok(Element::Autolink(AutolinkData { url: url_text }))
}

/// Parses an autolink enclosed in angle brackets (e.g. `<https://example.com>`).
pub fn parse_autolink<'s>(input: &mut &'s str) -> ModalResult<Element<'s>> {
    let url_text = delimited(
        "<",
        recognize((
            alt(("https://", "http://")),
            take_while(0.., |c: char| c != '>' && c != '<' && !c.is_whitespace()),
        )),
        ">",
    )
    .context(Label("autolink"))
    .parse_next(input)?;

    Ok(Element::Autolink(AutolinkData { url: url_text }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_autolink() {
        let mut input = "<https://example.com/foo>";
        let result = parse_autolink.parse_next(&mut input).unwrap();
        assert_eq!(result, Element::Autolink(AutolinkData { url: "https://example.com/foo" }));
    }

    #[test]
    fn test_parse_url_chars_basic() {
        // Stops at `|`, which is not a reserved char. Otherwise we would get an `Incomplete`.
        assert_eq!(
            parse_url_chars.parse_peek("abcdefg12345|"),
            Ok(("|", "abcdefg12345"))
        );
        // Stops at space.
        assert_eq!(
            parse_url_chars.parse_peek("hello world"),
            Ok((" world", "hello"))
        );
        assert!(parse_url_chars.parse_peek("").is_err());
    }

    #[test]
    fn test_parse_url_chars_reserved() {
        assert_eq!(
            // Stops at `|`.
            parse_url_chars.parse_peek(":/?#[]@!$&'()*+,,;%+=|"),
            Ok(("|", ":/?#[]@!$&'()*+,,;%+="))
        );
        // Mixed.
        assert_eq!(
            parse_url_chars.parse_peek("foo-bar.baz_qux~ "),
            Ok((" ", "foo-bar.baz_qux~"))
        );
    }

    #[test]
    fn test_parse_url_chars_url_like() {
        assert_eq!(
            parse_url_chars.parse_peek("some/path?query#fragment "),
            Ok((" ", "some/path?query#fragment"))
        );
        assert_eq!(
            parse_url_chars
                .parse_peek("http://example.com/path/to/resource?id=123&name=test#section "),
            Ok((
                " ",
                "http://example.com/path/to/resource?id=123&name=test#section"
            ))
        );
    }

    #[test]
    fn test_parse_url_chars_invalid() {
        assert_eq!(
            parse_url_chars.parse_peek("invalid<char"),
            Ok(("<char", "invalid"))
        );
        assert_eq!(
            parse_url_chars.parse_peek("newline\nchar"),
            Ok(("\nchar", "newline"))
        );
    }
}
