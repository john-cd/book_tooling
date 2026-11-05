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

- [#77: Epic: Validate and finalize Cargo profiles, output paths, and linker policy](https://github.com/john-cd/book_tooling/issues/77) — Controlled local measurements can proceed; Linux/macOS acceptance needs the matching hosts.

### Blocked or deferred

- [#80: Epic: human decisions, reviews, and host-dependent validation](https://github.com/john-cd/book_tooling/issues/80) — Tracks the human and host prerequisites above.

<!-- recommended-issue-order:end -->

Open work is tracked in [GitHub Issues](https://github.com/john-cd/book_tooling/issues).
Keep one topic per issue, except epics, with a problem, task list, and acceptance criteria.
Record status and validation evidence in the issue rather than duplicating checklists here.

## Blocker tracking

- [Human decisions, reviews, and host prerequisites](https://github.com/john-cd/book_tooling/issues/80).

## Maintenance

- [Validate and finalize Cargo profiles, output paths, and linker policy](https://github.com/john-cd/book_tooling/issues/77).

## Codebase Review: Fixes and Improvements

None currently open.

### 1. Unit Testing and Test Coverage Gaps

- [ ] **`parse_lib::errors`**: Add unit tests for `ParsingError::display` and `annotate-snippets` rendering across single-line, multiline, and out-of-bounds byte spans.
- [ ] **`parse_lib::visit` helpers**:
  - Add unit tests for `visit::debugger::Debugger` visitor output format.
  - Add unit tests for `visit::pretty_printer::PrettyPrinter` state transitions and indentation handling.
  - Add unit tests for `visit::io::IoVisitor` write failure propagation.
- [ ] **`parse_lib::ast` types**: Add Display formatting and equality unit tests for `DirectiveData`, `DestinationKind`, and `IndexAnchorKind`.
- [ ] **`parse_lib::markdown` submodules**: Add localized unit tests directly in `markdown/headings.rs`, `markdown/links.rs`, and `markdown/text.rs` rather than relying solely on high-level `parse_document` tests.
- [ ] **`rust_rate` testing**:
  - Add unit tests for `rust_rate::patterns` covering syntax error handling and edge cases in regex pattern extraction.
  - Add tests for `rust_rate::utils::test_utils` helper structures.

### 2. Integration Testing Enhancements

- [ ] **`rust_rate stats` workflow test**: Add an integration test with local mock HTTP responses testing the end-to-end `rust_rate stats` CLI workflow, rate limit handling, and summary output generation.
- [ ] **`refdefs-audit` JSON schema validation**: Add an integration test that verifies the structured JSON report produced by `book_tool refdefs-audit --report <file>` against a strict schema.
- [ ] **CLI error reporting & edge cases**:
  - Add integration tests verifying non-zero exit codes and helpful error messages when `book_tool` commands receive non-existent directories, unreadable files, or malformed CLI arguments.
  - Add integration tests exercising `book_tool directives` flag overrides and config file precedence.
- [ ] **`parse_lib` diagnostic snapshots**: Add integration tests verifying source snippet rendering for invalid directive syntax and corrupted input documents.

### 3. Benchmarks and Examples

- [ ] **Add criterion/divan benchmarks (`benches/`)**:
  - **`parse_lib` parser benchmarks**: Benchmark `parse_document` on small, medium, and large Markdown documents to validate zero-copy claims and track parsing throughput (MB/s).
  - **`parse_lib` reconstruction benchmarks**: Benchmark `MarkdownRenderer` and `PrettyPrinter` AST traversal and output generation.
  - **`book_tool` processing benchmarks**: Benchmark directory traversal, link extraction, and directive replacement on representative book corpus fixtures.
- [ ] **Add runnable examples (`examples/`)**:
  - **`parse_lib` external consumer example**: Add an example demonstrating `parse_lib::parse_document` with a custom `Visitor` implementation for analyzing Markdown documents.
  - **`parse_lib` Markdown round-trip example**: Add an example demonstrating in-memory document parsing, element transformation, and `MarkdownRenderer` AST reconstruction.
  - **`book_tool` API example**: Add an example demonstrating programmatic usage of `AsyncLinkChecker` and `find_unused_refdefs_in_scopes`.

### 4. Documentation and Doc Comments

- [ ] **Fix rustdoc link warnings**: Resolve ambiguous intra-doc link warning in `parse_lib/src/lib.rs` (`warning: parse_document is both a function and a module`) by linking explicitly with function syntax `parse_document()`.
- [ ] **Complete doc comments on all APIs and items**:
  - Add doc comments to `book_tool/src/directives/conf.rs` (`Config`).
  - Add doc comments to `book_tool/src/refdefs/scope.rs` (`find_unused_refdefs_in_scopes`).
  - Add doc comments to `book_tool/src/link_check/mod.rs` (`execute`).
  - Add doc comments to `parse_lib/src/markdown_renderer.rs` (`MarkdownRenderer`).
  - Add doc comments to `parse_lib/src/visit/refdefs.rs` (`refdefs`).
  - Add doc comments to internal helpers and trait methods (e.g. `implicit_reference_uses` in `book_tool/src/refdefs/occurrences.rs`).
- [ ] **Document error conditions and invariants**: Add `# Errors` and `# Panics` sections to all fallible and invariant-checking public functions across `book_tool` and `parse_lib`.

### 5. Architecture and Design Alignment

- [ ] **Fix `rust_rate` pattern analysis pipeline**:
  - In `rust_rate/src/main.rs`, `run_stats` incorrectly passes downloaded `Cargo.toml` manifest contents into `PatternAnalyzer::analyze_source`, which searches for Rust source syntax (imports and function calls).
  - Implement downloading of actual Rust source (`.rs`) files discovered in target repositories before invoking `PatternAnalyzer`, matching the design in `documentation/rust_rate/architecture.md`.
- [ ] **Error handling in `parse_lib`**:
  - Refactor `parse_lib` public APIs (`parse_document`, `visit::refdefs::refdefs`) to return structured, typed errors using `thiserror` rather than `anyhow::Result`, aligning with the library error guidelines in `AGENTS.md`.
  - Reserve `anyhow::Result` for application/CLI boundaries in `book_tool` and `rust_rate`.
- [ ] **Deduplicate template formatting**:
  - Extract badge and link template rendering shared between `book_tool/src/common/templates.rs` and `rust_rate/src/templates/mod.rs` into a shared module or crate to eliminate duplicated template logic.
- [ ] **Parser backtracking complexity audit**: Audit `winnow` parsers in `parse_lib/src/markdown` for potential catastrophic backtracking on deeply nested or unbalanced inputs.

### 6. Discrepancies Between Documentation and Code

- [ ] **Reconcile `rust_rate` architecture doc with implementation**: Update `documentation/rust_rate/architecture.md` and `rust_rate/src/main.rs` to either document the current manifest-only behavior or implement the missing source-file crawler step.
- [ ] **Verify link checking cache documentation**: Ensure repository documentation accurately reflects whether in-memory link caching (`moka`) is enabled or planned for `book_tool::links::AsyncLinkChecker`.
- [ ] **Align configuration file documentation**: Clarify the relationship between `config.toml` path resolution, CLI directory arguments, and default configuration behavior in `book_tool` user documentation.
