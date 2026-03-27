//! Parses a link destination enclosed in angle brackets `<...>`.
//!
//! Simplified from spec: <https://spec.commonmark.org/0.31.2/#link-destination>
//!
//! - A sequence of zero or more characters between an opening `<` and a
//!   closing `>` that contains no line endings or unescaped `<` or `>` characters.

use winnow::ModalResult;
use winnow::Parser;
use winnow::combinator::alt;
use winnow::combinator::delimited;
use winnow::combinator::repeat;
use winnow::error::StrContext::*;
use winnow::error::StrContextValue::*;
use winnow::token::none_of;
use winnow::token::take_while;

/// Parses a link destination bounded by `< >`.
///
/// Example: "<https://example.com>" -> "https://example.com".
pub fn parse_bounded_link_destination<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    let content = repeat::<_, _, (), _, _>(
        0..,
        alt((
            take_while(1.., |c: char| {
                c != '<' && c != '>' && c != '\\' && c != '\n' && c != '\r'
            })
            .void(),
            // Allow any backslash-escaped character (except newlines)
            ('\\', none_of(['\n', '\r'])).void(),
        )),
    )
    .take();

    delimited("<", content, ">")
        .context(Label("bounded link destination"))
        .context(Expected(Description("<url>")))
        .parse_next(input)
}

/// Helper parser for valid raw link destination characters (excluding parens, spaces, controls).
fn raw_link_chars<'s>(input: &mut &'s str) -> ModalResult<()> {
    alt((
        take_while(1.., |c: char| {
            !c.is_ascii_control() && c != ' ' && c != '\\' && c != '(' && c != ')'
        })
        .void(),
        // Allow any backslash-escaped character (except newlines)
        ('\\', none_of(['\n', '\r'])).void(),
    ))
    .parse_next(input)
}

/// Parses a "raw" link destination (not bounded by `< >`).
///
/// Example: "https://example.com" -> "https://example.com".
pub fn parse_raw_link_destination<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    let content = repeat::<_, _, (), _, _>(
        1..,
        alt((
            raw_link_chars,
            // Allow exactly one level of balanced parentheses per CommonMark spec
            delimited("(", repeat::<_, _, (), _, _>(0.., raw_link_chars), ")").void(),
        )),
    )
    .take();

    content
        .verify(|s: &str| !s.starts_with('<'))
        .context(Label("raw link destination"))
        .parse_next(input)
}

/// Parses any valid link destination (either bounded by `< >` or raw).
pub fn parse_link_destination<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    alt((parse_bounded_link_destination, parse_raw_link_destination)).parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bounded_link_destination_simple() {
        let mut input = "<https://example.com>";
        assert_eq!(parse_bounded_link_destination.parse_next(&mut input), Ok("https://example.com"));
        assert_eq!(input, "");
    }

    #[test]
    fn test_parse_bounded_link_destination_with_escaped_brackets() {
        let mut input = r"<https://example.com/\>>";
        assert_eq!(parse_bounded_link_destination.parse_next(&mut input), Ok(r"https://example.com/\>"));
    }

    #[test]
    fn test_parse_bounded_link_destination_fails_on_unescaped_brackets() {
        let mut input = "<https://example.com/<foo>>";
        assert!(parse_bounded_link_destination.parse_next(&mut input).is_err());
    }

    #[test]
    fn test_parse_bounded_link_destination_fails_on_newlines() {
        let mut input = "<https://example.com\n/path>";
        assert!(parse_bounded_link_destination.parse_next(&mut input).is_err());
    }

    #[test]
    fn test_parse_bounded_link_destination_empty() {
        let mut input = "<>";
        assert_eq!(parse_bounded_link_destination.parse_next(&mut input), Ok(""));
    }

    #[test]
    fn test_parse_raw_link_destination_simple() {
        let mut input = "https://example.com";
        assert_eq!(parse_raw_link_destination.parse_next(&mut input), Ok("https://example.com"));
        assert_eq!(input, "");
    }

    #[test]
    fn test_parse_raw_link_destination_with_parens() {
        let mut input = "https://example.com/foo(bar)baz";
        assert_eq!(parse_raw_link_destination.parse_next(&mut input), Ok("https://example.com/foo(bar)baz"));
        assert_eq!(input, "");
    }

    #[test]
    fn test_parse_raw_link_destination_with_escaped_parens() {
        let mut input = r"https://example.com/foo\(bar\)baz";
        assert_eq!(parse_raw_link_destination.parse_next(&mut input), Ok(r"https://example.com/foo\(bar\)baz"));
        assert_eq!(input, "");
    }

    #[test]
    fn test_parse_raw_link_destination_stops_at_space() {
        let mut input = "https://example.com/foo bar";
        assert_eq!(parse_raw_link_destination.parse_next(&mut input), Ok("https://example.com/foo"));
        assert_eq!(input, " bar"); // Correctly leaves the space and remaining text
    }

    #[test]
    fn test_parse_raw_link_destination_fails_on_start_bracket() {
        let mut input = "<https://example.com>";
        assert!(parse_raw_link_destination.parse_next(&mut input).is_err());
    }

    #[test]
    fn test_parse_link_destination_router() {
        let mut input1 = "<https://example.com>";
        assert_eq!(parse_link_destination.parse_next(&mut input1), Ok("https://example.com"));

        let mut input2 = "https://example.com";
        assert_eq!(parse_link_destination.parse_next(&mut input2), Ok("https://example.com"));
    }
}
