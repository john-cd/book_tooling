//! Regression tests for directive expansion inside full-page tables (john-cd/book_tooling#47).
//!
//! TODO Reviewed ??-??-2026

use book_tool::directives::Config;
use book_tool::directives::process_markdown;
use book_tool::directives::validate_no_unexpanded_directives;

/// A chapter-sized page: a preamble with intraword underscores, then a directive table.
const PAGE: &str = concat!(
    "# File System\n\n",
    "This chapter deals with files{{hi:Files}}.\n\n",
    "[`fs_extra`][c~fs_extra~docs]\u{2197}{{hi:fs_extra}} is useful for advanced operations. ",
    "[`tempfile`][c~tempfile~docs]\u{2197}{{hi:tempfile}} handles temporary files.\n\n",
    "| Recipe | Crates | Categories |\n",
    "|---|---|---|\n",
    "| [Copy a tree][ex~filesystem~copy] | {{#crate fs_extra}} | {{#categories filesystem=Filesystem}} |\n",
    "| [Make a temp file][ex~filesystem~temp] | {{#crate tempfile}} | {{#categories filesystem=Filesystem}} |\n",
    "| Names with `code_spans` | `std::fs` and `fs_extra` | {{#categories filesystem=Filesystem}} |\n",
);

/// Every table directive in a full page is expanded and the surrounding text is preserved.
#[test]
fn full_page_table_directives_are_expanded() {
    let rendered = process_markdown(PAGE, &Config::default()).expect("expand full page");

    validate_no_unexpanded_directives(&rendered).expect("no raw directives remain");
    assert!(rendered.contains("[`fs_extra`][c~fs_extra~docs]\u{2197}{{hi:fs_extra}} is useful"));
    assert!(rendered.contains("| Names with `code_spans` | `std::fs` and `fs_extra` |"));
    assert!(!rendered.contains("{{#crate"));
    assert!(!rendered.contains("{{#categories"));
}

/// Expanding an already expanded page changes nothing.
#[test]
fn full_page_expansion_is_idempotent() {
    let once = process_markdown(PAGE, &Config::default()).expect("first expansion");
    let twice = process_markdown(&once, &Config::default()).expect("second expansion");

    assert_eq!(once, twice);
}
