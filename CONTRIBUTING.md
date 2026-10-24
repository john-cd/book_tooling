# Contributing to book_tooling

Thanks for helping improve the Rust tools and documentation workflows in this repository. Keep changes focused, preserve existing behavior, and include the commands and results that support the change.

## Development setup

1. Install Rust with rustup. The repository's [rust-toolchain.toml](rust-toolchain.toml) selects the stable toolchain and components; the formatting recipe uses the pinned nightly rustfmt toolchain.
2. Install cargo-binstall with `cargo install cargo-binstall`, then install cargo-nextest, prek, dprint, and cargo-deny at the versions listed in the CI workflow with `cargo binstall --no-confirm cargo-nextest@0.9.145 prek@0.5.6 dprint@0.64.1 cargo-deny@0.20.2`.
3. Run prek install to enable the repository's pre-commit hooks.

The workspace uses portable cache paths. Set BOOK_TOOLING_CACHE_HOME to choose a cache root, BOOK_TOOLING_WORKTREE_NAME to give another checkout a separate cache, DPRINT_CACHE_DIR to choose dprint's plugin cache, or CARGO_TARGET_DIR and CARGO_BUILD_BUILD_DIR to override Cargo output paths. just selects a platform-appropriate cache default on Windows, macOS, and Linux.

## Checks

Run these commands from the repository root:

- `just fa` formats all workspace code.
- `dprint fmt` formats supported Markdown, JSON, YAML, and TOML files.
- `just ba` builds all workspace targets and features.
- `just ca` runs Clippy with warnings denied.
- `just deny` checks dependency advisories, licenses, duplicate versions, and registry sources.
- `just nta` runs workspace tests, including doctests.
- `prek run --all-files` runs the configured hooks before review.

Run the checks that cover the changed files, then inspect `git diff --check` and the complete diff. Cargo checks should run sequentially within a checkout because they share build outputs.

## Changes and commits

Use short-lived topic branches based on the current `develop` branch and keep history linear. Use focused commits with a conventional type prefix, for example `fix(parser): preserve reference links`. Keep README and contributor instructions in sync with changed commands, configuration, or behavior.

## Issue priority and review labels

Use `priority:P1` through `priority:P4` labels; do not put `[P#]` prefixes in issue titles. P1 blocks publication, P2 is needed for the first edition, P3 should be done, and P4 is later or optional. Use `human-review` when work needs the owner's decision or a manual acceptance check. In `rust_howto`, every `human-review` issue must be a sub-issue of epic [#1849](https://github.com/john-cd/rust_howto/issues/1849).

Priority labels rank work. Milestones track release scope: reserve `URGENT` for current publication blockers; use `PUBLISHABLE` for work in the publication-ready batch. These axes are independent, so a P3 item can be in `PUBLISHABLE` when it belongs in that batch, and P1/P2 items can use another milestone when their release target differs. A P4 item normally belongs in `LATER PUBLICATION`.

When a project board has a `Priority` field, mirror the issue's `priority:P#` label there for sorting. Use board status and scheduling fields to track workflow; the issue label remains the priority source of truth. Update the label and board field together when priority changes.
