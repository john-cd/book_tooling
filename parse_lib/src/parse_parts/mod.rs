//! Building blocks shared by the link and reference definition parsers.
//!
//! TODO Reviewed ??-??-2026.

pub(crate) mod inline_link;
pub(crate) mod link_destination;
pub(crate) mod link_label;
pub(crate) mod link_text;
pub(crate) mod reference_definition;

pub(crate) use inline_link::parse_link_title;
pub(crate) use link_destination::parse_link_destination;
pub(crate) use link_label::parse_link_label;
pub(crate) use link_text::parse_link_text;
