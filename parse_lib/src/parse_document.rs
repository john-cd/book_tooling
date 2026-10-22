// TODO Reviewed ??-??-2026
use winnow::ModalResult;
use winnow::Parser as _;
use winnow::ascii::line_ending;
use winnow::combinator::alt;
use winnow::combinator::delimited;
use winnow::combinator::eof;
use winnow::combinator::fail;
use winnow::combinator::opt;
use winnow::combinator::repeat;
use winnow::error::StrContext::{Expected, Label};
use winnow::error::StrContextValue::Description;
use winnow::error::{ContextError, ErrMode};
use winnow::seq;
use winnow::token::take_till;
use winnow::token::take_until;

// use crate::directives::parse_directive;
use crate::ast::BlockquoteData;
use crate::ast::Element;
use crate::ast::EmphasisData;
use crate::ast::FencedCodeBlockData;
use crate::ast::HtmlCommentData;
use crate::ast::ListData;
use crate::ast::ListItemData;
use crate::ast::ParagraphData;
use crate::ast::StrikethroughData;
use crate::ast::StrongData;
use crate::ast::TableData;
use crate::ast::TaskItemData;
use crate::markdown::{
    parse_atx_heading, parse_code_span, parse_fallback_char, parse_fenced_code_block,
    parse_inline_image, parse_inline_link, parse_link_reference_definition, parse_plain_text,
    parse_reference_style_image, parse_reference_style_link,
};
use crate::parse_directives::parse_directive_element;
use crate::parse_hidden::parse_hidden_html_div;
use crate::parse_urls::parse_autolink;
use crate::parse_urls::parse_naked_url;
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

/// Parses the text of each table cell into inline elements.
fn parse_table_cells<'a>(cells: &[&'a str]) -> ModalResult<Vec<Vec<Element<'a>>>> {
    cells
        .iter()
        .map(|cell| {
            let mut input = *cell;
            parse_inline_sequence.parse_next(&mut input)
        })
        .collect()
}

/// Parses a simplified GFM table.
fn parse_table<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    let (header, rows): (Vec<&'a str>, Vec<Vec<&'a str>>) = seq!(
        parse_table_row,
        _: repeat::<_, _, (), _, _>(1.., alt(("-", "|", " ", ":"))), // separator row
        _: alt((line_ending, eof)),
        repeat(1.., parse_table_row)
    )
    .parse_next(input)?;
    let header_elements = parse_table_cells(&header)?;
    let row_elements = rows
        .iter()
        .map(|row| parse_table_cells(row))
        .collect::<ModalResult<_>>()?;

    Ok(Element::Table(TableData {
        header,
        rows,
        header_elements,
        row_elements,
    }))
}

/// Parses a sequence of inline elements and text.
/// Used as the "secondary pass" for the inner text of block containers.
fn parse_inline_sequence<'a>(input: &mut &'a str) -> ModalResult<Vec<Element<'a>>> {
    repeat(
        0..,
        alt((parse_inline_element, parse_plain_text, parse_fallback_char)),
    )
    .parse_next(input)
}

/// Skip empty lines in a slice of lines.
fn parse_empty_lines(input: &mut &[&str]) -> ModalResult<()> {
    winnow::token::take_while(1.., |line: &str| line.trim().is_empty())
        .void()
        .parse_next(input)
}

/// Determines if a line should be consumed as part of an ongoing paragraph.
fn is_paragraph_continuation(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return false;
    }
    // Yield to the Fenced Code Block parser
    if trimmed.starts_with("```") {
        return false;
    }
    // Yield to Horizontal Rule parser
    let line_content = trimmed.trim();
    if line_content.len() >= 3 {
        let mut chars = line_content.chars();
        if let Some(first_char) = chars.next() {
            if matches!(first_char, '*' | '-' | '_') && chars.all(|c| c == first_char || c == ' ') {
                return false;
            }
        }
    }
    // Yield to HTML comment parser
    if trimmed.starts_with("<!--") {
        return false;
    }
    // Yield to the List parsers
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        return false;
    }
    ordered_list_marker_len(trimmed).is_none()
}

/// Returns the length of the leading `1. ` or `1) ` ordered-list marker, including the space, if there is one.
fn ordered_list_marker_len(trimmed: &str) -> Option<usize> {
    let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
    let rest = trimmed.get(digits..)?;
    (digits > 0 && (rest.starts_with(". ") || rest.starts_with(") "))).then_some(digits + 2)
}

/// Parses a continuous paragraph from a slice of lines.
fn parse_paragraph_from_lines<'a, 'b>(input: &mut &'b [&'a str]) -> ModalResult<Element<'a>> {
    let lines: &[&'a str] =
        winnow::token::take_while(1.., |line: &str| is_paragraph_continuation(line))
            .parse_next(input)?;

    let mut elements = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let mut line_ref = *line;
        if let Ok(mut parsed) = parse_inline_sequence.parse_next(&mut line_ref) {
            elements.append(&mut parsed);
        }
        // Interleave a newline text element to maintain Markdown soft breaks
        if i < lines.len() - 1 {
            elements.push(Element::Text(crate::ast::TextData { content: "\n" }));
        }
    }

    Ok(Element::Paragraph(ParagraphData { elements }))
}

/// Parses a fenced code block from a slice of lines.
fn parse_fenced_code_block_from_lines<'a, 'b>(
    input: &mut &'b [&'a str],
) -> ModalResult<Element<'a>> {
    let open_line: &'a str = winnow::token::any
        .verify(|line: &&str| line.trim_start().starts_with("```"))
        .parse_next(input)?;

    let lang_str = open_line.trim_start().trim_start_matches('`').trim();
    let language = if lang_str.is_empty() {
        None
    } else {
        Some(lang_str)
    };

    let content_lines: &[&'a str] =
        winnow::token::take_while(0.., |line: &str| !line.trim_start().starts_with("```"))
            .parse_next(input)?;

    let _close: &'a str = winnow::token::any
        .verify(|line: &&str| line.trim_start().starts_with("```"))
        .parse_next(input)?;

    Ok(Element::FencedCodeBlock(FencedCodeBlockData {
        language,
        lines: content_lines.to_vec(),
    }))
}

/// Parses consecutive lines indented by four spaces or a tab as one indented code block.
fn parse_indented_code_block_from_lines<'a>(input: &mut &[&'a str]) -> ModalResult<Element<'a>> {
    let backup = *input;
    let mut content_lines = Vec::new();
    let mut last_non_empty = 0;

    for (i, line) in backup.iter().enumerate() {
        if let Some(stripped) = line.strip_prefix("    ") {
            content_lines.push(stripped);
            last_non_empty = i + 1;
        } else if let Some(stripped) = line.strip_prefix('\t') {
            content_lines.push(stripped);
            last_non_empty = i + 1;
        } else if line.trim().is_empty() {
            content_lines.push("");
        } else {
            break;
        }
    }

    if last_non_empty == 0 {
        return fail.parse_next(input);
    }

    content_lines.truncate(last_non_empty);
    *input = input.get(last_non_empty..).unwrap_or_default();

    Ok(Element::FencedCodeBlock(FencedCodeBlockData {
        language: None,
        lines: content_lines,
    }))
}

/// Parses an HTML comment that may span several lines.
fn parse_html_comment_from_lines<'a>(input: &mut &[&'a str]) -> ModalResult<Element<'a>> {
    let backup = *input;
    let mut content_lines = Vec::new();
    let mut found_end = false;
    let mut consumed = 0;

    for (i, line) in backup.iter().enumerate() {
        consumed += 1;
        if i == 0 {
            let Some(after_open) = line.trim_start().strip_prefix("<!--") else {
                return fail.parse_next(input);
            };
            if let Some((before_close, _)) = after_open.split_once("-->") {
                content_lines.push(before_close);
                found_end = true;
                break;
            }
            content_lines.push(after_open);
        } else if let Some((before_close, _)) = line.split_once("-->") {
            content_lines.push(before_close);
            found_end = true;
            break;
        } else {
            content_lines.push(*line);
        }
    }

    if !found_end {
        return fail.parse_next(input);
    }

    *input = backup.get(consumed..).unwrap_or_default();
    Ok(Element::HtmlComment(HtmlCommentData {
        lines: content_lines,
    }))
}

/// Parses a list item and its indented continuation lines; `check_marker` returns the length of the list marker for the kind of list being parsed.
fn parse_list_item_from_lines_generic<'a, 'b, F>(
    input: &mut &'b [&'a str],
    check_marker: F,
) -> ModalResult<Element<'a>>
where
    F: Fn(&str) -> Option<usize>,
{
    let first_line: &'a str = winnow::token::any
        .verify(|line: &&str| {
            let trimmed = line.trim_start();
            check_marker(trimmed).is_some()
        })
        .parse_next(input)?;

    let trimmed = first_line.trim_start();
    let indent_spaces = first_line.len() - trimmed.len();
    // The `verify` above guarantees that a marker is present.
    let Some(marker_len) = check_marker(trimmed) else {
        return Err(ErrMode::Backtrack(ContextError::default()));
    };
    let indent_level = indent_spaces + marker_len;

    let mut lines = vec![trimmed.get(marker_len..).unwrap_or_default()];

    let cont_lines: &[&'a str] = winnow::token::take_while(0.., |line: &str| {
        let trimmed_line = line.trim();
        trimmed_line.is_empty() || {
            let line_indent = line.len() - line.trim_start().len();
            line_indent >= indent_level
        }
    })
    .parse_next(input)?;

    for line in cont_lines {
        if line.trim().is_empty() {
            lines.push("");
        } else {
            lines.push(line.get(indent_level..).unwrap_or_default());
        }
    }

    let mut lines_slice = lines.as_slice();
    let inner_elements = parse_blocks_from_lines
        .parse_next(&mut lines_slice)
        .unwrap_or_default();

    Ok(Element::ListItem(ListItemData {
        elements: inner_elements,
    }))
}

/// Parses a `-` or `*` list item from a slice of lines.
fn parse_unordered_list_item_from_lines<'a>(input: &mut &[&'a str]) -> ModalResult<Element<'a>> {
    parse_list_item_from_lines_generic(input, |trimmed| {
        (trimmed.starts_with("- ") || trimmed.starts_with("* ")).then_some(2)
    })
}

/// Parses consecutive `-` or `*` list items from a slice of lines into one list.
fn parse_unordered_list_from_lines<'a>(input: &mut &[&'a str]) -> ModalResult<Element<'a>> {
    let items = repeat(1.., parse_unordered_list_item_from_lines).parse_next(input)?;
    Ok(Element::List(ListData {
        ordered: false,
        items,
    }))
}

/// Parses a `1.` or `1)` list item from a slice of lines.
fn parse_ordered_list_item_from_lines<'a>(input: &mut &[&'a str]) -> ModalResult<Element<'a>> {
    parse_list_item_from_lines_generic(input, ordered_list_marker_len)
}

/// Parses consecutive numbered list items from a slice of lines into one list.
fn parse_ordered_list_from_lines<'a>(input: &mut &[&'a str]) -> ModalResult<Element<'a>> {
    let items = repeat(1.., parse_ordered_list_item_from_lines).parse_next(input)?;
    Ok(Element::List(ListData {
        ordered: true,
        items,
    }))
}

/// Parses a sequence of blocks from a slice of lines.
fn parse_blocks_from_lines<'a, 'b>(input: &mut &'b [&'a str]) -> ModalResult<Vec<Element<'a>>> {
    repeat(
        0..,
        alt((
            parse_empty_lines.map(|()| None),
            parse_fenced_code_block_from_lines.map(Some),
            parse_indented_code_block_from_lines.map(Some),
            parse_html_comment_from_lines.map(Some),
            parse_unordered_list_from_lines.map(Some),
            parse_ordered_list_from_lines.map(Some),
            parse_paragraph_from_lines.map(Some),
        )),
    )
    .map(|blocks: Vec<Option<Element<'a>>>| blocks.into_iter().flatten().collect())
    .parse_next(input)
}

/// Parses a Blockquote container block.
/// Solves the "contiguous slice" problem by extracting each line as its own contiguous `&'a str`,
/// bypassing the need for a custom Stream that would break slice memory layout.
fn parse_blockquote<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    let lines: Vec<&'a str> = repeat(
        1..,
        seq!(
            _: ">",
            _: opt(" "),
            take_till(0.., ['\n', '\r']),
            _: opt(line_ending)
        )
        .map(|(content,)| content),
    )
    .parse_next(input)?;

    // We need to parse the inner content of the blockquote from the collected lines.
    // To satisfy the borrow checker, we must ensure the returned elements don't borrow from
    // the local `lines` Vec itself, but only from the strings it contains (which live for 'a).

    let mut lines_slice = lines.as_slice();
    let inner_elements = parse_blocks_from_lines
        .parse_next(&mut lines_slice)
        .unwrap_or_default();
    Ok(Element::Blockquote(BlockquoteData {
        elements: inner_elements,
    }))
}

/// Parses a single-line list item whose marker is recognized by `parse_marker`.
fn parse_list_item_generic<'a, F>(input: &mut &'a str, parse_marker: F) -> ModalResult<Element<'a>>
where
    F: winnow::Parser<&'a str, &'a str, winnow::error::ContextError>,
{
    let (ws, marker, content, _): (&str, &str, &str, _) = (
        winnow::token::take_while(0..=3, ' '),
        parse_marker,
        take_till(0.., ['\n', '\r']),
        opt(line_ending),
    )
        .parse_next(input)
        .map_err(|e: winnow::error::ContextError| winnow::error::ErrMode::Backtrack(e))?;

    let indent_level = ws.len() + marker.len();
    let mut lines = vec![content];

    let mut cont_lines: Vec<&str> = repeat(
        0..,
        alt((
            seq!(winnow::token::take_while(0.., ' '), line_ending).map(|_| ""),
            seq!(
                (
                    winnow::token::take_while(0.., ' ')
                        .verify(move |s: &str| s.len() >= indent_level),
                    take_till(0.., ['\n', '\r'])
                )
                    .take(),
                opt(line_ending)
            )
            .map(|(full_line, _): (&str, _)| full_line.get(indent_level..).unwrap_or_default()),
        )),
    )
    .parse_next(input)
    .map_err(|e: winnow::error::ContextError| winnow::error::ErrMode::Backtrack(e))?;

    lines.append(&mut cont_lines);

    let mut lines_slice = lines.as_slice();
    let inner_elements = parse_blocks_from_lines
        .parse_next(&mut lines_slice)
        .unwrap_or_default();

    Ok(Element::ListItem(ListItemData {
        elements: inner_elements,
    }))
}

/// Parses a single-line `-` or `*` list item.
fn parse_unordered_list_item<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    parse_list_item_generic(input, alt(("- ", "* ")))
}

/// Parses consecutive single-line `-` or `*` list items into one list.
fn parse_unordered_list<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    let items = repeat(1.., parse_unordered_list_item).parse_next(input)?;
    Ok(Element::List(ListData {
        ordered: false,
        items,
    }))
}

/// Parses a single-line `1.` or `1)` list item.
fn parse_ordered_list_item<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    parse_list_item_generic(
        input,
        (
            winnow::token::take_while(1.., |c: char| c.is_ascii_digit()),
            alt((". ", ") ")),
        )
            .take(),
    )
}

/// Parses consecutive single-line numbered list items into one list.
fn parse_ordered_list<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    let items = repeat(1.., parse_ordered_list_item).parse_next(input)?;
    Ok(Element::List(ListData {
        ordered: true,
        items,
    }))
}

/// Parses a thematic break made of three or more `*`, `-` or `_` characters.
fn parse_horizontal_rule<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    seq!(
        winnow::token::take_while(0..=3, ' '),
        take_till(0.., ['\n', '\r']),
        line_ending
    )
    .take()
    .verify(|line: &str| {
        let line_content = line.trim();
        if line_content.len() < 3 {
            return false;
        }
        let mut chars = line_content.chars();
        let Some(first_char) = chars.next() else {
            return false;
        };
        if !matches!(first_char, '*' | '-' | '_') {
            return false;
        }
        chars.all(|c| c == first_char || c == ' ')
    })
    .map(|_| Element::HorizontalRule)
    .parse_next(input)
}

/// Parses an HTML comment from `<!--` to `-->`, splitting its content into lines.
fn parse_html_comment<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    let content = delimited("<!--", take_until(0.., "-->"), "-->")
        .context(Label("HTML comment"))
        .context(Expected(Description("<!-- ... -->")))
        .parse_next(input)?;

    let lines = content
        .split('\n')
        .map(|s| s.trim_end_matches('\r'))
        .collect();
    Ok(Element::HtmlComment(HtmlCommentData { lines }))
}

/// Parses consecutive lines indented by four spaces or a tab as one indented code block.
fn parse_indented_code_block<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    let backup = *input;
    let mut content_lines = Vec::new();
    let mut last_non_empty = 0;
    let mut bytes_to_consume = 0;
    let mut current_bytes = 0;

    for line_with_ending in backup.split_inclusive('\n') {
        let line = line_with_ending.trim_end_matches(['\n', '\r']);
        if let Some(stripped) = line.strip_prefix("    ") {
            content_lines.push(stripped);
            current_bytes += line_with_ending.len();
            last_non_empty = content_lines.len();
            bytes_to_consume = current_bytes;
        } else if let Some(stripped) = line.strip_prefix('\t') {
            content_lines.push(stripped);
            current_bytes += line_with_ending.len();
            last_non_empty = content_lines.len();
            bytes_to_consume = current_bytes;
        } else if line.trim().is_empty() {
            content_lines.push("");
            current_bytes += line_with_ending.len();
        } else {
            break;
        }
    }

    if last_non_empty == 0 {
        return fail.parse_next(input);
    }

    content_lines.truncate(last_non_empty);
    let _ = winnow::token::take(bytes_to_consume).parse_next(input)?;

    Ok(Element::FencedCodeBlock(FencedCodeBlockData {
        language: None,
        lines: content_lines,
    }))
}

/// Parses block-level elements (structural containers).
fn parse_block_element<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    alt((
        parse_blockquote,
        parse_hidden_html_div,
        parse_html_comment,
        parse_fenced_code_block,
        parse_indented_code_block,
        parse_horizontal_rule,
        alt((
            parse_atx_heading,
            parse_table,
            parse_task_item,
            parse_unordered_list,
            parse_ordered_list,
            parse_link_reference_definition,
            parse_directive_element, // Directives can act as blocks
        )),
    ))
    .parse_next(input)
}

/// Parses text delimited by `inner_parser` and builds an element from the inline elements inside it using `construct`.
fn parse_inline_formatting<'a, P, F>(
    input: &mut &'a str,
    mut inner_parser: P,
    construct: F,
) -> ModalResult<Element<'a>>
where
    P: winnow::Parser<&'a str, &'a str, winnow::error::ContextError>,
    F: Fn(Vec<Element<'a>>) -> Element<'a>,
{
    let inner_str = inner_parser
        .parse_next(input)
        .map_err(winnow::error::ErrMode::Backtrack)?;
    let mut inner_ref = inner_str;
    let elements = parse_inline_sequence
        .parse_next(&mut inner_ref)
        .unwrap_or_default();
    Ok(construct(elements))
}

/// Parses `**strong**` text.
fn parse_strong<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    parse_inline_formatting(
        input,
        alt((
            delimited(
                "**",
                repeat::<_, _, String, _, _>(
                    1..,
                    alt((
                        seq!(take_until(1.., "*"), opt("*")).take(),
                        take_until(1.., "**"),
                    )),
                )
                .take(),
                "**",
            ),
            delimited(
                "__",
                repeat::<_, _, String, _, _>(
                    1..,
                    alt((
                        seq!(take_until(1.., "_"), opt("_")).take(),
                        take_until(1.., "__"),
                    )),
                )
                .take(),
                "__",
            ),
        )),
        |elements| Element::Strong(StrongData { elements }),
    )
}

/// Parses `*emphasis*` or `_emphasis_` text.
fn parse_emphasis<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    parse_inline_formatting(
        input,
        alt((
            delimited("*", take_until(1.., "*"), "*"),
            delimited("_", take_until(1.., "_"), "_"),
        )),
        |elements| Element::Emphasis(EmphasisData { elements }),
    )
}

/// Parses `~~strikethrough~~` text.
fn parse_strikethrough<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    parse_inline_formatting(
        input,
        delimited("~~", take_until(1.., "~~"), "~~"),
        |elements| Element::Strikethrough(StrikethroughData { elements }),
    )
}

/// Parses inline-level elements (text formatting and links).
fn parse_inline_element<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    alt((
        parse_code_span,
        parse_strong,
        parse_emphasis,
        parse_strikethrough,
        parse_html_comment,
        parse_directive_element,
        parse_wikilink,
        alt((
            parse_reference_style_image,
            parse_inline_image,
            parse_reference_style_link,
            parse_inline_link,
            parse_autolink,
            parse_naked_url,
        )),
    ))
    .parse_next(input)
}

/// Parse a sequence of text elements.
/// It tries to parse specific elements first, then falls back to plain text.
fn parse_elements<'a>(input: &mut &'a str) -> ModalResult<Vec<Element<'a>>> {
    repeat(
        0..,
        alt((
            parse_block_element,
            parse_inline_element,
            parse_plain_text,
            parse_fallback_char,
            fail.context(Label("text element"))
                .context(Expected(Description("one of a hidden HTML div, fenced code block, code span, autolink, inline link, naked URL, plain text, or whitespace"))),
        )),
    )
    .parse_next(input)
}

/// Parses an entire Markdown document into a [`crate::Document`].
///
/// `name` is used for diagnostics, `path` is retained in the resulting AST when available,
/// and `input` must be valid UTF-8 Markdown text.
///
/// # Errors
///
/// Returns an error if the Markdown cannot be parsed; the message names the document and the parse failure.
pub fn parse_document<'s>(
    name: &'s str,
    path: Option<&'s std::path::Path>,
    input: &'s str,
) -> anyhow::Result<crate::Document<'s>> {
    let mut input_ref = input;
    match parse_elements.parse_next(&mut input_ref) {
        Ok(elements) => {
            let doc = crate::Document::new(name, path, input, elements);
            Ok(doc)
        }
        Err(e) => anyhow::bail!("Failed to parse document '{name}':\n{e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::DirectiveData;

    #[test]
    fn test_parser() {
        let text_input = "# Title

Hello [link](url)
[label]: url
";
        let doc = parse_document("test.md", None, text_input).expect("input should parse");
        // Since parse_plain_text is currently consuming character by character,
        // we'll have a lot of Text elements.
        assert!(doc.elements.len() >= 5);
    }

    #[test]
    fn test_parse_task_item() {
        let input = "- [x] task 1";
        let mut input_ref = input;
        let result = parse_task_item
            .parse_next(&mut input_ref)
            .expect("input should parse");
        if let Element::TaskItem(task) = result {
            assert!(task.completed);
            assert_eq!(task.content, "task 1");
        } else {
            panic!("Expected TaskItem");
        }
    }

    #[test]
    fn test_parse_task_item_incomplete() {
        let input = "- [ ] todo item";
        let mut input_ref = input;
        let result = parse_task_item
            .parse_next(&mut input_ref)
            .expect("input should parse");
        if let Element::TaskItem(task) = result {
            assert!(!task.completed);
            assert_eq!(task.content, "todo item");
        } else {
            panic!("Expected TaskItem");
        }
    }

    #[test]
    fn test_parse_task_item_invalid() {
        let input = "- [y] invalid";
        let mut input_ref = input;
        parse_task_item
            .parse_next(&mut input_ref)
            .expect_err("input should be rejected");
    }

    #[test]
    fn test_parse_table() {
        let input = "| h1 | h2 |\n|---|---|\n| r1c1 | r1c2 |";
        let mut input_ref = input;
        let result = parse_table
            .parse_next(&mut input_ref)
            .expect("input should parse");
        if let Element::Table(table) = result {
            assert_eq!(table.header, vec!["h1", "h2"]);
            assert_eq!(table.rows[0], vec!["r1c1", "r1c2"]);
        } else {
            panic!("Expected Table");
        }
    }

    #[test]
    fn test_parse_table_no_outer_pipes() {
        let input = "h1 | h2\n---|---\nr1c1 | r1c2";
        let mut input_ref = input;
        let result = parse_table
            .parse_next(&mut input_ref)
            .expect("input should parse");
        if let Element::Table(table) = result {
            assert_eq!(table.header, vec!["h1", "h2"]);
            assert_eq!(table.rows[0], vec!["r1c1", "r1c2"]);
        } else {
            panic!("Expected Table");
        }
    }

    #[test]
    fn test_parse_table_multiple_rows() {
        let input = "| a | b |\n| --- | --- |\n| 1 | 2 |\n| 3 | 4 |";
        let mut input_ref = input;
        let result = parse_table
            .parse_next(&mut input_ref)
            .expect("input should parse");
        if let Element::Table(table) = result {
            assert_eq!(table.rows.len(), 2);
            assert_eq!(table.rows[1], vec!["3", "4"]);
        } else {
            panic!("Expected Table");
        }
    }

    #[test]
    fn test_parse_table_inline_directives_in_cells() {
        let input = concat!(
            "| Recipe | Crates | Categories |\n",
            "|--------|--------|------------|\n",
            "| Sort values | {{#crate std}} | {{#categories algorithms=Algorithms}} |"
        );
        let mut input_ref = input;
        let result = parse_table
            .parse_next(&mut input_ref)
            .expect("input should parse");
        let Element::Table(table) = result else {
            panic!("Expected Table");
        };

        assert!(matches!(
            table.row_elements[0][1].as_slice(),
            [Element::CustomDirective(DirectiveData::CrateBlock {
                crate_name: "std",
                ..
            })]
        ));
        assert!(matches!(
            table.row_elements[0][2].as_slice(),
            [Element::CustomDirective(
                DirectiveData::CategoryBlock { .. }
            )]
        ));
    }

    #[test]
    fn test_simple_list_in_blockquote() {
        let input = "> - one
> - two
";
        let doc = parse_document("test.md", None, input).expect("input should parse");

        assert_eq!(
            doc.elements.len(),
            1,
            "Document should have one root element"
        );
        let blockquote = doc.elements.first().expect("element should exist");

        // Check that the root element is a Blockquote
        if let Element::Blockquote(blockquote_data) = blockquote {
            assert_eq!(
                blockquote_data.elements.len(),
                1,
                "Blockquote should contain one element"
            );
            let list = blockquote_data
                .elements
                .first()
                .expect("element should exist");

            // Check that the blockquote contains a List
            if let Element::List(list_data) = list {
                assert!(!list_data.ordered, "List should be unordered");
                assert_eq!(list_data.items.len(), 2, "List should have two items");
            } else {
                panic!("Expected List inside Blockquote, found {list:?}");
            }
        } else {
            panic!("Expected Blockquote element, found {blockquote:?}");
        }
    }

    #[test]
    #[expect(
        clippy::excessive_nesting,
        reason = "The assertions walk a deliberately nested blockquote, list and list item."
    )]
    fn test_nested_list_in_blockquote() {
        let input = "> A paragraph.
>
> - list item 1
>   - nested item 1.1
> - list item 2
";
        let doc = parse_document("test.md", None, input).expect("input should parse");

        assert_eq!(doc.elements.len(), 1);
        let blockquote = doc.elements.first().expect("element should exist");

        // Root element should be a Blockquote
        if let Element::Blockquote(blockquote_data) = blockquote {
            // Blockquote should contain a Paragraph and a List
            assert_eq!(blockquote_data.elements.len(), 2);

            // Check the List element
            let list = &blockquote_data.elements[1];
            if let Element::List(list_data) = list {
                assert!(!list_data.ordered);
                assert_eq!(list_data.items.len(), 2);

                // -- Check first list item and its nested list --
                let item1 = list_data.items.first().expect("element should exist");
                if let Element::ListItem(item1_data) = item1 {
                    // Item 1 should contain a Paragraph ("list item 1") and a nested List
                    assert_eq!(item1_data.elements.len(), 2);

                    let Some(Element::List(nested_list_data)) = item1_data.elements.get(1) else {
                        panic!("Expected nested List in first list item");
                    };
                    assert_eq!(
                        nested_list_data.items.len(),
                        1,
                        "Nested list should have one item"
                    );
                } else {
                    panic!("Expected ListItem");
                }

                // -- Check second list item --
                let item2 = &list_data.items[1];
                if let Element::ListItem(item2_data) = item2 {
                    // Item 2 should just contain a single Paragraph
                    assert_eq!(item2_data.elements.len(), 1);
                } else {
                    panic!("Expected ListItem");
                }
            } else {
                panic!("Expected List as the second element in the Blockquote");
            }
        } else {
            panic!("Expected Blockquote element");
        }
    }

    #[test]
    fn test_list_with_fenced_code_block() {
        let input = "- item 1\n  ```rust\n  let x = 1;\n  ```\n- item 2\n";
        let doc = parse_document("test.md", None, input).expect("input should parse");

        assert_eq!(doc.elements.len(), 1);
        let list = doc.elements.first().expect("element should exist");

        if let Element::List(list_data) = list {
            assert_eq!(list_data.items.len(), 2);
            if let Element::ListItem(item1) = &list_data.items[0] {
                assert_eq!(item1.elements.len(), 2); // Paragraph, then FencedCodeBlock
                assert!(matches!(item1.elements[0], Element::Paragraph(_)));
                assert!(matches!(item1.elements[1], Element::FencedCodeBlock(_)));
            } else {
                panic!("Expected ListItem");
            }
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_parse_indented_code_block() {
        let input = "    let x = 1;\n    let y = 2;\n\nParagraph";
        let mut input_ref = input;
        let result = parse_indented_code_block
            .parse_next(&mut input_ref)
            .expect("input should parse");

        if let Element::FencedCodeBlock(data) = result {
            assert_eq!(data.language, None);
            assert_eq!(data.lines, vec!["let x = 1;", "let y = 2;"]);
        } else {
            panic!("Expected FencedCodeBlock");
        }
        assert_eq!(input_ref, "\nParagraph");
    }

    #[test]
    fn test_parse_strong_and_emphasis() {
        let input = "**bold *italic***";
        let mut input_ref = input;
        let result = parse_inline_element
            .parse_next(&mut input_ref)
            .expect("input should parse");

        if let Element::Strong(strong) = result {
            assert!(strong.elements.len() >= 2);
            // Due to how fallback_char works, "bold " might be multiple Text elements
            let mut combined_text = String::new();
            let mut next_idx = 0;
            for (i, el) in strong.elements.iter().enumerate() {
                if let Element::Text(text) = el {
                    combined_text.push_str(text.content);
                } else {
                    next_idx = i;
                    break;
                }
            }
            assert_eq!(combined_text, "bold ");
            assert!(matches!(strong.elements[next_idx], Element::Emphasis(_)));
        } else {
            panic!("Expected Strong element");
        }
    }

    #[test]
    fn test_parse_strikethrough() {
        let input = "~~deleted~~";
        let mut input_ref = input;
        let result = parse_inline_element
            .parse_next(&mut input_ref)
            .expect("input should parse");

        if let Element::Strikethrough(strikethrough) = result {
            assert_eq!(strikethrough.elements.len(), 1);
            assert!(matches!(strikethrough.elements[0], Element::Text(_)));
        } else {
            panic!("Expected Strikethrough element");
        }
    }

    #[test]
    fn test_parse_horizontal_rule() {
        let mut input = "--- \n";
        assert_eq!(
            parse_horizontal_rule.parse_next(&mut input),
            Ok(Element::HorizontalRule)
        );
        assert_eq!(input, "");

        let mut rule_input = "  * * * \nnext line";
        assert_eq!(
            parse_horizontal_rule.parse_next(&mut rule_input),
            Ok(Element::HorizontalRule)
        );
        assert_eq!(rule_input, "next line");
    }

    #[test]
    fn test_parse_html_comment() {
        let mut input = "<!-- hidden \n text -->\n";
        let result = parse_html_comment
            .parse_next(&mut input)
            .expect("input should parse");
        if let Element::HtmlComment(comment) = result {
            assert_eq!(comment.lines, vec![" hidden ", " text "]);
        } else {
            panic!("Expected HtmlComment");
        }
        assert_eq!(input, "\n");
    }

    #[test]
    fn test_parse_html_comment_from_lines() {
        let input_doc = "- item 1\n  <!-- multi\n  line\n  comment -->\n  Paragraph";
        let doc = parse_document("test.md", None, input_doc).expect("input should parse");

        let list = doc.elements.first().expect("element should exist");
        if let Element::List(list_data) = list {
            if let Element::ListItem(item) = &list_data.items[0] {
                assert_eq!(item.elements.len(), 3); // Para, Comment, Para
                assert!(matches!(item.elements[1], Element::HtmlComment(_)));
            } else {
                panic!("expected list item");
            }
        } else {
            panic!("expected list");
        }
    }
}
