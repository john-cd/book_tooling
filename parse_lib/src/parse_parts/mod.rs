//! Building blocks shared by the link and reference definition parsers.
//!
//! TODO Reviewed ??-??-2026.

/// Implements inline link behavior for the parse parts component, keeping the related rules and
/// data transformations at one boundary. The parent module calls these routines when it needs this
/// component’s behavior.
pub(crate) mod inline_link;
/// Implements link destination behavior for the parse parts component, keeping the related rules
/// and data transformations at one boundary. The parent module calls these routines when it needs
/// this component’s behavior.
pub(crate) mod link_destination;
/// Implements link label behavior for the parse parts component, keeping the related rules and data
/// transformations at one boundary. The parent module calls these routines when it needs this
/// component’s behavior.
pub(crate) mod link_label;
/// Implements link text behavior for the parse parts component, keeping the related rules and data
/// transformations at one boundary. The parent module calls these routines when it needs this
/// component’s behavior.
pub(crate) mod link_text;
/// Implements reference definition behavior for the parse parts component, keeping the related
/// rules and data transformations at one boundary. The parent module calls these routines when it
/// needs this component’s behavior.
pub(crate) mod reference_definition;

pub(crate) use inline_link::parse_link_title;
pub(crate) use link_destination::parse_link_destination;
pub(crate) use link_label::parse_link_label;
pub(crate) use link_text::parse_link_text;
