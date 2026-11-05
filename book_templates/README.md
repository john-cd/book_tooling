<!-- TODO Reviewed ??-??-2026 -->

# `book_templates`

Markdown badge and link templates shared by `book_tool` and `rust_rate`.

```rust
use book_templates::{DestinationKind, render_badge, render_link};

assert_eq!(
    render_link(DestinationKind::Docs, "anyhow"),
    "[anyhow][p~anyhow~docs]\u{2197}{{hi:anyhow}}"
);
let badge = render_badge(DestinationKind::CratesIo, "anyhow");
assert!(badge.starts_with("[![anyhow]"));
```

Destinations: `Category`, `Crate`, `Docs`, `GithubRepo`, `LibRs`, `CratesIo`, `Web`. Output is
checked against the shared fixture `test_fixtures/template_output.tsv`.

Run the example with `cargo run -p book_templates --example render`.
