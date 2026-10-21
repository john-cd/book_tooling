<!-- TODO Reviewed ??-??-2026 -->
# book_tooling instructions

## Branches

`develop` is the default integration branch and the primary checkout branch. `main` starts with an empty root commit and has independent history. Keep the existing `main` build-cache directory identity across branch renames to reuse compiled artifacts.

## Build outputs

Cargo artifacts are stored outside the checkout in `D:\target\book_tooling\<worktree name>`. Cargo intermediate build artifacts are stored in `C:\code_build\book_tooling\<worktree name>`.

The root `justfile` exports `CARGO_TARGET_DIR` and `CARGO_BUILD_BUILD_DIR`. The `.cargo\config.toml` values cover direct Cargo invocations. The default worktree name is `main`; set `BOOK_TOOLING_WORKTREE_NAME` before running `just` in another worktree to keep its outputs separate.

## Build, lint, and test

Run commands from this repository root:

- `just ba` builds all targets and features.
- `just ca` runs Clippy across all targets and features with warnings denied.
- `just nta` runs workspace tests, including doctests.
- `just fa` formats the workspace with nightly rustfmt.
