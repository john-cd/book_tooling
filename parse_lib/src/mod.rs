use crate::ast::*;
use crate::Document;

/// A trait for walking the Abstract Syntax Tree (AST).
///
/// By default, it recursively visits all child elements of container nodes.
/// Implementors can override specific `visit_*` methods to perform actions.
pub trait Visitor<'a> {
    /// Entry point for walking an entire document.
    fn visit_document(&mut self, doc: &Document<'a>) {
        for element in &doc.elements {
            self.visit_element(element);
        }
    }

    /// Routes the element to its specific visit method.
    fn visit_element(&mut self, element: &Element<'a>) {
        match element {
            Element::Text(data) => self.visit_text(data),
            Element::Paragraph(data) => self.visit_paragraph(data),
            Element::Blockquote(data) => self.visit_blockquote(data),
            Element::List(data) => self.visit_list(data),
            Element::ListItem(data) => self.visit_list_item(data),
            Element::FencedCodeBlock(data) => self.visit_fenced_code_block(data),
            Element::CodeSpan(data) => self.visit_code_span(data),
            Element::InlineLink(data) => self.visit_inline_link(data),
            Element::ReferenceStyleLink(data) => self.visit_reference_style_link(data),
            Element::WikiLink(data) => self.visit_wikilink(data),
            Element::Heading(data) => self.visit_heading(data),
            Element::TaskItem(data) => self.visit_task_item(data),
            Element::Table(data) => self.visit_table(data),
            Element::Autolink(data) => self.visit_autolink(data),
            Element::InlineImage(data) => self.visit_inline_image(data),
            Element::ReferenceStyleImage(data) => self.visit_reference_style_image(data),
            Element::ReferenceDefinition(data) => self.visit_reference_definition(data),
            Element::HiddenHtmlDiv(data) => self.visit_hidden_html_div(data),
            Element::HtmlComment(data) => self.visit_html_comment(data),
            Element::CustomDirective(data) => self.visit_custom_directive(data),
            Element::Strong(data) => self.visit_strong(data),
            Element::Emphasis(data) => self.visit_emphasis(data),
            Element::Strikethrough(data) => self.visit_strikethrough(data),
            Element::HorizontalRule => self.visit_horizontal_rule(),
            _ => {} // Fallback for elements not explicitly handled
        }
    }

    // --- Leaf Nodes (No children) ---
    fn visit_text(&mut self, _data: &TextData<'a>) {}
    fn visit_fenced_code_block(&mut self, _data: &FencedCodeBlockData<'a>) {}
    fn visit_code_span(&mut self, _data: &CodeSpanData<'a>) {}
    fn visit_inline_link(&mut self, _data: &InlineLinkData<'a>) {}
    fn visit_reference_style_link(&mut self, _data: &ReferenceStyleLinkData<'a>) {}
    fn visit_wikilink(&mut self, _data: &WikiLinkData<'a>) {}
    fn visit_heading(&mut self, _data: &HeadingData<'a>) {}
    fn visit_task_item(&mut self, _data: &TaskItemData<'a>) {}
    fn visit_table(&mut self, _data: &TableData<'a>) {}
    fn visit_autolink(&mut self, _data: &AutolinkData<'a>) {}
    fn visit_inline_image(&mut self, _data: &InlineImageData<'a>) {}
    fn visit_reference_style_image(&mut self, _data: &ReferenceStyleImageData<'a>) {}
    fn visit_reference_definition(&mut self, _data: &ReferenceDefinitionData<'a>) {}
    fn visit_hidden_html_div(&mut self, _data: &HiddenHtmlDivData<'a>) {}
    fn visit_html_comment(&mut self, _data: &HtmlCommentData<'a>) {}
    fn visit_horizontal_rule(&mut self) {}
    fn visit_custom_directive(&mut self, _data: &DirectiveData<'a>) {}

    // --- Container Nodes (Recursive by default) ---
    fn visit_paragraph(&mut self, data: &ParagraphData<'a>) {
        for element in &data.elements {
            self.visit_element(element);
        }
    }

    fn visit_blockquote(&mut self, data: &BlockquoteData<'a>) {
        for element in &data.elements {
            self.visit_element(element);
        }
    }

    fn visit_list(&mut self, data: &ListData<'a>) {
        for item in &data.items {
            self.visit_element(item);
        }
    }

    fn visit_list_item(&mut self, data: &ListItemData<'a>) {
        for element in &data.elements {
            self.visit_element(element);
        }
    }

    fn visit_strong(&mut self, data: &StrongData<'a>) {
        for element in &data.elements {
            self.visit_element(element);
        }
    }

    fn visit_emphasis(&mut self, data: &EmphasisData<'a>) {
        for element in &data.elements {
            self.visit_element(element);
        }
    }

    fn visit_strikethrough(&mut self, data: &StrikethroughData<'a>) {
        for element in &data.elements {
            self.visit_element(element);
        }
    }
}
