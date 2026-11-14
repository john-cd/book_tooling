//! Maps AST fragments back to byte ranges of the source they were parsed from.
//! The AST borrows its text from the source string instead of storing spans, so a
//! fragment's position is recovered from its address. This is the one place that
//! relies on that invariant; callers get a checked range or `None`.
//!
//! TODO Reviewed ??-??-2026.

use std::ops::Range;

/// Returns the byte range of `fragment` within `source`.
///
/// Returns `None` when `fragment` does not borrow from `source` (for example an
/// owned copy with the same text), or when the range does not fall on character
/// boundaries inside `source`.
#[must_use]
pub fn source_span(source: &str, fragment: &str) -> Option<Range<usize>> {
    let start = fragment
        .as_ptr()
        .addr()
        .checked_sub(source.as_ptr().addr())?;
    let end = start.checked_add(fragment.len())?;
    (end <= source.len() && source.is_char_boundary(start) && source.is_char_boundary(end))
        .then_some(start..end)
}

/// Tests for mapping borrowed slices to source byte ranges.
#[cfg(test)]
mod tests {
    use super::*;

    /// A subslice maps to its offsets, including the empty and whole-source cases.
    #[test]
    fn subslices_map_to_their_offsets() {
        let source = "h\u{e9}llo [world]";
        let world = source.get(8..13).expect("char boundaries");
        assert_eq!(world, "world");
        assert_eq!(source_span(source, world), Some(8..13));
        assert_eq!(source_span(source, source), Some(0..source.len()));
        assert_eq!(
            source_span(source, source.get(3..3).expect("empty")),
            Some(3..3)
        );
    }

    /// Equal text that is not borrowed from the source is rejected.
    #[test]
    fn foreign_text_with_equal_content_is_rejected() {
        let source = "[a]: b";
        let copy = source.to_owned();
        assert_eq!(source_span(source, &copy), None);
        assert_eq!(source_span(source, "[a]"), None);
    }

    /// A fragment that starts inside the source but runs past its end is rejected.
    #[test]
    fn fragment_extending_past_source_is_rejected() {
        let buffer = String::from("abcdef");
        let source = buffer.get(..3).expect("prefix");
        let fragment = buffer.get(2..5).expect("middle");
        assert_eq!(source_span(source, fragment), None);
    }
}
