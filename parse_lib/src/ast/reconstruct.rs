//! Rebuilds Markdown source text from a sequence of AST elements.
//!
//! TODO Reviewed ??-??-2026.

use super::Element;

/// Reconstructs Markdown for a sequence of AST elements using their display implementation.
#[must_use]
pub fn reconstruct_markdown(elements: &[Element<'_>]) -> String {
    let mut result = String::new();
    for element in elements {
        result.push_str(&element.to_string());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    /// An empty element list reconstructs to an empty string.
    #[test]
    fn test_reconstruct_empty() {
        assert_eq!(reconstruct_markdown(&[]), "");
    }

    /// A single text element reconstructs to its own text.
    #[test]
    fn test_reconstruct_text_only() {
        let elements = vec![Element::Text(TextData { content: "hello" })];
        assert_eq!(reconstruct_markdown(&elements), "hello");
    }

    /// A heading, text and an autolink reconstruct in order, with a newline after the heading.
    #[test]
    fn test_reconstruct_multiple_elements() {
        let elements = vec![
            Element::Heading(HeadingData {
                level: 1,
                content: Some("Title"),
                id: None,
            }),
            Element::Text(TextData { content: "Hello " }),
            Element::Autolink(AutolinkData {
                url: "http://example.com",
            }),
        ];
        // Heading implementation in PrettyPrinter uses writeln!, so it adds a newline.
        assert_eq!(
            reconstruct_markdown(&elements),
            "# Title\nHello <http://example.com>"
        );
    }

    /// Unchecked and checked task items reconstruct as `- [ ]` and `- [x]` lines.
    #[test]
    fn test_reconstruct_task_items() {
        let elements = vec![
            Element::TaskItem(TaskItemData {
                completed: false,
                content: "task1",
            }),
            Element::TaskItem(TaskItemData {
                completed: true,
                content: "task2",
            }),
        ];
        // TaskItem implementation in PrettyPrinter uses writeln!, so it adds a newline after each.
        assert_eq!(
            reconstruct_markdown(&elements),
            "- [ ] task1\n- [x] task2\n"
        );
    }

    /// A heading with an explicit id, an inline link with a title and a reference-style link
    /// reconstruct to the original syntax.
    #[test]
    fn test_reconstruct_various_elements() {
        let elements = vec![
            Element::Heading(HeadingData {
                level: 2,
                content: Some("Section"),
                id: Some("sec"),
            }),
            Element::Text(TextData {
                content: "Check this ",
            }),
            Element::InlineLink(InlineLinkData {
                text: "link",
                url: "https://example.com",
                title: Some("title"),
            }),
            Element::Text(TextData { content: " and " }),
            Element::ReferenceStyleLink(ReferenceStyleLinkData {
                text: "this",
                label: "ref",
            }),
            Element::Text(TextData { content: "." }),
        ];
        // Heading with ID: "## Section {#sec}\n"
        assert_eq!(
            reconstruct_markdown(&elements),
            "## Section {#sec}\nCheck this [link](https://example.com \"title\") and [this][ref]."
        );
    }
}
