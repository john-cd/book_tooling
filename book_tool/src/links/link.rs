//! AST-based URL extraction helpers.
//!
//! TODO Reviewed ??-??-2026

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
    /// Collects the URL of an autolink if it parses as an absolute URL.
    fn visit_autolink(&mut self, data: &'v AutolinkData<'v>) {
        if let Ok(url) = Url::parse(data.url) {
            self.urls.insert(url);
        }
    }

    /// Collects the destination of an inline link if it parses as an absolute URL.
    fn visit_inline_link(&mut self, data: &'v InlineLinkData<'v>) {
        if let Ok(url) = Url::parse(data.url) {
            self.urls.insert(url);
        }
    }

    /// Collects the source of an inline image if it parses as an absolute URL.
    fn visit_inline_image(&mut self, data: &'v InlineImageData<'v>) {
        if let Ok(url) = Url::parse(data.url) {
            self.urls.insert(url);
        }
    }

    /// Collects the destination of a reference definition if it parses as an absolute URL.
    fn visit_reference_definition(&mut self, data: &'v ReferenceDefinitionData<'v>) {
        if let Ok(url) = Url::parse(data.url) {
            self.urls.insert(url);
        }
    }
}

/// Extracts every URL-like destination represented in a parsed Markdown document.
///
/// This includes autolinks, inline links, inline images, and reference definitions.
#[must_use]
pub fn extract_urls(doc: &Document<'_>) -> HashSet<Url> {
    let mut urls = HashSet::new();

    let mut collector = UrlCollector { urls: &mut urls };
    doc.accept(&mut collector);
    urls
}
