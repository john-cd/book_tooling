# Book Tooling

A collection of Rust-based tools for processing Markdown files, particularly for books and documentation projects.

## Overview

This workspace contains various crates designed to automate the management of Markdown content, including directive replacement, link conversion, and external link validation.

## Workspace Structure

### Libraries

- **[`core_lib`](./core_lib)**: Shared utilities for file processing and directory walking.
- **[`directive_lib`](./directive_lib)**: Logic for handling custom directives.
- **[`parse_lib`](./parse_lib)**: A robust Markdown and directive parser.

### Tools

- **[`process_directives`](./process_directives)**: CLI for expanding custom directives.
- **[`inline_links`](./inline_links)**: CLI for converting inline links to reference-style links.
- **[`link_checker`](./link_checker)**: CLI for verifying external links and managing reference definitions.
- **[`rust_rate`](./rust_rate)**: Tool for crawling GitHub and analyzing Rust crate usage patterns.

## Getting Started

### Prerequisites

- Rust (latest stable version)

### Installation

Clone the repository and build the workspace:

```bash
cargo build --release
```

## Documentation

- [Requirements](./requirements.md)
- [Architecture](./architecture.md)
- [TODO](./TODO.md)

## License

This project is licensed under either the [Apache License, Version 2.0](LICENSE-APACHE) or the [MIT license](LICENSE-MIT) at your option.
