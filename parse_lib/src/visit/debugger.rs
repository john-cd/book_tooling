//! A visitor that prints each element to the console, for debugging.
//!
//! TODO Reviewed ??-??-2026.

use super::Visitor;
use crate::ast::AutolinkData;
use crate::ast::CodeSpanData;
use crate::ast::DirectiveData;
use crate::ast::FencedCodeBlockData;
use crate::ast::HeadingData;
use crate::ast::HiddenHtmlDivData;
use crate::ast::InlineImageData;
use crate::ast::InlineLinkData;
use crate::ast::ReferenceDefinitionData;
use crate::ast::ReferenceStyleImageData;
use crate::ast::ReferenceStyleLinkData;
use crate::ast::TextData;
use crate::ast::WikiLinkData;

/// A simple visitor that prints the elements to the console.
pub struct Debugger;

impl Debugger {
    /// Construct a new Debugger.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for Debugger {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Visitor<'a> for Debugger {
    /// Prints the autolink URL.
    fn visit_autolink(&mut self, autolink: &AutolinkData<'a>) {
        let url = &autolink.url;
        println!("Autolink: <{url}>");
    }

    /// Prints the link text and URL, and the title when present.
    fn visit_inline_link(&mut self, link: &InlineLinkData<'a>) {
        if let Some(title) = link.title {
            println!("Inline link: [{}]({} \"{title}\")", link.text, link.url);
        } else {
            println!("Inline link: [{}]({})", link.text, link.url);
        }
    }

    /// Prints the link text and its reference label.
    fn visit_reference_style_link(&mut self, link: &ReferenceStyleLinkData<'a>) {
        println!("Reference style link: [{}][{}]", link.text, link.label);
    }

    /// Prints the image description and URL, and the title when present.
    fn visit_inline_image(&mut self, img: &InlineImageData<'a>) {
        if let Some(title) = img.title {
            println!(
                "Inline image: ![{}]({} \"{title}\")",
                img.image_description, img.url
            );
        } else {
            println!("Inline image: ![{}]({})", img.image_description, img.url);
        }
    }

    /// Prints the image description and its reference label.
    fn visit_reference_style_image(&mut self, img: &ReferenceStyleImageData<'a>) {
        println!(
            "Reference style image: ![{}][{}]",
            img.image_description, img.label
        );
    }

    /// Prints the definition label and URL, and the title when present.
    fn visit_reference_definition(&mut self, refdef: &ReferenceDefinitionData<'a>) {
        if let Some(title) = refdef.title {
            println!(
                "Reference definition: [{}]: {} \"{title}\"",
                refdef.label, refdef.url
            );
        } else {
            println!("Reference definition: [{}]: {}", refdef.label, refdef.url);
        }
    }

    /// Prints the wikilink display text and any text attached after it.
    fn visit_wikilink(&mut self, link: &WikiLinkData<'a>) {
        let display_text = link.display.unwrap_or(link.target);
        let after_text = link.immediately_after.unwrap_or("");
        println!(
            "Wiki link: [[{} | {display_text}]]{after_text}",
            link.target
        );
    }

    /// Prints the code span content.
    fn visit_code_span(&mut self, code_span: &CodeSpanData<'a>) {
        println!("Code span: `{}`", code_span.content);
    }

    /// Prints the number of lines in the code block.
    fn visit_fenced_code_block(&mut self, fenced_code_block: &FencedCodeBlockData<'a>) {
        println!(
            "Fenced code block with {} lines.",
            fenced_code_block.lines.len()
        );
    }

    /// Prints the content of the hidden HTML div.
    fn visit_hidden_html_div(&mut self, div: &HiddenHtmlDivData<'a>) {
        println!(
            "Hidden HTML div: <div class=\"hidden\">{}</div>",
            div.content
        );
    }

    /// Prints the heading level, text and explicit id when present.
    fn visit_heading(&mut self, heading: &HeadingData<'a>) {
        let id_text = heading.id.map_or(String::new(), |id| format!(" {{#{id}}}"));
        if let Some(content) = heading.content {
            println!("Heading {}: {content}{id_text}", heading.level);
        } else {
            println!("Heading {}: (no content){id_text}", heading.level);
        }
    }

    /// Prints the text content.
    fn visit_text(&mut self, text: &TextData<'a>) {
        println!("Text: {}", text.content);
    }

    /// Prints the directive in `Debug` form.
    fn visit_custom_directive(&mut self, directive: &DirectiveData<'a>) {
        println!("Custom directive: {directive:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_document;
    use crate::visit::Visitable as _;

    /// Verifies: default and new are equivalent unit visitors.
    #[test]
    fn default_and_new_are_equivalent_unit_visitors() {
        let _first = Debugger::new();
        let _second = <Debugger as Default>::default();
    }

    /// The visitor prints to stdout; this exercises every element kind to make sure it never
    /// panics.
    #[test]
    fn visits_every_element_kind_without_panicking() {
        let source = concat!(
            "# Heading {#id}\n\n",
            "[inline](https://x.example \"t\") [ref][label] ![img](p.png \"t\") ![r][label]\n\n",
            "<https://auto.example> `code` [[wiki | shown]] {{#crate serde}}\n\n",
            "```rust\nfn f() {}\n```\n\n",
            "[label]: https://x.example \"title\"\n",
        );
        let doc = parse_document("t", None, source).expect("parses");
        doc.accept(&mut Debugger::new());
    }
}
