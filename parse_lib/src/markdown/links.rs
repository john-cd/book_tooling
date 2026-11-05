//! Parser for reference-style links.
//!
//! TODO Reviewed ??-??-2026

use winnow::ModalResult;
use winnow::Parser as _;
use winnow::combinator::seq;
use winnow::error::StrContext::Expected;
use winnow::error::StrContext::Label;
use winnow::error::StrContextValue::Description;

use crate::ast::Element;
use crate::ast::ReferenceStyleLinkData;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies: link.
    fn link(source: &str) -> Option<(&str, &str, &str)> {
        let mut input = source;
        match parse_reference_style_link(&mut input) {
            Ok(Element::ReferenceStyleLink(ReferenceStyleLinkData { text, label })) => {
                Some((text, label, input))
            }
            _ => None,
        }
    }

    /// Verifies: parses text and label.
    #[test]
    fn parses_text_and_label() {
        assert_eq!(link("[text][label] rest"), Some(("text", "label", " rest")));
    }

    /// Verifies: missing label is rejected.
    #[test]
    fn missing_label_is_rejected() {
        assert_eq!(link("[text] rest"), None);
    }

    /// Verifies: space between brackets is rejected.
    #[test]
    fn space_between_brackets_is_rejected() {
        assert_eq!(link("[text] [label]"), None);
    }

    /// Verifies: plain text is rejected.
    #[test]
    fn plain_text_is_rejected() {
        assert_eq!(link("text"), None);
    }

    /// Verifies: unclosed label is rejected.
    #[test]
    fn unclosed_label_is_rejected() {
        assert_eq!(link("[text][label"), None);
    }
}
