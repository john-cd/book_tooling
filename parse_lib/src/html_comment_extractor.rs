//! A visitor that collects the HTML comments of a document.
//!
//! TODO Reviewed ??-??-2026

use crate::ast::HtmlCommentData;
use crate::visit::Visitor;

/// A visitor that extracts all HTML comments from a Markdown document.
#[derive(Default)]
pub struct HtmlCommentExtractor {
    /// Collected HTML comment bodies in encounter order.
    pub comments: Vec<String>,
}

impl HtmlCommentExtractor {
    /// Creates an empty HTML comment extractor.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'a> Visitor<'a> for HtmlCommentExtractor {
    /// Stores the comment text, with its lines rejoined by newlines.
    fn visit_html_comment(&mut self, data: &HtmlCommentData<'a>) {
        self.comments.push(data.lines.join("\n"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_document;

    /// A comment in the middle of a paragraph is extracted with its inner spacing, without the delimiters.
    #[test]
    fn test_html_comment_extraction() {
        let input = "Hello <!-- TODO: fix this section --> world!";
        let doc = parse_document("test.md", None, input).expect("input should parse");

        let mut extractor = HtmlCommentExtractor::new();
        extractor.visit_document(&doc);

        assert_eq!(
            extractor.comments,
            vec![" TODO: fix this section ".to_owned()]
        );
    }
}
