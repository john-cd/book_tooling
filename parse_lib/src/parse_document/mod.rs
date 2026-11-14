//! Parses a whole Markdown document into AST elements.
//!
//! TODO Reviewed ??-??-2026.

/// Regression tests for block and inline parsing, nesting, and source preservation.
#[cfg(test)]
mod parse_document_tests;

use winnow::ModalResult;
use winnow::Parser as _;
use winnow::ascii::line_ending;
use winnow::combinator::alt;
use winnow::combinator::delimited;
use winnow::combinator::eof;
use winnow::combinator::fail;
use winnow::combinator::opt;
use winnow::combinator::repeat;
use winnow::error::ContextError;
use winnow::error::ErrMode;
use winnow::error::StrContext::Expected;
use winnow::error::StrContext::Label;
use winnow::error::StrContextValue::Description;
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
use crate::markdown::parse_atx_heading;
use crate::markdown::parse_code_span;
use crate::markdown::parse_fallback_char;
use crate::markdown::parse_fenced_code_block;
use crate::markdown::parse_inline_image;
use crate::markdown::parse_inline_link;
use crate::markdown::parse_link_reference_definition;
use crate::markdown::parse_plain_text;
use crate::markdown::parse_reference_style_image;
use crate::markdown::parse_reference_style_link;
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

/// Consumes a pipe-delimited row and returns trimmed slices of its cells.
///
/// # Errors
/// Backtracks when the input has no cells or the row does not end at a line boundary.
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

/// Consumes a header, separator, and at least one data row. The table retains the cell text
/// alongside the parsed inline elements.
///
/// # Errors
/// Backtracks if any required row is missing or malformed, and propagates errors from inline
/// parsing.
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

/// Used as the "secondary pass" for the inner text of block containers.
fn parse_inline_sequence<'a>(input: &mut &'a str) -> ModalResult<Vec<Element<'a>>> {
    repeat(
        0..,
        alt((parse_inline_element, parse_plain_text, parse_fallback_char)),
    )
    .parse_next(input)
}
/// Consumes one or more whitespace-only lines from `input`.
///
/// # Errors
/// Backtracks when the first line contains text or the input is exhausted.
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
        if let Some(first_char) = chars.next()
            && matches!(first_char, '*' | '-' | '_')
            && chars.all(|c| c == first_char || c == ' ')
        {
            return false;
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

/// Returns the byte length of an initial decimal marker with its trailing space, or `None` if
/// `trimmed` does not start with a marker such as `1. ` or `2) `.
fn ordered_list_marker_len(trimmed: &str) -> Option<usize> {
    let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
    let rest = trimmed.get(digits..)?;
    (digits > 0 && (rest.starts_with(". ") || rest.starts_with(") "))).then_some(digits + 2)
}

/// Consumes consecutive paragraph lines and inserts soft line breaks between their inline elements.
///
/// # Errors
/// Backtracks when the first line is blank or starts a block that takes precedence over a
/// paragraph.
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

/// Consumes a backtick-fenced code block, preserving its inner lines and optional language.
///
/// # Errors
/// Backtracks if the opening or closing fence is missing.
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

/// Consumes a comment from the remaining lines and keeps the text between its delimiters.
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

/// Parses a list item and its indented continuation lines; `check_marker` returns the length of the
/// list marker for the kind of list being parsed.
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

/// Consumes consecutive bullet items and their indented continuations into an unordered list.
///
/// # Errors
/// Backtracks when the remaining lines do not begin with a `-` or `*` item.
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
/// Consumes consecutive numbered items and their indented continuations into an ordered list.
///
/// # Errors
/// Backtracks when the remaining lines do not begin with a decimal list marker.
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
/// Consumes consecutive single-line bullet items into an unordered list.
///
/// # Errors
/// Backtracks if the input does not start with a `-` or `*` item.
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
/// Consumes consecutive single-line numbered items into an ordered list.
///
/// # Errors
/// Backtracks if the input does not start with a decimal list marker.
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

/// Consumes `<!--` through the first `-->` and returns the enclosed text as a comment node.
///
/// # Errors
/// Backtracks if either delimiter is missing.
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

/// Consumes consecutive code lines introduced by four spaces or a tab and strips that indentation.
///
/// # Errors
/// Backtracks when the first line does not have code-block indentation.
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

/// Tries the block parsers in precedence order at the current input position and returns the first
/// match.
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
/// Parses text delimited by `inner_parser` and builds an element from the inline elements inside it
/// using `construct`.
fn parse_inline_formatting<'a, P, F>(
    input: &mut &'a str,
    mut inner_parser: P,
    construct: F,
) -> ModalResult<Element<'a>>
where
    P: winnow::Parser<&'a str, &'a str, winnow::error::ContextError>,
    F: Fn(Vec<Element<'a>>) -> Element<'a>,
{
    let checkpoint = *input;
    let inner_str = inner_parser
        .parse_next(input)
        .map_err(winnow::error::ErrMode::Backtrack)?;
    if contains_blank_line(inner_str) {
        *input = checkpoint;
        return Err(ErrMode::Backtrack(ContextError::new()));
    }
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

/// Returns `true` when `text` contains a blank line, which ends a paragraph and so cannot lie
/// inside inline formatting.
fn contains_blank_line(text: &str) -> bool {
    let mut lines = text.split('\n');
    // The first and last lines are partial: only the lines between them can be blank.
    lines.next();
    lines.next_back();
    lines.any(|line| line.trim().is_empty())
}

/// Parses emphasis, rejecting delimiters that violate the `CommonMark` flanking rules.
fn emphasis_inner<'a>(input: &mut &'a str, delimiter: char) -> Result<&'a str, ContextError> {
    let backtrack = ContextError::new;
    let body = input.strip_prefix(delimiter).ok_or_else(backtrack)?;
    if body.starts_with(|c: char| c.is_whitespace() || c == delimiter) {
        return Err(backtrack());
    }
    let mut previous = delimiter;
    for (index, character) in body.char_indices() {
        let closes = character == delimiter
            && index > 0
            && !previous.is_whitespace()
            && (delimiter != '_'
                || !body
                    .get(index + 1..)
                    .and_then(|rest| rest.chars().next())
                    .is_some_and(char::is_alphanumeric));
        if closes {
            let inner = body.get(..index).ok_or_else(backtrack)?;
            *input = body.get(index + 1..).ok_or_else(backtrack)?;
            return Ok(inner);
        }
        previous = character;
    }
    Err(backtrack())
}

/// Parses `*emphasis*` or `_emphasis_` text.
fn parse_emphasis<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    parse_inline_formatting(
        input,
        alt((
            |text: &mut &'a str| emphasis_inner(text, '*'),
            |text: &mut &'a str| emphasis_inner(text, '_'),
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
/// `name` is used for diagnostics, `path` is retained in the resulting AST when available,
/// and `input` must be valid UTF-8 Markdown text.
/// # Errors
/// Returns an error if the Markdown cannot be parsed; the message names the document and the parse
/// failure.
pub fn parse_document<'s>(
    name: &'s str,
    path: Option<&'s std::path::Path>,
    input: &'s str,
) -> Result<crate::Document<'s>, crate::errors::ParseDocumentError> {
    let mut input_ref = input;
    match parse_elements.parse_next(&mut input_ref) {
        Ok(elements) => {
            let doc = crate::Document::new(name, path, input, elements);
            Ok(doc)
        }
        Err(error) => Err(crate::errors::ParseDocumentError::Syntax {
            name: name.to_owned(),
            message: error.to_string(),
        }),
    }
}
