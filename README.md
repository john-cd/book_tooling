<!-- TODO Reviewed ??-??-2026. -->

# Book Tooling

A collection of Rust-based tools for processing Markdown files, particularly for books and documentation projects.

## Local Rust checks

Install the toolchain from `rust-toolchain.toml` with `rustup show` in the checkout. From the repository root, run `just check`, `just config-drift`, and `just test-all`. For formatting, use the nightly pin in `rustfmt-toolchain.toml`.

## Overview

This workspace provides a suite of tools to automate the management and enrichment of Markdown content. It includes a high-performance parser, a unified CLI for document processing, and a specialized tool for analyzing Rust crate metrics.

## Choosing between `book_tooling` and `mdbook-utils`

These are separate command-line tools with complementary responsibilities. Use `mdbook-utils` for
mdBook-specific workflows such as sitemaps, include and code-example management, crate/category
indexes, dependency reference definitions, badges, and read-only link reports. Use `book_tooling`
for authoring-time operations such as rewriting inline links in place, checking external HTTP links,
auditing unused reference definitions, searching Markdown, and extracting comments.

| Task                                                                              | Tool                 |
| --------------------------------------------------------------------------------- | -------------------- |
| Build sitemaps, resolve `book.toml` paths, or manage mdBook includes and examples | `mdbook-utils`       |
| Generate crate/category indexes, dependency references, or badges                 | `mdbook-utils`       |
| Report links or duplicate/unresolved references without editing Markdown          | `mdbook-utils links` |
| Rewrite inline links or audit reference definitions in place                      | `book_tool`          |
| Check external HTTP links, search content, or extract comments                    | `book_tool`          |

The command-line tools are separate. `mdbook-utils` uses pulldown-cmark for its read-only scans;
`book_tooling` uses `parse_lib` for source-preserving Markdown edits and analysis. The two repositories
currently keep their URL and badge rules in their own libraries.

## Workspace Structure

### Libraries

- **[`book_templates`](./book_templates)**: Markdown badge and link templates (`DestinationKind`, `render_badge`, `render_link`) shared by `book_tool` and `rust_rate`.
- **[`parse_lib`](./parse_lib)**: A non-destructive Markdown and directive parser built with `winnow`. It provides the core AST and visitor infrastructure.

### Tools

- **`book_tool`**: The primary unified CLI tool. It centralizes several document processing tasks:
  - **Directives**: Expands custom `{{ ... }}` directives like crate badges and include blocks.
  - **LinkCheck**: Validates external HTTP/HTTPS links and manages reference definitions.
  - **InlineLinks**: Automatically converts naked URLs and inline links into reference-style links.
  - **Search**: Performs content-aware text search across documents, ignoring Markdown formatting.
  - **ExtractComments**: Aggregates HTML comments for auditing and review.
- **`rust_rate`**: A standalone tool for crawling GitHub and analyzing Rust crate usage statistics and patterns.

## Module Map

### `book_tool` Modules

- `common`: shared file rewriting, directory walking, and template rendering helpers.
- `directives`: directive and wikilink expansion workflows plus `Config`.
- `extract_comments`: HTML comment aggregation workflow.
- `inline_links`: inline-link to reference-definition conversion workflow.
- `link_check`: external link verification and refdef cleanup workflow.
- `links`: lower-level URL extraction and async verification helpers.
- `refdefs`: unused reference-definition analysis and deletion helpers.
- `search`: plain-text search workflow over parsed Markdown.

### `parse_lib` Modules

- `ast`: `Document`, `Documents`, `Element`, directive enums, and related data payloads.
- `visit`: `Visitor`, `Visitable`, and helper visitors such as `PrettyPrinter` and `Debugger`.
- `directive_extractor`, `html_comment_extractor`, `link_extractor`, `plain_text_extractor`: reusable analysis visitors.
- `html_renderer`, `markdown_renderer`: AST-to-output renderers.
- `markdown`, `parse_directives`, `parse_hidden`, `parse_urls`, `parse_wikilinks`, `parse_document`: parser entry points.

### `rust_rate` Modules

- `crawler`: GitHub API client and response types.
- `patterns`: source-pattern analysis helpers.
- `stats`: Cargo manifest aggregation helpers.
- `crates_io`: crates.io summary workflow.
- `templates`: Markdown badge/link rendering helpers.
- `utils`: shared test helpers.

## Public APIs

### `book_tool` APIs

- Commands are exposed as module-level `execute(...)` functions for `directives`, `extract_comments`, `inline_links`, `link_check`, and `search`.
- Shared utilities include `common::replace_text_file_atomically`, `common::Scope`, `common::walk_directories_and_process_files`, `links::extract_urls`, `links::AsyncLinkChecker`, `refdefs::find_unused_refdefs`, and `refdefs::delete_unused_refdefs`.

### `parse_lib` APIs

- Core entry point: `parse_lib::parse_document(...) -> anyhow::Result<Document<'_>>`.
- Core model: `Document`, `Documents`, `Element`, `DirectiveData`, `DestinationKind`, `IndexAnchorKind`, and the `*Data` payload structs in `ast`.
- Traversal surface: `visit::Visitor`, `visit::Visitable`, `visit::pretty_printer::PrettyPrinter`, `visit::debugger::Debugger`, and `visit::refdefs::RefDefManager`.
- Reusable visitors: `DirectiveExtractor`, `HtmlCommentExtractor`, `LinkExtractor`, `PlainTextExtractor`.
- Renderers: `HtmlRenderer`, `MarkdownRenderer`, and the resolver callback types used by `MarkdownRenderer`.
- Public parser helpers: functions re-exported from `markdown`, plus `parse_directive_element`, `parse_hidden_html_div`, `parse_naked_url`, `parse_autolink`, `parse_wikilink`, and `url_kind`.

### `rust_rate` APIs

- Crawling surface: `GithubCrawler`, `GithubSearchResponse`, `GithubItem`, and `GithubRepo`.
- Analysis surface: `PatternAnalyzer` and `CrateStats`.
- Workflow surface: `rust_rate::crates_io::execute(crate_name)`.
- Formatting/test helpers: `templates::{DestinationKind, render_badge, render_link}` and `utils::{TestCase, test_with}`.

## `book_tool` Usage

`book_tool` provides several subcommands for different tasks.

### Process Directives

Replaces custom directives with their expanded Markdown/HTML content.

```bash
book_tool directives ./src
```

The filesystem command picks its configuration in this order: the file named by `--config <FILE>`
(an error if it is missing or malformed), otherwise `config.toml` in the current working directory
(not in the directory being processed), otherwise `Config::default()`, which enables every directive
family for `.md` files except `refs.incl.md` and `SUMMARY.md`. A configuration file replaces the
defaults key by key, so omitted keys keep their default values and unknown keys are ignored. The
directory arguments only choose which files to process and never select a configuration file; there
are no per-directive command-line overrides. Its example shorthand keeps the legacy
`examples/<name>.md` include mapping. Render-time adapters use
`process_markdown_with_example_resolver` to resolve chapter-relative paths and anchors in their own
book layout.

### Check Links

Verifies external links and optionally cleans up unused reference definitions.

```bash
book_tool link-check ./src --refdefs --delete
```

Link checking requires outbound network access; there is no offline mode. The checker sends `HEAD`
requests first and falls back to `GET` when a response is not successful. Only `2xx` responses pass.
Each method retries `429` and `503` responses up to three times using the `Retry-After` value (seconds or an HTTP-date, capped at 30 seconds),
or exponential backoff starting at one second when the header is absent or malformed. Requests time out after ten seconds, with up
to 32 links checked concurrently. Local HTTP fixtures can exercise the command without contacting
external sites. Each unique URL is checked once per run; results are not cached between runs, and
the checker has no in-memory response cache.

### Audit Reference Definitions

Writes a JSON report with definition locations, include provenance, and reference-use locations.
Additional roots are scanned for reference uses without requiring their include graphs to resolve;
these uses are reported separately and protect matching definitions from deletion. Generated or
otherwise maintained definition files can be protected explicitly.

```bash
book_tool refdefs-audit ./bk/src --report ./refdefs-audit.json \
  --also-scan-uses-in ./bk/drafts \
  --also-scan-uses-in ./later/src \
  --preserve-definitions-in ./bk/src/refs/dependency-refs.md \
  --delete
```

### Convert Inline Links

Transforms `[text](url)` and `https://url.com` into reference-style links.

```bash
book_tool inline-links ./src
```

Untitled links and naked URLs become reference-style links. Existing reference definitions are reused,
and generated numeric labels avoid collisions. Links with Markdown titles and rust_howto issue links
remain inline so their original semantics and source text are preserved.

### Search

Searches for text in the document's plain-text content.

```bash
book_tool search "some query" ./src
```

### Extract Comments

Extracts all `<!-- comment -->` blocks into a single output file.

```bash
book_tool extract-comments ./src --output comments.md
```

### Choosing Files

`link-check`, `inline-links`, `search` and `extract-comments` process `.md` files and skip
`refs.incl.md` and `SUMMARY.md` by default. Use `--extension` (repeatable; replaces `md`) and
`--exclude` (repeatable; adds to the skipped names) to change that:

```bash
book_tool search "some query" ./src --extension md --extension markdown --exclude drafts.md
```

`directives` takes its file filters from `config.toml`, and `refdefs-audit` always analyzes Markdown.

## Supported Directives

- `{{cat name}}`: Link to a crates.io category.
- `{{crate name}}`: Link to an internal crate page.
- `{{docs name}}`: Link to docs.rs documentation.
- `{{github repo}}`: Link to a GitHub repository.
- `{{!docs name}}`: A linked docs.rs badge; the other link kinds also support `!` badges.
- `{{#crate name cat1 cat2}}`: A crate badge followed by badges for its categories.
- `{{#badges name cat1 cat2}}`: The full badge set for a crate (`docs.rs`, `crates.io`, repository and `lib.rs`, sharing one index entry), followed by badges for its categories. Use `name=` to omit the crate index entry. Use `slug=Title` to add a category index term or `slug=` to omit it; quote category titles with spaces, as for `{{#categories}}`.
- `{{#categories filesystem,api-bindings}}`: Badges for categories. Use `slug=Title` to keep a display title, including spaces in quotes, such as `{{#categories filesystem=Filesystem,api-bindings="API bindings"}}`; use `slug=` to omit the index term.
- `{{#example name}}`: Inserts the legacy `examples/name.md` include when processed by the file-oriented command. In an mdBook adapter, the example resolver can instead map a chapter-relative include such as `{{#example ../../../crates/cats/filesystem/examples/read_write/read_file.rs:example}}`.

The render-time API, `directives::process_markdown`, applies the same configured transformations to a string without touching files. `process_markdown_with_example_resolver` lets an adapter supply its own example mapping and returns an error if that resolution fails.

## Getting Started

### Installation

Clone the repository and build the workspace:

```bash
cargo build --release
```

The binaries will be available in `target/release/book_tool` and `target/release/rust_rate`.

## Documentation

- [Workspace documentation](./documentation/README.md)
- [Contributing](./CONTRIBUTING.md)
- [TODO](./TODO.md)

## Development checks

- `just fa` formats Rust code with the pinned nightly toolchain.
- `just ca` runs Clippy across all targets and features.
- `just deny` checks dependency advisories, licenses, duplicate versions, and registry sources.
- `just nta` runs the workspace tests with cargo-nextest and includes doctests.
- `dprint fmt` formats Markdown, JSON, YAML, and TOML files.
- `prek run --all-files` runs repository hooks, including formatting and dependency policy checks.
- `just cargo bench -p parse_lib` and `just cargo bench -p book_tool` run the [divan](https://crates.io/crates/divan)
  benchmarks: parser throughput (`parse_lib/benches/parsing.rs`), reconstruction
  (`parse_lib/benches/reconstruction.rs`), and corpus processing (`book_tool/benches/processing.rs`).
  Benchmarks are not part of `just nta`; run them on an otherwise idle machine and compare runs on the same host.

## License

This project is proprietary. See the [LICENSE](LICENSE) for terms, including restrictions on AI model training.

## Rust tooling

Use `just` for build, lint, and test tasks. Install Rust, sccache, and Just. Windows recipes require Git Bash.
Build paths and diagnostics are implemented directly in the justfile.

- `just build-env` displays the effective environment, checkout, and branch.
- `just cargo <subcommand> [args...]` runs Cargo with this checkout's output paths.

### Compiler cache

Plain Cargo can run without a wrapper. The Just runner uses sccache with incremental compilation
disabled and a 45G cache limit. The runner exports `RUSTC_WRAPPER=sccache`,
`CARGO_BUILD_RUSTC_WRAPPER=sccache`, `CARGO_INCREMENTAL=0`, and
`SCCACHE_CACHE_SIZE=45G`, replacing inherited values. Explicit shell or Cargo
command-line overrides still take precedence for direct Cargo invocations.

`CODE_SCCACHE_ROOT` overrides `SCCACHE_DIR`; otherwise the default below applies.
The task runner sets `SCCACHE_BASEDIRS` to the main checkout and registered `.worktrees/a` through `.worktrees/j`,
discarding inherited unrelated paths. sccache reads daemon settings at startup;
wait for all builds to finish before restarting a daemon with stale settings.

### Output paths and worktrees

Use `.worktrees/a` through `.worktrees/j` for auxiliary checkouts.
Preserve active or dirty checkouts when reusing slots. The primary checkout's
output slot is `main`, regardless of its branch name.

| Variable            | Windows default    | Linux/macOS default                                       |
| ------------------- | ------------------ | --------------------------------------------------------- |
| `CODE_TARGET_ROOT`  | `D:/target`        | `$XDG_CACHE_HOME/cargo-target` or `~/.cache/cargo-target` |
| `CODE_BUILD_ROOT`   | `D:/code_build`    | `$XDG_CACHE_HOME/cargo-build` or `~/.cache/cargo-build`   |
| `CODE_SCCACHE_ROOT` | `D:/cache/sccache` | `$XDG_CACHE_HOME/sccache` or `~/.cache/sccache`           |
| `CODE_TEMP_ROOT`    | `C:/temp`          | Existing native `TMPDIR` or `/tmp`                        |

Set roots to absolute paths and create the directories before building. Keep
Windows intermediate builds on NTFS. Targets use
`CODE_TARGET_ROOT/book_tooling/<slot>` and intermediate files use
`CODE_BUILD_ROOT/book_tooling/<slot>`. The runner exports these as
`CARGO_TARGET_DIR`, `CARGO_BUILD_TARGET_DIR`, and `CARGO_BUILD_BUILD_DIR`.

On Windows, the runner sets `TEMP`, `TMP`, and `TMPDIR` from `CODE_TEMP_ROOT`.
Unix and WSL retain their native temporary directories.

## Consistent Python and hook tooling

Use `uv` for Python dependencies and commands. Each standalone Python project
commits `uv.lock` and `.python-version`; uv workspace members share their root's
lockfile and Python pin. Run `uv sync --locked` before development.

Use `prek` with `prek.toml`: run `prek install` and `prek run --all-files`.

## Checkout and storage policy

The primary checkout uses `develop`; `main` is an independent empty branch.
Auxiliary worktrees belong under `.worktrees/`, with names one lowercase letter from a through j only.
Storage roots are configurable through `CODE_REPOS_ROOT`, `CODE_TEMP_ROOT`,
`CODE_PACKAGES_ROOT`, and `CODE_TOOLS_ROOT`. Windows defaults are `D:/code`,
`C:/temp` (`C:/temp` fallback), `D:/packages`, and `C:/tools` respectively.
Unix uses native home/cache/temp paths. A root override changes destinations;
it does not move existing installations. Tool-specific overrides such as
`CARGO_HOME`, `npm_config_cache`, and `NUGET_PACKAGES` remain independent.
Rust builds also read `CODE_BUILD_ROOT`, `CODE_TARGET_ROOT`, and
`CODE_SCCACHE_ROOT`. Their Windows defaults are `D:/code_build`, `D:/target`,
and `D:/cache/sccache`. The runner rejects non-NTFS intermediate build roots.
Run `just configure-cargo-host` once per host (PowerShell 7 is required), and rerun it after changing roots. It preserves other Cargo settings, keeps the shared sccache and incremental defaults in `CARGO_HOME/config.toml`, and removes global output-directory overrides. Each checkout's Cargo config routes `target` and `build` through checkout-local links to `D:/target/<repo>/<slot>` and `D:/code_build/<repo>/<slot>`. Run the build-setup recipe once per checkout before using plain Cargo or rust-analyzer; task-runner Cargo commands supply the same per-slot paths and repository sccache settings.

## Repository configuration

See [CONFIGURATION.md](CONFIGURATION.md) for shared formatting, lint, dependency-update,
build-profile, cache, linker, and editor conventions. Keep it synchronized with configuration changes.
