<!-- TODO Reviewed ??-??-2026 -->
# parse_lib

A high-performance, non-destructive Markdown and directive parsing library built with `winnow`.

`parse_lib` is the core parsing engine for the Book Tooling workspace. It is designed to parse Markdown files into a rich Abstract Syntax Tree (AST), allowing for complex transformations while maintaining faithful reconstruction of the original document.

## Key Features

- **Robust AST**: A comprehensive Abstract Syntax Tree representing standard Markdown and custom extensions.
- **Custom Directives**: Built-in support for parsing `{{ ... }}` style directives used for book automation.
- **Wikilinks**: Full support for `[[target | display]]` style links.
- **Visitor Pattern**: A powerful `Visitor` trait for traversing and modifying the AST.
- **Non-Destructive**: Designed to support "pretty printing" back to Markdown, preserving as much of the original formatting as possible.
- **Zero-Copy**: Leverages Rust's lifetime system to avoid unnecessary string allocations during parsing.

## Usage

```rust
use parse_lib::parse_document;
use parse_lib::visit::Visitor;

let input = "# Hello World\n\nThis is a [[wikilink]].";
let doc = parse_document("example.md", None, input).unwrap();

// Access elements directly or use a Visitor
for element in &doc.elements {
    println!("{:?}", element);
}
```

## Supported Elements

- Headings (ATX style)
- Paragraphs
- Blockquotes
- Lists (Ordered and Unordered)
- Fenced Code Blocks
- Inline Code Spans
- Links (Inline, Reference-style, Autolinks)
- Images (Inline, Reference-style)
- Wikilinks
- HTML Comments
- Custom Directives (`{{#crate ...}}`, `{{cat ...}}`, etc.)
- Task Lists
- Tables (GFM style)
- Horizontal Rules

## Public Modules

- `ast`: core document and element types.
- `visit`: traversal traits plus helper visitors.
- `directive_extractor`, `html_comment_extractor`, `link_extractor`, `plain_text_extractor`: reusable analysis visitors.
- `html_renderer`, `markdown_renderer`: output renderers.
- `markdown`, `parse_directives`, `parse_hidden`, `parse_urls`, `parse_wikilinks`, `parse_document`: parser entry points.

## Public API Highlights

- `parse_document(name, path, input)`: parse a full Markdown document into `Document`.
- `Document`, `Documents`, `Element`, and the `*Data` structs: the AST surface used throughout the workspace.
- `DirectiveData`, `DestinationKind`, `IndexAnchorKind`: helper enums for directives and anchors.
- `Visitor` and `Visitable`: traversal traits for implementing custom analysis or rendering.
- `DirectiveExtractor`, `HtmlCommentExtractor`, `LinkExtractor`, `PlainTextExtractor`: ready-made analysis visitors.
- `HtmlRenderer`, `MarkdownRenderer`: render AST back into HTML or Markdown.
- `markdown::*`, `parse_directive_element`, `parse_hidden_html_div`, `parse_naked_url`, `parse_autolink`, `parse_wikilink`, and `url_kind`: parser helpers for individual syntax families.

## Module Notes

- The public `markdown` module is the supported surface for standalone Markdown element parsers.
- Lower-level parsing helpers remain internal unless they are intentionally re-exported.
- Most AST nodes borrow from the original input, so the source string must outlive the parsed `Document`.

## Invariants

- `parse_document` is total: Markdown has no syntax errors, so unrecognised or unbalanced syntax stays literal text.
- Rendering a parsed document with `MarkdownRenderer` and an identity directive resolver reproduces the input byte for byte, including CRLF line endings.
- Both invariants are enforced by `tests/round_trip.rs` with property tests (`proptest`) and the fixtures in `test_fixtures/markdown/`.

## Fuzzing

`fuzz/` holds a [`cargo fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html) target that checks the same two invariants on arbitrary UTF-8 input. It needs a nightly toolchain, is not part of the workspace and is not run in CI:

```bash
cargo install cargo-fuzz
cd parse_lib
cargo +nightly fuzz run round_trip
```

Seed it with `test_fixtures/markdown/` (`cargo +nightly fuzz run round_trip ../test_fixtures/markdown`). Add any crash it finds as a fixture or a regression test in `tests/round_trip.rs`.
