# TODO

## Suggested Improvements and Fixes

### Core Library (`core_lib`)
- [ ] Add more comprehensive tests for the template system.
- [ ] Implement a more robust `process_text_file` that handles atomic writes more gracefully.

### Parser Library (`parse_lib`)
- [ ] Add support for escaped brackets in `parse_link_label`.
- [ ] Further stabilize Markdown parsing for complex nested structures.
- [ ] Optimize the non-greedy plain text parser to improve performance on large files.

### Link Checker (`link_checker`)
- [ ] Implement detection and removal of unused reference definitions.
- [ ] Add support for concurrent link validation using multiple tasks.
- [ ] Implement a cache for validated URLs to avoid redundant network requests across runs.
- [ ] Add a configuration option to follow redirects or not.

### Inline Link Converter (`inline_links`)
- [ ] Improve label generation to avoid all special characters, not just dots.
- [ ] Add support for reconstructing all AST elements in the transformed output.
- [ ] Implement a mechanism to handle conflicts between generated labels and existing ones more intelligently.

### General
- [ ] Improve error reporting with better context in all CLI tools.
- [ ] Add documentation for the new template system.
- [ ] Standardize logging across all crates using `tracing`.
