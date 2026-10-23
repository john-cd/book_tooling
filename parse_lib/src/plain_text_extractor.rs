//! A visitor that extracts the plain text of a document.
//!
//! TODO Reviewed ??-??-2026

use crate::ast::AutolinkData;
use crate::ast::CodeSpanData;
use crate::ast::FencedCodeBlockData;
use crate::ast::HeadingData;
use crate::ast::HtmlCommentData;
use crate::ast::InlineImageData;
use crate::ast::InlineLinkData;
use crate::ast::ReferenceStyleImageData;
use crate::ast::ReferenceStyleLinkData;
use crate::ast::TaskItemData;
use crate::ast::TextData;
use crate::ast::WikiLinkData;
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
    /// Appends the text unchanged.
    fn visit_text(&mut self, data: &TextData<'a>) {
        self.text.push_str(data.content);
    }

    /// Appends the code span content without the backticks.
    fn visit_code_span(&mut self, data: &CodeSpanData<'a>) {
        self.text.push_str(data.content);
    }

    /// Appends each code line followed by a newline.
    fn visit_fenced_code_block(&mut self, data: &FencedCodeBlockData<'a>) {
        for line in &data.lines {
            self.text.push_str(line);
            self.text.push('\n');
        }
    }

    /// Appends the link text and drops the URL.
    fn visit_inline_link(&mut self, data: &InlineLinkData<'a>) {
        self.text.push_str(data.text);
    }

    /// Appends the link text and drops the label.
    fn visit_reference_style_link(&mut self, data: &ReferenceStyleLinkData<'a>) {
        self.text.push_str(data.text);
    }

    /// Appends the URL itself, since an autolink has no separate text.
    fn visit_autolink(&mut self, data: &AutolinkData<'a>) {
        self.text.push_str(data.url);
    }

    /// Appends the display text (or the target when there is none) and any text attached directly
    /// after the link.
    fn visit_wikilink(&mut self, data: &WikiLinkData<'a>) {
        self.text.push_str(data.display.unwrap_or(data.target));
        if let Some(after) = data.immediately_after {
            self.text.push_str(after);
        }
    }

    /// Appends the image description and drops the URL.
    fn visit_inline_image(&mut self, data: &InlineImageData<'a>) {
        self.text.push_str(data.image_description);
    }

    /// Appends the image description and drops the label.
    fn visit_reference_style_image(&mut self, data: &ReferenceStyleImageData<'a>) {
        self.text.push_str(data.image_description);
    }

    /// Appends the heading text without the `#` markers.
    fn visit_heading(&mut self, data: &HeadingData<'a>) {
        if let Some(content) = data.content {
            self.text.push_str(content);
        }
        self.text.push('\n');
    }

    /// Appends the task text on its own line, without the checkbox.
    fn visit_task_item(&mut self, data: &TaskItemData<'a>) {
        self.text.push_str(data.content);
        self.text.push('\n');
    }

    /// Ignores HTML comments: they are not visible text.
    fn visit_html_comment(&mut self, _data: &HtmlCommentData<'a>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_document;

    /// Link syntax is dropped: only the link text remains in the extracted plain text.
    #[test]
    fn test_plain_text_extraction() {
        let input = "Hello world! Check out [my link](https://example.com).";
        let doc = parse_document("test.md", None, input).expect("input should parse");

        let mut extractor = PlainTextExtractor::new();
        extractor.visit_document(&doc);

        assert_eq!(extractor.text.trim(), "Hello world! Check out my link.");
    }
}
