//! Benchmarks of `parse_document` throughput on small, medium and large Markdown documents.
//!
//! The documents repeat the checked-in fixture corpus, so they exercise headings, links, tables,
//! directives, code fences and reference definitions. Run with `cargo bench -p parse_lib`.
//!
//! TODO Reviewed ??-??-2026

#![expect(
    clippy::expect_used,
    reason = "Benchmark setup fails loudly when the fixtures cannot be built or parsed."
)]

use std::hint::black_box;

use divan::Bencher;
use divan::counter::BytesCount;
use parse_lib::Element;

/// The fixture corpus: every Markdown file under `test_fixtures/markdown`.
const FIXTURES: [&str; 4] = [
    include_str!("../../test_fixtures/markdown/crlf.md"),
    include_str!("../../test_fixtures/markdown/directive_table.md"),
    include_str!("../../test_fixtures/markdown/emphasis_and_text.md"),
    include_str!("../../test_fixtures/markdown/include_heavy.md"),
];

/// Builds a document of `copies` repetitions of the fixture corpus.
fn corpus(copies: usize) -> String {
    FIXTURES.concat().repeat(copies)
}

/// Parses the corpus of `copies` repetitions and reports bytes per second.
fn bench_parse(bencher: Bencher<'_, '_>, copies: usize) {
    let input = corpus(copies);
    bencher
        .counter(BytesCount::of_str(&input))
        .bench(|| parse_lib::parse_document("bench.md", None, black_box(&input)));
}

/// A few kilobytes: one chapter.
#[divan::bench]
fn small(bencher: Bencher<'_, '_>) {
    bench_parse(bencher, 1);
}

/// Tens of kilobytes: a long chapter.
#[divan::bench]
fn medium(bencher: Bencher<'_, '_>) {
    bench_parse(bencher, 50);
}

/// Hundreds of kilobytes: a large generated page.
#[divan::bench]
fn large(bencher: Bencher<'_, '_>) {
    bench_parse(bencher, 1_000);
}

/// Checks the zero-copy claim: every text node borrows from the input, so parsing allocates only
/// the element vectors, never the text. The check runs once before the benchmarks start.
fn assert_text_is_borrowed() {
    let input = corpus(10);
    let document = parse_lib::parse_document("bench.md", None, &input).expect("fixtures parse");
    let start = input.as_ptr().addr();
    let end = start + input.len();
    let mut checked = 0_usize;
    for element in &document.elements {
        if let Element::Text(text) = element {
            let address = text.content.as_ptr().addr();
            assert!(
                (start..=end).contains(&address),
                "text node was copied out of the input"
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "the corpus must contain text nodes");
}

/// Verifies the zero-copy claim, then runs the benchmarks.
fn main() {
    assert_text_is_borrowed();
    divan::main();
}
