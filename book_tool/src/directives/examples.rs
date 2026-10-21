// TODO Reviewed ??-??-2026
//! Process {{#example some_example}} directives.

use parse_lib::ast::DirectiveData;

/// The {{#example some_example}} should be replaced by an {{#include ...}} directive.
pub(super) fn render_example(directive: &DirectiveData<'_>) -> String {
    if let DirectiveData::ExampleBlock { name } = directive {
        let name = name.trim();
        if name.is_empty() {
            return "{{#example}}".to_string();
        }
        format!("{{{{#include examples/{}.md}}}}", name)
    } else {
        String::new()
    }
}
