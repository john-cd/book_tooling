//! Parse naked link destinations (e.g. naked URLs).
//! Simplified from <https://spec.commonmark.org/0.31.2/#link-destination>.
//! A link destination can be a nonempty sequence of characters that does not start with <,
//! does not include ASCII control characters or space character, and
//! includes parentheses only if (a) they are backslash-escaped or
//! (b) they are part of a balanced pair of unescaped parentheses.
//! (Implementations may impose limits on parentheses nesting to avoid performance issues,
//! but at least three levels of nesting should be supported.)
//!
//! TODO Reviewed ??-??-2026.

use winnow::ModalResult;
use winnow::Parser as _;
use winnow::error::ContextError;
use winnow::error::ErrMode;
use winnow::token::any;

/// Returns true if a character that is not control, space, < ( ) or \.
fn is_allowed_char(c: char) -> bool {
    !(c.is_ascii_control() || c == ' ' || c == '<' || c == '(' || c == ')' || c == '\\')
}
/// Returns the length in bytes of the longest link destination at the start of `text`, or `None`
/// if there is none or it contains an unmatched `(`.
///
/// Parentheses nest, so the scan counts depth iteratively: hostile input such as thousands of
/// `(` must not exhaust the stack. A backslash escapes the following character; the destination
/// stops at an unmatched `)`, a space, `<` or a control character, and a lone trailing backslash
/// is left out.
fn destination_len(text: &str) -> Option<usize> {
    let mut depth = 0_usize;
    let mut end = 0_usize;
    let mut chars = text.char_indices();
    while let Some((offset, c)) = chars.next() {
        match c {
            '\\' => {
                let Some((escaped_offset, escaped)) = chars.next() else {
                    break;
                };
                end = escaped_offset + escaped.len_utf8();
                continue;
            }
            '(' => depth += 1,
            ')' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            other if is_allowed_char(other) => {}
            _ => break,
        }
        end = offset + c.len_utf8();
    }
    // An unmatched `(` makes this not a destination, so the caller keeps the text literal.
    (depth == 0 && end > 0).then_some(end)
}

/// The main parser for a non-empty sequence of characters not starting by `<`.
pub(crate) fn parse_non_empty_sequence<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    // Ensure the first character is not '<'.
    winnow::combinator::peek(any.verify(|c| *c != '<' && *c != ' ' && *c != ')'))
        .parse_next(input)?;

    let source: &'s str = input;
    let Some(end) = destination_len(source) else {
        return Err(ErrMode::Backtrack(ContextError::new()));
    };
    let destination = source.get(..end).unwrap_or_default();
    *input = source.get(end..).unwrap_or_default();
    Ok(destination)
}

/// Tests for bare destinations, escapes, and balanced parentheses.
#[cfg(test)]
mod tests_parse_non_empty_sequence {
    use super::*;

    /// Plain text, escaped and balanced parentheses are consumed; the run stops at an unmatched
    /// `)`, a space or a control character.
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

    /// Tens of thousands of nested or unmatched parentheses must not overflow the stack.
    #[test]
    fn test_extreme_nesting_does_not_recurse() {
        let depth = 100_000;
        let balanced = format!("{}x{}", "(".repeat(depth), ")".repeat(depth));
        assert_eq!(
            parse_non_empty_sequence.parse_peek(balanced.as_str()),
            Ok(("", balanced.as_str()))
        );
        let unmatched = "(".repeat(depth);
        parse_non_empty_sequence(&mut unmatched.as_str()).expect_err("unmatched parentheses");
    }

    /// An unmatched `(` after a complete group rejects the whole destination, and a dangling
    /// backslash or multi-byte text is handled at character boundaries.
    #[test]
    fn test_unmatched_after_group_and_unicode() {
        parse_non_empty_sequence(&mut "a(b)c(d").expect_err("trailing unmatched");
        parse_non_empty_sequence(&mut "a(b c)").expect_err("space inside parentheses");
        assert_eq!(
            parse_non_empty_sequence.parse_peek("caf\u{e9}(\u{f6})x y"),
            Ok((" y", "caf\u{e9}(\u{f6})x"))
        );
        assert_eq!(
            parse_non_empty_sequence.parse_peek("ab\\"),
            Ok(("\\", "ab"))
        );
    }
}
