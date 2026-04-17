use std::fmt;

use super::Visitable;
use super::Visitor;
use crate::ast::*;

/// A visitor that reconstructs a Markdown string from an AST.
///
/// It aims for a faithful, but potentially normalized, representation of the
/// original document.
pub struct PrettyPrinter<W>
where
    W: fmt::Write,
{
    w: W,
}

impl<W: fmt::Write> PrettyPrinter<W> {
    /// Construct a new `PrettyPrinter`.
    pub fn new(w: W) -> Self {
        Self { w }
    }
}

impl<'a, W: fmt::Write + 'a> Visitor<'a> for PrettyPrinter<W> {
    fn visit_autolink(&mut self, link: &AutolinkData<'a>) {
        let _ = write!(self.w, "<{}>", link.url);
    }

    fn visit_inline_link(&mut self, link: &InlineLinkData<'a>) {
        if let Some(title) = link.title {
            let _ = write!(self.w, "[{}]({} \"{title}\")", link.text, link.url);
        } else {
            let _ = write!(self.w, "[{}]({})", link.text, link.url);
        }
    }

    fn visit_reference_style_link(&mut self, link: &ReferenceStyleLinkData<'a>) {
        let _ = write!(self.w, "[{}][{}]", link.text, link.label);
    }

    fn visit_inline_image(&mut self, img: &InlineImageData<'a>) {
        if let Some(title) = img.title {
            let _ = write!(
                self.w,
                "![{}]({} \"{title}\")",
                img.image_description, img.url
            );
        } else {
            let _ = write!(self.w, "![{}]({})", img.image_description, img.url);
        }
    }

    fn visit_reference_style_image(&mut self, img: &ReferenceStyleImageData<'a>) {
        let _ = write!(self.w, "![{}][{}]", img.image_description, img.label);
    }

    fn visit_reference_definition(&mut self, refdef: &'a ReferenceDefinitionData<'a>) {
        if let Some(title) = refdef.title {
            let _ = writeln!(self.w, "[{}]: {} \"{title}\"", refdef.label, refdef.url);
        } else {
            let _ = writeln!(self.w, "[{}]: {}", refdef.label, refdef.url);
        }
    }

    fn visit_wikilink(&mut self, link: &WikiLinkData<'a>) {
        let immediately_after = link.immediately_after.unwrap_or("");
        if let Some(display) = link.display {
            let _ = write!(self.w, "[[{} | {display}]]{immediately_after}", link.target);
        } else {
            let _ = write!(self.w, "[[{}]]{immediately_after}", link.target);
        }
    }

    fn visit_code_span(&mut self, code_span: &CodeSpanData<'a>) {
        let _ = write!(self.w, "`{}`", code_span.content);
    }

    fn visit_fenced_code_block(&mut self, fenced_code_block: &FencedCodeBlockData<'a>) {
        let lang = fenced_code_block.language.unwrap_or("");
        let _ = writeln!(self.w, "```{lang}");
        for line in &fenced_code_block.lines {
            let _ = writeln!(self.w, "{}", line);
        }
        let _ = writeln!(self.w, "```");
    }

    fn visit_hidden_html_div(&mut self, div: &HiddenHtmlDivData<'a>) {
        let _ = writeln!(self.w, "<div class=\"hidden\">");
        let _ = writeln!(self.w, "{}", div.content);
        let _ = writeln!(self.w, "</div>");
    }

    fn visit_heading(&mut self, heading: &HeadingData<'a>) {
        let hashes = "#".repeat(heading.level as usize);
        let content = heading
            .content
            .map(|c| format!(" {c}"))
            .unwrap_or_else(|| "".into());
        if let Some(id) = heading.id {
            let _ = writeln!(self.w, "{hashes}{content} {{#{id}}}");
        } else {
            let _ = writeln!(self.w, "{hashes}{content}");
        }
    }

    fn visit_text(&mut self, text: &TextData<'a>) {
        let _ = write!(self.w, "{}", text.content);
    }

    fn visit_custom_directive(&mut self, directive: &DirectiveData<'a>) {
        let _ = write!(self.w, "{}", directive);
    }

    fn visit_task_item(&mut self, task: &TaskItemData<'a>) {
        let check = if task.completed { 'x' } else { ' ' };
        let _ = writeln!(self.w, "- [{check}] {}", task.content);
    }

    fn visit_table(&mut self, data: &TableData<'a>) {
        let num_columns = data.header.len();
        if num_columns == 0 {
            return;
        }
        let mut max_widths = vec![0; num_columns];

        for i in 0..num_columns {
            let mut max_width = data.header.get(i).map_or(0, |s| s.len());
            for row in &data.rows {
                if let Some(cell) = row.get(i) {
                    max_width = max_width.max(cell.len());
                }
            }
            max_widths[i] = max_width.max(3);
        }

        let _ = write!(self.w, "|");
        for (i, cell) in data.header.iter().enumerate() {
            let _ = write!(self.w, " {:<width$} |", cell, width = max_widths[i]);
        }
        let _ = writeln!(self.w);

        let _ = write!(self.w, "|");
        for width in &max_widths {
            let _ = write!(self.w, " {:-<width$} |", "", width = *width);
        }
        let _ = writeln!(self.w);

        for row in &data.rows {
            let _ = write!(self.w, "|");
            for i in 0..num_columns {
                let cell = row.get(i).unwrap_or(&"");
                let _ = write!(self.w, " {:<width$} |", cell, width = max_widths[i]);
            }
            let _ = writeln!(self.w);
        }
    }

    fn visit_horizontal_rule(&mut self) {
        let _ = writeln!(self.w, "---");
    }
}

impl<'a> fmt::Display for Element<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut pp = PrettyPrinter::new(f);
        self.accept(&mut pp);
        Ok(())
    }
}

impl<'a> fmt::Display for Document<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut pp = PrettyPrinter::new(f);
        self.accept(&mut pp);
        Ok(())
    }
}
