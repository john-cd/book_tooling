//! Benchmarks of AST traversal and Markdown reconstruction: `MarkdownRenderer` and the pretty
//! printer behind `Document`'s `Display`.
//! Run with `cargo bench -p parse_lib --bench reconstruction`.
//!
//! TODO Reviewed ??-??-2026.

#![expect(
    clippy::expect_used,
    reason = "Benchmark setup fails loudly when the fixtures cannot be built or parsed."
)]

use std::fmt::Write as _;
use std::hint::black_box;

use divan::Bencher;
use divan::counter::BytesCount;
use parse_lib::ast::DirectiveData;
use parse_lib::markdown_renderer::MarkdownRenderer;
use parse_lib::visit::Visitor as _;

/// The fixture corpus: every Markdown file under `test_fixtures/markdown`.
const FIXTURES: [&str; 4] = [
    include_str!("../../test_fixtures/markdown/crlf.md"),
    include_str!("../../test_fixtures/markdown/directive_table.md"),
    include_str!("../../test_fixtures/markdown/emphasis_and_text.md"),
    include_str!("../../test_fixtures/markdown/include_heavy.md"),
];

/// Document sizes, as repetitions of the fixture corpus.
const COPIES: [usize; 3] = [1, 50, 1_000];

/// Renders the parsed corpus with `MarkdownRenderer`, leaving every directive as written.
#[divan::bench(args = COPIES)]
fn markdown_renderer(bencher: Bencher<'_, '_>, copies: usize) {
    let input = FIXTURES.concat().repeat(copies);
    let document = parse_lib::parse_document("bench.md", None, &input).expect("fixtures parse");
    bencher.counter(BytesCount::of_str(&input)).bench(|| {
        let mut renderer =
            MarkdownRenderer::new(|directive: &DirectiveData<'_>| directive.to_string());
        renderer.visit_document(black_box(&document));
        renderer.markdown
    });
}

/// Pretty-prints the document through its `Display` implementation, the public entry point to
/// the `PrettyPrinter` visitor.
#[divan::bench(args = COPIES)]
fn display(bencher: Bencher<'_, '_>, copies: usize) {
    let input = FIXTURES.concat().repeat(copies);
    let document = parse_lib::parse_document("bench.md", None, &input).expect("fixtures parse");
    bencher.counter(BytesCount::of_str(&input)).bench(|| {
        let mut output = String::with_capacity(input.len());
        write!(output, "{}", black_box(&document)).expect("writing to a String cannot fail");
        output
    });
}

/// Starts the Divan runner for the reconstruction benchmarks.
fn main() {
    divan::main();
}
