//! Parsers for plain text and for the fallback single character.
//!
//! TODO Reviewed ??-??-2026.

use winnow::ModalResult;
use winnow::Parser as _;
use winnow::combinator::alt;
use winnow::token::take_till;

use crate::ast::Element;

/// Characters that can begin other inline or block syntax, and therefore end a plain-text run.
const SPECIAL_CHARS: [char; 16] = [
    '<', '[', '(', '`', '{', '!', '#', '-', '|', ']', ')', '}', '*', '_', '~', '\\',
];

/// An underscore between two alphanumeric characters (`snake_case`, `fs_extra`) is intraword
/// and part of the text: as in `CommonMark`, it can neither open nor close emphasis.
/// # Errors
/// Returns a backtrack error if the input is empty or starts with a character that begins other
/// syntax.
pub fn parse_plain_text<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    let source: &'a str = input;
    // take_till with 1.. ensures we consume at least one non-special character.
    // This takes advantage of highly optimized SIMD operations in winnow.
    let mut consumed = take_till(1.., SPECIAL_CHARS).parse_next(input)?.len();

    while let Some(rest) = source.get(consumed..)
        && let Some(after_underscore) = rest.strip_prefix('_')
        && source
            .get(..consumed)
            .and_then(|before| before.chars().next_back())
            .is_some_and(char::is_alphanumeric)
        && after_underscore
            .chars()
            .next()
            .is_some_and(char::is_alphanumeric)
    {
        consumed += 1 + after_underscore
            .find(SPECIAL_CHARS)
            .unwrap_or(after_underscore.len());
    }

    let content = source.get(..consumed).unwrap_or(source);
    *input = source.get(consumed..).unwrap_or_default();
    Ok(Element::Text(crate::ast::TextData { content }))
}

/// Fallback parser that handles escaped characters and consumes exactly one character if not
/// escaped.
/// # Errors
/// Returns a backtrack error if the input is empty.
pub fn parse_fallback_char<'a>(input: &mut &'a str) -> ModalResult<Element<'a>> {
    alt((
        winnow::combinator::seq!(_: "\\", winnow::token::any).take(),
        winnow::token::any.take(),
    ))
    .map(|content| Element::Text(crate::ast::TextData { content }))
    .parse_next(input)
}

/// Tests for plain text and fallback character parsing.
#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies: plain.
    fn plain(source: &str) -> Option<(&str, &str)> {
        let mut input = source;
        match parse_plain_text(&mut input) {
            Ok(Element::Text(crate::ast::TextData { content })) => Some((content, input)),
            _ => None,
        }
    }

    /// Verifies: fallback.
    fn fallback(source: &str) -> Option<(&str, &str)> {
        let mut input = source;
        match parse_fallback_char(&mut input) {
            Ok(Element::Text(crate::ast::TextData { content })) => Some((content, input)),
            _ => None,
        }
    }

    /// Verifies: plain text stops at the next special character.
    #[test]
    fn plain_text_stops_at_the_next_special_character() {
        assert_eq!(plain("hello [link]"), Some(("hello ", "[link]")));
        assert_eq!(plain("a`code`"), Some(("a", "`code`")));
    }

    /// Verifies: plain text consumes everything without specials.
    #[test]
    fn plain_text_consumes_everything_without_specials() {
        assert_eq!(plain("just words"), Some(("just words", "")));
    }

    /// Verifies: plain text rejects empty input and leading specials.
    #[test]
    fn plain_text_rejects_empty_input_and_leading_specials() {
        assert_eq!(plain(""), None);
        assert_eq!(plain("[x]"), None);
        assert_eq!(plain("_emphasis_"), None);
    }

    /// Verifies: intraword underscores stay in the text.
    #[test]
    fn intraword_underscores_stay_in_the_text() {
        assert_eq!(plain("snake_case word"), Some(("snake_case word", "")));
        assert_eq!(plain("fs_extra_crate["), Some(("fs_extra_crate", "[")));
    }

    /// Verifies: leading or trailing underscore ends the run.
    #[test]
    fn leading_or_trailing_underscore_ends_the_run() {
        assert_eq!(plain("word_ more"), Some(("word", "_ more")));
        assert_eq!(plain("a _b"), Some(("a ", "_b")));
    }

    /// Verifies: unicode text is not split mid character.
    #[test]
    fn unicode_text_is_not_split_mid_character() {
        assert_eq!(
            plain("h\u{e9}llo_w\u{f6}rld!"),
            Some(("h\u{e9}llo_w\u{f6}rld", "!"))
        );
    }

    /// Verifies: fallback takes exactly one character.
    #[test]
    fn fallback_takes_exactly_one_character() {
        assert_eq!(fallback("[rest"), Some(("[", "rest")));
        assert_eq!(fallback("\u{e9}a"), Some(("\u{e9}", "a")));
    }

    /// Verifies: fallback keeps an escape with its character.
    #[test]
    fn fallback_keeps_an_escape_with_its_character() {
        assert_eq!(fallback(r"\[rest"), Some((r"\[", "rest")));
    }

    /// Verifies: fallback rejects empty input.
    #[test]
    fn fallback_rejects_empty_input() {
        assert_eq!(fallback(""), None);
    }
}
