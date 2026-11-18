<!-- TODO Reviewed ??-??-2026. -->

# Task tracking

Reviewed November 18, 2026. Recheck [live issues](https://github.com/john-cd/book_tooling/issues)
before starting work.

## Hosted validation

- **P1 — [#88](https://github.com/john-cd/book_tooling/issues/88):** PR #87 is merged at
  `9d4e477`; run [44247975472](https://github.com/john-cd/book_tooling/actions/runs/44247975472)
  is still queued on `e0bd8387bcafbb046b710586f6ac4d6895688d83` as of 2026-11-18 08:49 UTC,
  with zero jobs. Local tests (428), strict all-target Clippy, formatting, and dprint pass on
  `develop` at `9a7b290`; hosted acceptance remains open. The reason for the queue is unknown
  and needs owner review of account and runner availability.

## Review evidence

At `9a7b290`, `just cargo test --workspace --locked --offline` passed 428 tests, including the
Unicode CLI regression and doctests. `just cargo clippy --workspace --all-targets --locked
--offline -- -D warnings`, `just format-check`, and `dprint check` also passed. Human review
markers remain pending.
