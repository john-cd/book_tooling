# TODO

- [ ] add nextest.toml
- [ ] address all TODO comments in the codebase
- [ ] fix failing tests
- [ ] document all modules / functions / traits / structs / enums

- [ ] Add support for nested directives in `parse_lib`.
- [ ] Use memory mapping for large files in `core_lib::process_text_file`.
- [ ] Support GFM-style tables and task lists in `parse_lib`.
- [ ] Add more comprehensive integration tests for all CLI tools.
- [ ] Improve API documentation with more examples.
- [ ] Further stabilize Markdown parsing for complex nested structures.
- [ ] Optimize the non-greedy plain text parser to improve performance on large files.
- [ ] Improve error reporting with better context in all CLI tools.

# DONE

## Suggested Improvements and Fixes

### Core Library (`core_lib`)

- [x] Add more comprehensive tests for the template system.
- [x] Implement a more robust `process_text_file` that handles atomic writes more gracefully.

### Parser Library (`parse_lib`)

- [x] Add support for escaped brackets in `parse_link_label`.

### Link Checker (`link_checker`)

- [x] Implement detection and removal of unused reference definitions.
- [x] Add support for concurrent link validation using multiple tasks.
- [x] Implement a cache for validated URLs to avoid redundant network requests across runs.
- [x] Add a configuration option to follow redirects or not.

### Inline Link Converter (`inline_links`)

- [x] Improve label generation to avoid all special characters, not just dots.
- [x] Add support for reconstructing all AST elements in the transformed output.
- [x] Implement a mechanism to handle conflicts between generated labels and existing ones more intelligently.

### General

- [x] Add documentation for the new template system.
- [x] Standardize logging across all crates using `tracing`.
