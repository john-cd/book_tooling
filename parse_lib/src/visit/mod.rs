//! AST Visitor and Visitable traits for traversing the Markdown document.

pub mod debugger;
pub mod io;
pub mod pretty_printer;
pub mod refdefs;

use crate::Document;
use crate::Documents;
use crate::ast::*;

/// A trait for walking the Abstract Syntax Tree (AST).
///
/// By default, it recursively visits all child elements of container nodes.
/// Implementors can override specific `visit_*` methods to perform actions.
///
/// The lifetime `'a` refers to the lifetime of the document being visited.
pub trait Visitor<'a> {
    /// Entry point for walking an entire document.
    fn visit_document(&mut self, doc: &'a Document<'a>) {
        for element in &doc.elements {
            self.visit_element(element);
        }
    }

    /// Entry point for walking a document collection.
    fn visit_documents(&mut self, docs: &'a Documents<'a>) {
        for doc in &docs.documents {
            self.visit_document(doc);
        }
    }

    /// Routes the element to its specific visit method.
    fn visit_element(&mut self, element: &'a Element<'a>) {
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
        }
    }

    // --- Leaf Nodes (No children) ---
    fn visit_text(&mut self, _data: &'a TextData<'a>) {}
    fn visit_horizontal_rule(&mut self) {}
    fn visit_fenced_code_block(&mut self, _data: &'a FencedCodeBlockData<'a>) {}
    fn visit_code_span(&mut self, _data: &'a CodeSpanData<'a>) {}
    fn visit_inline_link(&mut self, _data: &'a InlineLinkData<'a>) {}
    fn visit_reference_style_link(&mut self, _data: &'a ReferenceStyleLinkData<'a>) {}
    fn visit_wikilink(&mut self, _data: &'a WikiLinkData<'a>) {}
    fn visit_heading(&mut self, _data: &'a HeadingData<'a>) {}
    fn visit_task_item(&mut self, _data: &'a TaskItemData<'a>) {}
    fn visit_table(&mut self, _data: &'a TableData<'a>) {}
    fn visit_autolink(&mut self, _data: &'a AutolinkData<'a>) {}
    fn visit_inline_image(&mut self, _data: &'a InlineImageData<'a>) {}
    fn visit_reference_style_image(&mut self, _data: &'a ReferenceStyleImageData<'a>) {}
    fn visit_reference_definition(&mut self, _data: &'a ReferenceDefinitionData<'a>) {}
    fn visit_hidden_html_div(&mut self, _data: &'a HiddenHtmlDivData<'a>) {}
    fn visit_html_comment(&mut self, _data: &'a HtmlCommentData<'a>) {}
    fn visit_custom_directive(&mut self, _data: &'a DirectiveData<'a>) {}

    // --- Container Nodes (Recursive by default) ---
    fn visit_paragraph(&mut self, data: &'a ParagraphData<'a>) {
        for element in &data.elements {
            self.visit_element(element);
        }
    }

    fn visit_blockquote(&mut self, data: &'a BlockquoteData<'a>) {
        for element in &data.elements {
            self.visit_element(element);
        }
    }

    fn visit_list(&mut self, data: &'a ListData<'a>) {
        for item in &data.items {
            self.visit_element(item);
        }
    }

    fn visit_list_item(&mut self, data: &'a ListItemData<'a>) {
        for element in &data.elements {
            self.visit_element(element);
        }
    }

    fn visit_strong(&mut self, data: &'a StrongData<'a>) {
        for element in &data.elements {
            self.visit_element(element);
        }
    }

    fn visit_emphasis(&mut self, data: &'a EmphasisData<'a>) {
        for element in &data.elements {
            self.visit_element(element);
        }
    }

    fn visit_strikethrough(&mut self, data: &'a StrikethroughData<'a>) {
        for element in &data.elements {
            self.visit_element(element);
        }
    }
}

pub trait Visitable<'a> {
    fn accept(&'a self, visitor: &mut dyn Visitor<'a>);
}

impl<'a> Visitable<'a> for Element<'a> {
    fn accept(&'a self, visitor: &mut dyn Visitor<'a>) {
        visitor.visit_element(self);
    }
}

impl<'a> Visitable<'a> for Document<'a> {
    fn accept(&'a self, visitor: &mut dyn Visitor<'a>) {
        visitor.visit_document(self);
    }
}

impl<'a> Visitable<'a> for Documents<'a> {
    fn accept(&'a self, visitor: &mut dyn Visitor<'a>) {
        visitor.visit_documents(self);
    }
}
