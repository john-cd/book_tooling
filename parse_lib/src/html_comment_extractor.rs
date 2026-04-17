use crate::ast::*;
use crate::visit::Visitor;

/// A visitor that extracts all HTML comments from a Markdown document.
#[derive(Default)]
pub struct HtmlCommentExtractor {
    pub comments: Vec<String>,
}

impl HtmlCommentExtractor {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'a> Visitor<'a> for HtmlCommentExtractor {
    fn visit_html_comment(&mut self, data: &HtmlCommentData<'a>) {
        self.comments.push(data.lines.join("\n"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_document;

    #[test]
    fn test_html_comment_extraction() {
        let input = "Hello <!-- TODO: fix this section --> world!";
        let doc = parse_document("test.md", None, input).unwrap();

        let mut extractor = HtmlCommentExtractor::new();
        extractor.visit_document(&doc);

        assert_eq!(
            extractor.comments,
            vec![" TODO: fix this section ".to_string()]
        );
    }
}
