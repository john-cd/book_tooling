use super::Visitor;
use crate::ast::*;

/// A simple visitor that prints the elements to the console.
pub struct Debugger;

impl Debugger {
    /// Construct a new Debugger.
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
    fn visit_autolink(&mut self, autolink: &AutolinkData<'a>) {
        let url = &autolink.url;
        println!("Autolink: <{url}>");
    }

    fn visit_inline_link(&mut self, link: &InlineLinkData<'a>) {
        if let Some(title) = link.title {
            println!("Inline link: [{}]({} \"{title}\")", link.text, link.url);
        } else {
            println!("Inline link: [{}]({})", link.text, link.url);
        }
    }

    fn visit_reference_style_link(&mut self, link: &ReferenceStyleLinkData<'a>) {
        println!("Reference style link: [{}][{}]", link.text, link.label);
    }

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

    fn visit_reference_style_image(&mut self, img: &ReferenceStyleImageData<'a>) {
        println!(
            "Reference style image: ![{}][{}]",
            img.image_description, img.label
        );
    }

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

    fn visit_wikilink(&mut self, link: &WikiLinkData<'a>) {
        let display_text = link.display.unwrap_or(link.target);
        let after_text = link.immediately_after.unwrap_or("");
        println!(
            "Wiki link: [[{} | {display_text}]]{after_text}",
            link.target
        );
    }

    fn visit_code_span(&mut self, code_span: &CodeSpanData<'a>) {
        println!("Code span: `{}`", code_span.content);
    }

    fn visit_fenced_code_block(&mut self, fenced_code_block: &FencedCodeBlockData<'a>) {
        println!(
            "Fenced code block with {} lines.",
            fenced_code_block.lines.len()
        );
    }

    fn visit_hidden_html_div(&mut self, div: &HiddenHtmlDivData<'a>) {
        println!(
            "Hidden HTML div: <div class=\"hidden\">{}</div>",
            div.content
        );
    }

    fn visit_heading(&mut self, heading: &HeadingData<'a>) {
        let id_text = heading.id.map_or(String::new(), |id| format!(" {{#{id}}}"));
        if let Some(content) = heading.content {
            println!("Heading {}: {content}{id_text}", heading.level);
        } else {
            println!("Heading {}: (no content){id_text}", heading.level);
        }
    }

    fn visit_text(&mut self, text: &TextData<'a>) {
        println!("Text: {}", text.content);
    }

    fn visit_custom_directive(&mut self, directive: &DirectiveData<'a>) {
        println!("Custom directive: {directive:?}");
    }
}
