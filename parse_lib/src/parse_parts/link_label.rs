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
use winnow::token::none_of;
use winnow::token::take_while;

/// Parses link label.
///
/// Example: "[my label]" -> "my label".
pub fn parse_link_label<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
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
    .verify(|s: &str| !s.trim().is_empty() && s.len() <= 999);

    let mut parser = delimited("[", label_content, "]")
        .context(Label("link label"))
        .context(Expected(Description(
            "[my label]. Max 999 chars. No newline allowed.",
        )));

    parser.parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use winnow::Parser;

    #[test]
    fn test_parse_link_label_simple() {
        let mut input = "[my label]";
        assert_eq!(parse_link_label.parse_next(&mut input), Ok("my label"));
        assert_eq!(input, "");
    }

    #[test]
    fn test_parse_link_label_with_arbitrary_escapes() {
        let mut input = r"[my \ label \*]";
        assert_eq!(parse_link_label.parse_next(&mut input), Ok(r"my \ label \*"));
    }

    #[test]
    fn test_parse_link_label_with_escaped_brackets() {
        let mut input = r"[my \[esc\] label]";
        assert_eq!(parse_link_label.parse_next(&mut input), Ok(r"my \[esc\] label"));
    }

    #[test]
    fn test_parse_link_label_with_escaped_backslash() {
        let mut input = r"[my \\ label]";
        assert_eq!(parse_link_label.parse_next(&mut input), Ok(r"my \\ label"));
    }

    #[test]
    fn test_parse_link_label_fails_on_empty() {
        let mut input = "[]";
        assert!(parse_link_label.parse_next(&mut input).is_err());
    }

    #[test]
    fn test_parse_link_label_fails_on_whitespace_only() {
        let mut input = "[   ]";
        assert!(parse_link_label.parse_next(&mut input).is_err());
    }

    #[test]
    fn test_parse_link_label_fails_on_unescaped_bracket() {
        let mut input = "[my [unescaped] label]";
        assert!(parse_link_label.parse_next(&mut input).is_err());
    }

    #[test]
    fn test_parse_link_label_fails_on_too_long() {
        let long_string = "a".repeat(1000);
        let input_string = format!("[{}]", long_string);
        let mut input_ref = input_string.as_str();
        assert!(parse_link_label.parse_next(&mut input_ref).is_err());
    }

    #[test]
    fn test_parse_link_label_exact_max_length() {
        let max_string = "a".repeat(999);
        let input_string = format!("[{}]", max_string);
        let mut input_ref = input_string.as_str();
        assert_eq!(parse_link_label.parse_next(&mut input_ref), Ok(max_string.as_str()));
    }
}
