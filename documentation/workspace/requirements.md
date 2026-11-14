<!-- TODO Reviewed ??-??-2026. -->

# Requirements

## Functional Requirements

- **Directive Expansion**: Automatically resolve and replace custom templates like `{{cat xyz}}` or `{{#crate my_crate}}` with expanded Markdown/HTML badges and links.
- **Reference-style Conversion**: Transform "naked" URLs (`https://...`) and inline links (`[text](url)`) into reference-style links for improved source readability.
- **Link Integrity**: Asynchronously verify external HTTP/HTTPS links and identify broken references.
- **Reference Definition Management**: Detect and optionally prune unused Markdown reference definitions (`[label]: url`) to keep documents clean.
- **Content-Aware Search**: Provide a CLI search tool that looks at the plain-text content of documents, bypassing Markdown formatting and metadata.
- **HTML Comment Auditing**: Extract and aggregate HTML comments across multiple files for review.

## Non-Functional Requirements

- **High Performance**: Utilize the `winnow` parser and `tokio` async runtime to process large document sets efficiently.
- **Non-Destructive Processing**: Ensure that transformations preserve original document structure and formatting as much as possible.
- **Zero-Copy Parsing**: Use Rust lifetimes to avoid unnecessary allocations during AST generation.
- **Robustness**: Correctly handle complex Markdown edge cases, such as escaped characters, nested formatting, and content within code blocks.
- **Atomic Operations**: Use temporary file swapping to prevent data loss during file updates.
