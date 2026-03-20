# Architecture

The project is organized as a Rust workspace containing several crates, each with a specific responsibility.

## Workspace Crates

### Core Libraries

- **`core_lib`**: Provides common utilities for file I/O, directory traversal, and text processing.
- **`directive_lib`**: Defines the model for directives and provides logic for processing them.
- **`parse_lib`**: A comprehensive Markdown parsing library using the `winnow` parser combinator crate. It handles standard Markdown elements and custom directives.

### Tools / Binaries

- **`process_directives`**: A CLI tool that uses `directive_lib` to find and replace custom directives in Markdown files.
- **`inline_links`**: A CLI tool that converts inline links and naked URLs into reference-style links.
- **`link_checker`**: A CLI tool for verifying external links in Markdown files.
- **`rust_rate`**: (Work in progress) A tool related to Rust project metrics or rating.

## Data Flow

1. **Input**: A directory path is provided to one of the CLI tools.
2. **Traversal**: `core_lib` walks through the directory, filtering for Markdown files.
3. **Parsing**: The tool uses `parse_lib` (or regex in some cases) to identify elements of interest (directives, links, etc.) within each file.
4. **Transformation**: The tool applies the necessary transformations to the content.
5. **Output**: `core_lib` handles the atomic update of the files on disk if changes were made.
