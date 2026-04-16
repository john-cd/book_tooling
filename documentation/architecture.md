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

## Data Flow (Document Processing)

1. **Discovery**: `book_tool` uses `common::walk` to recursively find Markdown files within target directories.
2. **Parsing**: `parse_lib::parse_document` transforms raw file content into an AST.
3. **Transformation/Analysis**: One or more `Visitor` implementations traverse the AST.
   - For conversion tasks (like `inline-links`), the visitor modifies the tree or builds a new representation.
   - For analysis tasks (like `link-check`), the visitor extracts data for external validation.
4. **Reconstruction**: If changes are made, a `PrettyPrinter` or specialized renderer converts the AST back into a UTF-8 string.
5. **Persistence**: `common::process_text_file` performs an atomic write back to disk, ensuring file integrity.
