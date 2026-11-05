//! Benchmarks of `book_tool` corpus processing: directory traversal, link extraction and
//! directive replacement.
//!
//! The corpus is a temporary directory of chapters built from the checked-in fixtures. Run with
//! `cargo bench -p book_tool`.
//!
//! TODO Reviewed ??-??-2026.

#![expect(
    clippy::expect_used,
    reason = "Benchmark setup fails loudly when the fixtures cannot be built or parsed."
)]

use std::hint::black_box;
use std::path::Path;

use anyhow::Result;
use book_tool::common::Scope;
use book_tool::common::walk_directories_and_process_files;
use book_tool::directives::Config;
use book_tool::directives::process_markdown_with_example_resolver;
use book_tool::links::extract_urls;
use divan::Bencher;

/// The fixture corpus: every Markdown file under `test_fixtures/markdown`.
const FIXTURES: [&str; 4] = [
    include_str!("../../test_fixtures/markdown/crlf.md"),
    include_str!("../../test_fixtures/markdown/directive_table.md"),
    include_str!("../../test_fixtures/markdown/emphasis_and_text.md"),
    include_str!("../../test_fixtures/markdown/include_heavy.md"),
];

/// Chapter counts for the corpus sizes.
const CHAPTERS: [usize; 3] = [10, 100, 1_000];

/// Writes `chapters` Markdown files, spread over ten part directories, into a temporary directory.
fn build_corpus(chapters: usize) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("create corpus directory");
    for index in 0..chapters {
        let directory = root.path().join(format!("part-{}", index.rem_euclid(10)));
        std::fs::create_dir_all(&directory).expect("create part directory");
        std::fs::write(
            directory.join(format!("chapter-{index}.md")),
            FIXTURES.concat(),
        )
        .expect("write chapter");
    }
    root
}

/// Walks the corpus and visits every Markdown file.
#[divan::bench(args = CHAPTERS)]
fn traversal(bencher: Bencher<'_, '_>, chapters: usize) {
    let corpus = build_corpus(chapters);
    let scope = Scope::default();
    bencher.bench(|| {
        let mut visited = 0_usize;
        walk_directories_and_process_files([corpus.path()], &scope, |path: &Path| {
            black_box(path);
            visited += 1;
            Ok(())
        })
        .expect("walk corpus");
        visited
    });
}

/// Parses every chapter and extracts its external URLs.
#[divan::bench(args = CHAPTERS)]
fn link_extraction(bencher: Bencher<'_, '_>, chapters: usize) {
    let sources = vec![FIXTURES.concat(); chapters];
    bencher.bench(|| {
        let mut urls = 0_usize;
        for source in &sources {
            let document =
                parse_lib::parse_document("bench.md", None, source).expect("fixtures parse");
            urls += extract_urls(&document).len();
        }
        urls
    });
}

/// Replaces the directives of every chapter in memory.
#[divan::bench(args = CHAPTERS)]
fn directive_replacement(bencher: Bencher<'_, '_>, chapters: usize) {
    let sources = vec![FIXTURES.concat(); chapters];
    let config = Config::default();
    bencher.bench(|| -> Result<usize> {
        let mut bytes = 0_usize;
        for source in &sources {
            let rendered = process_markdown_with_example_resolver(source, &config, |name| {
                Ok(format!("{{{{#include examples/{name}.md}}}}"))
            })?;
            bytes += rendered.len();
        }
        Ok(bytes)
    });
}

/// Runs the benchmarks.
fn main() {
    divan::main();
}
