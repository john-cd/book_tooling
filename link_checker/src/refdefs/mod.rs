use std::collections::HashSet;
use std::borrow::Cow;
use parse_lib::{Element, parse_document, reconstruct_markdown};

/// Detect unused reference definitions in a document.
pub fn detect_unused_reference_definitions(content: &str) -> HashSet<String> {
    let mut defined = HashSet::new();
    let mut used = HashSet::new();

    if let Ok(doc) = parse_document("content", None, content) {
        for element in doc.elements {
            match element {
                Element::ReferenceDefinition(data) => {
                    defined.insert(data.label.to_string());
                }
                Element::ReferenceStyleLink(data) => {
                    used.insert(data.label.to_string());
                }
                Element::ReferenceStyleImage(data) => {
                    used.insert(data.label.to_string());
                }
                _ => {}
            }
        }
    }

    defined.difference(&used).cloned().collect()
}

/// Delete unused reference definitions.
pub fn delete_unused_reference_definitions(content: &str) -> Cow<'_, str> {
    let unused = detect_unused_reference_definitions(content);
    if unused.is_empty() {
        return Cow::Borrowed(content);
    }

    if let Ok(doc) = parse_document("content", None, content) {
        let mut new_elements = Vec::new();
        for element in doc.elements {
            if let Element::ReferenceDefinition(data) = &element {
                if unused.contains(data.label) {
                    continue;
                }
            }
            new_elements.push(element);
        }
        return Cow::Owned(reconstruct_markdown(&new_elements));
    }

    Cow::Borrowed(content)
}
