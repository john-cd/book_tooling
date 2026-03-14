# `rust_rate`

`rust_rate` is a tool for crawling and analyzing Rust crate usage patterns across public repositories.

## Features

- Crawls GitHub for `Cargo.toml` files to track crate popularity.
- Analyzes Rust source code to identify common usage patterns and code snippets.
- Provides statistics per crate and per category.
- Highly concurrent using `tokio` and `reqwest`.

## Usage

Set your `GITHUB_TOKEN` environment variable for higher rate limits.

```bash
cargo run -p rust_rate
```

## Documentation

- [Requirements](requirements.md)
- [Architecture](architecture.md)
