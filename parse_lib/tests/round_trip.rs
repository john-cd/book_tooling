//! Property and corpus tests for the parser's totality and lossless round-trip invariants.
//!
//! Commands rewrite authored files in place, so `parse_document` must accept every input and
//! rendering the result without directive expansion must reproduce the input byte for byte.

use std::fs;
use std::path::Path;

use parse_lib::markdown_renderer::MarkdownRenderer;
use parse_lib::parse_document;
use parse_lib::visit::Visitor as _;
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::select;

/// Markdown fragments that exercise delimiters, directives, links, code and line endings.
const TOKENS: [&str; 40] = [
    "a",
    "b",
    " ",
    "\n",
    "\n\n",
    "\r\n",
    "\t",
    "|",
    "-",
    "*",
    "_",
    "`",
    "[",
    "]",
    "(",
    ")",
    "<",
    ">",
    "{{",
    "}}",
    "{{#crate x}}",
    "{{hi:x}}",
    "#",
    "> ",
    "~~",
    "!",
    "\\",
    "http://x.y",
    "<!--",
    "-->",
    "1. ",
    "- ",
    "```",
    "[x]: u",
    "[[w]]",
    "\u{e9}",
    "    ",
    "{",
    "}",
    "snake_case",
];

/// Parses `markdown` and renders it back with directives left as written.
fn round_trip(markdown: &str) -> Result<String, String> {
    let document = parse_document("test.md", None, markdown).map_err(|error| error.to_string())?;
    let mut renderer = MarkdownRenderer::new(|directive| directive.to_string());
    renderer.visit_document(&document);
    Ok(renderer.markdown)
}

proptest! {
    /// Any sequence of Markdown-ish fragments parses and renders back unchanged.
    #[test]
    fn markdown_like_input_round_trips(tokens in vec(select(&TOKENS[..]), 0..24)) {
        let markdown = tokens.concat();
        prop_assert_eq!(round_trip(&markdown), Ok(markdown));
    }

    /// Arbitrary Unicode text, including control characters, never fails to parse.
    #[test]
    fn arbitrary_text_round_trips(markdown in any::<String>()) {
        prop_assert_eq!(round_trip(&markdown), Ok(markdown));
    }
}

/// Every checked-in fixture parses and renders back unchanged.
#[test]
fn fixture_corpus_round_trips() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_fixtures/markdown");
    let mut checked = 0_usize;
    for entry in fs::read_dir(&directory).expect("read fixture directory") {
        let path = entry.expect("read fixture entry").path();
        let markdown = fs::read_to_string(&path).expect("read fixture");
        assert_eq!(
            round_trip(&markdown),
            Ok(markdown),
            "{} must round-trip",
            path.display()
        );
        checked += 1;
    }
    assert!(
        checked >= 4,
        "expected the fixture corpus, found {checked} files"
    );
}
