<!-- TODO Reviewed ??-??-2026. -->

# Task tracking

Reviewed November 18, 2026. Recheck [live issues](https://github.com/john-cd/book_tooling/issues)
before starting work.

## Hosted validation

- **P1 — [#88](https://github.com/john-cd/book_tooling/issues/88):** PR #87 is merged at
  `9d4e477`; run `44247975472` remained queued at review time. Obtain actual completed CI
  results before resolving the tracker.

## Review evidence

At `9a7b290`, `just cargo test --workspace --locked --offline` passed 428 tests, including the
Unicode CLI regression and doctests. `just cargo clippy --workspace --all-targets --locked
--offline -- -D warnings`, `just format-check`, and `dprint check` also passed. Human review
markers remain pending.
