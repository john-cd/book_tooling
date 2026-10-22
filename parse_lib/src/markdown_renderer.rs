//! Renders AST elements back to Markdown, with resolvers that rewrite links.
//!
//! TODO Reviewed ??-??-2026

use std::ops::Range;

use crate::ast::{
    AutolinkData, BlockquoteData, DirectiveData, Document, EmphasisData, FencedCodeBlockData,
    HeadingData, HtmlCommentData, InlineImageData, InlineLinkData, ListData, ListItemData,
    ParagraphData, ReferenceDefinitionData, ReferenceStyleImageData, ReferenceStyleLinkData,
    StrikethroughData, StrongData, TableData, TaskItemData, TextData, WikiLinkData,
};
use crate::visit::Visitor;
use std::fmt;
use std::fmt::Write as _;

/// A pending replacement of a byte range of the original source.
#[derive(Clone)]
struct SourceEdit {
    /// Byte range of the original source to replace.
    range: Range<usize>,
    /// Text that replaces the range.
    replacement: String,
}

/// Returns the byte offset of the parenthesis that closes the one at `open`, if any.
fn matching_parenthesis(source: &str, open: usize) -> Option<usize> {
    let mut depth = 0_usize;
    let mut escaped = false;
    for (offset, character) in source.get(open..)?.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '(' => depth += 1,
            ')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(open + offset + character.len_utf8());
                }
            }
            _ => {}
        }
    }
    None
}

/// Resolver used to rewrite wikilinks while rendering Markdown.
pub type WikiLinkResolver = Box<dyn for<'b> FnMut(&WikiLinkData<'b>) -> String>;

/// Resolver used to rewrite inline links while rendering Markdown.
pub type InlineLinkResolver = Box<dyn for<'b> FnMut(&InlineLinkData<'b>) -> String>;

/// Resolver used to rewrite autolinks while rendering Markdown.
pub type AutolinkResolver = Box<dyn for<'b> FnMut(&AutolinkData<'b>) -> String>;

/// An AST visitor that reconstructs a Markdown string from the AST.
///
/// It supports on-the-fly replacement of Custom Directives and optional resolution
/// of wikilinks and other link types through callback functions.
#[expect(
    clippy::partial_pub_fields,
    reason = "The callbacks are public configuration; the source-tracking state is an implementation detail."
)]
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
    /// Original Markdown source, kept to reproduce untouched text verbatim.
    source: Option<String>,
    /// Address of the start of the source, used to turn fragments into byte offsets.
    source_base: usize,
    /// Replacements recorded while visiting the document.
    source_edits: Vec<SourceEdit>,
    /// Whether an edit could not be located and the document must be re-rendered in full.
    source_edit_fallback: bool,
}

impl<F> MarkdownRenderer<F> {
    /// Appends formatted text to the output buffer; writing to a `String` cannot fail.
    fn emit(&mut self, args: fmt::Arguments<'_>) {
        let _unused = self.markdown.write_fmt(args);
    }

    /// Creates a new `MarkdownRenderer` with a mandatory directive resolver.
    pub fn new(directive_resolver: F) -> Self
    where
        F: FnMut(&DirectiveData<'_>) -> String,
    {
        Self {
            markdown: String::new(),
            directive_resolver,
            wikilink_resolver: None,
            inline_link_resolver: None,
            autolink_resolver: None,
            source: None,
            source_base: 0,
            source_edits: Vec::new(),
            source_edit_fallback: false,
        }
    }

    /// Locates `fragment` within the original source and returns its byte range.
    fn source_range(&self, fragment: &str) -> Option<Range<usize>> {
        let source = self.source.as_ref()?;
        let start = fragment.as_ptr().addr().checked_sub(self.source_base)?;
        let end = start.checked_add(fragment.len())?;
        (end <= source.len() && source.is_char_boundary(start) && source.is_char_boundary(end))
            .then_some(start..end)
    }

    /// Returns the byte range of the whole `{{...}}` directive in the source.
    fn directive_range(&self, directive: &DirectiveData<'_>) -> Option<Range<usize>> {
        let payload = match directive {
            DirectiveData::Link { name, .. }
            | DirectiveData::Badge { name, .. }
            | DirectiveData::ExampleBlock { name } => *name,
            DirectiveData::CrateBlock { crate_name, .. } => *crate_name,
            DirectiveData::CategoryBlock { categories } => categories.first().copied()?,
        };
        let payload_range = self.source_range(payload)?;
        let source = self.source.as_ref()?;
        let start = source.get(..payload_range.start)?.rfind("{{")?;
        let close = source.get(payload_range.end..)?.find("}}")?;
        Some(start..payload_range.end + close + 2)
    }

    /// Returns the byte range of the whole `[[...]]` wikilink in the source.
    fn wikilink_range(&self, data: &WikiLinkData<'_>) -> Option<Range<usize>> {
        let target_range = self.source_range(data.target)?;
        let source = self.source.as_ref()?;
        let start = source.get(..target_range.start)?.rfind("[[")?;
        let close = source.get(target_range.end..)?.find("]]")? + target_range.end + 2;
        let end = data
            .immediately_after
            .and_then(|suffix| self.source_range(suffix))
            .filter(|suffix| suffix.start == close)
            .map_or(close, |suffix| suffix.end);
        Some(start..end)
    }

    /// Returns the byte range of the whole inline link in the source.
    fn inline_link_range(&self, data: &InlineLinkData<'_>) -> Option<Range<usize>> {
        let text_range = self.source_range(data.text)?;
        let source = self.source.as_ref()?;
        let start = text_range.start.checked_sub(1)?;
        if source.get(start..text_range.start)? != "[" {
            return None;
        }
        let after_text = source.get(text_range.end..)?;
        let close_bracket = after_text.find(']')? + text_range.end;
        let mut open = close_bracket + 1;
        while source
            .as_bytes()
            .get(open)
            .is_some_and(u8::is_ascii_whitespace)
        {
            open += 1;
        }
        if source.as_bytes().get(open) != Some(&b'(') {
            return None;
        }
        let end = matching_parenthesis(source, open)?;
        Some(start..end)
    }

    /// Returns the byte range of the whole autolink in the source.
    fn autolink_range(&self, data: &AutolinkData<'_>) -> Option<Range<usize>> {
        let url_range = self.source_range(data.url)?;
        let source = self.source.as_ref()?;
        if url_range.start > 0
            && source.as_bytes().get(url_range.start - 1) == Some(&b'<')
            && source.as_bytes().get(url_range.end) == Some(&b'>')
        {
            Some(url_range.start - 1..url_range.end + 1)
        } else {
            Some(url_range)
        }
    }

    /// Records a replacement for `range`, or marks the render as needing the full fallback if the range is unusable.
    fn record_edit(
        &mut self,
        range: Option<Range<usize>>,
        replacement: String,
        canonical_source: &str,
    ) {
        if replacement == canonical_source {
            return;
        }
        if let Some(source) = &self.source {
            if let Some(valid_range) =
                range.filter(|candidate| source.get(candidate.clone()).is_some())
            {
                self.source_edits.push(SourceEdit {
                    range: valid_range,
                    replacement,
                });
            } else {
                // Do not silently discard a requested rewrite if source-span recovery fails.
                // The document renderer will fall back to its canonical AST rendering.
                self.source_edit_fallback = true;
            }
        }
    }

    /// Applies the recorded edits to the original source, if they are all valid and non-overlapping.
    fn apply_source_edits(&self) -> Option<String> {
        let source = self.source.as_ref()?;
        if self.source_edit_fallback {
            return None;
        }
        if self.source_edits.is_empty() {
            return Some(source.clone());
        }

        let mut edits = self.source_edits.clone();
        edits.sort_by_key(|edit| edit.range.start);
        let mut output = String::with_capacity(source.len());
        let mut cursor = 0;
        for edit in edits {
            if edit.range.start < cursor || edit.range.end > source.len() {
                return None;
            }
            output.push_str(source.get(cursor..edit.range.start)?);
            output.push_str(&edit.replacement);
            cursor = edit.range.end;
        }
        output.push_str(source.get(cursor..)?);
        Some(output)
    }

    /// Installs a resolver for wikilinks encountered during rendering.
    #[must_use]
    pub fn with_wikilink_resolver<R>(mut self, resolver: R) -> Self
    where
        R: for<'b> FnMut(&WikiLinkData<'b>) -> String + 'static,
    {
        self.wikilink_resolver = Some(Box::new(resolver));
        self
    }

    /// Installs a resolver for inline links encountered during rendering.
    #[must_use]
    pub fn with_inline_link_resolver<R>(mut self, resolver: R) -> Self
    where
        R: for<'b> FnMut(&InlineLinkData<'b>) -> String + 'static,
    {
        self.inline_link_resolver = Some(Box::new(resolver));
        self
    }

    /// Installs a resolver for autolinks encountered during rendering.
    #[must_use]
    pub fn with_autolink_resolver<R>(mut self, resolver: R) -> Self
    where
        R: for<'b> FnMut(&AutolinkData<'b>) -> String + 'static,
    {
        self.autolink_resolver = Some(Box::new(resolver));
        self
    }
}

impl<'a, F> Visitor<'a> for MarkdownRenderer<F>
where
    F: FnMut(&DirectiveData<'a>) -> String,
{
    fn visit_document(&mut self, doc: &'a Document<'a>) {
        self.markdown.clear();
        self.source_edits.clear();
        self.source_edit_fallback = false;
        self.source = Some(doc.source.to_owned());
        self.source_base = doc.source.as_ptr().addr();

        for element in &doc.elements {
            self.visit_element(element);
        }

        if let Some(output) = self.apply_source_edits() {
            self.markdown = output;
        }
        self.source = None;
        self.source_base = 0;
        self.source_edits.clear();
        self.source_edit_fallback = false;
    }

    fn visit_text(&mut self, data: &TextData<'a>) {
        self.markdown.push_str(data.content);
    }

    fn visit_inline_link(&mut self, data: &InlineLinkData<'a>) {
        if let Some(resolver) = &mut self.inline_link_resolver {
            let replacement = resolver(data);
            let range = self.inline_link_range(data);
            let canonical_source = format!("[{}]({})", data.text, data.url);
            self.record_edit(range, replacement.clone(), &canonical_source);
            self.markdown.push_str(&replacement);
        } else {
            self.emit(format_args!("[{}]({})", data.text, data.url));
        }
    }

    fn visit_fenced_code_block(&mut self, data: &FencedCodeBlockData<'a>) {
        let lang = data.language.unwrap_or("");
        self.emit(format_args!("```{lang}\n"));
        for line in &data.lines {
            self.markdown.push_str(line);
            self.markdown.push('\n');
        }
        self.markdown.push_str("```\n");
    }

    fn visit_wikilink(&mut self, data: &WikiLinkData<'a>) {
        if let Some(resolver) = &mut self.wikilink_resolver {
            let replacement = resolver(data);
            let range = self.wikilink_range(data);
            let suffix = data.immediately_after.unwrap_or("");
            let canonical_source = data.display.map_or_else(
                || format!("[[{}]]{}", data.target, suffix),
                |display| format!("[[{} | {}]]{}", data.target, display, suffix),
            );
            self.record_edit(range, replacement.clone(), &canonical_source);
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
            let range = self.autolink_range(data);
            self.record_edit(range, replacement.clone(), &format!("<{}>", data.url));
            self.markdown.push_str(&replacement);
        } else {
            self.markdown.push('<');
            self.markdown.push_str(data.url);
            self.markdown.push('>');
        }
    }

    fn visit_reference_style_link(&mut self, data: &ReferenceStyleLinkData<'a>) {
        if data.text == data.label {
            self.emit(format_args!("[{}]", data.text));
        } else {
            self.emit(format_args!("[{}][{}]", data.text, data.label));
        }
    }

    fn visit_reference_style_image(&mut self, data: &ReferenceStyleImageData<'a>) {
        self.emit(format_args!(
            "![{}][{}]",
            data.image_description, data.label
        ));
    }

    fn visit_reference_definition(&mut self, data: &ReferenceDefinitionData<'a>) {
        self.emit(format_args!("[{}]: {}", data.label, data.url));
        if let Some(title) = data.title {
            self.emit(format_args!(" \"{title}\""));
        }
        self.markdown.push('\n');
    }

    fn visit_inline_image(&mut self, data: &InlineImageData<'a>) {
        if let Some(title) = data.title {
            self.emit(format_args!(
                "![{}]({} \"{}\")",
                data.image_description, data.url, title
            ));
        } else {
            self.emit(format_args!("![{}]({})", data.image_description, data.url));
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
            self.emit(format_args!(" {{#{id}}}"));
        }
        self.markdown.push('\n');
    }

    fn visit_task_item(&mut self, data: &TaskItemData<'a>) {
        let check = if data.completed { "x" } else { " " };
        self.emit(format_args!("- [{}] {}\n", check, data.content));
    }

    fn visit_table(&mut self, data: &'a TableData<'a>) {
        let start_len = self.markdown.len();
        self.visit_table_cells(data);
        self.markdown.truncate(start_len);

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
        for (cell, width) in data.header.iter().zip(&max_widths) {
            self.emit(format_args!(" {:<width$} |", cell, width = *width));
        }
        self.markdown.push('\n');

        // Render Separator
        self.markdown.push('|');
        for width in &max_widths {
            self.emit(format_args!(" {:-<width$} |", "", width = *width));
        }
        self.markdown.push('\n');

        // Render Data Rows
        for row in &data.rows {
            self.markdown.push('|');
            for (i, &width) in max_widths.iter().enumerate() {
                let cell = row.get(i).unwrap_or(&"");
                self.emit(format_args!(" {cell:<width$} |"));
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
                "- ".to_owned()
            };

            let indent = " ".repeat(marker.len());

            for (line_index, line) in inner_rendered.trim_end().lines().enumerate() {
                if line_index == 0 {
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
        let range = self.directive_range(data);
        self.record_edit(range, replacement.clone(), &data.to_string());
        self.markdown.push_str(&replacement);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Element;
    use crate::parse_document;

    #[test]
    fn test_markdown_renderer_text() {
        let mut renderer = MarkdownRenderer::new(|_| String::new());
        renderer.visit_element(&Element::Text(TextData {
            content: "Hello, world!",
        }));
        assert_eq!(renderer.markdown, "Hello, world!");
    }

    #[test]
    fn document_no_op_render_preserves_titles_native_helpers_and_crlf() {
        let source = concat!(
            "# Heading\r\n\r\n",
            "<!-- keep this comment -->\r\n\r\n",
            "| name | value |\r\n| --- | --- |\r\n| book | mdBook |\r\n\r\n",
            "```rust\r\nfn main() {}\r\n```\r\n\r\n",
            "[Rust](https://rust-lang.org/ \"Rust title\")\r\n\r\n",
            "{{#include refs.incl.md:1:3}}\r\n"
        );
        let doc = parse_document("book.md", None, source).expect("parse fixture");
        let mut renderer = MarkdownRenderer::new(|directive| directive.to_string());

        renderer.visit_document(&doc);

        assert_eq!(renderer.markdown, source);
    }

    #[test]
    fn changed_directive_preserves_untouched_markdown_source() {
        let source = "[Rust](https://rust-lang.org/ \"Rust title\")\r\n{{#example demo}}\r\n";
        let doc = parse_document("book.md", None, source).expect("parse fixture");
        let mut renderer = MarkdownRenderer::new(|directive| match directive {
            DirectiveData::ExampleBlock { .. } => "```rust\nfn main() {}\n```".to_owned(),
            DirectiveData::Link { .. }
            | DirectiveData::Badge { .. }
            | DirectiveData::CrateBlock { .. }
            | DirectiveData::CategoryBlock { .. } => directive.to_string(),
        });

        renderer.visit_document(&doc);

        assert_eq!(
            renderer.markdown,
            "[Rust](https://rust-lang.org/ \"Rust title\")\r\n```rust\nfn main() {}\n```\r\n"
        );
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
                format!("[{display}][{target}]")
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
        renderer.visit_element(&Element::Table(TableData::new(
            vec!["Header 1", "H2"],
            vec![vec!["Cell 1", "C 2"], vec!["Longer Cell", "Val"]],
        )));
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
        renderer.visit_element(&Element::Table(TableData::new(
            vec!["Col 1", "Col 2"],
            vec![vec!["Val 1", "Val 2"], vec!["Val 3", "Val 4"]],
        )));
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
