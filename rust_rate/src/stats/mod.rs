// TODO Reviewed ??-??-2026
use std::collections::HashMap;

use serde::Deserialize;
use toml::Table;
use tracing::info;
use tracing::warn;

/// Aggregated usage statistics collected from Cargo manifests.
#[derive(Debug, Default)]
pub struct CrateStats {
    /// Number of times each dependency name was observed.
    pub usage_count: HashMap<String, usize>,
    /// Number of times each crate category was observed.
    pub category_stats: HashMap<String, usize>,
}

/// The parts of a `Cargo.toml` manifest that the statistics need.
#[derive(Debug, Deserialize)]
struct CargoToml {
    /// The `[dependencies]` table.
    dependencies: Option<Table>,
    /// The `[dev-dependencies]` table.
    #[serde(rename = "dev-dependencies")]
    dev_dependencies: Option<Table>,
    /// The `[package]` table.
    package: Option<PackageInfo>,
}

/// The parts of the `[package]` table that the statistics need.
#[derive(Debug, Deserialize)]
struct PackageInfo {
    /// The crates.io categories the package declares.
    categories: Option<Vec<String>>,
}

impl CrateStats {
    /// Creates an empty statistics accumulator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Processes Cargo manifest contents and updates the aggregated counters.
    pub fn process_cargo_toml(&mut self, content: &str) {
        let cargo: CargoToml = match toml::from_str(content) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to parse Cargo.toml: {}", e);
                return;
            }
        };

        if let Some(deps) = cargo.dependencies {
            for name in deps.keys() {
                *self.usage_count.entry(name.clone()).or_insert(0) += 1;
            }
        }

        if let Some(dev_deps) = cargo.dev_dependencies {
            for name in dev_deps.keys() {
                *self.usage_count.entry(name.clone()).or_insert(0) += 1;
            }
        }

        // Categories are usually only in the crate's own Cargo.toml.
        // If we are crawling random repos, we might see categories of the apps themselves.
        if let Some(pkg) = cargo.package
            && let Some(cats) = pkg.categories
        {
            for cat in cats {
                *self.category_stats.entry(cat).or_insert(0) += 1;
            }
        }
    }

    /// Prints a small top-usage summary to stdout.
    pub fn print_summary(&self) {
        info!("--- Crate Usage Statistics ---");
        let mut sorted_usage: Vec<_> = self.usage_count.iter().collect();
        sorted_usage.sort_by(|left, right| right.1.cmp(left.1));

        for (name, count) in sorted_usage.iter().take(20) {
            println!("{name}: {count} usages");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_cargo_toml() {
        let mut stats = CrateStats::new();
        let content = r#"
            [package]
            name = "test-pkg"
            categories = ["api-bindings", "development-tools"]

            [dependencies]
            serde = "1.0"
            tokio = { version = "1.0", features = ["full"] }

            [dev-dependencies]
            tempfile = "3.0"
        "#;
        stats.process_cargo_toml(content);

        assert_eq!(stats.usage_count.get("serde"), Some(&1));
        assert_eq!(stats.usage_count.get("tokio"), Some(&1));
        assert_eq!(stats.usage_count.get("tempfile"), Some(&1));
        assert_eq!(stats.category_stats.get("api-bindings"), Some(&1));
    }
}
