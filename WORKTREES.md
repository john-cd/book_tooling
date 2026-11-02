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
`CODE_TEMP_ROOT=D:/temp` (`C:/temp` fallback),
`CODE_PACKAGES_ROOT=D:/packages`, and `CODE_TOOLS_ROOT=C:/tools`.
Unix uses native home/cache/temp directories. Root overrides do not move existing
installations or rewrite independent package-manager settings.

## Rust

Use `just build-env` to inspect the effective environment and
`just cargo <arguments>` to run Cargo. Windows targets use
`CODE_TARGET_ROOT/book_tooling/<slot>` (default root `D:/target`);
intermediates use `CODE_BUILD_ROOT/book_tooling/<slot>` (`C:/code_build`).
The intermediate volume must be NTFS; the runner rejects ReFS.
Unix defaults use XDG cache or `~/.cache` under `cargo-target` and
`cargo-build`. The runner enables sccache with incremental compilation disabled
and computes base directories from registered allowed worktrees.

Run `just configure-cargo-host` once per host and after changing roots.
It configures the host fallback in `CARGO_HOME/config.toml`. Run the repository's
`build-setup` recipe per checkout. Direct Cargo and rust-analyzer use that checkout's
`target` link and the host's absolute per-workspace build fallback. The Just runner
uses absolute per-repo/per-slot paths; leave local `build-dir` unset so it does not
override the host fallback. See [README](README.md).

## Current state

Reviewed 2026-11-01. At review start, the primary checkout was clean on `develop` at `51aad9c`,
matching `origin/develop`. The issue branch was later fast-forwarded into local `develop`; the
remote was not changed. GitHub's all-state pull-request listing had no open PRs. PRs #63 and #64
were closed without merge; their `claude/project-thread-sf1c32` head is no longer a local or
remote branch. No PR branch was eligible for cleanup.

| Checkout        | Branch or revision               | Status and disposition                                                                                                                                         |
| --------------- | -------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository root | `develop` / issue branch tip     | Clean after local fast-forward integration; `origin/develop` remains the prior base.                                                                           |
| `.worktrees/a`  | detached / `29c339d`             | Clean snapshot at the independent `main` root; retain the checkout and `main` cache identity.                                                                  |
| `.worktrees/b`  | detached / `3701b6c`             | Clean snapshot; its mirrored `backup/tooling-20261031/worktree-b` ref has two commits not in `develop`. Retain the snapshot and cache.                         |
| `.worktrees/c`  | detached / `3be445c`             | Clean snapshot; its mirrored `backup/tooling-20261031/worktree-c` ref has seven commits not in `develop`. Retain the snapshot and cache.                       |
| `.worktrees/e`  | `codex/cargo-update` / `142f1b4` | Clean; contains one local-only Cargo.lock update based on `a639fc4`. Keep pending dependency-build validation; no associated PR or remote topic ref was found. |
| `.worktrees/f`  | `codex/issues-77-79-maintenance` | Clean and at the integrated issue tip; retain while the related issues remain open.                                                                            |

The mirrored recovery refs `backup/develop-before-sync-20261031-b73ade5a`,
`backup/tooling-20261031/develop-before`, and `backup/tooling-20261031/working-state`
retain 9, 4, and 5 commits respectively that are not in `develop`. They preserve broad,
older configuration and source snapshots, so do not merge or delete them as cleanup.
`backup/tooling-20261031/worktree-a` preserves the independent `main` root. The remote-only
`origin/codex/rust-howto-pin-404e8fe` is 64 commits ahead of `develop` and has no open PR;
retain it because its divergent history contains unmerged work.

The unregistered `.worktrees/d` directory contained only three managed junctions to the shared
`.sccache` and slot `d` build/target paths. After confirming there was no running Cargo or rustc
process, the link-only wrapper was removed; the external slot `d` directories and shared sccache
were preserved. Empty top-level `build` and `target` directories under the external target root
remain because their owner is unclear. No untracked files were found in the registered historical
checkouts, and no other temporary or generated file was confirmed safe to remove.

The clean `codex/cargo-update` lockfile change was reviewed but not merged because it lacks a
dependency build result. Keep `develop` linear and use `git worktree list --porcelain` plus
`git status --short --untracked-files=all` to refresh this snapshot before further cleanup.
