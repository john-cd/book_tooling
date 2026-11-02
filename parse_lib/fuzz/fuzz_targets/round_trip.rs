// TODO Reviewed ??-??-2026

//! Fuzz target: `parse_document` accepts any text and rendering it back is the identity.

#![no_main]

use libfuzzer_sys::fuzz_target;
use parse_lib::markdown_renderer::MarkdownRenderer;
use parse_lib::parse_document;
use parse_lib::visit::Visitor as _;

fuzz_target!(|markdown: &str| {
    let document = parse_document("fuzz.md", None, markdown).expect("parse_document is total");
    let mut renderer = MarkdownRenderer::new(|directive| directive.to_string());
    renderer.visit_document(&document);
    assert_eq!(renderer.markdown, markdown, "round trip must be lossless");
});
