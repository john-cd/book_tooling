//! Process links and badge directives.

use crate::common::templates::{DestinationKind, render_badge, render_link};
use parse_lib::ast::{DirectiveData, DestinationKind as AstDestinationKind};

/// Renders a link or badge directive.
pub(super) fn render_link_or_badge(directive: &DirectiveData<'_>) -> String {
    let (is_badge, ast_kind, name) = match directive {
        DirectiveData::Link { kind, name } => (false, kind, name),
        DirectiveData::Badge { kind, name } => (true, kind, name),
        _ => return String::new(),
    };

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
