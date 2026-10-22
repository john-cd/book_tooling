//! Parsers for naked URLs and autolinks.
//!
//! TODO Reviewed ??-??-2026

use url::Url;
use winnow::ModalResult;
use winnow::Parser as _;
use winnow::combinator::alt;
use winnow::combinator::delimited;
use winnow::error::StrContext::{Expected, Label};
use winnow::error::StrContextValue::Description;
use winnow::token::take_while;

use crate::ast::AutolinkData;
use crate::ast::Element;

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
pub(crate) fn parse_url_chars<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    take_while(1.., is_valid_url_char)
        .verify(|s: &str| !s.is_empty())
        .context(Label("valid URL characters"))
        .context(Expected(Description(
            "alphanumeric, -, ., _, ~, :, /, ?, #, [, ], @, !, $, &, ', (, ), *, +, ,, ;, %, =",
        )))
        .parse_next(input)
}

/// Parses a naked URL (e.g. `https://example.com`) directly from text.
///
/// # Errors
///
/// Returns a backtrack error if the input does not begin with a naked URL.
pub fn parse_naked_url<'s>(input: &mut &'s str) -> ModalResult<Element<'s>> {
    let original = *input;
    let parsed_url = (alt(("https://", "http://")), parse_url_chars)
        .take()
        .context(Label("naked URL"))
        .parse_next(input)?;
    let url_text = trim_naked_url_punctuation(parsed_url);
    if !is_valid_http_url(url_text) {
        *input = original;
        return winnow::combinator::fail.parse_next(input);
    }
    if let Some(remaining) = original.get(url_text.len()..) {
        *input = remaining;
    }

    Ok(Element::Autolink(AutolinkData { url: url_text }))
}

/// Removes sentence punctuation and unmatched closing delimiters from a naked URL.
fn trim_naked_url_punctuation(url: &str) -> &str {
    let mut end = url.len();
    loop {
        let Some(prefix) = url.get(..end) else {
            return url;
        };
        let Some(last) = prefix.chars().next_back() else {
            return prefix;
        };
        let unmatched_closing_delimiter = match last {
            ')' => prefix.matches(')').count() > prefix.matches('(').count(),
            ']' => prefix.matches(']').count() > prefix.matches('[').count(),
            _ => false,
        };
        if matches!(last, '.' | ',' | ';' | ':' | '!') || unmatched_closing_delimiter {
            end -= last.len_utf8();
        } else {
            return prefix;
        }
    }
}

/// Validates an absolute HTTP URL and rejects malformed percent escapes.
fn is_valid_http_url(url: &str) -> bool {
    has_valid_percent_encoding(url)
        && Url::parse(url).is_ok_and(|parsed| matches!(parsed.scheme(), "http" | "https"))
}

/// Checks that every percent sign in a URL is followed by two hexadecimal digits.
fn has_valid_percent_encoding(url: &str) -> bool {
    let mut bytes = url.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            match (bytes.next(), bytes.next()) {
                (Some(high), Some(low)) if high.is_ascii_hexdigit() && low.is_ascii_hexdigit() => {}
                _ => return false,
            }
        }
    }
    true
}

/// Parses an autolink enclosed in angle brackets (e.g. `<https://example.com>`).
///
/// # Errors
///
/// Returns a backtrack error if the input does not begin with an autolink.
pub fn parse_autolink<'s>(input: &mut &'s str) -> ModalResult<Element<'s>> {
    let url_text = delimited(
        "<",
        (
            alt(("https://", "http://")),
            take_while(0.., |c: char| c != '>' && c != '<' && !c.is_whitespace()),
        )
            .take(),
        ">",
    )
    .verify(is_valid_http_url)
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
        let result = parse_autolink
            .parse_next(&mut input)
            .expect("input should parse");
        assert_eq!(
            result,
            Element::Autolink(AutolinkData {
                url: "https://example.com/foo"
            })
        );
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
        parse_url_chars
            .parse_peek("")
            .expect_err("input should be rejected");
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

    /// Verifies path, query, fragment, and percent-encoded bytes stay inside a valid URL.
    #[test]
    fn naked_url_preserves_path_query_fragment_and_percent_encoding() {
        let source = "https://docs.rs/crate/path?query=api%2Fbindings#fragment";
        let mut input = source;
        assert_eq!(
            parse_naked_url(&mut input).expect("parse fully-qualified URL"),
            Element::Autolink(AutolinkData { url: source })
        );
        assert!(input.is_empty());
    }

    /// Verifies trailing sentence punctuation and unmatched closers remain in the input.
    #[test]
    fn naked_url_leaves_adjacent_punctuation_and_keeps_balanced_parentheses() {
        let mut input = "https://example.com/path(a), next";
        assert_eq!(
            parse_naked_url(&mut input).expect("parse URL followed by a comma"),
            Element::Autolink(AutolinkData {
                url: "https://example.com/path(a)"
            })
        );
        assert_eq!(input, ", next");

        let mut punctuated_input = "https://example.com/path).";
        assert_eq!(
            parse_naked_url(&mut punctuated_input)
                .expect("parse URL followed by unmatched punctuation"),
            Element::Autolink(AutolinkData {
                url: "https://example.com/path"
            })
        );
        assert_eq!(punctuated_input, ").");
    }

    /// Verifies malformed URLs fail without consuming their source text.
    #[test]
    fn malformed_naked_and_angle_urls_are_rejected_without_consuming_input() {
        for source in ["https://example.com/%GG", "https://[invalid-host]/path"] {
            let mut input = source;
            parse_naked_url(&mut input).expect_err("input should be rejected");
            assert_eq!(input, source);
        }

        let mut autolink = "<https://example.com/%GG>";
        parse_autolink(&mut autolink).expect_err("input should be rejected");
        assert_eq!(autolink, "<https://example.com/%GG>");
    }

    /// Verifies the public document parser and no-op renderer preserve URLs and neighboring text.
    #[test]
    fn document_rendering_preserves_url_punctuation_and_malformed_source() {
        use crate::markdown_renderer::MarkdownRenderer;
        use crate::visit::Visitor as _;

        let source = concat!(
            "See https://example.com/path(a), and ",
            "https://example.com/path?query=api%2Fffi#fragment.\n",
            "Malformed https://example.com/%GG stays literal.\n"
        );
        let document = crate::parse_document("fixture.md", None, source)
            .expect("parse mixed valid and malformed URL text");
        let mut renderer = MarkdownRenderer::new(|directive| directive.to_string());
        renderer.visit_document(&document);

        assert_eq!(renderer.markdown, source);
    }
}
