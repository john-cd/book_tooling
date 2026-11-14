//! A visitor that collects the link destinations of a document.
//!
//! TODO Reviewed ??-??-2026.

use std::collections::HashSet;

use crate::ast::AutolinkData;
use crate::ast::InlineImageData;
use crate::ast::InlineLinkData;
use crate::ast::ReferenceDefinitionData;
use crate::visit::Visitor;

/// A visitor that extracts all unique external URLs from a Markdown document.
#[derive(Default)]
pub struct LinkExtractor<'a> {
    /// Unique external HTTP and HTTPS links discovered during traversal.
    pub external_links: HashSet<&'a str>,
}

impl<'a> LinkExtractor<'a> {
    /// Creates an empty link extractor.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Helper to store only external HTTP/HTTPS links.
    fn add_link(&mut self, url: &'a str) {
        if url.starts_with("http://") || url.starts_with("https://") {
            self.external_links.insert(url);
        }
    }
}

impl<'a> Visitor<'a> for LinkExtractor<'a> {
    /// Records the inline link's destination text.
    fn visit_inline_link(&mut self, data: &InlineLinkData<'a>) {
        self.add_link(data.url);
    }

    /// Records the autolink's URL text.
    fn visit_autolink(&mut self, data: &AutolinkData<'a>) {
        self.add_link(data.url);
    }

    /// Records the inline image's source text.
    fn visit_inline_image(&mut self, data: &InlineImageData<'a>) {
        self.add_link(data.url);
    }

    /// Collects the destination of a reference definition.
    fn visit_reference_definition(&mut self, data: &ReferenceDefinitionData<'a>) {
        self.add_link(data.url);
    }
}

/// Tests for collecting destination text across link and image variants.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_document;
    use crate::visit::Visitable as _;

    /// Parses fixture Markdown and returns the destinations collected by the link visitor.
    ///
    /// # Panics
    /// Panics if parsing the fixture fails.
    fn extract(source: &str) -> Vec<String> {
        let doc = parse_document("t", None, source).expect("parses");
        let mut extractor = LinkExtractor::new();
        doc.accept(&mut extractor);
        let mut links: Vec<_> = extractor
            .external_links
            .iter()
            .map(|link| (*link).to_owned())
            .collect();
        links.sort();
        links
    }

    /// Verifies: collects autolinks inline links images and definitions.
    #[test]
    fn collects_autolinks_inline_links_images_and_definitions() {
        let links = extract(concat!(
            "<https://auto.example>\n\n",
            "[inline](http://inline.example) ![img](https://img.example/x.png)\n\n",
            "[def]: https://def.example\n"
        ));
        assert_eq!(
            links,
            [
                "http://inline.example",
                "https://auto.example",
                "https://def.example",
                "https://img.example/x.png",
            ]
        );
    }

    /// Verifies: collapsed and shortcut references carry no destination.
    #[test]
    fn collapsed_and_shortcut_references_carry_no_destination() {
        assert!(extract("[text][label] [collapsed][] [shortcut]\n").is_empty());
    }

    /// Verifies: relative and non http destinations are ignored.
    #[test]
    fn relative_and_non_http_destinations_are_ignored() {
        assert!(
            extract("[a](page.md) [b](mailto:x@y.z) [c](ftp://h/f) <ftp://host/file>\n").is_empty()
        );
    }

    /// Verifies: duplicates are collapsed.
    #[test]
    fn duplicates_are_collapsed() {
        assert_eq!(
            extract("[a](https://x.example) [b](https://x.example)\n"),
            ["https://x.example"]
        );
    }

    /// Verifies: new extractor is empty.
    #[test]
    fn new_extractor_is_empty() {
        assert!(LinkExtractor::new().external_links.is_empty());
    }
}
