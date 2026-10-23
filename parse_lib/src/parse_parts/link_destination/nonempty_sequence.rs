//! Parse naked link destinations (e.g. naked URLs).
//!
//! Simplified from <https://spec.commonmark.org/0.31.2/#link-destination>.
//!
//! A link destination can be a nonempty sequence of characters that does not start with <,
//! does not include ASCII control characters or space character, and
//! includes parentheses only if (a) they are backslash-escaped or
//! (b) they are part of a balanced pair of unescaped parentheses.
//! (Implementations may impose limits on parentheses nesting to avoid performance issues,
//! but at least three levels of nesting should be supported.)
//!
//! TODO Reviewed ??-??-2026

use winnow::ModalResult;
use winnow::Parser as _;
use winnow::combinator::alt;
use winnow::combinator::delimited;
use winnow::combinator::repeat;
use winnow::error::ContextError;
use winnow::error::ErrMode;
use winnow::error::StrContext::Expected;
use winnow::error::StrContext::Label;
use winnow::error::StrContextValue::Description;
use winnow::token::any;
use winnow::token::take_while;

/// Returns true if a character that is not control, space, < ( ) or \.
fn is_allowed_char(c: char) -> bool {
    !(c.is_ascii_control() || c == ' ' || c == '<' || c == '(' || c == ')' || c == '\\')
}

/// Parses text that may contain escaped parentheses, e.g., "\(" or "\)".
///
/// It must consume at least one character.
fn parse_text_with_escapes<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    repeat::<_, _, (), _, _>(
        1..,
        alt((take_while(1.., is_allowed_char).void(), ('\\', any).void())),
    )
    .take()
    .context(Label("text"))
    .context(Expected(Description(
        r"text that may contain escaped parentheses or backslashes, e.g., \( or \) or \\",
    )))
    .parse_next(input)
    .map_err(|e: ContextError| ErrMode::Backtrack(e))
}

/// Recursively parses content within balanced parentheses.
/// It can contain allowed characters, escaped parentheses, or nested balanced parentheses.
fn parse_balanced_parentheses_content<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    repeat::<_, _, (), _, _>(
        0..,
        alt((
            parse_balanced_parentheses.void(), // Recursively parse.
            parse_text_with_escapes.void(),
        )),
    )
    .take()
    .parse_next(input)
}

/// Parses a balanced pair of unescaped parentheses, e.g., "(foo(bar)baz)".
/// The content inside can be anything allowed by `balanced_parentheses_content`.
fn parse_balanced_parentheses<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    delimited(
        "(",
        parse_balanced_parentheses_content,
        ")", // An unmatched `(` backtracks so the text stays literal.
    )
    .take()
    .context(Label("balanced pair of unescaped parentheses"))
    .parse_next(input)
}

/// The main parser for a non-empty sequence of characters not starting by `<`.
pub(crate) fn parse_non_empty_sequence<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    // Ensure the first character is not '<'.
    winnow::combinator::peek(any.verify(|c| *c != '<' && *c != ' ' && *c != ')'))
        .parse_next(input)?;

    let destination = repeat::<_, _, (), _, _>(
        1..,
        alt((
            parse_balanced_parentheses.void(),
            parse_text_with_escapes.void(),
        )),
    )
    .take()
    .parse_next(input)?;

    // Parsing stopped at an unmatched `(`: this is not a valid destination, so let the caller keep the text literal.
    if input.starts_with('(') {
        return Err(ErrMode::Backtrack(ContextError::new()));
    }
    Ok(destination)
}

#[cfg(test)]
mod tests_parse_non_empty_sequence {
    use super::*;

    /// Plain text, escaped and balanced parentheses are consumed; the run stops at an unmatched `)`, a space or a control character.
    #[test]
    fn test_valid_sequences() {
        assert_eq!(
            parse_non_empty_sequence.parse_peek("hello"),
            Ok(("", "hello"))
        );
        assert_eq!(
            parse_non_empty_sequence.parse_peek(r"h\(ello\)"),
            Ok(("", r"h\(ello\)"))
        );
        assert_eq!(
            parse_non_empty_sequence.parse_peek(r"just\(a\)char"),
            Ok(("", r"just\(a\)char"))
        );
        assert_eq!(
            parse_non_empty_sequence.parse_peek("foo(bar)baz"),
            Ok(("", "foo(bar)baz"))
        );
        assert_eq!(
            parse_non_empty_sequence.parse_peek("a(b)c(d)e"),
            Ok(("", "a(b)c(d)e"))
        );
        assert_eq!(
            parse_non_empty_sequence.parse_peek("a(b(c)d)e"),
            Ok(("", "a(b(c)d)e"))
        );
        assert_eq!(parse_non_empty_sequence.parse_peek("(a)"), Ok(("", "(a)")));
        assert_eq!(
            parse_non_empty_sequence.parse_peek("foo)bar"),
            Ok((")bar", "foo"))
        );
        assert_eq!(
            parse_non_empty_sequence.parse_peek(r"parens\(escaped\)and(balanced)"),
            Ok(("", r"parens\(escaped\)and(balanced)"))
        );
        assert_eq!(
            parse_non_empty_sequence.parse_peek(r"a(b\(c\)d)e"),
            Ok(("", r"a(b\(c\)d)e"))
        ); // Nested with escaped.
        // Contains space.
        assert_eq!(
            parse_non_empty_sequence.parse_peek("hello world"),
            Ok((" world", "hello",),)
        );
        // Contains control character (null).
        assert_eq!(
            parse_non_empty_sequence.parse_peek("hello\0world"),
            Ok(("\0world", "hello",))
        );
    }

    /// Empty input, a leading `<`, space or `)`, and unbalanced parentheses are rejected.
    #[test]
    fn test_invalid_sequences() {
        // Empty string.
        parse_non_empty_sequence(&mut "").expect_err("input should be rejected");
        // Starts with '<'.
        parse_non_empty_sequence(&mut "<hello").expect_err("input should be rejected");
        // Starts with ' '.
        parse_non_empty_sequence(&mut " hello").expect_err("input should be rejected");
        // Starts with ')'.
        parse_non_empty_sequence(&mut ")hello").expect_err("input should be rejected");
        // Unbalanced parentheses.
        parse_non_empty_sequence(&mut "foo(bar").expect_err("input should be rejected");
        parse_non_empty_sequence(&mut "(foo").expect_err("input should be rejected");
        parse_non_empty_sequence(&mut "(a(b)c").expect_err("input should be rejected");
    }
}

#[cfg(test)]
mod text_with_escapes_tests {
    use super::*;

    /// Plain text and backslash-escaped parentheses or backslashes are consumed, wherever they appear in the run.
    #[test]
    fn test_text_with_escapes_basic() {
        // Simple allowed chars.
        assert_eq!(
            parse_text_with_escapes.parse_peek("hello"),
            Ok(("", "hello"))
        );
        // Escaped parenthesis.
        assert_eq!(parse_text_with_escapes.parse_peek(r"\("), Ok(("", r"\(")));
        assert_eq!(parse_text_with_escapes.parse_peek(r"\)"), Ok(("", r"\)")));
        // Escaped backslash.
        assert_eq!(
            parse_text_with_escapes.parse_peek(r"foo\\bar"),
            Ok(("", r"foo\\bar"))
        );
        // Mixed escapes.
        assert_eq!(
            parse_text_with_escapes.parse_peek(r"foo\)bar\("),
            Ok(("", r"foo\)bar\("))
        );
        // Escaped parenthesis in the middle.
        assert_eq!(
            parse_text_with_escapes.parse_peek(r"abc\)def"),
            Ok(("", r"abc\)def"))
        );
        // Escaped parenthesis at the start.
        assert_eq!(
            parse_text_with_escapes.parse_peek(r"\)abc"),
            Ok(("", r"\)abc"))
        );
        // Escaped parenthesis at the end.
        assert_eq!(
            parse_text_with_escapes.parse_peek(r"abc\("),
            Ok(("", r"abc\("))
        );
    }

    /// The run stops at the first unescaped parenthesis, also when escapes precede it.
    #[test]
    fn test_text_with_escapes_partial() {
        // Stops at unescaped parenthesis.
        assert_eq!(
            parse_text_with_escapes.parse_peek("foo(bar"),
            Ok(("(bar", "foo"))
        );
        // Stops at unescaped parenthesis after escapes.
        assert_eq!(
            parse_text_with_escapes.parse_peek(r"foo\)bar(baz"),
            Ok(("(baz", r"foo\)bar"))
        );
        // Stops at unescaped parenthesis after escaped.
        assert_eq!(
            parse_text_with_escapes.parse_peek(r"\(foo)bar"),
            Ok((")bar", r"\(foo"))
        );
    }

    /// Input starting with a parenthesis, `<`, a space or a control character, or ending in a lone backslash, is rejected.
    #[test]
    fn test_text_with_escapes_invalid() {
        // Starts with forbidden char.
        parse_text_with_escapes(&mut "(").expect_err("input should be rejected");
        parse_text_with_escapes(&mut ")").expect_err("input should be rejected");
        parse_text_with_escapes(&mut "<").expect_err("input should be rejected");
        parse_text_with_escapes(&mut " ").expect_err("input should be rejected");
        parse_text_with_escapes(&mut "\0").expect_err("input should be rejected");
        // A lone backslash at the end of the input has nothing to escape.
        parse_text_with_escapes(&mut "\\").expect_err("input should be rejected");
    }
}
