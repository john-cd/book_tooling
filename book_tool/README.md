<!-- TODO Reviewed ??-??-2026. -->

# `book_tool`

A unified CLI for maintaining Markdown books: directives, link checking,
reference-definition audits, inline-link conversion, search, and comment extraction.

## Subcommands

| Command            | Purpose                                                                                 |
| ------------------ | --------------------------------------------------------------------------------------- |
| `directives`       | Process custom directives such as `{{cat rust}}` in Markdown files                      |
| `link-check`       | Check for broken links; optionally report unused reference definitions                  |
| `refdefs-audit`    | Audit reference-definition provenance and uses; optionally delete proven-unused entries |
| `inline-links`     | Convert inline links to reference-style links                                           |
| `search`           | Search text across Markdown files, ignoring formatting                                  |
| `extract-comments` | Extract HTML comments into a single file for auditing                                   |

Run `cargo run -p book_tool -- --help` and `cargo run -p book_tool -- <command> --help` for
all options. Shared options select the file extensions and names to process.

## Development

- Tests: `cargo test -p book_tool` (integration tests in `tests/`).
- Benchmarks: `cargo bench -p book_tool`.
- Examples: see `examples/`.
- Design notes: [documentation](../documentation/README.md).
