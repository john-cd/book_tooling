//! Renders AST elements as HTML, with HTML escaping.
//!
//! TODO Reviewed ??-??-2026

use crate::ast::BlockquoteData;
use crate::ast::EmphasisData;
use crate::ast::FencedCodeBlockData;
use crate::ast::HtmlCommentData;
use crate::ast::InlineLinkData;
use crate::ast::ListData;
use crate::ast::ListItemData;
use crate::ast::ParagraphData;
use crate::ast::StrikethroughData;
use crate::ast::StrongData;
use crate::ast::TableData;
use crate::ast::TextData;
use crate::visit::Visitor;

/// Appends an HTML-escaped version of the input string to the destination buffer.
pub fn escape_html_to_string(input: &str, dest: &mut String) {
    let mut last_end = 0;

    // Iterate over bytes, which is faster than chars().
    // This is safe because HTML special characters are all single-byte ASCII.
    for (index, byte) in input.bytes().enumerate() {
        let escaped = match byte {
            b'<' => "&lt;",
            b'>' => "&gt;",
            b'&' => "&amp;",
            b'"' => "&quot;",
            b'\'' => "&#39;",
            _ => continue,
        };

        // Push the contiguous block of safe text prior to this special character
        dest.push_str(input.get(last_end..index).unwrap_or_default());
        // Push the escaped HTML entity
        dest.push_str(escaped);
        last_end = index + 1;
    }

    // Push any remaining safe text at the end of the string
    dest.push_str(input.get(last_end..).unwrap_or_default());
}

/// An AST visitor that compiles the document into an HTML string.
#[derive(Default)]
pub struct HtmlRenderer {
    /// Accumulated HTML output.
    pub html: String,
}

impl HtmlRenderer {
    /// Creates an empty HTML renderer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'a> Visitor<'a> for HtmlRenderer {
    /// Emits the text, HTML-escaped.
    fn visit_text(&mut self, data: &TextData<'a>) {
        escape_html_to_string(data.content, &mut self.html);
    }

    /// Emits an `<a>` element with an escaped `href` and escaped link text.
    fn visit_inline_link(&mut self, data: &InlineLinkData<'a>) {
        self.html.push_str("<a href=\"");
        escape_html_to_string(data.url, &mut self.html);
        self.html.push_str("\">");
        escape_html_to_string(data.text, &mut self.html);
        self.html.push_str("</a>");
    }

    /// Emits a `<p>` element around the rendered children.
    fn visit_paragraph(&mut self, data: &ParagraphData<'a>) {
        self.html.push_str("<p>");
        for element in &data.elements {
            self.visit_element(element);
        }
        self.html.push_str("</p>\n");
    }

    /// Emits a `<blockquote>` element around the rendered children.
    fn visit_blockquote(&mut self, data: &BlockquoteData<'a>) {
        self.html.push_str("<blockquote>\n");
        for element in &data.elements {
            self.visit_element(element);
        }
        self.html.push_str("</blockquote>\n");
    }

    /// Emits `<ol>` for an ordered list and `<ul>` otherwise, around the rendered items.
    fn visit_list(&mut self, data: &ListData<'a>) {
        if data.ordered {
            self.html.push_str("<ol>\n");
        } else {
            self.html.push_str("<ul>\n");
        }
        for item in &data.items {
            self.visit_element(item);
        }
        if data.ordered {
            self.html.push_str("</ol>\n");
        } else {
            self.html.push_str("</ul>\n");
        }
    }

    /// Emits an `<li>` element around the rendered children.
    fn visit_list_item(&mut self, data: &ListItemData<'a>) {
        self.html.push_str("<li>");
        for element in &data.elements {
            self.visit_element(element);
        }
        self.html.push_str("</li>\n");
    }

    /// Emits a `<strong>` element around the rendered children.
    fn visit_strong(&mut self, data: &StrongData<'a>) {
        self.html.push_str("<strong>");
        for element in &data.elements {
            self.visit_element(element);
        }
        self.html.push_str("</strong>");
    }

    /// Emits an `<em>` element around the rendered children.
    fn visit_emphasis(&mut self, data: &EmphasisData<'a>) {
        self.html.push_str("<em>");
        for element in &data.elements {
            self.visit_element(element);
        }
        self.html.push_str("</em>");
    }

    /// Emits a `<del>` element around the rendered children.
    fn visit_strikethrough(&mut self, data: &StrikethroughData<'a>) {
        self.html.push_str("<del>");
        for element in &data.elements {
            self.visit_element(element);
        }
        self.html.push_str("</del>");
    }

    /// Emits an `<hr>` element.
    fn visit_horizontal_rule(&mut self) {
        self.html.push_str("<hr>\n");
    }

    /// Emits the comment back as an HTML comment, one source line per line.
    fn visit_html_comment(&mut self, data: &HtmlCommentData<'a>) {
        self.html.push_str("<!--");
        for (i, line) in data.lines.iter().enumerate() {
            if i > 0 {
                self.html.push('\n');
            }
            self.html.push_str(line);
        }
        self.html.push_str("-->");
    }

    /// Emits a `<pre><code>` block, with a `language-` class when a language is given and escaped body lines.
    fn visit_fenced_code_block(&mut self, data: &FencedCodeBlockData<'a>) {
        if let Some(lang) = data.language {
            self.html.push_str("<pre><code class=\"language-");
            escape_html_to_string(lang, &mut self.html);
            self.html.push_str("\">");
        } else {
            self.html.push_str("<pre><code>");
        }
        for line in &data.lines {
            escape_html_to_string(line, &mut self.html);
            self.html.push('\n');
        }
        self.html.push_str("</code></pre>\n");
    }

    /// Emits a `<table>` with a `<thead>` for the header and a `<tbody>` for the rows, escaping cell text.
    fn visit_table(&mut self, data: &TableData<'a>) {
        self.html.push_str("<table>\n  <thead>\n    <tr>\n");
        for &cell in &data.header {
            self.html.push_str("      <th>");
            escape_html_to_string(cell, &mut self.html);
            self.html.push_str("</th>\n");
        }
        self.html.push_str("    </tr>\n  </thead>\n  <tbody>\n");
        for row in &data.rows {
            self.html.push_str("    <tr>\n");
            for &cell in row {
                self.html.push_str("      <td>");
                escape_html_to_string(cell, &mut self.html);
                self.html.push_str("</td>\n");
            }
            self.html.push_str("    </tr>\n");
        }
        self.html.push_str("  </tbody>\n</table>\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Element;

    /// `&`, `<`, `>` and both kinds of quote are escaped, and plain text passes through unchanged.
    #[test]
    fn test_escape_html_to_string() {
        let mut dest = String::new();

        escape_html_to_string("hello world", &mut dest);
        assert_eq!(dest, "hello world");

        dest.clear();
        escape_html_to_string("a < b > c", &mut dest);
        assert_eq!(dest, "a &lt; b &gt; c");

        dest.clear();
        escape_html_to_string("Mac & Cheese", &mut dest);
        assert_eq!(dest, "Mac &amp; Cheese");

        dest.clear();
        escape_html_to_string("\"quote\" and 'single'", &mut dest);
        assert_eq!(dest, "&quot;quote&quot; and &#39;single&#39;");

        dest.clear();
        escape_html_to_string("<script>alert('XSS & \"stuff\"')</script>", &mut dest);
        assert_eq!(
            dest,
            "&lt;script&gt;alert(&#39;XSS &amp; &quot;stuff&quot;&#39;)&lt;/script&gt;"
        );
    }

    /// Text nodes, link text and link URLs are HTML-escaped when rendered.
    #[test]
    fn test_html_renderer_escaping() {
        let mut renderer = HtmlRenderer::new();

        let text_node = Element::Text(TextData {
            content: "Me, Myself & I <3 Rust",
        });
        renderer.visit_element(&text_node);
        assert_eq!(renderer.html, "Me, Myself &amp; I &lt;3 Rust");

        renderer.html.clear();
        let link_node = Element::InlineLink(InlineLinkData {
            text: "Click <here>",
            url: "https://example.com?a=1&b=2",
            title: None,
        });
        renderer.visit_element(&link_node);
        assert_eq!(
            renderer.html,
            "<a href=\"https://example.com?a=1&amp;b=2\">Click &lt;here&gt;</a>"
        );
    }

    /// A paragraph renders as a `<p>` element.
    #[test]
    fn test_html_renderer_paragraph() {
        let mut renderer = HtmlRenderer::new();
        renderer.visit_element(&Element::Paragraph(ParagraphData {
            elements: vec![Element::Text(TextData { content: "Para 1" })],
        }));
        assert_eq!(renderer.html, "<p>Para 1</p>\n");
    }

    /// A blockquote renders as a `<blockquote>` element wrapping its children.
    #[test]
    fn test_html_renderer_blockquote() {
        let mut renderer = HtmlRenderer::new();
        renderer.visit_element(&Element::Blockquote(BlockquoteData {
            elements: vec![Element::Text(TextData { content: "Quote" })],
        }));
        assert_eq!(renderer.html, "<blockquote>\nQuote</blockquote>\n");
    }

    /// Unordered lists render as `<ul>` and ordered lists as `<ol>`, with each item in `<li>`.
    #[test]
    fn test_html_renderer_list() {
        let mut renderer = HtmlRenderer::new();
        renderer.visit_element(&Element::List(ListData {
            ordered: false,
            items: vec![Element::ListItem(ListItemData {
                elements: vec![Element::Text(TextData { content: "Item" })],
            })],
        }));
        assert_eq!(renderer.html, "<ul>\n<li>Item</li>\n</ul>\n");

        let mut renderer2 = HtmlRenderer::new();
        renderer2.visit_element(&Element::List(ListData {
            ordered: true,
            items: vec![Element::ListItem(ListItemData {
                elements: vec![Element::Text(TextData { content: "Item" })],
            })],
        }));
        assert_eq!(renderer2.html, "<ol>\n<li>Item</li>\n</ol>\n");
    }

    /// Strong, emphasis and strikethrough render as `<strong>`, `<em>` and `<del>`.
    #[test]
    fn test_html_renderer_inline_formatting() {
        let mut renderer = HtmlRenderer::new();
        renderer.visit_element(&Element::Strong(StrongData {
            elements: vec![Element::Text(TextData { content: "bold" })],
        }));
        assert_eq!(renderer.html, "<strong>bold</strong>");

        renderer.html.clear();
        renderer.visit_element(&Element::Emphasis(EmphasisData {
            elements: vec![Element::Text(TextData { content: "italic" })],
        }));
        assert_eq!(renderer.html, "<em>italic</em>");

        renderer.html.clear();
        renderer.visit_element(&Element::Strikethrough(StrikethroughData {
            elements: vec![Element::Text(TextData { content: "strike" })],
        }));
        assert_eq!(renderer.html, "<del>strike</del>");
    }

    /// A horizontal rule renders as `<hr>`.
    #[test]
    fn test_html_renderer_horizontal_rule() {
        let mut renderer = HtmlRenderer::new();
        renderer.visit_element(&Element::HorizontalRule);
        assert_eq!(renderer.html, "<hr>\n");
    }

    /// An HTML comment renders with its lines intact between the comment delimiters.
    #[test]
    fn test_html_renderer_html_comment() {
        let mut renderer = HtmlRenderer::new();
        renderer.visit_element(&Element::HtmlComment(HtmlCommentData {
            lines: vec![" comment line 1 ", " comment line 2 "],
        }));
        assert_eq!(renderer.html, "<!-- comment line 1 \n comment line 2 -->");
    }

    /// A fenced block renders as `<pre><code class="language-..">` with its body HTML-escaped.
    #[test]
    fn test_html_renderer_fenced_code_block() {
        let mut renderer = HtmlRenderer::new();
        renderer.visit_element(&Element::FencedCodeBlock(FencedCodeBlockData {
            language: Some("rust"),
            lines: vec!["fn main() {}", "println!(\"hello <world>\");"],
        }));
        assert_eq!(
            renderer.html,
            "<pre><code class=\"language-rust\">fn main() {}\nprintln!(&quot;hello &lt;world&gt;&quot;);\n</code></pre>\n"
        );

        renderer.html.clear();
        renderer.visit_element(&Element::FencedCodeBlock(FencedCodeBlockData {
            language: None,
            lines: vec!["plain text"],
        }));
        assert_eq!(renderer.html, "<pre><code>plain text\n</code></pre>\n");
    }

    /// A table renders `<thead>` and `<tbody>` sections with escaped cell text.
    #[test]
    fn test_html_renderer_table() {
        let mut renderer = HtmlRenderer::new();
        renderer.visit_element(&Element::Table(TableData::new(
            vec!["Col <1>", "Col 2"],
            vec![vec!["Val 1", "Val 2"]],
        )));
        let expected = "<table>\n  <thead>\n    <tr>\n      <th>Col &lt;1&gt;</th>\n      <th>Col 2</th>\n    </tr>\n  </thead>\n  <tbody>\n    <tr>\n      <td>Val 1</td>\n      <td>Val 2</td>\n    </tr>\n  </tbody>\n</table>\n";
        assert_eq!(renderer.html, expected);
    }
}
