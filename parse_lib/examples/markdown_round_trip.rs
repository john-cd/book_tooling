//! Parses a document in memory, transforms elements while rendering, and reconstructs Markdown.
//!
//! [`MarkdownRenderer`] reproduces untouched text byte for byte and applies only the replacements
//! its resolvers return. Run it with `cargo run -p parse_lib --example markdown_round_trip`.
//!
//! TODO Reviewed ??-??-2026

use parse_lib::ast::DirectiveData;
use parse_lib::markdown_renderer::MarkdownRenderer;
use parse_lib::visit::Visitor as _;

fn main() -> anyhow::Result<()> {
    let source = "\
# Notes

Keep   this   spacing.

{{#example demo}} and [[Some Page | shown]] plus <https://example.com>.
";
    let document = parse_lib::parse_document("notes.md", None, source)?;

    // Without resolvers that change anything, the output equals the input.
    let mut identity = MarkdownRenderer::new(|directive: &DirectiveData<'_>| directive.to_string());
    identity.visit_document(&document);
    assert_eq!(
        identity.markdown, source,
        "an identity render must reproduce the source exactly"
    );

    // Directives and wikilinks are rewritten; everything else is copied unchanged.
    let mut renderer = MarkdownRenderer::new(|directive: &DirectiveData<'_>| match directive {
        DirectiveData::ExampleBlock { name } => format!("{{{{#include examples/{name}.md}}}}"),
        DirectiveData::Link { .. }
        | DirectiveData::Badge { .. }
        | DirectiveData::CrateBlock { .. }
        | DirectiveData::BadgeSet { .. }
        | DirectiveData::CategoryBlock { .. } => directive.to_string(),
    })
    .with_wikilink_resolver(|link| {
        let label = link.display.unwrap_or(link.target);
        format!("[{label}](pages/{}.md)", link.target.replace(' ', "-"))
    });
    renderer.visit_document(&document);

    println!("{}", renderer.markdown);
    Ok(())
}
