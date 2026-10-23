//! Parse link destinations.
//!
//! A link destination consists of either
//! - a sequence of zero or more characters between an opening < and a closing > that contains no
//!   line endings or unescaped < or > characters, (implemented in `angle_brackets.rs`), or
//! - a nonempty sequence of characters that does not start with <, does not include ASCII control
//!   characters or space character, and includes parentheses only if (a) they are backslash-escaped
//!   or (b) they are part of a balanced pair of unescaped parentheses.
//!
//! (Implementations may impose limits on parentheses nesting to avoid performance issues,
//! but at least three levels of nesting should be supported.)
//!
//! See <https://spec.commonmark.org/0.31.2/#link-destination>.
//!
//! TODO Reviewed ??-??-2026

mod angle_brackets;
mod nonempty_sequence;

pub(crate) use angle_brackets::parse_angle_brackets;
pub(crate) use nonempty_sequence::parse_non_empty_sequence;
use winnow::combinator::alt;
use winnow::combinator::fail;
use winnow::error::StrContext::Expected;
use winnow::error::StrContext::Label;
use winnow::error::StrContextValue::Description;
use winnow::prelude::*;

/// Parses a link destination.
///
/// See <https://spec.commonmark.org/0.31.2/#link-destination>.
pub(crate) fn parse_link_destination<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    alt((
        parse_angle_brackets,
        parse_non_empty_sequence,
        fail
        .context(Label("link destination"))
        .context(Expected(Description("a sequence of zero or more characters between an opening < and a closing >; or a nonempty sequence of characters that does not start with <")))
    ))
    .parse_next(input)
}
