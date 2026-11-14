//! Custom parsing error.
//! See <https://docs.rs/winnow/0.7.11/winnow/_tutorial/chapter_7/index.html>.
//!
//! TODO Reviewed ??-??-2026.

use annotate_snippets::AnnotationKind;
use annotate_snippets::Group;
use winnow::error::ContextError;
use winnow::error::ParseError;

/// Error returned by [`parse_document()`](crate::parse_document()) when a document cannot be
/// parsed.
#[derive(Debug, thiserror::Error)]
pub enum ParseDocumentError {
    /// The parser rejected the input.
    #[error("Failed to parse document '{name}':\n{message}")]
    Syntax {
        /// Name of the document, such as its file name.
        name: String,
        /// Rendered description of the parse failure.
        message: String,
    },
}

/// Custom parsing error.
#[derive(Debug)]
pub struct ParsingError {
    /// Human-readable description of the parse failure.
    message: String,
    // Byte spans are tracked, rather than line and column.
    // This makes it easier to operate on programmatically
    // and doesn't limit us to one definition for column count
    // which can depend on the output medium and application.
    /// Byte range of the input where parsing failed.
    span: std::ops::Range<usize>,
    /// Copy of the input that was being parsed.
    input: String,
}

impl std::error::Error for ParsingError {}

impl std::fmt::Display for ParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message =
            &[
                Group::with_title(annotate_snippets::Level::ERROR.primary_title(&self.message))
                    .element(
                        annotate_snippets::Snippet::source(&self.input)
                            .fold(true)
                            .annotation(AnnotationKind::Primary.span(self.span.clone())),
                    ),
            ];
        let renderer = annotate_snippets::Renderer::styled();
        let rendered = renderer.render(message);
        rendered.fmt(f)
    }
}

impl ParsingError {
    /// Convert from `winnow::error::ParseError`.
    ///
    /// Avoiding `From`, so `winnow` types don't become part of our public API.
    #[must_use]
    pub fn from_parse(error: &ParseError<&str, ContextError>) -> Self {
        // The default renderer for `ContextError` is still used but that can be
        // customized as well to better fit your needs.
        let message = error.inner().to_string();
        let input = (*error.input()).to_owned();
        // Assume the error span is only for the first `char`.
        let span = error.char_span();
        Self {
            message,
            span,
            input,
        }
    }
}

/// Tests for converting parser failures to source diagnostics.
#[cfg(test)]
mod tests {
    use winnow::Parser as _;
    use winnow::token::literal;

    use super::*;

    /// Verifies: render.
    fn render(message: &str, input: &str, span: std::ops::Range<usize>) -> String {
        ParsingError {
            message: message.to_owned(),
            span,
            input: input.to_owned(),
        }
        .to_string()
    }

    /// Verifies: single line error shows message and source.
    #[test]
    fn single_line_error_shows_message_and_source() {
        let out = render("bad token", "let x = ?;", 8..9);
        assert!(out.contains("bad token"), "{out}");
        assert!(out.contains("let x = ?;"), "{out}");
    }

    /// Verifies: multiline error shows the offending line.
    #[test]
    fn multiline_error_shows_the_offending_line() {
        let out = render("oops", "line one\nline two\nline three\n", 14..17);
        assert!(out.contains("line two"), "{out}");
        assert!(out.contains("oops"), "{out}");
    }

    /// Verifies: empty input and empty span render without panicking.
    #[test]
    fn empty_input_and_empty_span_render_without_panicking() {
        let out = render("empty", "", 0..0);
        assert!(out.contains("empty"), "{out}");
    }

    /// Verifies: span at end of input renders.
    #[test]
    fn span_at_end_of_input_renders() {
        let out = render("eof", "abc", 3..3);
        assert!(out.contains("abc"), "{out}");
    }

    /// Verifies: from parse captures input and first character span.
    #[test]
    fn from_parse_captures_input_and_first_character_span() {
        let parse_error = literal::<_, _, ContextError>("abc")
            .parse("abx")
            .expect_err("mismatch");
        let error = ParsingError::from_parse(&parse_error);
        assert_eq!(error.input, "abx");
        assert_eq!(error.span, 0..1);
        assert!(error.to_string().contains("abx"));
    }

    /// Verifies: document errors name the document and carry the parser message.
    #[test]
    fn document_error_display_names_the_document() {
        let error = ParseDocumentError::Syntax {
            name: "chapter.md".to_owned(),
            message: "bad token".to_owned(),
        };
        assert_eq!(
            error.to_string(),
            "Failed to parse document 'chapter.md':
bad token"
        );
        let wrapped = anyhow::Error::new(error).context("while checking");
        assert!(format!("{wrapped:#}").contains("chapter.md"));
    }

    /// Verifies: implements std error.
    #[test]
    fn implements_std_error() {
        /// Applies the rules used by errors to keep parsing error state valid for its callers.
        fn assert_error<E: std::error::Error>() {}
        assert_error::<ParsingError>();
    }
}
