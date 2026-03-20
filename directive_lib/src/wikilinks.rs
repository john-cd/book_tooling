//! Process wikilinks and convert them to markdown links.

use core_lib::RegexAndReplacement;
use regex::Regex;

pub(super) fn wikilink_regexes() -> Vec<RegexAndReplacement> {
    let re_string: String = r"\[\[\s*([^|\]\s]+)\s*(?:\|\s*([^\]]+))?\s*\]\]".into();
    let re = Regex::new(&re_string).expect("Invalid regex");
    vec![RegexAndReplacement {
        re,
        replacement: Some(Box::new(|caps| {
            let target = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let display = caps.get(2).map(|m| m.as_str().trim()).unwrap_or(target);
            format!("[{}][p~{}]{{hi:{}}}", display, target, target)
        })),
    }]
}

// The following variations should be accepted:
//  [[file]]
// [[ file ]]
// [[ file |]]
// [[ file |  ]]
// [[ file | title1 ]]
// [[file|title2]]
// [[ file|title3]]
// [[file |title4]]
// [[file| title5]]
// [[file|title6 ]]
//
// Bad Wikilinks:
//
// [[]]
// [[|title7]]
// [[| title8]]
// [[|title9 ]]
// [[| title10 ]]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_wikilinks() {
        let text = "[[file | title]]";
        let expected = "[title][p~file]{hi:file}";
        let r = wikilink_regexes();
        let res = r[0].re.replace_all(text, |caps: &regex::Captures| {
            (r[0].replacement.as_ref().unwrap())(caps)
        });
        assert_eq!(res, expected);
    }
}
