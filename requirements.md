# Requirements

## Functional Requirements

- **Directive Processing**: Automatically replace custom directives (e.g., `{{cat xyz}}`, `{{#crate my_crate}}`) in Markdown files with corresponding Markdown links, badges, or blocks.
- **Reference-style Link Conversion**: Convert inline links and naked URLs into reference-style links for cleaner Markdown source.
- **External Link Checking**: Verify that external links in Markdown files are valid and reachable.
- **Directory Traversal**: Recursively walk through directories to find and process Markdown files.
- **Atomic Updates**: Ensure file updates are atomic by writing to a temporary file and then renaming it.

## Non-Functional Requirements

- **Performance**: Efficiently process large directories of Markdown files.
- **Reliability**: Handle various edge cases in Markdown parsing (e.g., links inside code blocks or hidden sections).
- **Maintainability**: Modularized architecture using a workspace with multiple specialized crates.
- **Extensibility**: Easy to add new types of directives or parsing rules.
