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
