//! Process links and badge directives.
//!
//! * Crate links:
//!
//! - Internal crate page: {{crate xyz}}
//! - Variations:
//!   - {{ crate xyz}}
//!   - {{crate xyz }}
//!   - {{crate: xyz}}
//!   - {{crate : xyz}}
//!   - {{crate x_y-z}}
//! - `docs.rs` link: {{docs xyz}}
//! - Github link: {{github xyz}}
//! - `lib.rs` link: {{lib.rs xyz}}
//! - `crates.io` link: {{crates.io xyz}}
//! - Website for the crate: {{web xyz}}
//!
//! * Category links:
//!
//! {{cat: accessibility}} -> [Accessibility][cat~accessibility]↗{{hi:accessibility}}
//!
//! * Crate badges:
//!
//!- Internal crate page: {{!crate xyz}}
//!   - {{!crate xyz}}
//!   - {{!crate xyz }}
//!   - {{ ! crate xyz }}
//!   - {{!crate: x_y-z}}
//!   - {{!crate : x_y-z}}
//! - {{!docs xyz}}
//! - {{!github xyz}}
//! - {{!lib.rs xyz}}
//! - {{!crates.io xyz}}
//! - {{!web xyz}}
//!
//! * Category badges:
//!
//! {{!cat mathematics}} -> [![cat~mathematics][cat~mathematics~badge]][cat~mathematics]{{hi:Mathematics}}
//! {{!cat no-std}} -> [![cat~no-std][cat~no-std~badge]][cat~no-std]{{hi:No standard library}}

use core_lib::templates::DestinationKind;
use core_lib::templates::render_badge;
use core_lib::templates::render_link;
use parse_lib::ast::DirectiveData;

/// Renders a link or badge directive.
pub(super) fn render_link_or_badge(directive: &DirectiveData<'_>) -> String {
    let is_badge = directive.name.starts_with('!');
    let kind_str = if is_badge { &directive.name[1..] } else { directive.name };
    let content = directive.content.trim();

    let kind = match kind_str {
        "cat" => DestinationKind::Category,
        "crate" => DestinationKind::Crate,
        "docs" => DestinationKind::Docs,
        "github" => DestinationKind::GithubRepo,
        "lib.rs" => DestinationKind::LibRs,
        "crates.io" => DestinationKind::CratesIo,
        "web" => DestinationKind::Web,
        _ => return String::new(),
    };

    if is_badge {
        render_badge(&kind, content)
    } else {
        render_link(&kind, content)
    }
}
