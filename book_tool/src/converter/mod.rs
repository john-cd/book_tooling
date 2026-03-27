// Note on old code: This module was moved to `book_tool`.
// It manages transforming inline/naked links to reference-style links.

use std::collections::BTreeMap;
use std::fmt::Write;

use parse_lib::visit::PrettyPrinter;
use parse_lib::visit::Visitor;
use parse_lib::*;

/// Transformation visitor that converts inline and naked links to reference-style links.
pub struct LinkConverter {
    pub refdefs: BTreeMap<String, String>,
    pub output: String,
    pub next_id: usize,
}

impl LinkConverter {
    pub fn new() -> Self {
        Self {
            refdefs: BTreeMap::new(),
            output: String::new(),
            next_id: 1,
        }
    }

    fn add_refdef(&mut self, url: &str, preferred_label: Option<&str>) -> String {
        // Check if URL already has a label.
        for (label, existing_url) in &self.refdefs {
            if existing_url == url {
                return label.clone();
            }
        }

        let label = if let Some(pref) = preferred_label {
            let sanitized = pref
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>();
            if sanitized.is_empty() || self.refdefs.contains_key(&sanitized) {
                let l = format!("link{}", self.next_id);
                self.next_id += 1;
                l
            } else {
                sanitized
            }
        } else {
            let l = format!("link{}", self.next_id);
            self.next_id += 1;
            l
        };

        self.refdefs.insert(label.clone(), url.to_string());
        label
    }
}

impl<'v> Visitor<'v> for LinkConverter {
    fn visit_autolink(&mut self, data: &'v AutolinkData) -> std::fmt::Result {
        // Skip GitHub issues as requested in requirements
        if data.url.contains("github.com") && data.url.contains("/issues/") {
            return write!(self.output, "<{}>", data.url);
        }

        let label = self.add_refdef(data.url, None);
        write!(self.output, "[{}][{}]", data.url, label)
    }

    fn visit_inline_link(&mut self, data: &'v InlineLinkData) -> std::fmt::Result {
        // Skip GitHub issues
        if data.url.contains("github.com") && data.url.contains("/issues/") {
            return write!(self.output, "[{}]({})", data.text, data.url);
        }

        let label = self.add_refdef(data.url, Some(data.text));
        write!(self.output, "[{}][{}]", data.text, label)
    }

    fn visit_reference_style_link(&mut self, data: &'v ReferenceStyleLinkData) -> std::fmt::Result {
        write!(self.output, "[{}][{}]", data.text, data.label)
    }

    fn visit_inline_image(&mut self, data: &'v InlineImageData) -> std::fmt::Result {
        let label = self.add_refdef(data.url, Some(data.image_description));
        write!(self.output, "![{}][{}]", data.image_description, label)
    }

    fn visit_reference_style_image(
        &mut self,
        data: &'v ReferenceStyleImageData,
    ) -> std::fmt::Result {
        write!(self.output, "![{}][{}]", data.image_description, data.label)
    }

    fn visit_reference_definition(
        &mut self,
        data: &'v ReferenceDefinitionData,
    ) -> std::fmt::Result {
        // Collect existing refdefs
        self.refdefs
            .insert(data.label.to_string(), data.url.to_string());
        Ok(())
    }

    fn visit_wiki_link(&mut self, data: &'v WikiLinkData) -> std::fmt::Result {
        let mut pp = PrettyPrinter::new(&mut self.output);
        pp.visit_wiki_link(data)
    }

    fn visit_code_span(&mut self, data: &'v CodeSpanData) -> std::fmt::Result {
        write!(self.output, "`{}`", data.content)
    }

    fn visit_fenced_code_block(&mut self, data: &'v FencedCodeBlockData) -> std::fmt::Result {
        write!(self.output, "```\n{}\n```", data.content)
    }

    fn visit_hidden_html_div(&mut self, data: &'v HiddenHtmlDivData) -> std::fmt::Result {
        write!(
            self.output,
            "<div class=\"hidden\">\n{}\n</div>",
            data.content
        )
    }

    fn visit_heading(&mut self, data: &'v HeadingData) -> std::fmt::Result {
        let mut pp = PrettyPrinter::new(&mut self.output);
        pp.visit_heading(data)?;
        writeln!(self.output)
    }

    fn visit_text(&mut self, data: &'v TextData) -> std::fmt::Result {
        write!(self.output, "{}", data.content)
    }

    fn visit_custom_directive(&mut self, data: &'v DirectiveData) -> std::fmt::Result {
        write!(self.output, "{}", data)
    }
}
