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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_becomes_include_directive() {
        let out = render_example(&DirectiveData::ExampleBlock { name: "read_file" });
        assert_eq!(out, "{{#include examples/read_file.md}}");
    }

    #[test]
    fn example_name_is_trimmed() {
        let out = render_example(&DirectiveData::ExampleBlock {
            name: "  padded \t",
        });
        assert_eq!(out, "{{#include examples/padded.md}}");
    }

    #[test]
    fn blank_example_name_is_left_as_is() {
        for name in ["", "   "] {
            assert_eq!(
                render_example(&DirectiveData::ExampleBlock { name }),
                "{{#example}}"
            );
        }
    }

    #[test]
    fn other_directives_render_empty() {
        let out = render_example(&DirectiveData::CategoryBlock { categories: vec![] });
        assert_eq!(out, "");
    }
}
