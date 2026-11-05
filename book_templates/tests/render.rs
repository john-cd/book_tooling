//! Integration tests for rendering badges and links through the public API only.
//!
//! TODO Reviewed ??-??-2026

use book_templates::DestinationKind;
use book_templates::render_badge;
use book_templates::render_badge_without_index;
use book_templates::render_link;

const ALL: [DestinationKind; 7] = [
    DestinationKind::Category,
    DestinationKind::Crate,
    DestinationKind::Docs,
    DestinationKind::GithubRepo,
    DestinationKind::LibRs,
    DestinationKind::CratesIo,
    DestinationKind::Web,
];

#[test]
fn every_badge_has_one_index_entry_and_balanced_markup() {
    for kind in ALL {
        let badge = render_badge(kind, "serde");
        assert!(badge.ends_with("{{hi:serde}}"), "{kind:?}: {badge}");
        assert_eq!(badge.matches('[').count(), badge.matches(']').count());
    }
}

#[test]
fn badge_without_index_is_the_indexed_badge_minus_the_suffix() {
    for kind in ALL {
        let full = render_badge(kind, "serde");
        let bare = render_badge_without_index(kind, "serde", "serde");
        assert_eq!(full.strip_suffix("{{hi:serde}}"), Some(bare.as_str()));
    }
}

#[test]
fn links_carry_the_external_arrow_and_index_entry() {
    for kind in ALL {
        let link = render_link(kind, "serde");
        assert!(link.contains('\u{2197}'), "{kind:?}: {link}");
        assert!(link.ends_with("{{hi:serde}}"), "{kind:?}: {link}");
    }
}

#[test]
fn only_crate_destinations_use_the_package_prefix() {
    assert!(render_link(DestinationKind::Crate, "x").starts_with("[x][p~x~crate]"));
    assert!(render_link(DestinationKind::Docs, "x").starts_with("[x][c~x~docs]"));
}
