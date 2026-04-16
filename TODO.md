# TODO

- [ ] P0 finish major refactor: get rid of directive_lib, link_checker
  - [ ] Merge `directive_lib` into `book_tool`**: Move the implementation from the `directive_lib` crate into the `book_tool::directives` module. do not leave orphan code behind
  - [ ] Move `process_text_file.rs` to `crate::common`.
  - [ ] Remove `walk_directory/mod.rs` and update all imports to use `crate::common` instead.
  - [ ] Update `inline_links`, `link_check`, and `search` modules to use `crate::common` instead of `core_lib`.
  - [ ] Move `refdefs/fs.rs` file system utilities into `crate::common`.
  - [ ] move all parsing functionality into parse_lib
  - [ ] keep `rust_rate` separate from `book_tool`**. make it its own CLI
  - [ ] review link_checker\src\main.rs and move where needed if needed
  - [ ] do not leave orphan code behind. add it as modules and notes (possible use / lack of use / duplication) for human review
- [ ] P1 review codebase for duplicated code
  - [ ] refactor the list item parsers in `parse_document.rs` to remove the duplicated indentation logic
  - [ ] Consolidate link verification logic: `verifier/mod.rs`, `inline_links/verifier/mod.rs`, and `links/asynchro.rs` contain duplicate or overlapping link checking code.
- [ ] P2 add tests everywhere (Added tests for link parts and html renderer)
- [ ] P2 rewrite parse_lib\README.md
- [ ] P2 create detailed documentation of all supported directives in root README.md
- [ ] P2 detailed documentation of available CLI commands in root README.md
- [ ] update documentation/

## LATER

- [ ] move code to a develop branch and recreate a clean main branch
