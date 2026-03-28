# Book Tooling

A collection of Rust-based tools for processing Markdown files, particularly for books and documentation projects.

## Overview

This workspace provides a suite of tools to automate the management and enrichment of Markdown content. It includes a high-performance parser, a unified CLI for document processing, and a specialized tool for analyzing Rust crate metrics.

## Workspace Structure

### Libraries

- **[`parse_lib`](./parse_lib)**: A non-destructive Markdown and directive parser built with `winnow`. It provides the core AST and visitor infrastructure.

### Tools

- **`book_tool`**: The primary unified CLI tool. It centralizes several document processing tasks:
  - **Directives**: Expands custom `{{ ... }}` directives like crate badges and include blocks.
  - **LinkCheck**: Validates external HTTP/HTTPS links and manages reference definitions.
  - **InlineLinks**: Automatically converts naked URLs and inline links into reference-style links.
  - **Search**: Performs content-aware text search across documents, ignoring Markdown formatting.
  - **ExtractComments**: Aggregates HTML comments for auditing and review.
- **`rust_rate`**: A standalone tool for crawling GitHub and analyzing Rust crate usage statistics and patterns.

## `book_tool` Usage

`book_tool` provides several subcommands for different tasks.

### Process Directives
Replaces custom directives with their expanded Markdown/HTML content.
```bash
book_tool directives ./src
```

### Check Links
Verifies external links and optionally cleans up unused reference definitions.
```bash
book_tool link-check ./src --refdefs --delete
```

### Convert Inline Links
Transforms `[text](url)` and `https://url.com` into reference-style links.
```bash
book_tool inline-links ./src
```

### Search
Searches for text in the document's plain-text content.
```bash
book_tool search "some query" ./src
```

### Extract Comments
Extracts all `<!-- comment -->` blocks into a single output file.
```bash
book_tool extract-comments ./src --output comments.md
```

## Supported Directives

- `{{cat name}}`: Link to a crates.io category.
- `{{crate name}}`: Link to an internal crate page.
- `{{docs name}}`: Link to docs.rs documentation.
- `{{github repo}}`: Link to a GitHub repository.
- `{{#crate name cat1 cat2}}`: A "crate block" displaying multiple badges.
- `{{#example name}}`: Includes a code example block.

## Getting Started

### Installation
Clone the repository and build the workspace:
```bash
cargo build --release
```
The binaries will be available in `target/release/book_tool` and `target/release/rust_rate`.

## Documentation

- [Requirements](./documentation/requirements.md)
- [Architecture](./documentation/architecture.md)
- [TODO](./TODO.md)

## License

This project is licensed under either the [Apache License, Version 2.0](LICENSE-APACHE) or the [MIT license](LICENSE-MIT) at your option.
