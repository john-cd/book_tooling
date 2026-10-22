//! A visitor that extracts the plain text of a document.
//!
//! TODO Reviewed ??-??-2026

use crate::ast::{
    AutolinkData, CodeSpanData, FencedCodeBlockData, HeadingData, HtmlCommentData, InlineImageData,
    InlineLinkData, ReferenceStyleImageData, ReferenceStyleLinkData, TaskItemData, TextData,
    WikiLinkData,
};
use crate::visit::Visitor;

/// A visitor that extracts pure plain text by stripping away all Markdown structural formatting.
#[derive(Default)]
pub struct PlainTextExtractor {
    /// Accumulated plain-text output.
    pub text: String,
}

impl PlainTextExtractor {
    /// Creates an empty plain-text extractor.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'a> Visitor<'a> for PlainTextExtractor {
    fn visit_text(&mut self, data: &TextData<'a>) {
        self.text.push_str(data.content);
    }

    fn visit_code_span(&mut self, data: &CodeSpanData<'a>) {
        self.text.push_str(data.content);
    }

    fn visit_fenced_code_block(&mut self, data: &FencedCodeBlockData<'a>) {
        for line in &data.lines {
            self.text.push_str(line);
            self.text.push('\n');
        }
    }

    fn visit_inline_link(&mut self, data: &InlineLinkData<'a>) {
        self.text.push_str(data.text);
    }

    fn visit_reference_style_link(&mut self, data: &ReferenceStyleLinkData<'a>) {
        self.text.push_str(data.text);
    }

    fn visit_autolink(&mut self, data: &AutolinkData<'a>) {
        self.text.push_str(data.url);
    }

    fn visit_wikilink(&mut self, data: &WikiLinkData<'a>) {
        self.text.push_str(data.display.unwrap_or(data.target));
        if let Some(after) = data.immediately_after {
            self.text.push_str(after);
        }
    }

    fn visit_inline_image(&mut self, data: &InlineImageData<'a>) {
        self.text.push_str(data.image_description);
    }

    fn visit_reference_style_image(&mut self, data: &ReferenceStyleImageData<'a>) {
        self.text.push_str(data.image_description);
    }

    fn visit_heading(&mut self, data: &HeadingData<'a>) {
        if let Some(content) = data.content {
            self.text.push_str(content);
        }
        self.text.push('\n');
    }

    fn visit_task_item(&mut self, data: &TaskItemData<'a>) {
        self.text.push_str(data.content);
        self.text.push('\n');
    }

    fn visit_html_comment(&mut self, _data: &HtmlCommentData<'a>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_document;

    /// Covers plain text extraction.
    #[test]
    fn test_plain_text_extraction() {
        let input = "Hello world! Check out [my link](https://example.com).";
        let doc = parse_document("test.md", None, input).expect("input should parse");

        let mut extractor = PlainTextExtractor::new();
        extractor.visit_document(&doc);

        assert_eq!(extractor.text.trim(), "Hello world! Check out my link.");
    }
}
