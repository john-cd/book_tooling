//! AST-based URL extraction helpers.
//!
//! TODO Reviewed ??-??-2026.

use std::collections::HashSet;

use parse_lib::AutolinkData;
use parse_lib::Document;
use parse_lib::InlineImageData;
use parse_lib::InlineLinkData;
use parse_lib::ReferenceDefinitionData;
use parse_lib::visit::Visitable as _;
use parse_lib::visit::Visitor;
use url::Url;

/// Visitor that records every URL-like destination it sees.
struct UrlCollector<'a> {
    /// Receives each parsed URL.
    urls: &'a mut HashSet<Url>,
}

impl<'v> Visitor<'v> for UrlCollector<'_> {
    /// Adds the autolink URL to the collector if it is absolute and parses successfully.
    fn visit_autolink(&mut self, data: &'v AutolinkData<'v>) {
        if let Ok(url) = Url::parse(data.url) {
            self.urls.insert(url);
        }
    }

    /// Adds the inline link destination to the collector if it is absolute and parses successfully.
    fn visit_inline_link(&mut self, data: &'v InlineLinkData<'v>) {
        if let Ok(url) = Url::parse(data.url) {
            self.urls.insert(url);
        }
    }

    /// Adds the image source to the collector if it is absolute and parses successfully.
    fn visit_inline_image(&mut self, data: &'v InlineImageData<'v>) {
        if let Ok(url) = Url::parse(data.url) {
            self.urls.insert(url);
        }
    }

    /// Adds the reference definition target to the collector if it is absolute and parses
    /// successfully.
    fn visit_reference_definition(&mut self, data: &'v ReferenceDefinitionData<'v>) {
        if let Ok(url) = Url::parse(data.url) {
            self.urls.insert(url);
        }
    }
}

/// Returns distinct absolute URLs from the document's autolinks, inline links, inline images, and
/// reference definitions. Each destination is normalized by the URL parser.
#[must_use]
pub fn extract_urls(doc: &Document<'_>) -> HashSet<Url> {
    let mut urls = HashSet::new();

    let mut collector = UrlCollector { urls: &mut urls };
    doc.accept(&mut collector);
    urls
}

/// Tests for URL extraction and normalization.
#[cfg(test)]
mod tests {
    use super::*;

    /// Parses fixture Markdown and returns its distinct absolute URLs as sorted strings.
    ///
    /// # Panics
    /// Panics if the fixture Markdown cannot be parsed.
    fn urls(source: &str) -> Vec<String> {
        let doc = parse_lib::parse_document("t", None, source).expect("parses");
        let mut found: Vec<_> = extract_urls(&doc).into_iter().map(String::from).collect();
        found.sort();
        found
    }

    /// Checks that all four destination-bearing node types contribute absolute URLs.
    #[test]
    fn extracts_autolinks_inline_links_images_and_definitions() {
        let found = urls(concat!(
            "<https://auto.example/a>\n\n",
            "[inline](https://inline.example/b) ![img](https://img.example/c.png)\n\n",
            "[ref]: https://ref.example/d\n"
        ));
        assert_eq!(
            found,
            [
                "https://auto.example/a",
                "https://img.example/c.png",
                "https://inline.example/b",
                "https://ref.example/d",
            ]
        );
    }

    /// Checks that relative paths and malformed URLs are skipped.
    #[test]
    fn relative_and_unparseable_destinations_are_skipped() {
        assert!(urls("[rel](../page.md) [anchor](#top) [file](chapter.md)").is_empty());
    }

    /// Checks that equivalent destinations collapse after URL normalization.
    #[test]
    fn duplicates_collapse_and_urls_are_normalized() {
        let found = urls("[a](https://EXAMPLE.com) [b](https://example.com/)\n");
        assert_eq!(found, ["https://example.com/"]);
    }

    /// Checks that URL extraction includes other absolute schemes.
    #[test]
    fn non_http_schemes_are_still_extracted() {
        assert_eq!(
            urls("[mail](mailto:a@example.com)"),
            ["mailto:a@example.com"]
        );
    }
}
