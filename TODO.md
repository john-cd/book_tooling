# TODO

## Suggested Improvements and Fixes

### Parser Library (`parse_lib`)

- [ ] Further stabilize Markdown parsing for complex nested structures.
- [ ] Optimize the non-greedy plain text parser to improve performance on large files.
- [ ] Finish URL parsing logic.

### General

- [ ] Improve error reporting with better context in all CLI tools.

### Cargo.toml

- [ ] Update `description` in `workspace.package`.
- [ ] Update `documentation`, `homepage`, and `repository` URLs to correct values for this project.
- [ ] Fill in `keywords` and `categories` for discoverability.

### Codebase

- [ ] Review and complete the implementation of `directive_lib`, `parse_lib` and `core_lib`, removing the placeholder TODOs.
- [ ] In `core_lib`, consider using `Box<dyn Replacer>` instead of `Box<dyn Fn(&Captures) -> String>`.
- [ ] In `core_lib`, evaluate if `utils/cow_map_ext.rs` is necessary and either integrate or remove it.
