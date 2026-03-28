use crate::ast::*;
use crate::visit::Visitor;

/// A visitor that identifies and counts custom directives within a Markdown document.
///
/// It ignores directives located inside code spans or fenced code blocks.
#[derive(Default)]
pub struct DirectiveExtractor {
    // Store the extracted directive data or specific fields like the raw text.
    // This example simply keeps a count, but you can collect string slices here.
    pub directives_found: usize,
}

impl DirectiveExtractor {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'a> Visitor<'a> for DirectiveExtractor {
    fn visit_custom_directive(&mut self, _data: &'a DirectiveData<'a>) {
        self.directives_found += 1;

        // Example: If `DirectiveData` had a `raw_text` field, you could do:
        // self.found_directives.push(data.raw_text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_document;

    #[test]
    fn test_directive_extractor_skips_code_blocks() {
        let input = "\
This is a normal directive: {{#crate foo}}

This is in a code span: `{{#crate bar}}`

```rust
// Fenced code block
{{#crate baz}}
```

Another normal one: {{#example qux}}
";
        let doc = parse_document("test.md", None, input).expect("Failed to parse document");

        let mut extractor = DirectiveExtractor::new();
        extractor.visit_document(&doc);

        // We expect exactly 2 directives to be extracted (foo and qux).
        // The others are trapped safely inside CodeSpan and FencedCodeBlock nodes!
        assert_eq!(extractor.directives_found, 2);
    }
}
