//! Process crate block directives e.g. `{{#crate crt}}`
//!
//! Crate blocks combine multiple crate and categories badges.

use parse_lib::ast::DirectiveData;

use crate::common::templates::DestinationKind;
use crate::common::templates::render_badge;

/// Renders a crate block based on the provided DirectiveData.
pub(super) fn render_crate_block(directive: &DirectiveData<'_>) -> String {
    if let DirectiveData::CrateBlock {
        crate_name,
        additional_categories,
    } = directive
    {
        let mut badges = vec![render_badge(&DestinationKind::Crate, crate_name)];

        for cat in additional_categories {
            badges.push(render_badge(&DestinationKind::Category, cat));
        }

        badges.join(" ")
    } else {
        String::new()
    }
}
