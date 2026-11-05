//! Parses a link label, which is enclosed in square brackets `[]`.
//!
//! Simplified from spec: <https://spec.commonmark.org/0.31.2/#link-label>
//!
//! - A link label begins with a left bracket ([) and ends with the first right bracket (]) that is
//!   not backslash-escaped.
//! - Between these brackets there must be at least one character that is not a space, tab, or line
//!   ending.
//! - Unescaped square bracket characters are not allowed inside the opening and closing square
//!   brackets of link labels.
//! - A link label can have at most 999 characters inside the square brackets.
//!
//! TODO Reviewed ??-??-2026.

use winnow::ModalResult;
use winnow::Parser as _;
use winnow::combinator::alt;
use winnow::combinator::delimited;
use winnow::combinator::repeat;
use winnow::error::StrContext::Expected;
use winnow::error::StrContext::Label;
use winnow::error::StrContextValue::Description;
use winnow::token::none_of;
use winnow::token::take_while;

/// Parses link label.
///
/// Example: "[my label]" -> "my label".
pub(crate) fn parse_link_label<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    // According to spec, link labels can contain escaped brackets.
    // However, they cannot contain unescaped brackets.
    let label_content = repeat::<_, _, (), _, _>(
        1..,
        alt((
            take_while(1.., |c: char| {
                c != '[' && c != ']' && c != '\\' && c != '\n' && c != '\r'
            })
            .void(),
            // Allow any backslash-escaped character (except newlines)
            ('\\', none_of(['\n', '\r'])).void(),
        )),
    )
    .take()
    .verify(|s: &str| !s.trim().is_empty() && s.chars().count() <= 999);

    delimited("[", label_content, "]")
        .context(Label("link label"))
        .context(Expected(Description(
            "[my label]. Max 999 chars. No newline allowed.",
        )))
        .parse_next(input)
}

#[cfg(test)]
mod tests {

    use super::*;

    /// A bracketed label yields its text and consumes the closing bracket.
    #[test]
    fn test_parse_link_label_simple() {
        let mut input = "[my label]";
        assert_eq!(parse_link_label.parse_next(&mut input), Ok("my label"));
        assert_eq!(input, "");
    }

    /// A backslash before a non-bracket character is kept literally in the label.
    #[test]
    fn test_parse_link_label_with_arbitrary_escapes() {
        let mut input = r"[my \ label \*]";
        assert_eq!(
            parse_link_label.parse_next(&mut input),
            Ok(r"my \ label \*")
        );
    }

    /// Escaped brackets inside the label do not end it and are kept with their backslashes.
    #[test]
    fn test_parse_link_label_with_escaped_brackets() {
        let mut input = r"[my \[esc\] label]";
        assert_eq!(
            parse_link_label.parse_next(&mut input),
            Ok(r"my \[esc\] label")
        );
    }

    /// An escaped backslash is kept as written and does not escape the closing bracket.
    #[test]
    fn test_parse_link_label_with_escaped_backslash() {
        let mut input = r"[my \\ label]";
        assert_eq!(parse_link_label.parse_next(&mut input), Ok(r"my \\ label"));
    }

    /// `[]` is rejected: a label needs at least one character.
    #[test]
    fn test_parse_link_label_fails_on_empty() {
        let mut input = "[]";
        parse_link_label
            .parse_next(&mut input)
            .expect_err("input should be rejected");
    }

    /// A label of only spaces is rejected, as `CommonMark` requires a non-blank label.
    #[test]
    fn test_parse_link_label_fails_on_whitespace_only() {
        let mut input = "[   ]";
        parse_link_label
            .parse_next(&mut input)
            .expect_err("input should be rejected");
    }

    /// An unescaped `[` inside the label is rejected, since labels cannot nest.
    #[test]
    fn test_parse_link_label_fails_on_unescaped_bracket() {
        let mut input = "[my [unescaped] label]";
        parse_link_label
            .parse_next(&mut input)
            .expect_err("input should be rejected");
    }

    /// A label of 1000 characters exceeds the 999-character `CommonMark` limit and is rejected.
    #[test]
    fn test_parse_link_label_fails_on_too_long() {
        let long_string = "a".repeat(1000);
        let input_string = format!("[{long_string}]");
        let mut input_ref = input_string.as_str();
        parse_link_label
            .parse_next(&mut input_ref)
            .expect_err("input should be rejected");
    }

    /// A label of exactly 999 characters is accepted.
    #[test]
    fn test_parse_link_label_exact_max_length() {
        let max_string = "a".repeat(999);
        let input_string = format!("[{max_string}]");
        let mut input_ref = input_string.as_str();
        assert_eq!(
            parse_link_label.parse_next(&mut input_ref),
            Ok(max_string.as_str())
        );
    }
}
