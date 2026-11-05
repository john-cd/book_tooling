//! Renders a badge and a link for each destination kind.
//!
//! Run with `cargo run -p book_templates --example render`.

use book_templates::DestinationKind;
use book_templates::render_badge;
use book_templates::render_link;

fn main() {
    for kind in [
        DestinationKind::Crate,
        DestinationKind::Docs,
        DestinationKind::CratesIo,
        DestinationKind::LibRs,
        DestinationKind::GithubRepo,
        DestinationKind::Web,
    ] {
        println!("{}", render_badge(kind, "anyhow"));
        println!("{}", render_link(kind, "anyhow"));
    }
    println!("{}", render_link(DestinationKind::Category, "science"));
}
