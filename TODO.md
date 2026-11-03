<!-- TODO Reviewed ??-??-2026 -->

# Task tracking

<!-- recommended-issue-order:start -->

## Recommended issue order

Snapshot: November 3, 2026. Read each list from top to bottom: quick wins first,
then the remaining ready work in priority/dependency order. Quick wins are bounded
candidates, not time estimates. Recheck the live issue and current source before
starting; this index does not mark acceptance complete. Existing notes below are
context and may reference issues that have since closed.

Ready means a concrete engineering slice can start without a new owner decision or
external account/host. Where an issue mixes local work and external acceptance, only
the named local slice is ready. Blocked entries state what releases the block;
fixture preparation and investigation can continue independently.

### Ready: quick wins

No clearly bounded quick win is currently open; start with the priority list below.

### Ready: priority order

- [#78: Epic: Complete dependency-update coverage and consistent CI caching](https://github.com/john-cd/book_tooling/issues/78) — Start with updater/action/cache configuration; hosted acceptance waits on #62.
- [#81: Review parser and directive work on the stale rust-howto-pin branch](https://github.com/john-cd/book_tooling/issues/81) — Compare the candidate patches before retaining any behavior.
- [#77: Epic: Validate and finalize Cargo profiles, output paths, and linker policy](https://github.com/john-cd/book_tooling/issues/77) — Controlled local measurements can proceed; Linux/macOS acceptance needs the matching hosts.

### Blocked or deferred

- [#62: Re-enable CI and verify the refactored workflows](https://github.com/john-cd/book_tooling/issues/62) — Account/settings access and a hosted three-OS run.
- [#67: Human decision: book_tooling and mdbook-utils feature ownership (mdbook-utils#223)](https://github.com/john-cd/book_tooling/issues/67) — Maintainer feature-ownership decision with mdbook-utils#223.
- [#68: Human check: run the fuzz target and record the coverage baseline (#49)](https://github.com/john-cd/book_tooling/issues/68) — Nightly fuzzing host; coverage acceptance also waits on #62.
- [#80: Epic: human decisions, reviews, and host-dependent validation](https://github.com/john-cd/book_tooling/issues/80) — Tracks the human and host prerequisites above.

<!-- recommended-issue-order:end -->

Open work is tracked in [GitHub Issues](https://github.com/john-cd/book_tooling/issues).
Keep one topic per issue, except epics, with a problem, task list, and acceptance criteria.
Record status and validation evidence in the issue rather than duplicating checklists here.

## Blocker tracking

- [Human decisions, reviews, and host prerequisites](https://github.com/john-cd/book_tooling/issues/80).

## Maintenance

- [Finish shared formatting, lint, and hook policy validation](https://github.com/john-cd/book_tooling/issues/76).
- [Validate and finalize Cargo profiles, output paths, and linker policy](https://github.com/john-cd/book_tooling/issues/77).
- [Complete dependency-update coverage and consistent CI caching](https://github.com/john-cd/book_tooling/issues/78).
- [Review and reconcile repository branches and worktrees](https://github.com/john-cd/book_tooling/issues/79).
