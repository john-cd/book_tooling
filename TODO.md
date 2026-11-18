<!-- TODO Reviewed ??-??-2026. -->

# Task tracking

Reviewed November 17, 2026. Recheck [live issues](https://github.com/john-cd/book_tooling/issues)
before starting work.

## Ready

- **P1 — [#89](https://github.com/john-cd/book_tooling/issues/89):** Unicode case folding is
  required before cleanup can classify reference definitions as unused. The audit deletes
  `[Straße]` even when `[STRASSE]` resolves to it in mdBook. The issue has a CLI reproduction
  and regression acceptance criteria.

## Hosted validation

- **P1 — [#88](https://github.com/john-cd/book_tooling/issues/88):** PR #87 is merged at
  `9d4e477`; run `44247975472` remained queued at review time. Obtain actual completed CI
  results before resolving the tracker.

## Review evidence

At `9d4e477`, `just cargo test --workspace --locked --offline` passed 422 tests, including
doctests. `just cargo clippy --workspace --all-targets --locked --offline -- -D warnings`
also passed. These checks did not detect #89. Human review markers remain pending.
