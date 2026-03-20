//! Parses a link label, which is enclosed in square brackets `[]`.
//!
//! Simplified from spec: <https://spec.commonmark.org/0.31.2/#link-label>
//!
//! - A link label begins with a left bracket ([) and ends with the first right bracket (]) that is not backslash-escaped.
//! - Between these brackets there must be at least one character that is not a space, tab, or line ending.
//! - Unescaped square bracket characters are not allowed inside the opening and closing square brackets of link labels.
//! - A link label can have at most 999 characters inside the square brackets.

use winnow::ModalResult;
use winnow::Parser;
use winnow::combinator::alt;
use winnow::combinator::delimited;
use winnow::combinator::repeat;
use winnow::error::StrContext::*;
use winnow::error::StrContextValue::*;
use winnow::token::one_of;
use winnow::token::take_while;

/// Parses link label.
///
/// Example: "[my label]" -> "my label".
/// Handles escaped brackets: "\[" and "\]".
pub(crate) fn parse_link_label<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    let parse_escaped = ('\\', one_of(['[', ']', '\\'])).take();
    let parse_normal = take_while(1.., |c: char| {
        c != '[' && c != ']' && c != '\\' && c != '\n' && c != '\r'
    });

    let mut label_content = repeat::<_, _, (), _, _>(1.., alt((parse_escaped, parse_normal)))
        .take()
        .verify(|s: &str| {
            let len = s.chars().count();
            (1..=999).contains(&len) && !s.trim().is_empty()
        });

    delimited("[", label_content, "]")
        .context(Label("link label"))
        .context(Expected(Description("[my label]. No newline allowed.")))
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_label() {
        assert_eq!(
            parse_link_label.parse_peek("[my label]"),
            Ok(("", "my label"))
        );
    }

    #[test]
    fn test_label_with_trailing_text() {
        assert_eq!(
            parse_link_label.parse_peek("[label] and more"),
            Ok((" and more", "label"))
        );
    }

    #[test]
    fn test_label_with_internal_whitespace() {
        assert_eq!(
            parse_link_label.parse_peek("[  my label  ]"),
            Ok(("", "  my label  "))
        );
    }

    #[test]
    fn test_label_max_length() {
        let max_label = "a".repeat(999);
        let input = format!("[{max_label}]");
        assert_eq!(
            parse_link_label.parse_peek(&input),
            Ok(("", max_label.as_str()))
        );
    }

    #[test]
    fn test_fail_on_empty_label() {
        assert!(parse_link_label(&mut "[]").is_err());
    }

    #[test]
    fn test_fail_on_whitespace_only_label() {
        assert!(parse_link_label(&mut "[ ]").is_err());
        assert!(parse_link_label(&mut "[\t\t]").is_err());
    }

    #[test]
    fn test_fail_on_label_too_long() {
        let long_label = "a".repeat(1000);
        let s = format!("[{long_label}]");
        let mut input = s.as_str();
        assert!(parse_link_label(&mut input).is_err());
    }

    #[test]
    fn test_fail_on_unclosed_bracket() {
        assert!(parse_link_label(&mut "[unclosed").is_err());
    }

    #[test]
    fn test_fail_on_nested_brackets() {
        assert!(parse_link_label(&mut "[a[b]c]").is_err());
    }

    #[test]
    fn test_fail_on_newline() {
        assert!(parse_link_label(&mut "[label\nwith newline]").is_err());
    }

    #[test]
    fn test_escaped_brackets() {
        assert_eq!(
            parse_link_label.parse_peek(r"[a\[b\]c]"),
            Ok(("", r"a\[b\]c"))
        );
    }

    #[test]
    fn test_escaped_backslash() {
        assert_eq!(parse_link_label.parse_peek(r"[a\\b]"), Ok(("", r"a\\b")));
    }
}
