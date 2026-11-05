<!-- TODO Reviewed ??-??-2026 -->

# `rust_rate` Architecture

## Components

### 1. Crawler (`crawler.rs`)

Responsible for interacting with external APIs (GitHub Search) to find Rust repositories and fetch `Cargo.toml` and `.rs` files.

- **GitHub Client**: Manages authentication and rate limiting for the GitHub API.
- **File Downloader**: Efficiently downloads raw file content.

### 2. Stats Analyzer (`stats.rs`)

Parses `Cargo.toml` files and aggregates usage statistics.

- **Crate Extractor**: Uses `toml` crate to parse dependencies.
- **Aggregator**: Maintains counts per crate and category.

### 3. Pattern Matcher (`patterns.rs`)

Analyzes Rust source code to find recurring patterns.

- **Snippet Extractor**: Uses regex or basic parsing to find code blocks related to specific crates (e.g., imports and function calls).
- **Pattern Grouper**: Groups similar snippets to identify common usage.

### 4. Main Controller (`main.rs`)

Orchestrates the crawling and analysis process.

- **Task Manager**: Uses `tokio` to manage concurrent crawling and processing tasks.

## Data Flow

1. **Main** initiates search via **Crawler**.
2. **Crawler** returns list of repositories and URLs for `Cargo.toml`.
3. **Crawler** downloads `Cargo.toml` files.
4. **Stats Analyzer** processes `Cargo.toml` files and updates usage stats.
5. **Crawler** searches for Rust (`.rs`) files that import the crate (`use <crate>::`) and downloads them.
6. **Pattern Matcher** analyzes the downloaded `.rs` files, not the manifests, for usage patterns.
7. **Main** generates the final report.

`--limit` bounds how many manifests and how many source files are downloaded, and `--api-base`
points the crawler at another GitHub API root, such as a local test server.

## Libraries Used

- `reqwest`: HTTP client for API calls.
- `tokio`: Async runtime.
- `serde` / `toml`: For parsing JSON API responses and `Cargo.toml`.
- `tracing`: For logging and monitoring.
- `regex`: For pattern matching in source code.
