# Book Tooling

A collection of Rust-based tools for processing Markdown files, particularly for books and documentation projects.

## Overview

This workspace contains various crates designed to automate the management of Markdown content, including directive replacement, link conversion, and external link validation.

## Workspace Structure

### Libraries

- **[`parse_lib`](./parse_lib)**: A robust Markdown and directive parser.

### Tools

- **`book_tool`**: A unified CLI encompassing all the Markdown processing and utility tools into a single, cohesive binary.

## Getting Started

### Prerequisites

- Rust (latest stable version)

### Installation

Clone the repository and build the workspace:

```bash
cargo build --release --bin book_tool
```

## Documentation

- [Requirements](./documentation/requirements.md)
- [Architecture](./documentation/architecture.md)
- [TODO](./TODO.md)

## License

This project is licensed under either the [Apache License, Version 2.0](LICENSE-APACHE) or the [MIT license](LICENSE-MIT) at your option.
