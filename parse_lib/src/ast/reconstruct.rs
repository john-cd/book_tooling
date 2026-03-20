use super::Element;

pub fn reconstruct_markdown(elements: &[Element]) -> String {
    let mut result = String::new();
    for element in elements {
        result.push_str(&format!("{element}"));
    }
    result
}
