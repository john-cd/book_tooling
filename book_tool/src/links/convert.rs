//! Inline-link conversion utilities shared by CLI workflows and reusable callers.
//!
//! TODO Reviewed ??-??-2026.

use std::cell::Cell;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Write as _;

use parse_lib::ast::AutolinkData;
use parse_lib::ast::InlineLinkData;
use parse_lib::ast::ReferenceDefinitionData;
use parse_lib::markdown_renderer::MarkdownRenderer;
use parse_lib::visit::Visitor;

use crate::refdefs::normalize_reference_label;

/// URL fragment that marks links to Rust How-To issues, which stay inline.
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

/// Existing reference definitions are reused when possible. Titled inline links remain unchanged
/// because converting them without a title-aware definition could drop their hover text. New
/// definitions are appended to the end of the rendered Markdown in ascending numeric-label order.
pub fn convert_inline_links_to_reference_definitions(
    doc: &parse_lib::Document<'_>,
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

/// Visitor that gathers the reference definitions already present in a document.
struct ExistingRefDefCollector<'a> {
    /// Receives each existing definition URL with its label.
    existing_url_to_label: &'a mut HashMap<String, String>,
    /// Receives the labels of the existing definitions.
    existing_labels: &'a mut HashSet<String>,
}

impl<'ast> Visitor<'ast> for ExistingRefDefCollector<'_> {
    /// Records an existing reference definition so conversion can reuse its label.
    fn visit_reference_definition(&mut self, data: &'ast ReferenceDefinitionData<'ast>) {
        if self.existing_labels.insert(normalize_reference_label(data.label)) {
            self.existing_url_to_label
                .entry(data.url.to_owned())
                .or_insert_with(|| data.label.to_owned());
        }
    }
}

/// Tracks reference labels so inline links can be rewritten to reuse or allocate numeric labels.
struct LabelRegistry {
    /// URL-to-label mapping of the definitions already in the document.
    existing_url_to_label: HashMap<String, String>,
    /// Every label that exists or has been allocated.
    labels_in_use: RefCell<HashSet<String>>,
    /// Definitions allocated during this conversion, ordered by URL.
    new_url_to_label: RefCell<BTreeMap<String, String>>,
    /// Next numeric label to try.
    next_label: Cell<usize>,
    /// Number of times an existing definition was reused.
    reused_reference_definitions: Cell<usize>,
    /// Number of issue links left inline.
    preserved_issue_links: Cell<usize>,
}

impl LabelRegistry {
    /// Initializes conversion with the document's existing URL-to-label mappings and the set of
    /// occupied labels.
    fn from_doc(doc: &parse_lib::Document<'_>) -> Self {
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

    /// Returns the number of distinct URLs already mapped to reference labels.
    fn existing_reference_count(&self) -> usize {
        self.existing_url_to_label.len()
    }

    /// Renders an inline link as a reference-style link; issue links and titled links stay inline.
    fn render_inline_link(&self, link: &InlineLinkData<'_>) -> String {
        if should_preserve_issue_link(link.url) {
            self.preserved_issue_links
                .set(self.preserved_issue_links.get() + 1);
            tracing::trace!(url = link.url, "Preserving issue link as inline link");
            return format!("[{}]({})", link.text, link.url);
        }

        if link.title.is_some() {
            tracing::trace!(url = link.url, "Preserving titled inline link");
            return format!("[{}]({})", link.text, link.url);
        }

        let label = self.label_for_url(link.url);
        format!("[{}][{}]", link.text, label)
    }

    /// Renders an autolink through a reference label, preserving issue links in angle brackets.
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

    /// Returns the existing label for `url` when available. Otherwise, allocates a fresh label and
    /// saves the new definition.
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
            .insert(url.to_owned(), label.clone());
        label
    }

    /// Returns the next available numeric label as a decimal string and advances the allocation
    /// cursor.
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

    /// Appends the newly allocated definitions to `markdown` and returns the conversion statistics.
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
                let _unused = writeln!(markdown, "[{label}]: {url}");
            }
        }

        stats
    }
}

/// Returns whether `url` points to a Rust How-To issue, whose links stay inline.
fn should_preserve_issue_link(url: &str) -> bool {
    url.contains(ISSUE_LINK_PASSTHROUGH)
}

/// Tests for reference-label allocation and link conversion.
#[cfg(test)]
mod tests {
    use parse_lib::parse_document;

    use super::*;

    /// A link whose URL already has a definition reuses that label.
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

    /// New labels continue after the highest existing numeric label.
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

    /// Issue links stay inline when configured to be preserved.
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
