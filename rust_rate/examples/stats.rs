//! Aggregates crate usage from sample `Cargo.toml` contents without network access.
//!
//! Run with `cargo run -p rust_rate --example stats`.

use rust_rate::stats::CrateStats;

fn main() {
    let manifests = [
        "[dependencies]\nanyhow = \"1\"\nserde = \"1\"\n",
        "[dependencies]\nanyhow = \"1\"\n",
    ];
    let mut stats = CrateStats::new();
    for manifest in manifests {
        stats.process_cargo_toml(manifest);
    }
    for (name, count) in stats.ranked_usage() {
        println!("{name}: {count}");
    }
}
