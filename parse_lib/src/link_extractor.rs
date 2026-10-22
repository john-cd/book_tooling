// TODO Reviewed ??-??-2026
use std::collections::HashSet;

use crate::ast::{AutolinkData, InlineImageData, InlineLinkData, ReferenceDefinitionData};
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
    fn visit_inline_link(&mut self, data: &InlineLinkData<'a>) {
        self.add_link(data.url);
    }

    fn visit_autolink(&mut self, data: &AutolinkData<'a>) {
        self.add_link(data.url);
    }

    fn visit_inline_image(&mut self, data: &InlineImageData<'a>) {
        self.add_link(data.url);
    }

    fn visit_reference_definition(&mut self, data: &ReferenceDefinitionData<'a>) {
        self.add_link(data.url);
    }
}
