// TODO Reviewed ??-??-2026
use winnow::ModalResult;
use winnow::Parser;
use winnow::combinator::seq;
use winnow::error::StrContext::*;
use winnow::error::StrContextValue::*;

use crate::ast::*;
use crate::parse_parts::parse_link_label;
use crate::parse_parts::parse_link_text;

/// Parses a reference-style link: `[text][label]`.
pub fn parse_reference_style_link<'s>(input: &mut &'s str) -> ModalResult<Element<'s>> {
    seq!(parse_link_text, parse_link_label,)
        .map(|(text, label)| Element::ReferenceStyleLink(ReferenceStyleLinkData { text, label }))
        .context(Label("reference style link"))
        .context(Expected(Description("[text][label]")))
        .parse_next(input)
}
