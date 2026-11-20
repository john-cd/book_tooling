<!-- TODO Reviewed 2026-11-20. -->

# Task tracking

Reviewed November 20, 2026. Recheck [live issues](https://github.com/john-cd/book_tooling/issues)
before starting work.

## Review evidence

At `9a7b290`, `just cargo test --workspace --locked --offline` passed 428 tests, including the
Unicode CLI regression and doctests. `just cargo clippy --workspace --all-targets --locked
--offline -- -D warnings`, `just format-check`, and `dprint check` also passed. Human review
markers remain pending.
