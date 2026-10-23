//! Parsers for inline and reference-style images.
//!
//! TODO Reviewed ??-??-2026

use winnow::ModalResult;
use winnow::Parser as _;
use winnow::ascii::space0;
use winnow::combinator::opt;
use winnow::combinator::preceded;
use winnow::combinator::seq;
use winnow::error::StrContext::{Expected, Label};
use winnow::error::StrContextValue::Description;

use crate::ast::{Element, InlineImageData, ReferenceStyleImageData};
use crate::parse_parts::{
    parse_link_destination, parse_link_label, parse_link_text, parse_link_title,
};

/// Parses an inline image: `![desc](url "title")`.
///
/// # Errors
///
/// Returns a backtrack error if the input does not begin with an inline image.
pub fn parse_inline_image<'s>(input: &mut &'s str) -> ModalResult<Element<'s>> {
    seq!(
        _: '!',
        parse_link_text,
        _: '(',
        parse_link_destination,
        opt(preceded(space0, parse_link_title)),
        _: ')',
    )
    .map(|(image_description, url, title)| {
        Element::InlineImage(InlineImageData {
            image_description,
            url,
            title,
        })
    })
    .context(Label("inline image"))
    .context(Expected(Description(r#"![desc](url "title")"#)))
    .parse_next(input)
}

/// Parses a reference-style image: `![desc][label]`.
///
/// # Errors
///
/// Returns a backtrack error if the input does not begin with a reference-style image.
pub fn parse_reference_style_image<'s>(input: &mut &'s str) -> ModalResult<Element<'s>> {
    seq!(
        _: "!",
        parse_link_text,
        parse_link_label,
    )
    .map(|(image_description, label)| {
        Element::ReferenceStyleImage(ReferenceStyleImageData {
            image_description,
            label,
        })
    })
    .context(Label("reference-style image"))
    .context(Expected(Description("![desc][label]")))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `![alt](url "title")` yields the description, URL and title.
    #[test]
    fn test_parse_inline_image() {
        let input = r#"![img](http://example.com/image.png "An example image")"#;
        let expected = Element::InlineImage(InlineImageData {
            image_description: "img",
            url: "http://example.com/image.png",
            title: Some("An example image"),
        });
        assert_eq!(
            parse_inline_image.parse(input).expect("input should parse"),
            expected
        );

        let input_with_desc = "![img](http://example.com/image.png)";
        let expected_with_desc = Element::InlineImage(InlineImageData {
            image_description: "img",
            url: "http://example.com/image.png",
            title: None,
        });
        assert_eq!(
            parse_inline_image
                .parse(input_with_desc)
                .expect("input should parse"),
            expected_with_desc
        );

        let input_without_desc = r#"![](http://example.com/image.png "No description")"#;
        let expected_without_desc = Element::InlineImage(InlineImageData {
            image_description: "",
            url: "http://example.com/image.png",
            title: Some("No description"),
        });
        assert_eq!(
            parse_inline_image
                .parse(input_without_desc)
                .expect("input should parse"),
            expected_without_desc
        );
    }

    /// `![alt][label]` yields the description and the reference label.
    #[test]
    fn test_parse_reference_style_image() {
        let input = "![img][label]";
        let expected = Element::ReferenceStyleImage(ReferenceStyleImageData {
            image_description: "img",
            label: "label",
        });
        assert_eq!(
            parse_reference_style_image
                .parse(input)
                .expect("input should parse"),
            expected
        );

        let input_with_desc = "![img with spaces][label]";
        let expected_with_desc = Element::ReferenceStyleImage(ReferenceStyleImageData {
            image_description: "img with spaces",
            label: "label",
        });
        assert_eq!(
            parse_reference_style_image
                .parse(input_with_desc)
                .expect("input should parse"),
            expected_with_desc
        );
        let input_without_desc = "![][label]";
        let expected_without_desc = Element::ReferenceStyleImage(ReferenceStyleImageData {
            image_description: "",
            label: "label",
        });
        assert_eq!(
            parse_reference_style_image
                .parse(input_without_desc)
                .expect("input should parse"),
            expected_without_desc
        );
    }

    /// An unterminated title or a missing closing parenthesis is rejected.
    #[test]
    fn test_parse_inline_image_with_invalid_input() {
        let invalid_input = r#"![img](http://example.com/image.png "An example image"#;
        parse_inline_image
            .parse(invalid_input)
            .expect_err("input should be rejected");

        let invalid_input_no_closing_paren = "![img](http://example.com/image.png";
        parse_inline_image
            .parse(invalid_input_no_closing_paren)
            .expect_err("input should be rejected");

        let invalid_input_no_destination = "![img]()";
        parse_inline_image
            .parse(invalid_input_no_destination)
            .expect_err("input should be rejected");

        let invalid_input_no_label = "![img][";
        parse_reference_style_image
            .parse(invalid_input_no_label)
            .expect_err("input should be rejected");

        let invalid_input_no_closing_bracket = "![img][label";
        parse_reference_style_image
            .parse(invalid_input_no_closing_bracket)
            .expect_err("input should be rejected");

        let invalid_input_no_label_content = "![img][]";
        parse_reference_style_image
            .parse(invalid_input_no_label_content)
            .expect_err("input should be rejected");
    }
}
