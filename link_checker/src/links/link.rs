use std::collections::HashSet;

use parse_lib::visit::Visitor;
use parse_lib::*;
use url::Url;

pub fn extract_urls(doc: &Document) -> HashSet<Url> {
    let mut urls = HashSet::new();

    struct UrlCollector<'a> {
        urls: &'a mut HashSet<Url>,
    }

    impl<'v> Visitor<'v> for UrlCollector<'_> {
        fn visit_autolink(&mut self, data: &'v AutolinkData) -> std::fmt::Result {
            if let Ok(url) = Url::parse(data.url) {
                self.urls.insert(url);
            }
            Ok(())
        }
        fn visit_inline_link(&mut self, data: &'v InlineLinkData) -> std::fmt::Result {
            if let Ok(url) = Url::parse(data.url) {
                self.urls.insert(url);
            }
            Ok(())
        }
        fn visit_inline_image(&mut self, data: &'v InlineImageData) -> std::fmt::Result {
            if let Ok(url) = Url::parse(data.url) {
                self.urls.insert(url);
            }
            Ok(())
        }
        fn visit_reference_definition(
            &mut self,
            data: &'v ReferenceDefinitionData,
        ) -> std::fmt::Result {
            if let Ok(url) = Url::parse(data.url) {
                self.urls.insert(url);
            }
            Ok(())
        }
    }

    let mut collector = UrlCollector { urls: &mut urls };
    let _ = doc.accept(&mut collector);
    urls
}
