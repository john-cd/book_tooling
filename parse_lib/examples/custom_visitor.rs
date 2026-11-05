//! Analyzes a Markdown document with a custom [`Visitor`].
//!
//! The visitor counts headings, collects external link destinations and lists the directives it
//! meets. Run it with `cargo run -p parse_lib --example custom_visitor`.
//!
//! TODO Reviewed ??-??-2026

use parse_lib::ast::DirectiveData;
use parse_lib::ast::HeadingData;
use parse_lib::ast::InlineLinkData;
use parse_lib::visit::Visitor;

/// Statistics gathered while walking one document.
#[derive(Debug, Default)]
struct DocumentSummary<'a> {
    /// Number of headings per level, indexed by `level - 1`.
    headings_by_level: [usize; 6],
    /// Destinations of inline links that start with `http`.
    external_links: Vec<&'a str>,
    /// Rendered form of every custom directive.
    directives: Vec<String>,
}

impl<'a> Visitor<'a> for DocumentSummary<'a> {
    /// Counts the heading under its level.
    fn visit_heading(&mut self, heading: &HeadingData<'a>) {
        if let Some(count) = usize::from(heading.level)
            .checked_sub(1)
            .and_then(|index| self.headings_by_level.get_mut(index))
        {
            *count += 1;
        }
    }

    /// Records external destinations; relative links are ignored.
    fn visit_inline_link(&mut self, link: &InlineLinkData<'a>) {
        if link.url.starts_with("http") {
            self.external_links.push(link.url);
        }
    }

    /// Records the directive in its canonical source form.
    fn visit_custom_directive(&mut self, directive: &DirectiveData<'a>) {
        self.directives.push(directive.to_string());
    }
}

fn main() -> anyhow::Result<()> {
    let source = "\
# Guide

## Setup

See [the book](https://doc.rust-lang.org/book/) or the [local page](setup.md).

{{#crate serde}}

## Usage

{{docs anyhow}} and [crates.io](https://crates.io).
";
    let document = parse_lib::parse_document("guide.md", None, source)?;

    let mut summary = DocumentSummary::default();
    summary.visit_document(&document);

    println!("headings per level: {:?}", summary.headings_by_level);
    println!("external links: {:?}", summary.external_links);
    println!("directives: {:?}", summary.directives);
    Ok(())
}
