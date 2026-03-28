//! Process crate block directives e.g. `{{#crate crt}}`
//!
//! Crate blocks combine multiple crate and categories badges.

use core_lib::RegexAndReplacement;
use core_lib::templates::DestinationKind;
use core_lib::templates::render_badge;
use regex::Regex;

// TODO [finish](https://github.com/john-cd/rust_howto/issues/1438)

/// {{cat parsing }} -> [parsing][cat~parsing]↗{{hi:parsing}}
pub(super) fn crate_block_regexes() -> Vec<RegexAndReplacement> {
    let re_string: String = r"\{\{\s*#crate\s*:?\s+([^}]+)\s*\}\}".into();
    let re = Regex::new(&re_string).expect("Invalid regex");

    vec![RegexAndReplacement {
        re,
        replacement: Some(Box::new(|caps| {
            let content = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let parts: Vec<&str> = content.split_whitespace().collect();
            if parts.is_empty() {
                return "".into();
            }

            let crate_name = parts[0];
            let mut badges = vec![render_badge(&DestinationKind::Crate, crate_name)];

            for cat in &parts[1..] {
                badges.push(render_badge(&DestinationKind::Category, cat));
            }

            badges.join(" ")
        })),
    }]
}

// // The following variations should be accepted:
// // {{#crate crt}}
// // {{#crate crt }}
// // {{#crate: crt}}
// // {{#crate : crt}}
// // {{#crate x_y-z}}
// //
// // * Crate Blocks with Additional Categories
// //
// // {{#crate: crt cat1 cat-2 cat-2-2 cat3::sub-cat-3 }}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_block_regexes() {
        let text = "{{#crate crt}}";
        let expected = "[![crt][p~crt~crate~badge]][p~crt~crate]{{hi:crt}}";
        let r = crate_block_regexes();
        let res = r[0]
            .re
            .replace_all(text, r[0].replacement.as_ref().unwrap());
        assert_eq!(res, expected);
    }
}
