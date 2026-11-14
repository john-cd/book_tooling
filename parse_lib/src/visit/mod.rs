//! AST visitor traits and helper visitors for traversing Markdown documents.
//!
//! TODO Reviewed ??-??-2026.

/// Debug-print visitor implementations.
mod debugger;

/// I/O-oriented helpers for visitor-based workflows.
mod io;

/// Markdown reconstruction visitor implementations.
mod pretty_printer;

/// Reference-definition analysis visitors.
mod refdefs;

pub use debugger::*;
pub use pretty_printer::*;
pub use refdefs::*;

use crate::Document;
use crate::Documents;
use crate::ast::AutolinkData;
use crate::ast::BlockquoteData;
use crate::ast::CodeSpanData;
use crate::ast::DirectiveData;
use crate::ast::Element;
use crate::ast::EmphasisData;
use crate::ast::FencedCodeBlockData;
use crate::ast::HeadingData;
use crate::ast::HiddenHtmlDivData;
use crate::ast::HtmlCommentData;
use crate::ast::InlineImageData;
use crate::ast::InlineLinkData;
use crate::ast::ListData;
use crate::ast::ListItemData;
use crate::ast::ParagraphData;
use crate::ast::ReferenceDefinitionData;
use crate::ast::ReferenceStyleImageData;
use crate::ast::ReferenceStyleLinkData;
use crate::ast::StrikethroughData;
use crate::ast::StrongData;
use crate::ast::TableData;
use crate::ast::TaskItemData;
use crate::ast::TextData;
use crate::ast::WikiLinkData;

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
    /// Visits a text node; the default implementation does nothing.
    fn visit_text(&mut self, _data: &'a TextData<'a>) {}
    /// Visits a horizontal rule node; the default implementation does nothing.
    fn visit_horizontal_rule(&mut self) {}
    /// Visits a fenced code block node; the default implementation does nothing.
    fn visit_fenced_code_block(&mut self, _data: &'a FencedCodeBlockData<'a>) {}
    /// Visits a code span node; the default implementation does nothing.
    fn visit_code_span(&mut self, _data: &'a CodeSpanData<'a>) {}
    /// Visits an inline link node; the default implementation does nothing.
    fn visit_inline_link(&mut self, _data: &'a InlineLinkData<'a>) {}
    /// Visits a reference style link node; the default implementation does nothing.
    fn visit_reference_style_link(&mut self, _data: &'a ReferenceStyleLinkData<'a>) {}
    /// Visits a wikilink node; the default implementation does nothing.
    fn visit_wikilink(&mut self, _data: &'a WikiLinkData<'a>) {}
    /// Visits a heading node; the default implementation does nothing.
    fn visit_heading(&mut self, _data: &'a HeadingData<'a>) {}
    /// Visits a task item node; the default implementation does nothing.
    fn visit_task_item(&mut self, _data: &'a TaskItemData<'a>) {}
    /// Visits a table node by visiting the inline elements in its cells.
    fn visit_table(&mut self, data: &'a TableData<'a>) {
        self.visit_table_cells(data);
    }

    /// Visits the inline elements in each table cell.
    fn visit_table_cells(&mut self, data: &'a TableData<'a>) {
        for cell in &data.header_elements {
            for element in cell {
                self.visit_element(element);
            }
        }
        for row in &data.row_elements {
            for cell in row {
                for element in cell {
                    self.visit_element(element);
                }
            }
        }
    }

    /// Visits an autolink node; the default implementation does nothing.
    fn visit_autolink(&mut self, _data: &'a AutolinkData<'a>) {}
    /// Visits an inline image node; the default implementation does nothing.
    fn visit_inline_image(&mut self, _data: &'a InlineImageData<'a>) {}
    /// Visits a reference style image node; the default implementation does nothing.
    fn visit_reference_style_image(&mut self, _data: &'a ReferenceStyleImageData<'a>) {}
    /// Visits a reference definition node; the default implementation does nothing.
    fn visit_reference_definition(&mut self, _data: &'a ReferenceDefinitionData<'a>) {}
    /// Visits a hidden html div node; the default implementation does nothing.
    fn visit_hidden_html_div(&mut self, _data: &'a HiddenHtmlDivData<'a>) {}
    /// Visits a html comment node; the default implementation does nothing.
    fn visit_html_comment(&mut self, _data: &'a HtmlCommentData<'a>) {}
    /// Visits a custom directive node; the default implementation does nothing.
    fn visit_custom_directive(&mut self, _data: &'a DirectiveData<'a>) {}

    // --- Container Nodes (Recursive by default) ---
    /// Visits a paragraph node, then recurses into its children by default.
    fn visit_paragraph(&mut self, data: &'a ParagraphData<'a>) {
        for element in &data.elements {
            self.visit_element(element);
        }
    }

    /// Visits a blockquote node, then recurses into its children by default.
    fn visit_blockquote(&mut self, data: &'a BlockquoteData<'a>) {
        for element in &data.elements {
            self.visit_element(element);
        }
    }

    /// Visits a list node, then recurses into its children by default.
    fn visit_list(&mut self, data: &'a ListData<'a>) {
        for item in &data.items {
            self.visit_element(item);
        }
    }

    /// Visits a list item node, then recurses into its children by default.
    fn visit_list_item(&mut self, data: &'a ListItemData<'a>) {
        for element in &data.elements {
            self.visit_element(element);
        }
    }

    /// Visits a strong-emphasis node, then recurses into its children by default.
    fn visit_strong(&mut self, data: &'a StrongData<'a>) {
        for element in &data.elements {
            self.visit_element(element);
        }
    }

    /// Visits an emphasis node, then recurses into its children by default.
    fn visit_emphasis(&mut self, data: &'a EmphasisData<'a>) {
        for element in &data.elements {
            self.visit_element(element);
        }
    }

    /// Visits a strikethrough node, then recurses into its children by default.
    fn visit_strikethrough(&mut self, data: &'a StrikethroughData<'a>) {
        for element in &data.elements {
            self.visit_element(element);
        }
    }
}

/// Trait implemented by AST nodes that can accept a [`Visitor`].
pub trait Visitable<'a> {
    /// Accepts a visitor and dispatches to the appropriate traversal entry point.
    fn accept(&'a self, visitor: &mut dyn Visitor<'a>);
}

impl<'a> Visitable<'a> for Element<'a> {
    /// Hands this element to [`Visitor::visit_element`].
    fn accept(&'a self, visitor: &mut dyn Visitor<'a>) {
        visitor.visit_element(self);
    }
}

impl<'a> Visitable<'a> for Document<'a> {
    /// Hands this document to [`Visitor::visit_document`].
    fn accept(&'a self, visitor: &mut dyn Visitor<'a>) {
        visitor.visit_document(self);
    }
}

impl<'a> Visitable<'a> for Documents<'a> {
    /// Hands this document collection to [`Visitor::visit_documents`].
    fn accept(&'a self, visitor: &mut dyn Visitor<'a>) {
        visitor.visit_documents(self);
    }
}
