# TODO

- [ ] create detailed documentation of all supported directives in root README.md
- [ ] detailed documentation of available CLI commands in root README.md
- [ ] review codebase for duplicated code
- [x] **Merge `directive_lib` into `book_tool`**: Move the implementation from the `directive_lib` crate into the `book_tool::directives` module.
  - [ ] **Merge `core_lib` into `book_tool`**: Move the common utilities from `core_lib` into a new `book_tool::common` module.
  - [ ] **Merge `rust_rate` crate into `book_tool`**: Move the implementation from the `rust_rate` crate into a `book_tool::rust_rate` module.
- [ ] refactor the list item parsers in `parse_document.rs` to remove the duplicated indentation logic
- [ ] add tests everywhere (Added tests for link parts and html renderer)
- [ ] rewrite parse_lib\README.md

## LATER

- [ ] move code to a develop branch and recreate a clean main branch
