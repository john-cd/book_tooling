use super::Element;
use super::DirectiveData;

pub fn reconstruct_markdown(elements: &[Element]) -> String {
    let mut result = String::new();
    for element in elements {
        match element {
            Element::Autolink(data) => {
                result.push('<');
                result.push_str(data.url);
                result.push('>');
            }
            Element::InlineLink(data) => {
                result.push('[');
                result.push_str(data.text);
                result.push_str("](");
                result.push_str(data.url);
                if let Some(title) = data.title {
                    result.push_str(" \"");
                    result.push_str(title);
                    result.push('"');
                }
                result.push(')');
            }
            Element::ReferenceStyleLink(data) => {
                result.push('[');
                result.push_str(data.text);
                result.push_str("][");
                result.push_str(data.label);
                result.push(']');
            }
            Element::InlineImage(data) => {
                result.push_str("![");
                result.push_str(data.image_description);
                result.push_str("](");
                result.push_str(data.url);
                if let Some(title) = data.title {
                    result.push_str(" \"");
                    result.push_str(title);
                    result.push('"');
                }
                result.push(')');
            }
            Element::ReferenceStyleImage(data) => {
                result.push_str("![");
                result.push_str(data.image_description);
                result.push_str("][");
                result.push_str(data.label);
                result.push(']');
            }
            Element::ReferenceDefinition(data) => {
                result.push('[');
                result.push_str(data.label);
                result.push_str("]: ");
                result.push_str(data.url);
                if let Some(title) = data.title {
                    result.push_str(" \"");
                    result.push_str(title);
                    result.push('"');
                }
                result.push('\n');
            }
            Element::WikiLink(data) => {
                result.push_str("[[");
                result.push_str(data.target);
                if let Some(display) = data.display {
                    result.push_str(" | ");
                    result.push_str(display);
                }
                result.push_str("]]");
                if let Some(after) = data.immediately_after {
                    result.push_str(after);
                }
            }
            Element::CodeSpan(data) => {
                result.push('`');
                result.push_str(data.content);
                result.push_str("` ");
            }
            Element::FencedCodeBlock(data) => {
                result.push_str("```\n");
                result.push_str(data.content);
                result.push_str("\n```\n");
            }
            Element::HiddenHtmlDiv(data) => {
                result.push_str("<div class=\"hidden\">\n");
                result.push_str(data.content);
                result.push_str("\n</div>\n");
            }
            Element::Heading(data) => {
                for _ in 0..data.level {
                    result.push('#');
                }
                result.push(' ');
                if let Some(content) = data.content {
                    result.push_str(content);
                }
                if let Some(id) = data.id {
                    result.push_str(" {#");
                    result.push_str(id);
                    result.push('}');
                }
                result.push('\n');
            }
            Element::Text(data) => {
                result.push_str(data.content);
            }
            Element::CustomDirective(data) => {
                match data {
                    DirectiveData::Link { kind, name } => {
                        result.push_str(&format!("{{{{{kind} {name}}}}}"));
                    }
                    DirectiveData::Badge { kind, name } => {
                        result.push_str(&format!("{{{{!{kind} {name}}}}}"));
                    }
                    DirectiveData::CrateBlock { crate_name, additional_categories } => {
                        if additional_categories.is_empty() {
                            result.push_str(&format!("{{{{#crate {crate_name}}}}}"));
                        } else {
                            let cats = additional_categories.join(" ");
                            result.push_str(&format!("{{{{#crate: {crate_name} {cats}}}}}"));
                        }
                    }
                    DirectiveData::ExampleBlock { name } => {
                        result.push_str(&format!("{{{{#example {name}}}}}"));
                    }
                }
            }
        }
    }
    result
}
