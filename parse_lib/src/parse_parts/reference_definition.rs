//! Parses a complete Markdown reference definition: `[label]: url "title"`.
//!
//! TODO Reviewed ??-??-2026

use winnow::ModalResult;
use winnow::Parser as _;
use winnow::combinator::fail;
use winnow::combinator::opt;
use winnow::combinator::seq;
use winnow::token::take_while;

use super::inline_link::parse_link_title;
use super::link_destination::parse_link_destination;
use super::link_label::parse_link_label;
use crate::ast::Element;
use crate::ast::ReferenceDefinitionData;

/// Parses a complete link reference definition.
///
/// # Errors
///
/// Returns a backtrack error if the input does not begin with a link reference definition.
pub fn parse_link_reference_definition<'s>(input: &mut &'s str) -> ModalResult<Element<'s>> {
    let (label, _, _, dest, ws2, title, _) = seq!(
        parse_link_label,
        ":",
        take_while(0.., [' ', '\t', '\n', '\r']),
        parse_link_destination,
        take_while(0.., [' ', '\t', '\n', '\r']),
        opt(parse_link_title),
        take_while(0.., [' ', '\t'])
    )
    .parse_next(input)?;

    // CommonMark spec dictates that if a title is present, it must be separated from the
    // destination by whitespace.
    if title.is_some() && ws2.is_empty() {
        return fail.parse_next(input);
    }

    Ok(Element::ReferenceDefinition(ReferenceDefinitionData {
        label,
        url: dest,
        title,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `[label]: url` yields the label and URL with no title.
    #[test]
    fn test_parse_reference_definition_simple() {
        let mut input = "[my-label]: https://example.com";
        assert_eq!(
            parse_link_reference_definition.parse_next(&mut input),
            Ok(Element::ReferenceDefinition(ReferenceDefinitionData {
                label: "my-label",
                url: "https://example.com",
                title: None
            }))
        );
        assert_eq!(input, "");
    }

    /// The URL may be in angle brackets and the destination and title may sit on following indented
    /// lines.
    #[test]
    fn test_parse_reference_definition_with_title_and_newlines() {
        let mut input = "[my-label]: \n  <https://example.com> \n  \"A Title\"";
        assert_eq!(
            parse_link_reference_definition.parse_next(&mut input),
            Ok(Element::ReferenceDefinition(ReferenceDefinitionData {
                label: "my-label",
                url: "https://example.com",
                title: Some("A Title")
            }))
        );
    }

    /// A bracketed label followed by a URL without the colon is not a definition.
    #[test]
    fn test_parse_reference_definition_fails_missing_colon() {
        let mut input = "[my-label] https://example.com";
        parse_link_reference_definition
            .parse_next(&mut input)
            .expect_err("input should be rejected");
    }
}
