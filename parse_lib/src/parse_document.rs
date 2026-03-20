use winnow::ModalResult;
use winnow::Parser;
use winnow::ascii::line_ending;
use winnow::combinator::alt;
use winnow::combinator::eof;
use winnow::combinator::fail;
use winnow::combinator::opt;
use winnow::combinator::repeat;
use winnow::combinator::seq;
use winnow::error::StrContext::*;
use winnow::error::StrContextValue::*;
use winnow::token::take_till;

use crate::ast::Element;
<<<<<<< HEAD
// TODO [finish URL parsing](https://github.com/john-cd/rust_howto/issues/1427)
// use crate::urls::parse_naked_url;
// use crate::directives::parse_directive;
=======
use crate::ast::TableData;
use crate::ast::TaskItemData;
>>>>>>> jules-3582745748344015560-6d4fedb4
use crate::markdown::*;
use crate::parse_directives::parse_directive_element;
use crate::parse_hidden::parse_hidden_html_div;
use crate::parse_wikilinks::parse_wikilink;

/// Parses a GFM task list item.
fn parse_task_item<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    seq!(
        "- [",
        alt(("x", "X", " ")),
        "] ",
        take_till(0.., ['\n', '\r'])
    )
    .map(|(_, completed, _, content)| {
        Element::TaskItem(TaskItemData {
            completed: completed == "x" || completed == "X",
            content,
        })
    })
    .parse_next(input)
}

/// Parses a simplified GFM table row.
fn parse_table_row<'a>(input: &mut &'a str) -> ModalResult<Vec<&'a str>> {
    seq!(
        _: opt("|"),
        repeat(1.., seq!(take_till(1.., ['|', '\n', '\r']), _: opt("|")).map(|(c,): (&str,)| c.trim())),
        _: alt((line_ending, eof))
    ).map(|(cells,): (Vec<&str>,)| cells)
    .parse_next(input)
}

/// Parses a simplified GFM table.
fn parse_table<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    seq!(
        parse_table_row,
        _: repeat::<_, _, (), _, _>(1.., alt(("-", "|", " ", ":"))), // separator row
        _: alt((line_ending, eof)),
        repeat(1.., parse_table_row)
    )
    .map(|(header, rows)| Element::Table(TableData { header, rows }))
    .parse_next(input)
}

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
            parse_task_item,
            parse_table,
            parse_plain_text,
            parse_fallback_char,
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

    #[test]
    fn test_parse_task_item() {
        let input = "- [x] task 1";
        let mut input_ref = input;
        let result = parse_task_item.parse_next(&mut input_ref).unwrap();
        if let Element::TaskItem(task) = result {
            assert!(task.completed);
            assert_eq!(task.content, "task 1");
        } else {
            panic!("Expected TaskItem");
        }
    }

    #[test]
    fn test_parse_table() {
        let input = "| h1 | h2 |\n|---|---|\n| r1c1 | r1c2 |";
        let mut input_ref = input;
        let result = parse_table.parse_next(&mut input_ref).unwrap();
        if let Element::Table(table) = result {
            assert_eq!(table.header, vec!["h1", "h2"]);
            assert_eq!(table.rows[0], vec!["r1c1", "r1c2"]);
        } else {
            panic!("Expected Table");
        }
    }
}
