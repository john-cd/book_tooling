# `rust_rate` Requirements

## Overview

`rust_rate` is a tool designed to analyze the usage of Rust crates in the wild. It crawls public repositories (primarily GitHub) to identify which crates are being used, how they are used (common code patterns), and provides statistics on their popularity and categories.

## Functional Requirements

1. **Crate Crawling**:
    - Search GitHub for `Cargo.toml` files.
    - Parse `Cargo.toml` to extract dependencies and their versions.
2. **Usage Statistics**:
    - Compute usage statistics per crate (e.g., in how many projects it is found).
    - Aggregate statistics per category (as defined in `crates.io` or `lib.rs`).
3. **Pattern Identification**:
    - Inspect Rust source code (`.rs` files) in identified repositories.
    - Identify commonly found code snippets or patterns for specific crates.
4. **Reporting**:
    - Generate reports in Markdown or JSON format summarizing the findings.
5. **Integration**:
    - Use `reqwest` for HTTP requests.
    - Use `tokio` for asynchronous processing.
    - Integrate with GitHub Search API.

## Non-Functional Requirements

1. **Performance**: Efficiently handle a large number of repositories using asynchronous tasks.
2. **Reliability**: Handle API rate limits (especially GitHub) gracefully.
3. **Extensibility**: Allow adding new search engines or analysis patterns easily.
4. **Documentation**: All code must be well-documented with a clear README.
