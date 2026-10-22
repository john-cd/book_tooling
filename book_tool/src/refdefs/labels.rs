//! Label-based analysis of a single Markdown document.
//!
//! TODO Reviewed ??-??-2026

use std::collections::HashMap;
use std::collections::HashSet;

use parse_lib::Document;
use parse_lib::ReferenceDefinitionData;
use parse_lib::ReferenceStyleImageData;
use parse_lib::ReferenceStyleLinkData;
use parse_lib::visit::Visitable as _;
use parse_lib::visit::Visitor;

use super::normalize_reference_label;

/// Labels used by references and labels defined by definitions in one document.
#[derive(Default)]
struct LabelSets {
    /// Normalized labels that at least one reference link or image uses.
    used: HashSet<String>,
    /// Defined labels as `(label, normalized_label)` pairs.
    definitions: Vec<(String, String)>,
}

impl<'a> Visitor<'a> for LabelSets {
    fn visit_reference_style_link(&mut self, link: &'a ReferenceStyleLinkData<'a>) {
        self.used.insert(normalize_reference_label(link.label));
    }

    fn visit_reference_style_image(&mut self, image: &'a ReferenceStyleImageData<'a>) {
        self.used.insert(normalize_reference_label(image.label));
    }

    fn visit_reference_definition(&mut self, definition: &'a ReferenceDefinitionData<'a>) {
        self.definitions.push((
            definition.label.to_owned(),
            normalize_reference_label(definition.label),
        ));
    }
}

/// Find labels that are defined but never used in links or images.
#[must_use]
pub fn find_unused_refdefs(doc: &Document<'_>) -> HashSet<String> {
    let mut defined_labels = HashMap::<String, (String, usize)>::new();

    let mut labels = LabelSets::default();
    doc.accept(&mut labels);
    let used_labels = labels.used;
    for (label, normalized) in labels.definitions {
        let entry = defined_labels
            .entry(normalized)
            .or_insert_with(|| (label.clone(), 0));
        entry.1 += 1;
    }

    defined_labels
        .into_iter()
        .filter_map(|(normalized, (label, count))| {
            (count == 1 && !used_labels.contains(&normalized)).then_some(label)
        })
        .collect()
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
            "Unused labels should contain 'unused', got: {unused:?}"
        );
        assert_eq!(unused.len(), 1);
    }

    #[test]
    fn labels_match_case_insensitively_and_collapse_whitespace() {
        let content = "[Full][SeRdE] [serde][] [SERDE] ![crate][ SErDe ]\n[  seRde   ]: https://example.com\n";
        let doc = parse_document("test.md", None, content).expect("parse label variants");

        assert!(find_unused_refdefs(&doc).is_empty());
    }

    #[test]
    fn duplicate_labels_are_preserved_as_ambiguous() {
        let content = "[Use][x]\n[x]: https://one.example\n[X]: https://two.example\n";
        let doc = parse_document("test.md", None, content).expect("parse duplicate definitions");

        assert!(find_unused_refdefs(&doc).is_empty());
    }
}
