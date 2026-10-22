//! Process `{{#example some_example}}` directives.
//!
//! TODO Reviewed ??-??-2026

use parse_lib::ast::DirectiveData;

/// The {{#example `some_example`}} should be replaced by an {{#include ...}} directive.
pub(super) fn render_example(directive: &DirectiveData<'_>) -> String {
    if let DirectiveData::ExampleBlock { name } = directive {
        let name = name.trim();
        if name.is_empty() {
            return "{{#example}}".to_owned();
        }
        format!("{{{{#include examples/{name}.md}}}}")
    } else {
        String::new()
    }
}
