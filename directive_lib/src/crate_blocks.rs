//! Process crate block directives e.g. `{{#crate crt}}`
//!
//! Crate blocks combine multiple crate and categories badges.

use core_lib::templates::DestinationKind;
use core_lib::templates::render_badge;
use parse_lib::ast::DirectiveData;

/// Renders a crate block based on the provided DirectiveData.
/// {{#crate parsing }} -> [![parsing]...
pub(super) fn render_crate_block(directive: &DirectiveData<'_>) -> String {
    // Note: Adjust `directive.content` to match your actual field name
    // in `DirectiveData` (e.g., `argument` or `value`).
    // If it's an Option, you may need to add `.unwrap_or_default()`.
    render_crate_block_inner(directive.content)
}

fn render_crate_block_inner(content: &str) -> String {
    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.is_empty() {
        return String::new();
    }

    let crate_name = parts[0];
    let mut badges = vec![render_badge(&DestinationKind::Crate, crate_name)];

    for cat in &parts[1..] {
        badges.push(render_badge(&DestinationKind::Category, cat));
    }

    badges.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Since we now parse the directive first, we only need to test
    // the inner rendering logic, not regex spacing and colon variations!

    #[test]
    fn test_crate_block() {
        assert_eq!(
            render_crate_block_inner("crt"),
            "[![crt][p~crt~crate~badge]][p~crt~crate]{{hi:crt}}"
        );
    }

    #[test]
    fn test_crate_block_with_hyphen() {
        assert_eq!(
            render_crate_block_inner("x_y-z"),
            "[![x_y-z][p~x_y-z~crate~badge]][p~x_y-z~crate]{{hi:x_y-z}}"
        );
    }

    #[test]
    fn test_crate_block_with_categories() {
        assert_eq!(
            render_crate_block_inner("crt cat1 cat-2"),
            "[![crt][p~crt~crate~badge]][p~crt~crate]{{hi:crt}} \
             [![cat1][c~cat1~cat~badge]][c~cat1~cat]{{hi:cat1}} \
             [![cat-2][c~cat-2~cat~badge]][c~cat-2~cat]{{hi:cat-2}}"
        );
    }
}
