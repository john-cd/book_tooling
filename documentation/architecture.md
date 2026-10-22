<!-- TODO Reviewed ??-??-2026 -->
# Architecture

The project is organized as a Rust workspace optimized for high-performance Markdown processing and transformation.

## Workspace Crates

### Libraries

- **`parse_lib`**: The central parsing engine.
  - Built with `winnow` for maximum performance and zero-copy parsing.
  - Provides a rich AST (Abstract Syntax Tree) representing both standard GFM Markdown and custom extensions (directives, wikilinks).
  - Implements the Visitor pattern for non-destructive document transformation and analysis.
  - Includes a `PrettyPrinter` for faithful reconstruction of Markdown from the AST.

### Tools

- **`book_tool`**: A unified CLI that consolidates several previously separate tools.
  - **Modules**:
    - `directives`: Handles the expansion of custom template-like directives into standard Markdown.
    - `links`: Provides high-level link checking (via `AsyncLinkChecker`) and transformation logic.
    - `refdefs`: Manages Markdown reference-style link definitions, including detection and deletion of unused labels.
    - `common`: Centralized utilities for atomic file updates, directory traversal, and shared templates.
- **`rust_rate`**: A specialized tool for crawling GitHub and analyzing Rust crate ecosystems. It operates independently of the document processing pipeline.

## Module Layout

### `book_tool`

- `common`
  - `replace_text_file_atomically`: atomic in-place text rewriting helper.
  - `walk`: directory traversal with extension and filename filtering.
  - `templates`: Markdown badge and link formatting helpers.
- `directives`
  - `execute`: file-oriented directive expansion workflow.
  - `process_all_directives`: configurable batch processor.
  - `process_markdown` and `process_markdown_with_example_resolver`: in-memory transformations for render-time adapters.
  - `conf::Config`: TOML-backed directive configuration.
- `inline_links`
  - `execute`: converts inline links and autolinks into reference-style links.
- `link_check`
  - `execute`: verifies HTTP/HTTPS links and optionally deletes unused refdefs.
- `links`
  - `AsyncLinkChecker`: retry-aware concurrent URL checker.
  - `extract_urls`: AST-based URL extraction helper.
- `refdefs`
  - `find_unused_refdefs`: analysis helper.
  - `delete_unused_refdefs`: cleanup helper.
- `search`
  - `execute`: case-insensitive search over extracted plain text.
- `extract_comments`
  - `execute`: aggregates HTML comments into an output file.

### `parse_lib`

- `ast`
  - `Document`, `Documents`, `Element`, and all `*Data` payload structs.
  - `DirectiveData`, `DestinationKind`, `IndexAnchorKind`.
  - `reconstruct_markdown` for AST-to-Markdown reconstruction.
- `visit`
  - `Visitor` and `Visitable` traversal contracts.
  - `PrettyPrinter`, `Debugger`, and `RefDefManager` helpers.
- Extractors
  - `DirectiveExtractor`, `HtmlCommentExtractor`, `LinkExtractor`, `PlainTextExtractor`.
- Renderers
  - `HtmlRenderer` and `MarkdownRenderer` plus resolver callback types.
- Parsers
  - `parse_document`: document-level entry point.
  - `markdown`: public parsers for common Markdown elements.
  - `parse_directives`, `parse_hidden`, `parse_urls`, `parse_wikilinks`: syntax-family entry points.

### `rust_rate`

- `crawler`
  - `GithubCrawler`, `GithubSearchResponse`, `GithubItem`, `GithubRepo`.
- `patterns`
  - `PatternAnalyzer`.
- `stats`
  - `CrateStats`.
- `crates_io`
  - `fetch_summary`, `validate_crate_name`, `execute`: crates.io summary workflow.
- `templates`
  - `DestinationKind`, `render_badge`, `render_link`.
- `utils`
  - `TestCase`, `test_with`.

## Data Flow (Document Processing)

1. **Discovery**: `book_tool` uses `common::walk` to recursively find Markdown files within target directories.
2. **Parsing**: `parse_lib::parse_document` transforms raw file content into an AST.
3. **Transformation/Analysis**: One or more `Visitor` implementations traverse the AST.
   - For conversion tasks (like `inline-links`), the visitor modifies the tree or builds a new representation.
   - For analysis tasks (like `link-check`), the visitor extracts data for external validation.
4. **Reconstruction**: If changes are made, a `PrettyPrinter` or specialized renderer converts the AST back into a UTF-8 string.
5. **Persistence**: `common::replace_text_file_atomically` performs an atomic write back to disk, ensuring file integrity.

## API Boundaries

- `book_tool` is the command/workflow layer and owns file-system side effects.
- `parse_lib` is the reusable parsing and traversal layer and owns the public AST.
- `rust_rate` is independent from the Markdown pipeline and provides repository-analysis helpers.
- `parse_lib::markdown` is the supported standalone parser surface; lower-level parsing helpers are intentionally kept internal unless re-exported.
