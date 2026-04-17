# TODO

- [ ] P0 keep exactly one link-conversion implementation
- [ ] P0 if link conversion should be reusable beyond the CLI command, extract it from `book_tool::inline_links` into a shared module instead of letting orphan converters reappear.
- [ ] P1 add a lightweight repo hygiene check for orphan source files
- [ ] P1 refactors have left uncompiled `src/**/*.rs` files behind before; add a check or documented review step so the filesystem and module tree stay aligned.

## LATER

- [ ] revisit branch strategy once the refactor backlog is smaller
