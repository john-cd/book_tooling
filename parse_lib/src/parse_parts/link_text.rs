//! Parses link text.
//!
//! Simplified from <https://spec.commonmark.org/0.31.2/#link-text>:
//!
//! A link text consists of a sequence of zero or more inline elements enclosed by square brackets ([ and ]). The following rules apply:
//! - Links may not contain other links, at any level of nesting. If multiple otherwise valid link definitions appear nested inside each other, the inner-most definition is used.
//! - Brackets are allowed in the link text only if (a) they are backslash-escaped or (b) they appear as a matched pair of brackets, with an open bracket [, a sequence of zero or more inlines, and a close bracket ].
//! - Backtick code spans, autolinks, and raw HTML tags bind more tightly than the brackets in link text. Thus, for example, [foo`]` could not be a link text, since the second ] is part of a code span.
//! - The brackets in link text bind more tightly than markers for emphasis and strong emphasis. Thus, for example, `*[foo*](url)` is a link.

use winnow::Parser;
use winnow::ascii::take_escaped;
use winnow::combinator::alt;
use winnow::combinator::cut_err;
use winnow::combinator::delimited;
use winnow::combinator::repeat;
use winnow::error::ModalResult;
use winnow::error::StrContext::*;
use winnow::error::StrContextValue::*;
use winnow::token::one_of;
use winnow::token::take_while;
use winnow::token::take_while;

/// Parses text that may contain escaped brackets or backslashes: \[ or \] or \\.
///
/// It must consume at least one character.
fn parse_text_with_escapes<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    let allowed_chars = take_while(1.., |c: char| {
        c != '[' && c != ']' && c != '\\' && c != '`' && c != '<' && c != '!'
    });

    take_escaped(allowed_chars, '\\', one_of(['[', ']', '\\']))
        .verify(|s: &str| !s.is_empty())
        .context(Label("text with Markdown 'link text'"))
        .context(Expected(Description(
            r"characters. May contain \-escaped [ or ] or \",
        )))
        .parse_next(input)
}

/// Parses content within balanced brackets.
/// It can contain text with escaped brackets, or nested balanced parentheses.
fn parse_balanced_brackets_content<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    repeat::<_, _, (), _, _>(
    repeat::<_, _, (), _, _>(
        0..,
        alt((
            parse_balanced_brackets.void(), // Recursively parse.
            parse_text_with_escapes.void(),
        )),
    )
    .take()
    .parse_next(input)
}

// Parse pairs of brackets within Markdown 'link text' (e.g., `[bar]` within `[foo [bar] baz]`)
// This inner content *cannot* contain full links.
fn parse_balanced_brackets<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    delimited(
        "[",
        parse_balanced_brackets_content,
        cut_err("]"), // `cut` ensures a matching `]` must be found.
    )
    .take()
    .context(Label(
        "balanced brackets nested within Markdown 'link text'",
    ))
    .context(Expected(Description(
        "matched pairs of brackets within the link text, e.g., [foo [bar] baz]",
    )))
    .parse_next(input)
}

/// Parses the contents of a Markdown link's text.
fn parse_link_text_content<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    repeat::<_, _, (), _, _>(
    repeat::<_, _, (), _, _>(
        0..,
        alt((
            // [parse image](https://github.com/john-cd/rust_howto/issues/1424)
            parse_balanced_brackets.void(),
            parse_text_with_escapes.void(),
            parse_balanced_brackets.void(),
            parse_text_with_escapes.void(),
        )),
    )
    .take()
    .parse_next(input)
}

/// Parses a Markdown link's text, which is enclosed in square brackets `[]`.
///
/// This parser handles:
/// - Any character within the brackets.
/// - Escaped square brackets (`\[` and `\]`) which should be treated as literal characters.
///
/// # Arguments
/// * `input` - The input string slice to parse.
///
/// # Returns
/// A `Result` containing the parsed link text as a `String` if successful,
/// or a `winnow::error::Err` if parsing fails.
///
/// # Examples
/// ```rust,ignore
/// use winnow::Parser;
/// use parse_lib::parts::parse_link_text;
///
/// assert_eq!(parse_link_text.parse("[Hello World]").unwrap(), "Hello World");
/// assert_eq!(parse_link_text.parse("[Link with \\[bracket\\]]").unwrap(), "Link with [bracket]");
/// assert_eq!(parse_link_text.parse("[]").unwrap(), "");
/// ```
pub(crate) fn parse_link_text<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    // The entire link text is delimited by `[` and `]`.
    delimited('[', parse_link_text_content, cut_err(']'))
        .context(Label("link text"))
        .context(Expected(Description(
            "content of a Markdown link's text, enclosed in square brackets",
        )))
        .parse_next(input)
}
