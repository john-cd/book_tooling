//! Parses link destinations between < and >.
//!
//! A link destination can consist of a sequence of zero or more characters between an opening < and
//! a closing > that contains no line endings or unescaped < or > characters.
//! See <https://spec.commonmark.org/0.31.2/#link-destination>.
//!
//! TODO Reviewed ??-??-2026.

use winnow::Result;
use winnow::combinator::alt;
use winnow::combinator::delimited;
use winnow::combinator::fail;
use winnow::combinator::repeat;
use winnow::error::ContextError;
use winnow::error::ErrMode;
use winnow::error::StrContext::Expected;
use winnow::error::StrContext::Label;
use winnow::error::StrContextValue::Description;
use winnow::prelude::*;
use winnow::token::take_while;

/// Parses a sequence of characters that are not < > \ or newlines.
fn parse_non_special_chars<'s>(input: &mut &'s str) -> Result<&'s str> {
    take_while(1.., |c: char| {
        c != '<' && c != '>' && c != '\\' && c != '\n' && c != '\r'
    })
    .context(Label("non-special characters"))
    .context(Expected(Description(
        r"a sequence of characters that are not < > \ or newlines",
    )))
    .parse_next(input)
}

/// Parses an escaped angle bracket e.g., \< or \> or \\.
fn parse_escaped_chars<'s>(input: &mut &'s str) -> Result<&'s str> {
    ('\\', alt(("<", ">", r"\")))
        .take()
        .context(Label("escaped angle bracket"))
        .context(Expected(Description(r"\< or \> or \\")))
        .parse_next(input)
}

/// Parses characters allowed inside < and > brackets: non-special or \< or \>.
fn parse_content_chars<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    repeat::<_, _, String, _, _>(
        0..,
        alt((
            parse_escaped_chars,
            parse_non_special_chars,
            fail.context(Label("character between < and >"))
                .context(Expected(Description(
                    "chars that are not '<', '>', or newlines; or escaped < or >",
                ))),
        )),
    )
    .take()
    .parse_next(input)
    .map_err(|e: ContextError| ErrMode::Backtrack(e))
}

/// Parse link destination between angle brackets: '<' followed by zero or more `parse_content_char`
/// sequences, followed by '>'. Returns the content between the angle brackets.
pub(crate) fn parse_angle_brackets<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    delimited("<", parse_content_chars, ">")
        .context(Label("link destination between angle brackets"))
        .context(Expected(Description(
            "< followed by zero or more content characters, followed by >",
        )))
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    // parse_escaped_chars -----------

    /// A backslash and the character after it are consumed together and returned with the
    /// backslash.
    #[test]
    fn test_parse_escaped_char() {
        assert_eq!(
            parse_escaped_chars.parse_peek(r"\<text"),
            Ok(("text", r"\<"))
        );
    }

    // parse_content_chars ------------

    /// Ordinary characters are consumed up to the end of the input.
    #[test]
    fn test_parse_content_chars_basic() {
        assert_eq!(parse_content_chars.parse_peek("abc123"), Ok(("", "abc123")));
    }

    /// An escaped `<` is content, not a stop character.
    #[test]
    fn test_parse_content_chars_with_escaped_lt() {
        assert_eq!(parse_content_chars.parse_peek(r"\<foo"), Ok(("", r"\<foo")));
    }

    /// An escaped `>` is content, not a stop character.
    #[test]
    fn test_parse_content_chars_with_escaped_gt() {
        assert_eq!(parse_content_chars.parse_peek(r"\>bar"), Ok(("", r"\>bar")));
    }

    /// Plain text, escaped angle brackets and an escaped backslash can appear together in one run.
    #[test]
    fn test_parse_content_chars_mixed_escaped_and_normal() {
        assert_eq!(
            parse_content_chars.parse_peek(r"foo\>bar\<baz\\quux"),
            Ok(("", r"foo\>bar\<baz\\quux"))
        );
    }

    /// Empty input yields empty content without failing.
    #[test]
    fn test_parse_content_chars_empty() {
        assert_eq!(parse_content_chars.parse_peek(""), Ok(("", "")));
    }

    /// An unescaped `<` ends the content run and stays in the remainder.
    #[test]
    fn test_parse_content_chars_stops_at_special_char() {
        // Should stop before the '<'.
        assert_eq!(
            parse_content_chars.parse_peek("foo<bar"),
            Ok(("<bar", "foo"))
        );
    }

    /// A newline ends the content run, because link destinations cannot span lines.
    #[test]
    fn test_parse_content_chars_stops_at_newline() {
        // Should stop before the newline.
        assert_eq!(
            parse_content_chars.parse_peek("foo\nbar"),
            Ok(("\nbar", "foo"))
        );
    }

    /// Input starting with an unescaped `<` yields empty content and consumes nothing.
    #[test]
    fn test_parse_content_chars_only_special_char() {
        // Should fail to parse any content.
        assert_eq!(parse_content_chars.parse_peek("<"), Ok(("<", "")));
    }

    // parse_angle_brackets -----------

    /// `<hello world>` yields its contents, spaces included, without the brackets.
    #[test]
    fn test_basic_parse() {
        assert_eq!(
            parse_angle_brackets.parse_peek("<hello world>"),
            Ok(("", "hello world"))
        );
    }

    /// An escaped `<` inside the brackets stays in the destination text.
    #[test]
    fn test_with_escaped_chars() {
        assert_eq!(
            parse_angle_brackets.parse_peek(r"<with\<escaped chars>"),
            Ok(("", r"with\<escaped chars"))
        );
    }

    /// `<>` is a valid, empty destination.
    #[test]
    fn test_empty_content() {
        assert_eq!(parse_angle_brackets.parse_peek("<>"), Ok(("", "")));
    }

    /// A newline between the brackets makes the destination invalid.
    #[test]
    fn test_contains_newline_error() {
        // This should fail because of the newline.
        parse_angle_brackets(&mut "<no\nnewlines>").expect_err("input should be rejected");
    }

    /// Input that does not start with `<` is rejected.
    #[test]
    fn test_starts_with_invalid_char_error() {
        // This should fail because it doesn't start with '<'.
        parse_angle_brackets(&mut "just text").expect_err("input should be rejected");
    }
}
