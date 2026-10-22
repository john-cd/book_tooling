//! Parser for reference-style links.
//!
//! TODO Reviewed ??-??-2026

use winnow::ModalResult;
use winnow::Parser as _;
use winnow::combinator::seq;
use winnow::error::StrContext::{Expected, Label};
use winnow::error::StrContextValue::Description;

use crate::ast::{Element, ReferenceStyleLinkData};
use crate::parse_parts::parse_link_label;
use crate::parse_parts::parse_link_text;

/// Parses a reference-style link: `[text][label]`.
///
/// # Errors
///
/// Returns a backtrack error if the input does not begin with a reference-style link.
pub fn parse_reference_style_link<'s>(input: &mut &'s str) -> ModalResult<Element<'s>> {
    seq!(parse_link_text, parse_link_label,)
        .map(|(text, label)| Element::ReferenceStyleLink(ReferenceStyleLinkData { text, label }))
        .context(Label("reference style link"))
        .context(Expected(Description("[text][label]")))
        .parse_next(input)
}
