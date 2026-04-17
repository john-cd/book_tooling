//! Parses a link text, which is enclosed in square brackets `[]`.
//!
//! Simplified from spec: <https://spec.commonmark.org/0.31.2/#link-text>
//!
//! - A link text begins with a left bracket ([) and ends with the first right bracket (]) that is not backslash-escaped.
//! - Unescaped square bracket characters are allowed only if they are nested and balanced.

use winnow::ModalResult;
use winnow::Parser;
use winnow::combinator::alt;
use winnow::combinator::delimited;
use winnow::combinator::repeat;
use winnow::error::StrContext::*;
use winnow::error::StrContextValue::*;
use winnow::token::none_of;
use winnow::token::take_while;

/// Recursive parser for the inner content of link text, enabling balanced brackets.
fn link_text_content(input: &mut &str) -> ModalResult<()> {
    repeat(
        0.., // Can be empty
        alt((
            take_while(1.., |c: char| {
                c != '[' && c != ']' && c != '\\' && c != '\n' && c != '\r'
            })
            .void(),
            // Allow any backslash-escaped character (except newlines)
            ('\\', none_of(['\n', '\r'])).void(),
            // Allow balanced nested brackets recursively
            delimited('[', link_text_content, ']').void(),
        )),
    )
    .parse_next(input)
}

/// Parses link text.
///
/// Example: "[my text]" -> "my text".
pub fn parse_link_text<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    delimited("[", link_text_content.take(), "]")
        .context(Label("link text"))
        .context(Expected(Description("[link text]")))
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use winnow::Parser;

    use super::*;

    #[test]
    fn test_parse_link_text_simple() {
        let mut input = "[my text]";
        assert_eq!(parse_link_text.parse_next(&mut input), Ok("my text"));
        assert_eq!(input, "");
    }

    #[test]
    fn test_parse_link_text_with_escaped_brackets() {
        let mut input = r"[my \[esc\] text]";
        assert_eq!(
            parse_link_text.parse_next(&mut input),
            Ok(r"my \[esc\] text")
        );
    }

    #[test]
    fn test_parse_link_text_with_arbitrary_escapes() {
        let mut input = r"[my \ text \*]";
        assert_eq!(parse_link_text.parse_next(&mut input), Ok(r"my \ text \*"));
    }

    #[test]
    fn test_parse_link_text_empty() {
        let mut input = "[]";
        assert_eq!(parse_link_text.parse_next(&mut input), Ok(""));
    }

    #[test]
    fn test_parse_link_text_fails_on_unbalanced_bracket() {
        let mut input = "[my [unbalanced text]";
        assert!(parse_link_text.parse_next(&mut input).is_err());
    }

    #[test]
    fn test_parse_link_text_with_balanced_brackets() {
        let mut input = "[my [nested [and] balanced] text]";
        assert_eq!(
            parse_link_text.parse_next(&mut input),
            Ok("my [nested [and] balanced] text")
        );
    }
}
