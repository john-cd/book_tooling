<!-- TODO Reviewed ??-??-2026 -->
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

## Module Map

### `book_tool` Modules

- `common`: shared file rewriting, directory walking, and template rendering helpers.
- `directives`: directive and wikilink expansion workflows plus `Config`.
- `extract_comments`: HTML comment aggregation workflow.
- `inline_links`: inline-link to reference-definition conversion workflow.
- `link_check`: external link verification and refdef cleanup workflow.
- `links`: lower-level URL extraction and async verification helpers.
- `refdefs`: unused reference-definition analysis and deletion helpers.
- `search`: plain-text search workflow over parsed Markdown.

### `parse_lib` Modules

- `ast`: `Document`, `Documents`, `Element`, directive enums, and related data payloads.
- `visit`: `Visitor`, `Visitable`, and helper visitors such as `PrettyPrinter` and `Debugger`.
- `directive_extractor`, `html_comment_extractor`, `link_extractor`, `plain_text_extractor`: reusable analysis visitors.
- `html_renderer`, `markdown_renderer`: AST-to-output renderers.
- `markdown`, `parse_directives`, `parse_hidden`, `parse_urls`, `parse_wikilinks`, `parse_document`: parser entry points.

### `rust_rate` Modules

- `crawler`: GitHub API client and response types.
- `patterns`: source-pattern analysis helpers.
- `stats`: Cargo manifest aggregation helpers.
- `crates_io`: crates.io summary workflow.
- `templates`: Markdown badge/link rendering helpers.
- `utils`: shared test helpers.

## Public APIs

### `book_tool` APIs

- Commands are exposed as module-level `execute(...)` functions for `directives`, `extract_comments`, `inline_links`, `link_check`, and `search`.
- Shared utilities include `common::process_text_file`, `common::walk::Scope`, `common::walk::walk_directory_and_process_files`, `links::extract_urls`, `links::AsyncLinkChecker`, `refdefs::find_unused_refdefs`, and `refdefs::delete_unused_refdefs`.

### `parse_lib` APIs

- Core entry point: `parse_lib::parse_document(...) -> anyhow::Result<Document<'_>>`.
- Core model: `Document`, `Documents`, `Element`, `DirectiveData`, `DestinationKind`, `IndexAnchorKind`, and the `*Data` payload structs in `ast`.
- Traversal surface: `visit::Visitor`, `visit::Visitable`, `visit::pretty_printer::PrettyPrinter`, `visit::debugger::Debugger`, and `visit::refdefs::RefDefManager`.
- Reusable visitors: `DirectiveExtractor`, `HtmlCommentExtractor`, `LinkExtractor`, `PlainTextExtractor`.
- Renderers: `HtmlRenderer`, `MarkdownRenderer`, and the resolver callback types used by `MarkdownRenderer`.
- Public parser helpers: functions re-exported from `markdown`, plus `parse_directive_element`, `parse_hidden_html_div`, `parse_naked_url`, `parse_autolink`, `parse_wikilink`, and `url_kind`.

### `rust_rate` APIs

- Crawling surface: `GithubCrawler`, `GithubSearchResponse`, `GithubItem`, and `GithubRepo`.
- Analysis surface: `PatternAnalyzer` and `CrateStats`.
- Workflow surface: `rust_rate::crates_io::execute(crate_name)`.
- Formatting/test helpers: `templates::{DestinationKind, render_badge, render_link}` and `utils::{TestCase, test_with}`.

## `book_tool` Usage

`book_tool` provides several subcommands for different tasks.

### Process Directives

Replaces custom directives with their expanded Markdown/HTML content.

```bash
book_tool directives ./src
```

The filesystem command reads an optional `config.toml` from the current working directory and uses
`Config::default()` when the file is absent. Its example shorthand keeps the legacy
`examples/<name>.md` include mapping. Render-time adapters use
`process_markdown_with_example_resolver` to resolve chapter-relative paths and anchors in their own
book layout.

### Check Links

Verifies external links and optionally cleans up unused reference definitions.

```bash
book_tool link-check ./src --refdefs --delete
```

Link checking requires outbound network access; there is no offline mode. The checker sends `HEAD`
requests first and falls back to `GET` when a response is not successful. Only `2xx` responses pass.
Each method retries `429` and `503` responses up to three times using a numeric `Retry-After` value,
or one second when the header is absent or not numeric. Requests time out after ten seconds, with up
to 32 links checked concurrently. Local HTTP fixtures can exercise the command without contacting
external sites.

### Audit Reference Definitions

Writes a JSON report with definition locations, include provenance, and reference-use locations.
Additional roots are scanned for reference uses without requiring their include graphs to resolve;
these uses are reported separately and protect matching definitions from deletion. Generated or
otherwise maintained definition files can be protected explicitly.

```bash
book_tool refdefs-audit ./bk/src --report ./refdefs-audit.json \
  --also-scan-uses-in ./bk/drafts \
  --also-scan-uses-in ./later/src \
  --preserve-definitions-in ./bk/src/refs/dependency-refs.md \
  --delete
```

### Convert Inline Links

Transforms `[text](url)` and `https://url.com` into reference-style links.

```bash
book_tool inline-links ./src
```

Untitled links and naked URLs become reference-style links. Existing reference definitions are reused,
and generated numeric labels avoid collisions. Links with Markdown titles and rust_howto issue links
remain inline so their original semantics and source text are preserved.

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
- `{{!docs name}}`: A linked docs.rs badge; the other link kinds also support `!` badges.
- `{{#crate name cat1 cat2}}`: A crate badge followed by badges for its categories.
- `{{#categories filesystem,api-bindings}}`: Badges for categories. Use `slug=Title` to keep a display title, including spaces in quotes, such as `{{#categories filesystem=Filesystem,api-bindings="API bindings"}}`.
- `{{#example name}}`: Inserts the legacy `examples/name.md` include when processed by the file-oriented command. In an mdBook adapter, the example resolver can instead map a chapter-relative include such as `{{#example ../../../crates/cats/filesystem/examples/read_write/read_file.rs:example}}`.

The render-time API, `directives::process_markdown`, applies the same configured transformations to a string without touching files. `process_markdown_with_example_resolver` lets an adapter supply its own example mapping and returns an error if that resolution fails.

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
