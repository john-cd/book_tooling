//! A visitor that pretty-prints the AST to any writer.
//!
//! TODO Reviewed ??-??-2026

use std::fmt;

use super::Visitable as _;
use super::Visitor;
use crate::ast::AutolinkData;
use crate::ast::CodeSpanData;
use crate::ast::DirectiveData;
use crate::ast::Document;
use crate::ast::Element;
use crate::ast::FencedCodeBlockData;
use crate::ast::HeadingData;
use crate::ast::HiddenHtmlDivData;
use crate::ast::InlineImageData;
use crate::ast::InlineLinkData;
use crate::ast::ReferenceDefinitionData;
use crate::ast::ReferenceStyleImageData;
use crate::ast::ReferenceStyleLinkData;
use crate::ast::TableData;
use crate::ast::TaskItemData;
use crate::ast::TextData;
use crate::ast::WikiLinkData;

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
    /// Prints `<url>`.
    fn visit_autolink(&mut self, link: &AutolinkData<'a>) {
        self.emit(format_args!("<{}>", link.url));
    }

    /// Prints `[text](url)`, with the title in quotes when present.
    fn visit_inline_link(&mut self, link: &InlineLinkData<'a>) {
        if let Some(title) = link.title {
            self.emit(format_args!("[{}]({} \"{title}\")", link.text, link.url));
        } else {
            self.emit(format_args!("[{}]({})", link.text, link.url));
        }
    }

    /// Prints `[text][label]`.
    fn visit_reference_style_link(&mut self, link: &ReferenceStyleLinkData<'a>) {
        self.emit(format_args!("[{}][{}]", link.text, link.label));
    }

    /// Prints `![description](url)`, with the title in quotes when present.
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

    /// Prints `![description][label]`.
    fn visit_reference_style_image(&mut self, img: &ReferenceStyleImageData<'a>) {
        self.emit(format_args!("![{}][{}]", img.image_description, img.label));
    }

    /// Prints `[label]: url` on its own line, with the title in quotes when present.
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

    /// Prints `[[target]]` or `[[target|display]]` followed by any text attached after it.
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

    /// Prints the content between backticks.
    fn visit_code_span(&mut self, code_span: &CodeSpanData<'a>) {
        self.emit(format_args!("`{}`", code_span.content));
    }

    /// Prints the lines between a fence carrying the language tag and a closing fence.
    fn visit_fenced_code_block(&mut self, fenced_code_block: &FencedCodeBlockData<'a>) {
        let lang = fenced_code_block.language.unwrap_or("");
        self.emit_line(format_args!("```{lang}"));
        for line in &fenced_code_block.lines {
            self.emit_line(format_args!("{line}"));
        }
        self.emit_line(format_args!("```"));
    }

    /// Prints the content inside a `<div class="hidden">` element, one line each for the tags and
    /// the content.
    fn visit_hidden_html_div(&mut self, div: &HiddenHtmlDivData<'a>) {
        self.emit_line(format_args!("<div class=\"hidden\">"));
        self.emit_line(format_args!("{}", div.content));
        self.emit_line(format_args!("</div>"));
    }

    /// Prints the heading with its `#` markers and its `{#id}` when present.
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

    /// Prints the text unchanged.
    fn visit_text(&mut self, text: &TextData<'a>) {
        self.emit(format_args!("{}", text.content));
    }

    /// Prints the directive in its source form.
    fn visit_custom_directive(&mut self, directive: &DirectiveData<'a>) {
        self.emit(format_args!("{directive}"));
    }

    /// Prints `- [x]` or `- [ ]` followed by the task text, on its own line.
    fn visit_task_item(&mut self, task: &TaskItemData<'a>) {
        let check = if task.completed { 'x' } else { ' ' };
        self.emit_line(format_args!("- [{check}] {}", task.content));
    }

    /// Prints a pipe table; a table without columns prints nothing.
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

    /// Prints `---` on its own line.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_document;

    /// Verifies: print.
    fn print(source: &str) -> String {
        parse_document("t", None, source)
            .expect("parses")
            .to_string()
    }

    /// Verifies: links images and titles.
    #[test]
    fn links_images_and_titles() {
        let out = print("[a](https://x.example \"T\") ![i](p.png) <https://auto.example>\n");
        assert!(out.contains("[a](https://x.example \"T\")"), "{out}");
        assert!(out.contains("![i](p.png)"), "{out}");
        assert!(out.contains("<https://auto.example>"), "{out}");
    }

    /// Verifies: reference definition is on its own line.
    #[test]
    fn reference_definition_is_on_its_own_line() {
        let out = print("[d]: https://x.example \"Title\"\n");
        assert_eq!(out.trim_end(), "[d]: https://x.example \"Title\"");
    }

    /// Verifies: heading levels and ids.
    #[test]
    fn heading_levels_and_ids() {
        let out = print("## Two {#anchor}\n\n### Three\n");
        assert!(out.contains("## Two {#anchor}\n"), "{out}");
        assert!(out.contains("### Three\n"), "{out}");
    }

    /// Verifies: fenced code block keeps language and lines.
    #[test]
    fn fenced_code_block_keeps_language_and_lines() {
        let out = print("```rust\nfn main() {}\n```\n");
        assert_eq!(out.trim_end(), "```rust\nfn main() {}\n```");
    }

    /// Verifies: table columns are padded to the widest cell.
    #[test]
    fn table_columns_are_padded_to_the_widest_cell() {
        let out = print("| a | bbbbb |\n|---|---|\n| longer | c |\n");
        let lines: Vec<_> = out.trim_end().lines().collect();
        assert_eq!(lines.len(), 3, "{out}");
        assert!(
            lines.iter().all(|line| line.len() == lines[0].len()),
            "{out}"
        );
        assert!(lines[1].contains("| ------ | ----- |"), "{out}");
    }

    /// Verifies: writes into any fmt writer and ignores sink failures.
    #[test]
    fn writes_into_any_fmt_writer_and_ignores_sink_failures() {
        struct Failing;
        impl fmt::Write for Failing {
            /// Verifies: write str.
            fn write_str(&mut self, _text: &str) -> fmt::Result {
                Err(fmt::Error)
            }
        }
        let doc = parse_document("t", None, "# Title\n").expect("parses");
        let mut printer = PrettyPrinter::new(Failing);
        doc.accept(&mut printer);
    }

    /// Verifies: printing twice is stable.
    #[test]
    fn printing_twice_is_stable() {
        let source = "# H\n\n[a](https://x.example) text `code`\n\n[d]: https://d.example\n";
        let once = print(source);
        assert_eq!(print(&once), once);
    }
}
