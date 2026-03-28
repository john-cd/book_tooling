// Note on old code: This module was moved to `book_tool`.
// It manages transforming inline/naked links to reference-style links.

use std::collections::BTreeMap;
use std::fmt::Write;

use parse_lib::visit::pretty_printer::PrettyPrinter;
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
    fn visit_autolink(&mut self, data: &'v AutolinkData<'v>) {
        // Skip GitHub issues as requested in requirements
        if data.url.contains("github.com") && data.url.contains("/issues/") {
            let _ = write!(self.output, "<{}>", data.url);
            return;
        }

        let label = self.add_refdef(data.url, None);
        let _ = write!(self.output, "[{}][{}]", data.url, label);
    }

    fn visit_inline_link(&mut self, data: &'v InlineLinkData<'v>) {
        // Skip GitHub issues
        if data.url.contains("github.com") && data.url.contains("/issues/") {
            let _ = write!(self.output, "[{}]({})", data.text, data.url);
            return;
        }

        let label = self.add_refdef(data.url, Some(data.text));
        let _ = write!(self.output, "[{}][{}]", data.text, label);
    }

    fn visit_reference_style_link(&mut self, data: &'v ReferenceStyleLinkData<'v>) {
        let _ = write!(self.output, "[{}][{}]", data.text, data.label);
    }

    fn visit_inline_image(&mut self, data: &'v InlineImageData<'v>) {
        let label = self.add_refdef(data.url, Some(data.image_description));
        let _ = write!(self.output, "![{}][{}]", data.image_description, label);
    }

    fn visit_reference_style_image(&mut self, data: &'v ReferenceStyleImageData<'v>) {
        let _ = write!(self.output, "![{}][{}]", data.image_description, data.label);
    }

    fn visit_reference_definition(&mut self, data: &'v ReferenceDefinitionData<'v>) {
        // Collect existing refdefs
        self.refdefs
            .insert(data.label.to_string(), data.url.to_string());
    }

    fn visit_wikilink(&mut self, data: &'v WikiLinkData<'v>) {
        let mut pp = PrettyPrinter::new(&mut self.output);
        pp.visit_wikilink(data)
    }

    fn visit_code_span(&mut self, data: &'v CodeSpanData<'v>) {
        let _ = write!(self.output, "`{}`", data.content);
    }

    fn visit_fenced_code_block(&mut self, data: &'v FencedCodeBlockData<'v>) {
        let _ = write!(self.output, "```");
        if let Some(lang) = data.language {
            let _ = write!(self.output, "{}", lang);
        }
        let _ = writeln!(self.output);
        for line in &data.lines {
            let _ = writeln!(self.output, "{}", line);
        }
        let _ = writeln!(self.output, "```");
    }

    fn visit_hidden_html_div(&mut self, data: &'v HiddenHtmlDivData<'v>) {
        let _ = write!(
            self.output,
            "<div class=\"hidden\">\n{}\n</div>",
            data.content
        );
    }

    fn visit_heading(&mut self, data: &'v HeadingData<'v>) {
        let mut pp = PrettyPrinter::new(&mut self.output);
        pp.visit_heading(data);
    }

    fn visit_text(&mut self, data: &'v TextData<'v>) {
        let _ = write!(self.output, "{}", data.content);
    }

    fn visit_custom_directive(&mut self, data: &'v DirectiveData<'v>) {
        let mut pp = PrettyPrinter::new(&mut self.output);
        pp.visit_custom_directive(data);
    }
}
