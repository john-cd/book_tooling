// TODO Reviewed ??-??-2026
use std::fmt;

use super::Visitable as _;
use super::Visitor;
use crate::ast::{
    AutolinkData, CodeSpanData, DirectiveData, Document, Element, FencedCodeBlockData, HeadingData,
    HiddenHtmlDivData, InlineImageData, InlineLinkData, ReferenceDefinitionData,
    ReferenceStyleImageData, ReferenceStyleLinkData, TableData, TaskItemData, TextData,
    WikiLinkData,
};

/// A visitor that reconstructs a Markdown string from an AST.
///
/// It aims for a faithful, but potentially normalized, representation of the
/// original document.
pub struct PrettyPrinter<W>
where
    W: fmt::Write,
{
    /// Destination that receives the reconstructed Markdown.
    w: W,
}

impl<W: fmt::Write> PrettyPrinter<W> {
    /// Construct a new `PrettyPrinter`.
    pub fn new(w: W) -> Self {
        Self { w }
    }

    /// Writes formatted text to the sink.
    ///
    /// Write failures are deliberately ignored because the `Visitor` methods cannot return errors.
    fn emit(&mut self, args: fmt::Arguments<'_>) {
        let _unused = self.w.write_fmt(args);
    }

    /// Writes formatted text followed by a newline; see [`Self::emit`].
    fn emit_line(&mut self, args: fmt::Arguments<'_>) {
        let _unused = self
            .w
            .write_fmt(args)
            .and_then(|()| self.w.write_char('\n'));
    }

    /// Writes a newline; see [`Self::emit`].
    fn emit_newline(&mut self) {
        let _unused = self.w.write_char('\n');
    }
}

impl<'a, W: fmt::Write + 'a> Visitor<'a> for PrettyPrinter<W> {
    fn visit_autolink(&mut self, link: &AutolinkData<'a>) {
        self.emit(format_args!("<{}>", link.url));
    }

    fn visit_inline_link(&mut self, link: &InlineLinkData<'a>) {
        if let Some(title) = link.title {
            self.emit(format_args!("[{}]({} \"{title}\")", link.text, link.url));
        } else {
            self.emit(format_args!("[{}]({})", link.text, link.url));
        }
    }

    fn visit_reference_style_link(&mut self, link: &ReferenceStyleLinkData<'a>) {
        self.emit(format_args!("[{}][{}]", link.text, link.label));
    }

    fn visit_inline_image(&mut self, img: &InlineImageData<'a>) {
        if let Some(title) = img.title {
            self.emit(format_args!(
                "![{}]({} \"{title}\")",
                img.image_description, img.url
            ));
        } else {
            self.emit(format_args!("![{}]({})", img.image_description, img.url));
        }
    }

    fn visit_reference_style_image(&mut self, img: &ReferenceStyleImageData<'a>) {
        self.emit(format_args!("![{}][{}]", img.image_description, img.label));
    }

    fn visit_reference_definition(&mut self, refdef: &'a ReferenceDefinitionData<'a>) {
        if let Some(title) = refdef.title {
            self.emit_line(format_args!(
                "[{}]: {} \"{title}\"",
                refdef.label, refdef.url
            ));
        } else {
            self.emit_line(format_args!("[{}]: {}", refdef.label, refdef.url));
        }
    }

    fn visit_wikilink(&mut self, link: &WikiLinkData<'a>) {
        let immediately_after = link.immediately_after.unwrap_or("");
        if let Some(display) = link.display {
            self.emit(format_args!(
                "[[{} | {display}]]{immediately_after}",
                link.target
            ));
        } else {
            self.emit(format_args!("[[{}]]{immediately_after}", link.target));
        }
    }

    fn visit_code_span(&mut self, code_span: &CodeSpanData<'a>) {
        self.emit(format_args!("`{}`", code_span.content));
    }

    fn visit_fenced_code_block(&mut self, fenced_code_block: &FencedCodeBlockData<'a>) {
        let lang = fenced_code_block.language.unwrap_or("");
        self.emit_line(format_args!("```{lang}"));
        for line in &fenced_code_block.lines {
            self.emit_line(format_args!("{line}"));
        }
        self.emit_line(format_args!("```"));
    }

    fn visit_hidden_html_div(&mut self, div: &HiddenHtmlDivData<'a>) {
        self.emit_line(format_args!("<div class=\"hidden\">"));
        self.emit_line(format_args!("{}", div.content));
        self.emit_line(format_args!("</div>"));
    }

    fn visit_heading(&mut self, heading: &HeadingData<'a>) {
        let hashes = "#".repeat(usize::from(heading.level));
        let content = heading
            .content
            .map_or_else(String::new, |c| format!(" {c}"));
        if let Some(id) = heading.id {
            self.emit_line(format_args!("{hashes}{content} {{#{id}}}"));
        } else {
            self.emit_line(format_args!("{hashes}{content}"));
        }
    }

    fn visit_text(&mut self, text: &TextData<'a>) {
        self.emit(format_args!("{}", text.content));
    }

    fn visit_custom_directive(&mut self, directive: &DirectiveData<'a>) {
        self.emit(format_args!("{directive}"));
    }

    fn visit_task_item(&mut self, task: &TaskItemData<'a>) {
        let check = if task.completed { 'x' } else { ' ' };
        self.emit_line(format_args!("- [{check}] {}", task.content));
    }

    fn visit_table(&mut self, data: &TableData<'a>) {
        let num_columns = data.header.len();
        if num_columns == 0 {
            return;
        }
        let mut max_widths = vec![0; num_columns];

        for (i, width_ref) in max_widths.iter_mut().enumerate() {
            let mut max_width = data.header.get(i).map_or(0, |s| s.len());
            for row in &data.rows {
                if let Some(cell) = row.get(i) {
                    max_width = max_width.max(cell.len());
                }
            }
            *width_ref = max_width.max(3);
        }

        self.emit(format_args!("|"));
        for (cell, width) in data.header.iter().zip(&max_widths) {
            self.emit(format_args!(" {:<width$} |", cell, width = *width));
        }
        self.emit_newline();

        self.emit(format_args!("|"));
        for width in &max_widths {
            self.emit(format_args!(" {:-<width$} |", "", width = *width));
        }
        self.emit_newline();

        for row in &data.rows {
            self.emit(format_args!("|"));
            for (i, &width) in max_widths.iter().enumerate() {
                let cell = row.get(i).unwrap_or(&"");
                self.emit(format_args!(" {cell:<width$} |"));
            }
            self.emit_newline();
        }
    }

    fn visit_horizontal_rule(&mut self) {
        self.emit_line(format_args!("---"));
    }
}

impl fmt::Display for Element<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut pp = PrettyPrinter::new(f);
        self.accept(&mut pp);
        Ok(())
    }
}

impl fmt::Display for Document<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut pp = PrettyPrinter::new(f);
        self.accept(&mut pp);
        Ok(())
    }
}
