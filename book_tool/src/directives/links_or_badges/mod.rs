// TODO Reviewed ??-??-2026
//! Process links and badge directives.

use parse_lib::ast::DestinationKind as AstDestinationKind;
use parse_lib::ast::DirectiveData;

use crate::common::DestinationKind;
use crate::common::render_badge;
use crate::common::render_link;

/// Renders a link or badge directive.
pub(super) fn render_link_or_badge(directive: &DirectiveData<'_>) -> String {
    let (is_badge, ast_kind, name) = match directive {
        DirectiveData::Link { kind, name } => (false, kind, name),
        DirectiveData::Badge { kind, name } => (true, kind, name),
        _ => return String::new(),
    };

    // Keep this conversion explicit and exhaustive while this adapter has one caller.
    let kind = match ast_kind {
        AstDestinationKind::Category => DestinationKind::Category,
        AstDestinationKind::Crate => DestinationKind::Crate,
        AstDestinationKind::Docs => DestinationKind::Docs,
        AstDestinationKind::GithubRepo => DestinationKind::GithubRepo,
        AstDestinationKind::LibRs => DestinationKind::LibRs,
        AstDestinationKind::CratesIo => DestinationKind::CratesIo,
        AstDestinationKind::Web => DestinationKind::Web,
    };

    if is_badge {
        render_badge(&kind, name)
    } else {
        render_link(&kind, name)
    }
}

#[cfg(test)]
mod tests {
    use parse_lib::ast::DestinationKind as AstDestinationKind;
    use parse_lib::ast::DirectiveData;

    use super::render_link_or_badge;
    use crate::common::DestinationKind;

    /// Verifies every AST destination maps to the expected link and badge templates.
    #[test]
    fn destination_mapping_covers_every_link_and_badge_shape() {
        let cases = [
            (
                AstDestinationKind::Category,
                DestinationKind::Category,
                "[serde][cat~serde]↗{{hi:serde}}",
                "[![serde][cat~serde~badge]][cat~serde]{{hi:serde}}",
            ),
            (
                AstDestinationKind::Crate,
                DestinationKind::Crate,
                "[serde][p~serde~crate]↗{{hi:serde}}",
                "[![serde][p~serde~crate~badge]][p~serde~crate]{{hi:serde}}",
            ),
            (
                AstDestinationKind::Docs,
                DestinationKind::Docs,
                "[serde][c~serde~docs]↗{{hi:serde}}",
                "[![serde][c~serde~docs~badge]][c~serde~docs]{{hi:serde}}",
            ),
            (
                AstDestinationKind::GithubRepo,
                DestinationKind::GithubRepo,
                "[serde][c~serde~repo]↗{{hi:serde}}",
                "[![serde][c~serde~repo~badge]][c~serde~repo]{{hi:serde}}",
            ),
            (
                AstDestinationKind::LibRs,
                DestinationKind::LibRs,
                "[serde][c~serde~lib.rs]↗{{hi:serde}}",
                "[![serde][c~serde~lib.rs~badge]][c~serde~lib.rs]{{hi:serde}}",
            ),
            (
                AstDestinationKind::CratesIo,
                DestinationKind::CratesIo,
                "[serde][c~serde~crates.io]↗{{hi:serde}}",
                "[![serde][c~serde~crates.io~badge]][c~serde~crates.io]{{hi:serde}}",
            ),
            (
                AstDestinationKind::Web,
                DestinationKind::Web,
                "[serde][c~serde~website]↗{{hi:serde}}",
                "[![serde][c~serde~website~badge]][c~serde~website]{{hi:serde}}",
            ),
        ];

        for (ast_kind, template_kind, expected_link, expected_badge) in cases {
            let link = DirectiveData::Link {
                kind: ast_kind.clone(),
                name: "serde",
            };
            let badge = DirectiveData::Badge {
                kind: ast_kind,
                name: "serde",
            };

            assert_eq!(render_link_or_badge(&link), expected_link);
            assert_eq!(render_link_or_badge(&badge), expected_badge);
            assert_eq!(
                render_link_or_badge(&link),
                crate::common::render_link(&template_kind, "serde")
            );
            assert_eq!(
                render_link_or_badge(&badge),
                crate::common::render_badge(&template_kind, "serde")
            );
        }
    }
}
