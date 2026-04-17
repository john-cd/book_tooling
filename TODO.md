# TODO

- [ ] P0 define one parser boundary in `parse_lib`
- [ ] P0 stop mixing `markdown::*` and `parse_parts::*` entry points in `parse_document.rs`; pick one public surface and keep the lower-level helpers internal.
- [ ] P0 keep exactly one link-conversion implementation
- [ ] P0 if link conversion should be reusable beyond the CLI command, extract it from `book_tool::inline_links` into a shared module instead of letting orphan converters reappear.
- [ ] P0 consolidate link verification around `book_tool::links`
- [ ] P0 fold any retry, rate-limit, and batching behavior into the active checker instead of keeping alternate verifier implementations.
- [ ] P1 make `link_check` actually concurrent
- [ ] P1 `AsyncLinkChecker` is async, but `link_check::execute` still awaits URLs one at a time; batch checks so network I/O is parallelized.
- [ ] P1 reduce repeated file-processing orchestration in CLI commands
- [ ] P1 `directives`, `inline_links`, `link_check`, and `extract_comments` all rebuild similar walk/read/parse/write flow; extract the common command pipeline where it improves clarity.
- [ ] P1 add a lightweight repo hygiene check for orphan source files
- [ ] P1 refactors have left uncompiled `src/**/*.rs` files behind before; add a check or documented review step so the filesystem and module tree stay aligned.
- [ ] P2 expand CLI-level integration coverage
- [ ] P2 add end-to-end tests for `inline-links`, `link-check --refdefs --delete`, and directive expansion over temporary fixture directories.
- [ ] P2 refresh docs for the current workspace shape
- [ ] P2 update `README.md`, `documentation/architecture.md`, and `parse_lib/README.md` to match the consolidated crates and command behavior.

## LATER

- [ ] revisit branch strategy once the refactor backlog is smaller
