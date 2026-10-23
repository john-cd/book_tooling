<!-- TODO Reviewed ??-??-2026 -->

# `rust_rate`

`rust_rate` is a tool for crawling and analyzing Rust crate usage patterns across public repositories.

## Features

- Crawls GitHub for `Cargo.toml` files to track crate popularity.
- Analyzes Rust source code to identify common usage patterns and code snippets.
- Provides statistics per crate and per category.
- Highly concurrent using `tokio` and `reqwest`.

## Public Modules

- `crawler`: GitHub API client and response models.
- `patterns`: simple crate-specific source analysis.
- `stats`: usage and category aggregation from Cargo manifests.
- `crates_io`: crates.io summary workflow (`fetch_summary`, `validate_crate_name`, `execute`).
- `templates`: Markdown badge and link rendering helpers.
- `utils`: shared test helpers.

## Public API Highlights

- `GithubCrawler::new(token)`, `search_cargo_tomls(query)`, and `download_content(url)`.
- `PatternAnalyzer::new()` and `analyze_source(content, crate_name)`.
- `CrateStats::new()`, `process_cargo_toml(content)`, and `print_summary()`.
- `crates_io::execute(crate_name)` for a simple crates.io summary workflow.
- `templates::{DestinationKind, render_badge, render_link}` and `utils::{TestCase, test_with}`.

## Usage

Set your `GITHUB_TOKEN` environment variable for higher rate limits.

```bash
cargo run -p rust_rate -- stats anyhow --limit 5
cargo run -p rust_rate -- crates-io anyhow
```

## Documentation

- [Requirements](../documentation/rust_rate/requirements.md)
- [Architecture](../documentation/rust_rate/architecture.md)
