//! Diagnostics and degradation behavior of the parser for invalid directive syntax and corrupted
//! input documents.
//!
//! The document parser is deliberately lenient: text it cannot interpret becomes plain text
//! instead of an error. Hard errors surface only from the individual element parsers, and
//! `ParsingError` renders them with a source snippet.
//!
//! TODO Reviewed ??-??-2026.

use parse_lib::errors::ParsingError;
use parse_lib::parse_directives::parse_directive_element;
use parse_lib::parse_document;
use winnow::Parser as _;

/// Malformed directives are kept verbatim as text rather than rejected or dropped.
#[test]
fn malformed_directives_survive_parsing_as_text() {
    let inputs = [
        "{{#crate",
        "{{#crate }}",
        "{{#categories}}",
        "{{docs}}",
        "{{!unknown-kind name}}",
        "{{#example}}",
        "{{ docs serde",
        "}} stray close {{",
        "[[unterminated wikilink",
    ];
    for input in inputs {
        let document = parse_document("t.md", None, input)
            .unwrap_or_else(|error| panic!("{input:?} must not fail: {error:#}"));
        let rendered = document.to_string();
        let squashed = |text: &str| text.split_whitespace().collect::<String>();
        assert_eq!(
            squashed(&rendered),
            squashed(input),
            "{input:?} must keep all of its characters"
        );
    }
}

/// A directive parser failure renders a snippet that quotes the offending source line.
#[test]
fn directive_parser_errors_render_a_source_snippet() {
    let input = "{{#bogus-directive x}}";
    let error = parse_directive_element
        .parse(input)
        .expect_err("an unknown directive keyword is rejected");
    let rendered = ParsingError::from_parse(&error).to_string();

    assert!(rendered.contains(input), "snippet missing: {rendered}");
    assert!(rendered.contains("error"), "label missing: {rendered}");
    assert!(
        rendered.lines().count() >= 3,
        "expected a multi-line diagnostic: {rendered}"
    );
}

/// Inputs with trailing garbage after a valid directive report the failure location.
#[test]
fn trailing_input_after_a_directive_is_reported() {
    let input = "{{#crate serde}} trailing";
    let error = parse_directive_element
        .parse(input)
        .expect_err("the whole input must be one directive");
    let rendered = ParsingError::from_parse(&error).to_string();
    assert!(rendered.contains("trailing"), "{rendered}");
}

/// Hostile or corrupted documents parse and re-render without panicking.
#[test]
fn corrupted_documents_do_not_panic() {
    let long_line = "x".repeat(200_000);
    let deep_brackets = "[".repeat(5_000);
    let deep_nesting = format!("{}x{}", "[".repeat(2_000), "](u)".repeat(2_000));
    let unbalanced_backticks = "`a ``b ``` c ```` d".repeat(500);
    let cases = [
        "\u{0}\u{0}\u{0}[a](\u{0})".to_owned(),
        "\u{feff}# Heading with BOM\r\n".to_owned(),
        "mixed\rline\r\nendings\nhere\r".to_owned(),
        "```rust\nunterminated fence".to_owned(),
        "| a | b |\n|---|\n| only one |\n".to_owned(),
        "<div class=\"hidden\">never closed".to_owned(),
        "\u{202e}rtl override [x](\u{202e})".to_owned(),
        long_line,
        deep_brackets,
        deep_nesting,
        unbalanced_backticks,
    ];
    for input in &cases {
        let document = parse_document("corrupt.md", None, input).expect("lenient parse");
        let rendered = document.to_string();
        assert!(
            !rendered.is_empty() || input.is_empty(),
            "non-empty input rendered empty: {:?}",
            input.chars().take(40).collect::<String>()
        );
    }
}

/// Empty input is a valid, empty document.
#[test]
fn empty_input_parses_to_an_empty_document() {
    let document = parse_document("empty.md", None, "").expect("empty input is valid");
    assert!(document.elements.is_empty());
    assert_eq!(document.to_string(), "");
}
