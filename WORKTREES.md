<!-- TODO Reviewed ??-??-2026. -->

# Worktrees

Updated 2026-11-01 (America/Los_Angeles). This file describes the managed layout;
use `git worktree list --porcelain` and `git status --short` for live state.

- The primary checkout stays on `develop` and uses cache slot `main`.
- `main` is an independent empty root commit, with no development history.
- Auxiliary checkouts must be direct children of `.worktrees/`, named with one
  lowercase letter: a through j only. Reuse clean slots and preserve their caches.
- Before moving a detached checkout, preserve its unique commits on a named
  branch and push that branch. Dirty work must be committed before switching.
- Squash related unpublished commits, rebase or fast-forward to synchronize
  `develop`, and never introduce synchronization merge commits.
- Keep backup branches until their unique commits are integrated or deliberately
  retired. Delete ordinary topic branches only after verifying they are merged.

## Storage roots

Override the `CODE_*_ROOT` environment variables before invoking tooling.
Windows defaults are `CODE_REPOS_ROOT=D:/code`,
`CODE_TEMP_ROOT=C:/temp` (`C:/temp` fallback),
`CODE_PACKAGES_ROOT=D:/packages`, and `CODE_TOOLS_ROOT=C:/tools`.
Unix uses native home/cache/temp directories. Root overrides do not move existing
installations or rewrite independent package-manager settings.

## Rust

Use `just build-env` to inspect the effective environment and
`just cargo <arguments>` to run Cargo. Windows targets use
`CODE_TARGET_ROOT/book_tooling/<slot>` (default root `D:/target`);
intermediates use `CODE_BUILD_ROOT/book_tooling/<slot>` (`D:/code_build`).
The intermediate volume may use NTFS or ReFS with Rust 1.100.0 or newer.
Unix defaults use XDG cache or `~/.cache` under `cargo-target` and
`cargo-build`. The runner enables sccache with incremental compilation disabled
and computes base directories from registered allowed worktrees.

Run `just configure-cargo-host` once per host and after changing roots. It keeps shared compiler defaults in `CARGO_HOME/config.toml` and removes global target/build output overrides. Run build-setup once per checkout. Task-runner Cargo commands pass absolute `D:/target/<repo>/<slot>` and `D:/code_build/<repo>/<slot>` paths; use the checkout links or explicit Cargo environment paths for direct Cargo and rust-analyzer as required by the host filesystem. See [README](README.md).

## Current state

Reviewed 2026-11-01. The primary checkout is clean on `develop`, synchronized with
`origin/develop` after a fast-forward push. The integrated history is linear and includes the
reviewed #76/#77/#78/#79 work, the validated Cargo.lock update, and the `{{#badges crate=}}`
feature for [rust_howto#2145](https://github.com/john-cd/rust_howto/issues/2145). The lockfile
change passed `just cargo check --workspace --all-targets --locked` on Windows after rebasing
onto `origin/develop`. The badge feature passed focused `parse_lib`/`book_tool` tests and
targeted all-feature Clippy with warnings denied. Repository hooks passed with
`prek run --all-files`.

GitHub's all-state pull-request listing had no open PRs. PRs #63 and #64 were closed without
merge; their `claude/project-thread-sf1c32` head is no longer a local or remote branch. No PR
branch was eligible for cleanup.

| Checkout        | Branch or revision       | Status and disposition                                                                                                           |
| --------------- | ------------------------ | -------------------------------------------------------------------------------------------------------------------------------- |
| Repository root | `develop`                | Clean; fast-forward integrated and synchronized with `origin/develop`.                                                           |
| `.worktrees/a`  | detached / `29c339d`     | Clean snapshot at the independent `main` root; retain the checkout and `main` cache identity.                                    |
| `.worktrees/b`  | detached / `develop` tip | Clean after integrating the #2145 badge feature; retain the slot and warm cache.                                                 |
| `.worktrees/c`  | detached / `3be445c`     | Clean snapshot; its mirrored `backup/tooling-20261031/worktree-c` ref has seven commits not in `develop`. Retain snapshot/cache. |

The local topic refs `codex/cargo-update`, `codex/issues-77-79-maintenance`, and
`codex/issue-2145-badges-no-index` were removed after confirming their commits are ancestors of
`develop`. The #2145 implementation is commit `1e9cc34` after rebasing onto the integrated
develop history. The clean `.worktrees/e` and `.worktrees/f` checkouts were subsequently removed
as requested; `.worktrees/b` remains detached at the current `develop` tip.

The mirrored recovery refs `backup/develop-before-sync-20261031-b73ade5a`,
`backup/tooling-20261031/develop-before`, and `backup/tooling-20261031/working-state`
retain 9, 4, and 5 commits respectively that are not in `develop`. They preserve broad,
older configuration and source snapshots, so do not merge or delete them as cleanup.
`backup/tooling-20261031/worktree-a` preserves the independent `main` root, while
`backup/tooling-20261031/worktree-b` preserves a two-commit historical snapshot no longer
checked out. The remote-only
`origin/codex/rust-howto-pin-404e8fe` is 64 commits ahead of `develop` and has no open PR;
retain it because its divergent history contains unmerged work.

The unregistered `.worktrees/d` directory contained only managed junctions and was removed.
After confirming no Cargo, rustc, or rust-analyzer process was active, the `.worktrees/e` and
`.worktrees/f` directories were removed. The external per-slot `d`, `e`, and `f` target and
intermediate-build directories were also removed after verifying their resolved paths and that
they contained no reparse points. The shared `D:/cache/sccache`, primary `main` outputs, and the kept
`b` outputs were preserved. Older `D:/target/book_tooling/build/b` and
`D:/target/book_tooling/target/b` directories remain with the kept `b` slot. The dirty changes
in `.worktrees/b` were reviewed and committed as the #2145 badge feature; no unrelated
untracked, temporary, or generated file was confirmed safe to remove.

Use `git worktree list --porcelain` plus `git status --short --untracked-files=all` to refresh
this snapshot before further cleanup.

## Notification review 2026-11-17

Slot b: Codex, issue #86, branch fix/issue-86-ci-workflow-reference. Previous detached commit preserved on backup/notification-review-20261117-b. Other checkouts preserved.

Notification review result: slot b clean at 45ff80f; fix published in https://github.com/john-cd/book_tooling/pull/87 (open, unmerged). Existing primary-checkout changes preserved.

## PR integration 2026-11-17

PR #87 is squash-merged at 9d4e477. Primary checkout remains on develop, synchronized exactly with origin/develop (0 ahead, 0 behind). Topic branches and their additional local commits are preserved; existing working-tree edits are preserved. No worktrees, branches, or build caches were removed.
