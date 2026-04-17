use crate::ast::*;
use crate::visit::Visitor;

/// An AST visitor that reconstructs a Markdown string from the AST.
///
/// It supports on-the-fly replacement of Custom Directives and optional resolution
/// of wikilinks and other link types through callback functions.
pub type WikiLinkResolver = Box<dyn for<'b> FnMut(&WikiLinkData<'b>) -> String>;
pub type InlineLinkResolver = Box<dyn for<'b> FnMut(&InlineLinkData<'b>) -> String>;
pub type AutolinkResolver = Box<dyn for<'b> FnMut(&AutolinkData<'b>) -> String>;

pub struct MarkdownRenderer<F> {
    /// The accumulating Markdown output.
    pub markdown: String,
    /// A callback to resolve a directive into its Markdown equivalent.
    pub directive_resolver: F,
    /// An optional callback to resolve a wikilink.
    pub wikilink_resolver: Option<WikiLinkResolver>,
    /// An optional callback to resolve an inline link.
    pub inline_link_resolver: Option<InlineLinkResolver>,
    /// An optional callback to resolve an autolink.
    pub autolink_resolver: Option<AutolinkResolver>,
}

impl<F> MarkdownRenderer<F>
where
    F: FnMut(&DirectiveData<'_>) -> String,
{
    /// Creates a new `MarkdownRenderer` with a mandatory directive resolver.
    pub fn new(directive_resolver: F) -> Self {
        Self {
            markdown: String::new(),
            directive_resolver,
            wikilink_resolver: None,
            inline_link_resolver: None,
            autolink_resolver: None,
        }
    }

    pub fn with_wikilink_resolver(
        mut self,
        resolver: impl for<'b> FnMut(&WikiLinkData<'b>) -> String + 'static,
    ) -> Self {
        self.wikilink_resolver = Some(Box::new(resolver));
        self
    }

    pub fn with_inline_link_resolver(
        mut self,
        resolver: impl for<'b> FnMut(&InlineLinkData<'b>) -> String + 'static,
    ) -> Self {
        self.inline_link_resolver = Some(Box::new(resolver));
        self
    }

    pub fn with_autolink_resolver(
        mut self,
        resolver: impl for<'b> FnMut(&AutolinkData<'b>) -> String + 'static,
    ) -> Self {
        self.autolink_resolver = Some(Box::new(resolver));
        self
    }
}

impl<'a, F> Visitor<'a> for MarkdownRenderer<F>
where
    F: FnMut(&DirectiveData<'a>) -> String,
{
    fn visit_text(&mut self, data: &TextData<'a>) {
        self.markdown.push_str(data.content);
    }

    fn visit_inline_link(&mut self, data: &InlineLinkData<'a>) {
        if let Some(resolver) = &mut self.inline_link_resolver {
            let replacement = resolver(data);
            self.markdown.push_str(&replacement);
        } else {
            self.markdown
                .push_str(&format!("[{}]({})", data.text, data.url));
        }
    }

    fn visit_fenced_code_block(&mut self, data: &FencedCodeBlockData<'a>) {
        let lang = data.language.unwrap_or("");
        self.markdown.push_str(&format!("```{lang}\n"));
        for line in &data.lines {
            self.markdown.push_str(line);
            self.markdown.push('\n');
        }
        self.markdown.push_str("```\n");
    }

    fn visit_wikilink(&mut self, data: &WikiLinkData<'a>) {
        if let Some(resolver) = &mut self.wikilink_resolver {
            let replacement = resolver(data);
            self.markdown.push_str(&replacement);
        } else {
            self.markdown.push_str("[[");
            self.markdown.push_str(data.target);
            if let Some(display) = data.display {
                self.markdown.push_str(" | ");
                self.markdown.push_str(display);
            }
            self.markdown.push_str("]]");
            if let Some(after) = data.immediately_after {
                self.markdown.push_str(after);
            }
        }
    }

    fn visit_autolink(&mut self, data: &AutolinkData<'a>) {
        if let Some(resolver) = &mut self.autolink_resolver {
            let replacement = resolver(data);
            self.markdown.push_str(&replacement);
        } else {
            self.markdown.push('<');
            self.markdown.push_str(data.url);
            self.markdown.push('>');
        }
    }

    fn visit_reference_style_link(&mut self, data: &ReferenceStyleLinkData<'a>) {
        if data.text == data.label {
            self.markdown.push_str(&format!("[{}]", data.text));
        } else {
            self.markdown
                .push_str(&format!("[{}][{}]", data.text, data.label));
        }
    }

    fn visit_reference_style_image(&mut self, data: &ReferenceStyleImageData<'a>) {
        self.markdown
            .push_str(&format!("![{}][{}]", data.image_description, data.label));
    }

    fn visit_reference_definition(&mut self, data: &ReferenceDefinitionData<'a>) {
        self.markdown
            .push_str(&format!("[{}]: {}", data.label, data.url));
        if let Some(title) = data.title {
            self.markdown.push_str(&format!(" \"{}\"", title));
        }
        self.markdown.push('\n');
    }

    fn visit_inline_image(&mut self, data: &InlineImageData<'a>) {
        if let Some(title) = data.title {
            self.markdown.push_str(&format!(
                "![{}]({} \"{}\")",
                data.image_description, data.url, title
            ));
        } else {
            self.markdown
                .push_str(&format!("![{}]({})", data.image_description, data.url));
        }
    }

    fn visit_heading(&mut self, data: &HeadingData<'a>) {
        for _ in 0..data.level {
            self.markdown.push('#');
        }
        if let Some(content) = data.content {
            self.markdown.push(' ');
            self.markdown.push_str(content);
        }
        if let Some(id) = data.id {
            self.markdown.push_str(&format!(" {{#{}}}", id));
        }
        self.markdown.push('\n');
    }

    fn visit_task_item(&mut self, data: &TaskItemData<'a>) {
        let check = if data.completed { "x" } else { " " };
        self.markdown
            .push_str(&format!("- [{}] {}\n", check, data.content));
    }

    fn visit_table(&mut self, data: &TableData<'a>) {
        let num_columns = data.header.len();
        if num_columns == 0 {
            return;
        }
        let mut max_widths = vec![0; num_columns];

        // Calculate max width for each column, including header, rows, and separator (min 3 dashes)
        for (i, width_ref) in max_widths.iter_mut().enumerate() {
            let mut max_width = data.header.get(i).map_or(0, |s| s.len());
            for row in &data.rows {
                if let Some(cell) = row.get(i) {
                    max_width = max_width.max(cell.len());
                }
            }
            *width_ref = max_width.max(3);
        }

        // Render Header
        self.markdown.push('|');
        for (i, cell) in data.header.iter().enumerate() {
            self.markdown
                .push_str(&format!(" {:<width$} |", cell, width = max_widths[i]));
        }
        self.markdown.push('\n');

        // Render Separator
        self.markdown.push('|');
        for width in &max_widths {
            self.markdown
                .push_str(&format!(" {:-<width$} |", "", width = *width));
        }
        self.markdown.push('\n');

        // Render Data Rows
        for row in &data.rows {
            self.markdown.push('|');
            for (i, &width) in max_widths.iter().enumerate() {
                let cell = row.get(i).unwrap_or(&"");
                self.markdown
                    .push_str(&format!(" {:<width$} |", cell, width = width));
            }
            self.markdown.push('\n');
        }
    }

    fn visit_paragraph(&mut self, data: &'a ParagraphData<'a>) {
        for element in &data.elements {
            self.visit_element(element);
        }
        self.markdown.push_str("\n\n");
    }

    fn visit_blockquote(&mut self, data: &'a BlockquoteData<'a>) {
        let start_len = self.markdown.len();
        for element in &data.elements {
            self.visit_element(element);
        }
        let inner_rendered = self.markdown.split_off(start_len);

        for line in inner_rendered.trim_end().lines() {
            if line.is_empty() {
                self.markdown.push_str(">\n");
            } else {
                self.markdown.push_str("> ");
                self.markdown.push_str(line);
                self.markdown.push('\n');
            }
        }
        self.markdown.push('\n');
    }

    fn visit_list(&mut self, data: &'a ListData<'a>) {
        for (i, item) in data.items.iter().enumerate() {
            let start_len = self.markdown.len();
            self.visit_element(item);
            let inner_rendered = self.markdown.split_off(start_len);

            let marker = if data.ordered {
                format!("{}. ", i + 1)
            } else {
                "- ".to_string()
            };

            let indent = " ".repeat(marker.len());

            for (idx, line) in inner_rendered.trim_end().lines().enumerate() {
                if idx == 0 {
                    self.markdown.push_str(&marker);
                    self.markdown.push_str(line);
                } else if line.is_empty() {
                    self.markdown.push('\n');
                } else {
                    self.markdown.push_str(&indent);
                    self.markdown.push_str(line);
                }
                self.markdown.push('\n');
            }
        }
        self.markdown.push('\n');
    }

    fn visit_list_item(&mut self, data: &'a ListItemData<'a>) {
        for element in &data.elements {
            self.visit_element(element);
        }
    }

    fn visit_strong(&mut self, data: &'a StrongData<'a>) {
        self.markdown.push_str("**");
        for element in &data.elements {
            self.visit_element(element);
        }
        self.markdown.push_str("**");
    }

    fn visit_emphasis(&mut self, data: &'a EmphasisData<'a>) {
        self.markdown.push('*');
        for element in &data.elements {
            self.visit_element(element);
        }
        self.markdown.push('*');
    }

    fn visit_strikethrough(&mut self, data: &'a StrikethroughData<'a>) {
        self.markdown.push_str("~~");
        for element in &data.elements {
            self.visit_element(element);
        }
        self.markdown.push_str("~~");
    }

    fn visit_horizontal_rule(&mut self) {
        self.markdown.push_str("\n---\n\n");
    }

    fn visit_html_comment(&mut self, data: &HtmlCommentData<'a>) {
        self.markdown.push_str("<!--");
        for (i, line) in data.lines.iter().enumerate() {
            if i > 0 {
                self.markdown.push('\n');
            }
            self.markdown.push_str(line);
        }
        self.markdown.push_str("-->");
    }

    fn visit_custom_directive(&mut self, data: &DirectiveData<'a>) {
        // Call the resolver to generate the replacement Markdown
        let replacement = (self.directive_resolver)(data);
        self.markdown.push_str(&replacement);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_renderer_text() {
        let mut renderer = MarkdownRenderer::new(|_| String::new());
        renderer.visit_element(&Element::Text(TextData {
            content: "Hello, world!",
        }));
        assert_eq!(renderer.markdown, "Hello, world!");
    }

    #[test]
    fn test_markdown_renderer_inline_link() {
        let mut renderer = MarkdownRenderer::new(|_| String::new());
        renderer.visit_element(&Element::InlineLink(InlineLinkData {
            text: "Rust",
            url: "https://rust-lang.org",
            title: None,
        }));
        assert_eq!(renderer.markdown, "[Rust](https://rust-lang.org)");
    }

    #[test]
    fn test_markdown_renderer_wikilink_default() {
        let mut renderer = MarkdownRenderer::new(|_| String::new());
        renderer.visit_element(&Element::WikiLink(WikiLinkData {
            target: "page",
            display: Some("display"),
            immediately_after: Some("s"),
        }));
        assert_eq!(renderer.markdown, "[[page | display]]s");
    }

    #[test]
    fn test_markdown_renderer_wikilink_resolver() {
        let mut renderer =
            MarkdownRenderer::new(|_| String::new()).with_wikilink_resolver(|data| {
                let display = data.display.unwrap_or(data.target);
                let target = data.target.replace(' ', "-");
                format!("[{}][{}]", display, target)
            });

        renderer.visit_element(&Element::WikiLink(WikiLinkData {
            target: "my page",
            display: Some("My Page"),
            immediately_after: None,
        }));
        assert_eq!(renderer.markdown, "[My Page][my-page]");
    }

    #[test]
    fn test_markdown_renderer_table_aligned() {
        let mut renderer = MarkdownRenderer::new(|_| String::new());
        renderer.visit_element(&Element::Table(TableData {
            header: vec!["Header 1", "H2"],
            rows: vec![vec!["Cell 1", "C 2"], vec!["Longer Cell", "Val"]],
        }));
        let expected = "\
| Header 1    | H2  |
| ----------- | --- |
| Cell 1      | C 2 |
| Longer Cell | Val |
";
        // We trim to handle potential trailing newlines from the renderer.
        assert_eq!(renderer.markdown.trim(), expected.trim());
    }

    #[test]
    fn test_markdown_renderer_fenced_code_block() {
        let mut renderer = MarkdownRenderer::new(|_| String::new());
        renderer.visit_element(&Element::FencedCodeBlock(FencedCodeBlockData {
            language: Some("rust"),
            lines: vec!["fn main() {", "    println!(\"Hello\");", "}"],
        }));
        assert_eq!(
            renderer.markdown,
            "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```\n"
        );
    }

    #[test]
    fn test_markdown_renderer_heading() {
        let mut renderer = MarkdownRenderer::new(|_| String::new());
        renderer.visit_element(&Element::Heading(HeadingData {
            level: 3,
            content: Some("My Header"),
            id: Some("my-header"),
        }));
        assert_eq!(renderer.markdown, "### My Header {#my-header}\n");

        let mut renderer_no_id = MarkdownRenderer::new(|_| String::new());
        renderer_no_id.visit_element(&Element::Heading(HeadingData {
            level: 1,
            content: Some("Top Level"),
            id: None,
        }));
        assert_eq!(renderer_no_id.markdown, "# Top Level\n");
    }

    #[test]
    fn test_markdown_renderer_task_item() {
        let mut renderer = MarkdownRenderer::new(|_| String::new());
        renderer.visit_element(&Element::TaskItem(TaskItemData {
            completed: true,
            content: "Done task",
        }));
        assert_eq!(renderer.markdown, "- [x] Done task\n");

        let mut renderer_uncompleted = MarkdownRenderer::new(|_| String::new());
        renderer_uncompleted.visit_element(&Element::TaskItem(TaskItemData {
            completed: false,
            content: "Todo task",
        }));
        assert_eq!(renderer_uncompleted.markdown, "- [ ] Todo task\n");
    }

    #[test]
    fn test_markdown_renderer_table() {
        let mut renderer = MarkdownRenderer::new(|_| String::new());
        renderer.visit_element(&Element::Table(TableData {
            header: vec!["Col 1", "Col 2"],
            rows: vec![vec!["Val 1", "Val 2"], vec!["Val 3", "Val 4"]],
        }));
        let expected =
            "| Col 1 | Col 2 |\n| ----- | ----- |\n| Val 1 | Val 2 |\n| Val 3 | Val 4 |\n";
        assert_eq!(renderer.markdown, expected);
    }

    #[test]
    fn test_markdown_renderer_paragraph() {
        let mut renderer = MarkdownRenderer::new(|_| String::new());
        renderer.visit_element(&Element::Paragraph(ParagraphData {
            elements: vec![Element::Text(TextData { content: "Para 1" })],
        }));
        assert_eq!(renderer.markdown, "Para 1\n\n");
    }

    #[test]
    fn test_markdown_renderer_blockquote() {
        let mut renderer = MarkdownRenderer::new(|_| String::new());
        renderer.visit_element(&Element::Blockquote(BlockquoteData {
            elements: vec![
                Element::Paragraph(ParagraphData {
                    elements: vec![Element::Text(TextData { content: "Line 1" })],
                }),
                Element::Paragraph(ParagraphData {
                    elements: vec![Element::Text(TextData { content: "Line 2" })],
                }),
            ],
        }));

        // Correctly prefixes each paragraph, preserving the blank line between them!
        assert_eq!(renderer.markdown, "> Line 1\n>\n> Line 2\n\n");
    }

    #[test]
    fn test_markdown_renderer_strong_and_emphasis() {
        let mut renderer = MarkdownRenderer::new(|_| String::new());
        renderer.visit_element(&Element::Strong(StrongData {
            elements: vec![
                Element::Text(TextData {
                    content: "bold and ",
                }),
                Element::Emphasis(EmphasisData {
                    elements: vec![Element::Text(TextData { content: "italic" })],
                }),
            ],
        }));
        assert_eq!(renderer.markdown, "**bold and *italic***");
    }

    #[test]
    fn test_markdown_renderer_strikethrough() {
        let mut renderer = MarkdownRenderer::new(|_| String::new());
        renderer.visit_element(&Element::Strikethrough(StrikethroughData {
            elements: vec![Element::Text(TextData { content: "deleted" })],
        }));
        assert_eq!(renderer.markdown, "~~deleted~~");
    }

    #[test]
    fn test_markdown_renderer_horizontal_rule() {
        let mut renderer = MarkdownRenderer::new(|_| String::new());
        renderer.visit_element(&Element::HorizontalRule);
        assert_eq!(renderer.markdown, "\n---\n\n");
    }
}
