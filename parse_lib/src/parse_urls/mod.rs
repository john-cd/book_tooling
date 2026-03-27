//! - Recognize an absolute HTTP / HTTPS URL.
//! - Recognize specific URLs (e.g., specific domains and path patterns).

pub mod url_kind;
mod url_valid_chars;

pub use url_kind::*;
use url_valid_chars::*;
use winnow::ModalResult;
use winnow::Parser;
use winnow::combinator::alt;
use winnow::combinator::seq;
use winnow::error::ContextError;
use winnow::error::ErrMode;
use winnow::stream::Stream;

/// Recognize an absolute HTTP / HTTPS URL.
/// Returns both the raw URL string and its classified kind.
pub fn parse_naked_url<'s>(input: &mut &'s str) -> ModalResult<(&'s str, UrlKind<'s>)> {
    // If the child parser was successful, return the consumed input as produced value.
    let mut url_raw = seq!(
        alt(("http://", "https://")), // Protocols.
        parse_url_chars,
    )
    .take()
    .map_err(ErrMode::Backtrack);

    let start = input.checkpoint();
    let url_text = url_raw.parse_next(input)?;

    match url_kind(url_text) {
        Ok(kind) => Ok((url_text, kind)),
        Err(_e) => {
            input.reset(&start);
            // This shouldn't really happen if parse_url_chars is correct.
            Err(ErrMode::Backtrack(ContextError::default()))
        }
    }
}
