//! Process {{#example some_example}} directives.

use parse_lib::ast::DirectiveData;

/// The {{#example some_example}} should be replaced by an {{#include ...}} directive
/// that embeds the contents of the file `some_example.md` in the proper `crates` subfolder.
pub(super) fn render_example(directive: &DirectiveData<'_>) -> String {
    render_example_inner(directive.content)
}

fn render_example_inner(content: &str) -> String {
    let name = content.trim();
    if name.is_empty() {
        return format!("{{{{#example{}}}}}", content);
    }
    format!("{{{{#include examples/{}.md}}}}", name)
}

// The following variations should be accepted:
//
// {{#example some_example}}
// {{#example some_example }}
// {{# example some_example}}
// {{ #example some_example}}
//
// The following are incomplete directives:
//
// {{#example}}
// {{#example }}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_example() {
        let expected = "{{#include examples/some_example.md}}";
        assert_eq!(render_example_inner(" some_example"), expected);
    }

    #[test]
    fn test_replace_example_with_spaces() {
        let expected = "{{#include examples/some_example_2.md}}";
        assert_eq!(render_example_inner("  some_example_2 "), expected);
    }

    #[test]
    fn test_no_replace_incomplete_example() {
        let content = "";
        let expected = "{{#example}}";
        assert_eq!(render_example_inner(content), expected);
    }

    #[test]
    fn test_no_replace_incomplete_example_with_spaces() {
        let content = " ";
        let expected = "{{#example }}";
        assert_eq!(render_example_inner(content), expected);
    }
}
