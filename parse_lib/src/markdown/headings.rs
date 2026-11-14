//! Parses Markdown ATX Headings (with {# anchor} mdbook extension).
//! See <https://spec.commonmark.org/0.31.2/#atx-headings>.
//!
//! TODO Reviewed ??-??-2026.

use winnow::ModalResult;
use winnow::Parser as _;
use winnow::ascii::line_ending;
use winnow::ascii::space0;
use winnow::ascii::space1;
use winnow::combinator::alt;
use winnow::combinator::delimited;
use winnow::combinator::eof;
use winnow::combinator::opt;
use winnow::combinator::preceded;
use winnow::combinator::repeat;
use winnow::combinator::seq;
use winnow::combinator::terminated;
use winnow::error::StrContext::Expected;
use winnow::error::StrContext::Label;
use winnow::error::StrContextValue::Description;
use winnow::stream::Stream as _;
use winnow::token::take_while;

use crate::ast::Element;
use crate::ast::HeadingData;

/// Parses the opening run of one to six `#` characters and returns the heading level.
fn parse_opening_hashes(input: &mut &str) -> ModalResult<u8> {
    preceded(
        // Parse indentation. Up to three spaces of indentation are allowed.
        take_while(0..=3, ' '),
        // Opening sequence of 1–6 unescaped # characters
        repeat(1.., "#")
            .verify_map(|count: usize| u8::try_from(count).ok().filter(|level| *level <= 6)),
    )
    .parse_next(input)
}
/// Consumes an mdBook heading attribute of the form `{#id}` and returns the trimmed ID.
///
/// # Errors
/// Backtracks if the delimiters or nonblank ID are missing or the ID contains an unsupported
/// character.
fn parse_attribute_block<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    let attributes_content_chars = |c: char| {
        c != '{' && c != '}' && c != '<' && c != '>' && c != '\\' && c != '\n' && c != '\r'
    };

    let parse_attributes_content = delimited(
        space0,
        preceded("#", take_while(1.., attributes_content_chars)),
        space0,
    )
    .verify_map(|raw_id: &str| {
        let id = raw_id.trim();
        if id.is_empty() {
            None // No ID specified.
        } else {
            Some(id) // Return the ID.
        }
    });

    terminated(delimited("{", parse_attributes_content, "}"), space0)
        .context(Label("attribute block"))
        .context(Expected(Description(
            r"attribute block with ID within {# }. No { } < > \ or newlines inside.",
        )))
        .parse_next(input)
}

/// Consumes the rest of an ATX heading line and returns its optional trimmed text and explicit
/// anchor ID.
fn parse_heading_content<'s>(
    input: &mut &'s str,
) -> ModalResult<(Option<&'s str>, Option<&'s str>)> {
    // Take characters until we hit a newline or a potential attribute block start '{' or #.
    let mut content =
        take_while(0.., |c| c != '\n' && c != '\r' && c != '{' && c != '#').map(|content: &str| {
            let cnt = content.trim();
            (!cnt.is_empty()).then_some(cnt)
        });

    // Optional closing sequence of any number of unescaped # characters.
    // The optional closing sequence of #s must be preceded by spaces or tabs and may be followed by
    // spaces or tabs only.
    let mut opt_closing_hashes = opt(terminated(take_while(1.., |c| c == '#'), space0));

    seq!(
        // The opening sequence of # characters must be followed by spaces or tabs.
        _: opt(space1),
        content, // Content of the heading.
        _: opt_closing_hashes,
        opt(parse_attribute_block),
        _: alt((line_ending, eof))
    )
    .context(Label("heading content"))
    .context(Expected(Description(
        " a new line or heading content followed by an optional attribute block",
    )))
    .parse_next(input)
}

/// Consumes an ATX heading and returns its level, text, and optional mdBook anchor.
///
/// # Errors
/// Backtracks if the input does not start with a valid heading.
pub fn parse_atx_heading<'s>(input: &mut &'s str) -> ModalResult<Element<'s>> {
    let start = input.checkpoint();
    let mut hashes = parse_opening_hashes;
    let mut content = parse_heading_content;

    match hashes.parse_next(input) {
        Ok(level) => {
            if let Ok((text, id)) = content.parse_next(input) {
                Ok(Element::Heading(HeadingData {
                    level,
                    content: text,
                    id,
                }))
            } else {
                input.reset(&start);
                // Return a backtrack error instead of a cut error.
                Err(winnow::error::ErrMode::Backtrack(
                    winnow::error::ContextError::default(),
                ))
            }
        }
        Err(e) => {
            input.reset(&start);
            Err(e)
        }
    }
}

/// Tests for heading levels and explicit anchors.
#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies: heading.
    fn heading(source: &str) -> Option<(u8, Option<&str>, Option<&str>, &str)> {
        let mut input = source;
        match parse_atx_heading(&mut input) {
            Ok(Element::Heading(HeadingData { level, content, id })) => {
                Some((level, content, id, input))
            }
            _ => None,
        }
    }

    /// Verifies: levels one through six are accepted.
    #[test]
    fn levels_one_through_six_are_accepted() {
        for level in 1..=6_u8 {
            let source = format!("{} Title\n", "#".repeat(usize::from(level)));
            assert_eq!(heading(&source), Some((level, Some("Title"), None, "")));
        }
    }

    /// Verifies: seven hashes are not a heading.
    #[test]
    fn seven_hashes_are_not_a_heading() {
        assert_eq!(heading("####### Title\n"), None);
    }

    /// Verifies: text without hashes is not a heading.
    #[test]
    fn text_without_hashes_is_not_a_heading() {
        assert_eq!(heading("Title\n"), None);
    }

    /// Verifies: up to three leading spaces are allowed.
    #[test]
    fn up_to_three_leading_spaces_are_allowed() {
        assert_eq!(heading("   # Title\n"), Some((1, Some("Title"), None, "")));
        assert_eq!(heading("    # Title\n"), None);
    }

    /// Verifies: closing hashes are stripped.
    #[test]
    fn closing_hashes_are_stripped() {
        assert_eq!(heading("## Title ##\n"), Some((2, Some("Title"), None, "")));
    }

    /// Verifies: attribute block provides the id.
    #[test]
    fn attribute_block_provides_the_id() {
        assert_eq!(
            heading("# Title {#my-id}\n"),
            Some((1, Some("Title"), Some("my-id"), ""))
        );
    }

    /// Verifies: empty heading has no content.
    #[test]
    fn empty_heading_has_no_content() {
        assert_eq!(heading("#\n"), Some((1, None, None, "")));
    }

    /// Verifies: heading at end of input needs no newline.
    #[test]
    fn heading_at_end_of_input_needs_no_newline() {
        assert_eq!(heading("# Title"), Some((1, Some("Title"), None, "")));
    }

    /// Verifies: only the first line is consumed.
    #[test]
    fn only_the_first_line_is_consumed() {
        assert_eq!(
            heading("# One\nnext line\n"),
            Some((1, Some("One"), None, "next line\n"))
        );
    }

    /// Verifies: crlf line endings are consumed.
    #[test]
    fn crlf_line_endings_are_consumed() {
        assert_eq!(
            heading("# One\r\nrest"),
            Some((1, Some("One"), None, "rest"))
        );
    }
}
