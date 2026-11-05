//! Parses a link text, which is enclosed in square brackets `[]`.
//!
//! Simplified from spec: <https://spec.commonmark.org/0.31.2/#link-text>
//!
//! - A link text begins with a left bracket ([) and ends with the first right bracket (]) that is
//!   not backslash-escaped.
//! - Unescaped square bracket characters are allowed only if they are nested and balanced.
//!
//! TODO Reviewed ??-??-2026.

use winnow::ModalResult;
use winnow::error::ContextError;
use winnow::error::ErrMode;
use winnow::error::StrContext::Expected;
use winnow::error::StrContext::Label;
use winnow::error::StrContextValue::Description;

/// Returns the byte offset of the `]` that closes the bracket at the start of `text`.
///
/// Brackets nest, so the scan counts depth iteratively: hostile input such as thousands of
/// unmatched `[` must not exhaust the stack. A backslash escapes the following character, and a
/// line break anywhere inside the text makes it invalid.
fn closing_bracket(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0_usize;
    let mut index = 0_usize;
    while let Some(&byte) = bytes.get(index) {
        match byte {
            b'\\' => {
                if matches!(bytes.get(index + 1), None | Some(b'\n' | b'\r')) {
                    return None;
                }
                index += 1;
            }
            b'\n' | b'\r' => return None,
            b'[' => depth += 1,
            b']' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

/// Parses link text.
///
/// Example: "[my text]" -> "my text".
///
/// Nested balanced brackets are part of the text. The parser is iterative, so nesting depth is
/// limited only by the input size.
pub(crate) fn parse_link_text<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    let source: &'s str = input;
    let Some(close) = source
        .starts_with('[')
        .then(|| closing_bracket(source))
        .flatten()
    else {
        let mut error = ContextError::new();
        error.push(Label("link text"));
        error.push(Expected(Description("[link text]")));
        return Err(ErrMode::Backtrack(error));
    };
    let text = source.get(1..close).unwrap_or_default();
    *input = source.get(close + 1..).unwrap_or_default();
    Ok(text)
}

#[cfg(test)]
mod tests {

    use winnow::Parser as _;

    use super::*;

    /// Bracketed text yields its contents and consumes the closing bracket.
    #[test]
    fn test_parse_link_text_simple() {
        let mut input = "[my text]";
        assert_eq!(parse_link_text.parse_next(&mut input), Ok("my text"));
        assert_eq!(input, "");
    }

    /// Escaped brackets inside the text do not end it and are kept with their backslashes.
    #[test]
    fn test_parse_link_text_with_escaped_brackets() {
        let mut input = r"[my \[esc\] text]";
        assert_eq!(
            parse_link_text.parse_next(&mut input),
            Ok(r"my \[esc\] text")
        );
    }

    /// A backslash before an ordinary character is kept literally.
    #[test]
    fn test_parse_link_text_with_arbitrary_escapes() {
        let mut input = r"[my \ text \*]";
        assert_eq!(parse_link_text.parse_next(&mut input), Ok(r"my \ text \*"));
    }

    /// `[]` is valid link text and yields an empty string.
    #[test]
    fn test_parse_link_text_empty() {
        let mut input = "[]";
        assert_eq!(parse_link_text.parse_next(&mut input), Ok(""));
    }

    /// An unclosed nested `[` leaves the brackets unbalanced and the text is rejected.
    #[test]
    fn test_parse_link_text_fails_on_unbalanced_bracket() {
        let mut input = "[my [unbalanced text]";
        parse_link_text
            .parse_next(&mut input)
            .expect_err("input should be rejected");
    }

    /// Nested, balanced brackets are part of the text.
    #[test]
    fn test_parse_link_text_with_balanced_brackets() {
        let mut input = "[my [nested [and] balanced] text]";
        assert_eq!(
            parse_link_text.parse_next(&mut input),
            Ok("my [nested [and] balanced] text")
        );
    }

    /// Line breaks, a trailing backslash and missing opening brackets are rejected.
    #[test]
    fn test_parse_link_text_rejects_line_breaks_and_dangling_escapes() {
        for source in ["[a\nb]", "[a\rb]", "[a\\", "[a\\\n]", "no bracket", ""] {
            let mut input = source;
            parse_link_text
                .parse_next(&mut input)
                .expect_err("input should be rejected");
            assert_eq!(input, source, "rejected input must not be consumed");
        }
    }

    /// Text after the closing bracket is left for the caller, and multi-byte characters survive.
    #[test]
    fn test_parse_link_text_leaves_the_remainder_and_keeps_unicode() {
        let mut input = "[h\u{e9}llo [w\u{f6}rld]] rest";
        assert_eq!(
            parse_link_text.parse_next(&mut input),
            Ok("h\u{e9}llo [w\u{f6}rld]")
        );
        assert_eq!(input, " rest");
    }

    /// Tens of thousands of nested or unmatched brackets must not overflow the stack.
    #[test]
    fn test_parse_link_text_handles_extreme_nesting_without_recursion() {
        let depth = 100_000;
        let unmatched = "[".repeat(depth);
        let mut input = unmatched.as_str();
        parse_link_text
            .parse_next(&mut input)
            .expect_err("unbalanced");

        let balanced = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        let mut balanced_input = balanced.as_str();
        let text = parse_link_text
            .parse_next(&mut balanced_input)
            .expect("balanced");
        assert_eq!(text.len(), balanced.len() - 2);
        assert_eq!(balanced_input, "");
    }
}
