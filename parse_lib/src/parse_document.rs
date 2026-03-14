use winnow::ModalResult;
use winnow::Parser;
use winnow::combinator::alt;
use winnow::combinator::fail;
use winnow::combinator::repeat;
use winnow::error::StrContext::*;
use winnow::error::StrContextValue::*;

use crate::ast::Element;
use crate::markdown::*;
use crate::parse_directives::parse_directive_element;
use crate::parse_hidden::parse_hidden_html_div;
use crate::parse_wikilinks::parse_wikilink;

/// Parse a sequence of text elements.
/// It tries to parse specific elements first, then falls back to plain text.
fn parse_elements<'a>(input: &mut &'a str) -> ModalResult<Vec<Element<'a>>> {
    repeat(
        0..,
        alt((
            parse_hidden_html_div,
            parse_fenced_code_block,
            parse_code_span,
            parse_directive_element,
            parse_atx_heading,
            parse_link_reference_definition,
            parse_wikilink,
            parse_reference_style_image,
            parse_inline_image,
            parse_reference_style_link,
            parse_inline_link,
            parse_autolink,
            parse_plain_text,
            fail.context(Label("text element"))
                .context(Expected(Description("one of a hidden HTML div, fenced code block, code span, autolink, inline link, naked URL, plain text, or whitespace"))),
        )),
    )
    .parse_next(input)
}

/// Parse a Markdown document.
pub fn parse_document<'s>(
    name: &'s str,
    path: Option<&'s std::path::Path>,
    input: &'s str,
) -> anyhow::Result<crate::Document<'s>> {
    let mut input_ref = input;
    match parse_elements.parse_next(&mut input_ref) {
        Ok(elements) => {
            let doc = crate::Document::new(name, path, elements);
            Ok(doc)
        }
        Err(e) => anyhow::bail!("Parsing Failure: {:?}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser() {
        let text_input = r#"# Title

Hello [link](url)
[label]: url
"#;
        let doc = parse_document("test.md", None, text_input).unwrap();
        // Since parse_plain_text is currently consuming character by character,
        // we'll have a lot of Text elements.
        assert!(doc.elements.len() >= 5);
    }
}
