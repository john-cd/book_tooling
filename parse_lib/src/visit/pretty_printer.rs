use std::fmt;

use fmt::Result;

use super::Visitor;
use crate::ast::*;

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

impl<W: fmt::Write> Visitor<'_> for PrettyPrinter<W> {
    fn visit_autolink(&mut self, link: &AutolinkData) -> Result {
        write!(self.w, "<{}>", link.url)?;
        Ok(())
    }

    fn visit_inline_link(&mut self, link: &InlineLinkData) -> Result {
        if let Some(title) = link.title {
            write!(self.w, "[{}]({} \"{title}\")", link.text, link.url)?;
        } else {
            write!(self.w, "[{}]({})", link.text, link.url)?;
        }
        Ok(())
    }

    fn visit_reference_style_link(&mut self, link: &ReferenceStyleLinkData) -> Result {
        write!(self.w, "[{}][{}]", link.text, link.label)?;
        Ok(())
    }

    fn visit_inline_image(&mut self, img: &InlineImageData) -> Result {
        if let Some(title) = img.title {
            write!(
                self.w,
                "![{}]({} \"{title}\")",
                img.image_description, img.url
            )?;
        } else {
            write!(self.w, "![{}]({})", img.image_description, img.url)?;
        }
        Ok(())
    }

    fn visit_reference_style_image(&mut self, img: &ReferenceStyleImageData) -> Result {
        write!(self.w, "![{}][{}]", img.image_description, img.label)?;
        Ok(())
    }

    fn visit_reference_definition(&mut self, refdef: &ReferenceDefinitionData) -> Result {
        if let Some(title) = refdef.title {
            writeln!(self.w, "[{}]: {} \"{title}\"", refdef.label, refdef.url)?;
        } else {
            writeln!(self.w, "[{}]: {}", refdef.label, refdef.url)?;
        }
        Ok(())
    }

    fn visit_wiki_link(&mut self, link: &WikiLinkData) -> Result {
        let immediately_after = link.immediately_after.unwrap_or("");
        if let Some(display) = link.display {
            write!(self.w, "[[{} | {display}]]{immediately_after}", link.target)?;
        } else {
            write!(self.w, "[[{}]]{immediately_after}", link.target)?;
        }
        Ok(())
    }

    fn visit_code_span(&mut self, code_span: &CodeSpanData) -> Result {
        write!(self.w, "`{}`", code_span.content)?;
        Ok(())
    }

    fn visit_fenced_code_block(&mut self, fenced_code_block: &FencedCodeBlockData) -> Result {
<<<<<<< HEAD
        write!(self.w, "```{}```", fenced_code_block.content)?;
=======
        writeln!(self.w, "```")?;
        writeln!(self.w, "{}", fenced_code_block.content)?;
        writeln!(self.w, "```")?;
>>>>>>> jules-3582745748344015560-6d4fedb4
        Ok(())
    }

    fn visit_hidden_html_div(&mut self, div: &HiddenHtmlDivData) -> Result {
<<<<<<< HEAD
        write!(self.w, "<div class=\"hidden\">{}</div>", div.content)?;
=======
        writeln!(self.w, "<div class=\"hidden\">")?;
        writeln!(self.w, "{}", div.content)?;
        writeln!(self.w, "</div>")?;
>>>>>>> jules-3582745748344015560-6d4fedb4
        Ok(())
    }

    fn visit_heading(&mut self, heading: &HeadingData) -> Result {
        let hashes = "#".repeat(heading.level as usize);
        let content = heading
            .content
            .map(|c| format!(" {c}"))
            .unwrap_or_else(|| "".into());
        if let Some(id) = heading.id {
            writeln!(self.w, "{hashes}{content} {{#{id}}}")?;
        } else {
            writeln!(self.w, "{hashes}{content}")?;
        }
        Ok(())
    }

    fn visit_text(&mut self, text: &TextData) -> Result {
        write!(self.w, "{}", text.content)?;
        Ok(())
    }

    fn visit_custom_directive(&mut self, directive: &DirectiveData) -> Result {
        write!(self.w, "{directive}")?;
        Ok(())
    }

    fn visit_task_item(&mut self, task: &TaskItemData) -> Result {
        let check = if task.completed { 'x' } else { ' ' };
        writeln!(self.w, "- [{check}] {}", task.content)?;
        Ok(())
    }

    fn visit_table(&mut self, table: &TableData) -> Result {
        writeln!(self.w, "| {} |", table.header.join(" | "))?;
        writeln!(
            self.w,
            "| {} |",
            table
                .header
                .iter()
                .map(|_| "---")
                .collect::<Vec<_>>()
                .join(" | ")
        )?;
        for row in &table.rows {
            writeln!(self.w, "| {} |", row.join(" | "))?;
        }
        Ok(())
    }
}

impl fmt::Display for Element<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut pp = PrettyPrinter::new(f);
        self.accept(&mut pp)?;
        Ok(())
    }
}

impl fmt::Display for Document<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut pp = PrettyPrinter::new(f);
        self.accept(&mut pp)?;
        Ok(())
    }
}
