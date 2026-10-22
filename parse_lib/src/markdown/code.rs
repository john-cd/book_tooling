//! Parsers for code spans and fenced code blocks.
//!
//! TODO Reviewed ??-??-2026

use winnow::ModalResult;
use winnow::Parser as _;
use winnow::combinator::delimited;
use winnow::token::take_until;

use crate::ast::{CodeSpanData, Element, FencedCodeBlockData};

/// Parses text enclosed in single backticks.
/// Simplified from <https://spec.commonmark.org/0.31.2/#code-spans>.
///
/// # Errors
///
/// Returns a backtrack error if the input does not begin with a code span.
pub fn parse_code_span<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    delimited(
        '`',                  // Opening backtick.
        take_until(0.., '`'), // Content.
        '`',                  // Closing backtick.
    )
    .map(|content| Element::CodeSpan(CodeSpanData { content }))
    .parse_next(input)
}

/// Parses text enclosed in triple backticks.
/// Simplified from <https://spec.commonmark.org/0.31.2/#fenced-code-blocks>.
///
/// # Errors
///
/// Returns a backtrack error if the input does not begin with a fenced code block.
pub fn parse_fenced_code_block<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    let content = delimited(
        "```",                  // Opening triple backticks.
        take_until(0.., "```"), // Content.
        "```",                  // Closing triple backticks.
    )
    .parse_next(input)?;

    let mut lines_iter = content.split('\n');
    let first_line = lines_iter.next().unwrap_or("").trim_end_matches('\r');
    let lang_str = first_line.trim();
    let language = if lang_str.is_empty() {
        None
    } else {
        Some(lang_str)
    };

    let mut lines = Vec::new();
    for line in lines_iter {
        lines.push(line.trim_end_matches('\r'));
    }

    if lines.last().is_some_and(|last| last.is_empty()) {
        lines.pop();
    }

    Ok(Element::FencedCodeBlock(FencedCodeBlockData {
        language,
        lines,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Tests for parse_code_span ---

    #[test]
    fn test_parse_code_span_basic() {
        assert_eq!(
            parse_code_span.parse_peek("`hello world`"),
            Ok((
                "",
                Element::CodeSpan(CodeSpanData {
                    content: "hello world"
                })
            ))
        );
    }

    #[test]
    fn test_parse_code_span_empty_content() {
        assert_eq!(
            parse_code_span.parse_peek("``"),
            Ok(("", Element::CodeSpan(CodeSpanData { content: "" })))
        );
    }

    #[test]
    fn test_parse_code_span_with_trailing_text() {
        assert_eq!(
            parse_code_span.parse_peek("`code here` more text"),
            Ok((
                " more text",
                Element::CodeSpan(CodeSpanData {
                    content: "code here"
                })
            ))
        );
    }

    #[test]
    fn test_parse_code_span_no_closing_backtick() {
        parse_code_span(&mut "`unclosed span").expect_err("input should be rejected");
    }

    #[test]
    fn test_parse_code_span_no_opening_backtick() {
        parse_code_span(&mut "no opening backtick`").expect_err("input should be rejected");
    }

    #[test]
    fn test_parse_code_span_only_backticks() {
        parse_code_span(&mut "`").expect_err("input should be rejected");
    }

    #[test]
    fn test_parse_code_span_not_a_code_span() {
        parse_code_span(&mut "just plain text").expect_err("input should be rejected");
    }

    // --- Tests for parse_fenced_code_block ---

    #[test]
    fn test_parse_fenced_code_block_basic() {
        let input = "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
        let expected = Element::FencedCodeBlock(FencedCodeBlockData {
            language: Some("rust"),
            lines: vec!["fn main() {", "    println!(\"Hello\");", "}"],
        });
        assert_eq!(
            parse_fenced_code_block.parse_peek(input),
            Ok(("", expected))
        );
    }

    #[test]
    fn test_parse_fenced_code_block_empty_content() {
        assert_eq!(
            parse_fenced_code_block.parse_peek("``````"),
            Ok((
                "",
                Element::FencedCodeBlock(FencedCodeBlockData {
                    language: None,
                    lines: vec![]
                })
            ))
        );
    }

    #[test]
    fn test_parse_fenced_code_block_with_trailing_text() {
        let input = "```rust,editable\nprintln!(\"hi\");\n``` more text";
        let expected = Element::FencedCodeBlock(FencedCodeBlockData {
            language: Some("rust,editable"),
            lines: vec!["println!(\"hi\");"],
        });
        assert_eq!(
            parse_fenced_code_block.parse_peek(input),
            Ok((" more text", expected))
        );
    }

    #[test]
    fn test_parse_fenced_code_block_no_closing_delimiter() {
        parse_fenced_code_block(&mut "```unclosed block").expect_err("input should be rejected");
    }

    #[test]
    fn test_parse_fenced_code_block_no_opening_delimiter() {
        parse_fenced_code_block(&mut "unopened block```").expect_err("input should be rejected");
    }

    #[test]
    fn test_parse_fenced_code_block_inner_triple_backticks() {
        // `take_until` stops at the *first* closing delimiter.
        assert_eq!(
            parse_fenced_code_block.parse_peek("```\nfoo \n``` bar```"),
            Ok((
                " bar```",
                Element::FencedCodeBlock(FencedCodeBlockData {
                    language: None,
                    lines: vec!["foo "]
                })
            ))
        );
    }

    #[test]
    fn test_parse_fenced_code_block_only_delimiters() {
        parse_fenced_code_block(&mut "```").expect_err("input should be rejected");
    }

    #[test]
    fn test_parse_fenced_code_block_not_a_code_block() {
        parse_fenced_code_block(&mut "plain text").expect_err("input should be rejected");
    }

    #[test]
    fn test_parse_fenced_code_block_contains_single_backticks() {
        let input = "```\ncode with `single` backticks\n```";
        let expected = Element::FencedCodeBlock(FencedCodeBlockData {
            language: None,
            lines: vec!["code with `single` backticks"],
        });
        assert_eq!(
            parse_fenced_code_block.parse_peek(input),
            Ok(("", expected))
        );
    }
}
