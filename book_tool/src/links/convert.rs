// TODO Reviewed ??-??-2026
//! Inline-link conversion utilities shared by CLI workflows and reusable callers.

use std::cell::Cell;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;

use parse_lib::ast::AutolinkData;
use parse_lib::ast::InlineLinkData;
use parse_lib::ast::ReferenceDefinitionData;
use parse_lib::markdown_renderer::MarkdownRenderer;
use parse_lib::visit::Visitor;

const ISSUE_LINK_PASSTHROUGH: &str = "github.com/john-cd/rust_howto/issues";

/// Summary statistics produced while converting inline links to reference links.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct InlineLinkConversionStats {
    /// Number of reference definitions already present in the source document.
    pub existing_reference_definitions: usize,
    /// Number of link occurrences that reused an existing reference definition.
    pub reused_reference_definitions: usize,
    /// Number of new reference definitions appended to the rendered Markdown.
    pub appended_reference_definitions: usize,
    /// Number of issue links intentionally preserved as inline links or autolinks.
    pub preserved_issue_links: usize,
}

/// Result of converting inline links and autolinks into reference-style links.
#[derive(Debug, PartialEq, Eq)]
pub struct InlineLinkConversion {
    /// Rendered Markdown after conversion.
    pub markdown: String,
    /// Aggregate statistics describing how the conversion behaved.
    pub stats: InlineLinkConversionStats,
}

/// Converts inline links and autolinks into reference-style links for a parsed document.
///
/// Existing reference definitions are reused when possible. New definitions are appended to
/// the end of the rendered Markdown in ascending numeric-label order.
pub fn convert_inline_links_to_reference_definitions(
    doc: &parse_lib::Document,
) -> InlineLinkConversion {
    let registry = std::rc::Rc::new(LabelRegistry::from_doc(doc));

    tracing::debug!(
        existing_reference_definitions = registry.existing_reference_count(),
        "Collected existing reference definitions for inline-link conversion"
    );

    let mut renderer = MarkdownRenderer::new(|directive| directive.to_string());

    let inline_registry = std::rc::Rc::clone(&registry);
    renderer =
        renderer.with_inline_link_resolver(move |link| inline_registry.render_inline_link(link));

    let autolink_registry = std::rc::Rc::clone(&registry);
    renderer = renderer.with_autolink_resolver(move |link| autolink_registry.render_autolink(link));

    renderer.visit_document(doc);
    let mut markdown = std::mem::take(&mut renderer.markdown);
    drop(renderer);

    let stats = registry.finish(&mut markdown);

    tracing::debug!(
        existing_reference_definitions = stats.existing_reference_definitions,
        reused_reference_definitions = stats.reused_reference_definitions,
        appended_reference_definitions = stats.appended_reference_definitions,
        preserved_issue_links = stats.preserved_issue_links,
        "Completed inline-link conversion"
    );

    InlineLinkConversion { markdown, stats }
}

struct ExistingRefDefCollector<'a> {
    existing_url_to_label: &'a mut HashMap<String, String>,
    existing_labels: &'a mut HashSet<String>,
}

impl<'a, 'ast> Visitor<'ast> for ExistingRefDefCollector<'a> {
    fn visit_reference_definition(&mut self, data: &'ast ReferenceDefinitionData<'ast>) {
        self.existing_url_to_label
            .insert(data.url.to_string(), data.label.to_string());
        self.existing_labels.insert(data.label.to_string());
    }
}

struct LabelRegistry {
    existing_url_to_label: HashMap<String, String>,
    labels_in_use: RefCell<HashSet<String>>,
    new_url_to_label: RefCell<BTreeMap<String, String>>,
    next_label: Cell<usize>,
    reused_reference_definitions: Cell<usize>,
    preserved_issue_links: Cell<usize>,
}

impl LabelRegistry {
    fn from_doc(doc: &parse_lib::Document) -> Self {
        let mut existing_url_to_label = HashMap::new();
        let mut existing_labels = HashSet::new();

        let mut collector = ExistingRefDefCollector {
            existing_url_to_label: &mut existing_url_to_label,
            existing_labels: &mut existing_labels,
        };
        collector.visit_document(doc);

        Self {
            existing_url_to_label,
            labels_in_use: RefCell::new(existing_labels),
            new_url_to_label: RefCell::new(BTreeMap::new()),
            next_label: Cell::new(1),
            reused_reference_definitions: Cell::new(0),
            preserved_issue_links: Cell::new(0),
        }
    }

    fn existing_reference_count(&self) -> usize {
        self.existing_url_to_label.len()
    }

    fn render_inline_link(&self, link: &InlineLinkData<'_>) -> String {
        if should_preserve_issue_link(link.url) {
            self.preserved_issue_links
                .set(self.preserved_issue_links.get() + 1);
            tracing::trace!(url = link.url, "Preserving issue link as inline link");
            return format!("[{}]({})", link.text, link.url);
        }

        let label = self.label_for_url(link.url);
        format!("[{}][{}]", link.text, label)
    }

    fn render_autolink(&self, link: &AutolinkData<'_>) -> String {
        if should_preserve_issue_link(link.url) {
            self.preserved_issue_links
                .set(self.preserved_issue_links.get() + 1);
            tracing::trace!(url = link.url, "Preserving issue link as autolink");
            return format!("<{}>", link.url);
        }

        let label = self.label_for_url(link.url);
        format!("[{}][{}]", link.url, label)
    }

    fn label_for_url(&self, url: &str) -> String {
        if let Some(label) = self.existing_url_to_label.get(url) {
            self.reused_reference_definitions
                .set(self.reused_reference_definitions.get() + 1);
            tracing::trace!(url, label, "Reusing existing reference definition");
            return label.clone();
        }

        if let Some(label) = self.new_url_to_label.borrow().get(url).cloned() {
            tracing::trace!(
                url,
                label,
                "Reusing new reference definition allocated earlier"
            );
            return label;
        }

        let label = self.allocate_label();
        tracing::trace!(url, label, "Allocated new reference definition label");
        self.new_url_to_label
            .borrow_mut()
            .insert(url.to_string(), label.clone());
        label
    }

    fn allocate_label(&self) -> String {
        loop {
            let candidate = self.next_label.get();
            self.next_label.set(candidate + 1);
            let candidate_label = candidate.to_string();

            if self
                .labels_in_use
                .borrow_mut()
                .insert(candidate_label.clone())
            {
                return candidate_label;
            }
        }
    }

    fn finish(&self, markdown: &mut String) -> InlineLinkConversionStats {
        let mut stats = InlineLinkConversionStats {
            existing_reference_definitions: self.existing_url_to_label.len(),
            reused_reference_definitions: self.reused_reference_definitions.get(),
            appended_reference_definitions: 0,
            preserved_issue_links: self.preserved_issue_links.get(),
        };

        let new_url_to_label = self.new_url_to_label.borrow();
        if !new_url_to_label.is_empty() {
            markdown.push('\n');

            let mut sorted_refdefs: Vec<_> = new_url_to_label
                .iter()
                .map(|(url, label)| (label.parse::<usize>().unwrap_or(0), label, url))
                .collect();
            sorted_refdefs.sort_by_key(|entry| entry.0);
            stats.appended_reference_definitions = sorted_refdefs.len();

            for (_, label, url) in sorted_refdefs {
                markdown.push_str(&format!("[{}]: {}\n", label, url));
            }
        }

        stats
    }
}

fn should_preserve_issue_link(url: &str) -> bool {
    url.contains(ISSUE_LINK_PASSTHROUGH)
}

#[cfg(test)]
mod tests {
    use parse_lib::parse_document;

    use super::*;

    #[test]
    fn conversion_reuses_existing_reference_definitions() {
        let input = "[Rust](https://www.rust-lang.org/)\n\n[docs]: https://www.rust-lang.org/\n";
        let doc = parse_document("test.md", None, input).expect("parse markdown");

        let conversion = convert_inline_links_to_reference_definitions(&doc);

        assert!(conversion.markdown.contains("[Rust][docs]"));
        assert!(
            !conversion
                .markdown
                .contains("[1]: https://www.rust-lang.org/")
        );
        assert_eq!(conversion.stats.existing_reference_definitions, 1);
        assert_eq!(conversion.stats.reused_reference_definitions, 1);
        assert_eq!(conversion.stats.appended_reference_definitions, 0);
    }

    #[test]
    fn conversion_appends_new_reference_definitions_after_existing_numeric_labels() {
        let input = "[Rust](https://www.rust-lang.org/)\n\n[1]: https://example.com/already-used\n";
        let doc = parse_document("test.md", None, input).expect("parse markdown");

        let conversion = convert_inline_links_to_reference_definitions(&doc);

        assert!(conversion.markdown.contains("[Rust][2]"));
        assert!(
            conversion
                .markdown
                .contains("[2]: https://www.rust-lang.org/")
        );
        assert_eq!(conversion.stats.appended_reference_definitions, 1);
    }

    #[test]
    fn conversion_preserves_configured_issue_links() {
        let input = "[Issue](https://github.com/john-cd/rust_howto/issues/42)\n";
        let doc = parse_document("test.md", None, input).expect("parse markdown");

        let conversion = convert_inline_links_to_reference_definitions(&doc);

        assert!(conversion.markdown.contains(input.trim_end()));
        assert_eq!(conversion.stats.preserved_issue_links, 1);
        assert_eq!(conversion.stats.appended_reference_definitions, 0);
    }
}
