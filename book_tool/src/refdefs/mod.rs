//! Reference-definition analysis and cleanup helpers.

use std::collections::HashSet;
use std::fmt::Write;

use parse_lib::visit::PrettyPrinter;
use parse_lib::visit::Visitable;
use parse_lib::visit::Visitor;
use parse_lib::*;

/// Find labels that are defined but never used in links or images.
pub fn find_unused_refdefs(doc: &Document) -> HashSet<String> {
    let mut used_labels = HashSet::new();
    let mut defined_labels = HashSet::new();

    struct LabelCollector<'a> {
        used: &'a mut HashSet<String>,
        defined: &'a mut HashSet<String>,
    }

    impl<'a> Visitor<'a> for LabelCollector<'_> {
        fn visit_reference_style_link(&mut self, link: &'a ReferenceStyleLinkData<'a>) {
            self.used.insert(link.label.to_string());
        }
        fn visit_reference_style_image(&mut self, img: &'a ReferenceStyleImageData<'a>) {
            self.used.insert(img.label.to_string());
        }
        fn visit_reference_definition(&mut self, refdef: &'a ReferenceDefinitionData<'a>) {
            self.defined.insert(refdef.label.to_string());
        }
    }

    let mut collector = LabelCollector {
        used: &mut used_labels,
        defined: &mut defined_labels,
    };
    doc.accept(&mut collector);

    defined_labels.difference(&used_labels).cloned().collect()
}

/// Delete unused reference definitions from the document and return the new content.
pub fn delete_unused_refdefs(doc: &Document, unused_labels: &HashSet<String>) -> String {
    let mut output = String::new();

    struct DeletingPrinter<'a, W: Write> {
        pp: PrettyPrinter<W>,
        unused: &'a HashSet<String>,
    }

    impl<'a, W: Write + 'a> Visitor<'a> for DeletingPrinter<'a, W> {
        fn visit_autolink(&mut self, data: &'a AutolinkData<'a>) {
            self.pp.visit_autolink(data)
        }
        fn visit_inline_link(&mut self, data: &'a InlineLinkData<'a>) {
            self.pp.visit_inline_link(data)
        }
        fn visit_reference_style_link(&mut self, data: &'a ReferenceStyleLinkData<'a>) {
            self.pp.visit_reference_style_link(data)
        }
        fn visit_inline_image(&mut self, data: &'a InlineImageData<'a>) {
            self.pp.visit_inline_image(data)
        }
        fn visit_reference_style_image(&mut self, data: &'a ReferenceStyleImageData<'a>) {
            self.pp.visit_reference_style_image(data)
        }
        fn visit_wikilink(&mut self, data: &'a WikiLinkData<'a>) {
            self.pp.visit_wikilink(data)
        }
        fn visit_code_span(&mut self, data: &'a CodeSpanData<'a>) {
            self.pp.visit_code_span(data)
        }
        fn visit_fenced_code_block(&mut self, data: &'a FencedCodeBlockData<'a>) {
            self.pp.visit_fenced_code_block(data)
        }
        fn visit_hidden_html_div(&mut self, data: &'a HiddenHtmlDivData<'a>) {
            self.pp.visit_hidden_html_div(data)
        }
        fn visit_heading(&mut self, data: &'a HeadingData<'a>) {
            self.pp.visit_heading(data)
        }
        fn visit_text(&mut self, data: &'a TextData<'a>) {
            self.pp.visit_text(data)
        }
        fn visit_custom_directive(&mut self, data: &'a DirectiveData<'a>) {
            self.pp.visit_custom_directive(data)
        }

        fn visit_reference_definition(&mut self, refdef: &'a ReferenceDefinitionData<'a>) {
            if self.unused.contains(refdef.label) {
                // To avoid leaving blank lines, we might need a more sophisticated printer,
                // but for now this removes the definition.
            } else {
                self.pp.visit_reference_definition(refdef)
            }
        }
    }

    let mut printer = DeletingPrinter {
        pp: PrettyPrinter::new(&mut output),
        unused: unused_labels,
    };
    doc.accept(&mut printer);
    output
}

#[cfg(test)]
mod tests {
    use parse_lib::parse_document;

    use super::*;

    #[test]
    fn test_unused_refdefs() {
        let content = "# Title\n\n[Used][used]\n\n[used]: https://example.com\n[unused]: https://unused.com\n";
        let doc = parse_document("test.md", None, content).expect("parse failed");
        let unused = find_unused_refdefs(&doc);
        assert!(
            unused.contains("unused"),
            "Unused labels should contain 'unused', got: {:?}",
            unused
        );
        assert_eq!(unused.len(), 1);
    }

    #[test]
    fn test_delete_unused_refdefs() {
        let content = "# Title\n\n[Used][used]\n\n[used]: https://example.com\n[unused]: https://unused.com\n";
        let doc = parse_document("test.md", None, content).expect("parse failed");
        let unused = find_unused_refdefs(&doc);
        let cleaned = delete_unused_refdefs(&doc, &unused);
        assert!(
            cleaned.contains("[used]: https://example.com"),
            "Cleaned content should contain [used] refdef, got:\n{}",
            cleaned
        );
        assert!(!cleaned.contains("[unused]: https://unused.com"));
    }
}
