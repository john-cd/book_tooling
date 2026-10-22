//! AST-based URL extraction helpers.
//!
//! TODO Reviewed ??-??-2026

use std::collections::HashSet;

use parse_lib::visit::Visitable;
use parse_lib::visit::Visitor;
use parse_lib::*;
use url::Url;

/// Extracts every URL-like destination represented in a parsed Markdown document.
///
/// This includes autolinks, inline links, inline images, and reference definitions.
pub fn extract_urls(doc: &Document) -> HashSet<Url> {
    let mut urls = HashSet::new();

    struct UrlCollector<'a> {
        urls: &'a mut HashSet<Url>,
    }

    impl<'v> Visitor<'v> for UrlCollector<'_> {
        fn visit_autolink(&mut self, data: &'v AutolinkData<'v>) {
            if let Ok(url) = Url::parse(data.url) {
                self.urls.insert(url);
            }
        }
        fn visit_inline_link(&mut self, data: &'v InlineLinkData<'v>) {
            if let Ok(url) = Url::parse(data.url) {
                self.urls.insert(url);
            }
        }
        fn visit_inline_image(&mut self, data: &'v InlineImageData<'v>) {
            if let Ok(url) = Url::parse(data.url) {
                self.urls.insert(url);
            }
        }
        fn visit_reference_definition(&mut self, data: &'v ReferenceDefinitionData<'v>) {
            if let Ok(url) = Url::parse(data.url) {
                self.urls.insert(url);
            }
        }
    }

    let mut collector = UrlCollector { urls: &mut urls };
    doc.accept(&mut collector);
    urls
}
