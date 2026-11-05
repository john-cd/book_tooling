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

None. No issue is currently open.

### Ready: priority order

None.

### Blocked or deferred

None.

<!-- recommended-issue-order:end -->

Open work is tracked in [GitHub Issues](https://github.com/john-cd/book_tooling/issues).
Keep one topic per issue, except epics, with a problem, task list, and acceptance criteria.
Record status and validation evidence in the issue rather than duplicating checklists here.

## Codebase Review: Fixes and Improvements

### 5. Architecture and Design Alignment

- [ ] **Deduplicate template formatting**:
  - Extract badge and link template rendering shared between `book_tool/src/common/templates.rs` and `rust_rate/src/templates/mod.rs` into a shared module or crate to eliminate duplicated template logic.
