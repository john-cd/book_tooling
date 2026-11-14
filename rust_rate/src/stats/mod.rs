//! Statistics about the crates a project depends on, from `Cargo.toml` files.
//!
//! TODO Reviewed ??-??-2026.

use std::collections::BTreeSet;
use std::collections::HashMap;

use toml::Value;
use tracing::warn;

/// Aggregated usage statistics collected from Cargo manifests.
#[derive(Debug, Default)]
pub struct CrateStats {
    /// Number of times each dependency name was observed.
    pub usage_count: HashMap<String, usize>,
    /// Number of times each crate category was observed.
    pub category_stats: HashMap<String, usize>,
}

/// Dependency table names that can appear at the manifest root or under `[target.<cfg>]`.
const DEPENDENCY_TABLES: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];

/// Returns the crate a dependency entry refers to: its `package` rename target, or the key itself.
fn dependency_package_name<'a>(key: &'a str, value: &'a Value) -> &'a str {
    value.get("package").and_then(Value::as_str).unwrap_or(key)
}
/// Adds the crate names of every dependency in `table` to `names`.
fn collect_dependency_names<'a>(table: Option<&'a Value>, names: &mut BTreeSet<&'a str>) {
    let Some(entries) = table.and_then(Value::as_table) else {
        return;
    };
    for (key, value) in entries {
        names.insert(dependency_package_name(key, value));
    }
}

impl CrateStats {
    /// Creates an empty statistics accumulator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Processes Cargo manifest contents and updates the aggregated counters.
    ///
    /// Each crate is counted once per manifest, however many dependency tables name it. The
    /// `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, `[workspace.dependencies]`
    /// and `[target.<cfg>.*dependencies]` tables are read, and renamed dependencies
    /// (`package = "..."`) count under their real crate name. A manifest that does not parse is
    /// logged and skipped.
    pub fn process_cargo_toml(&mut self, content: &str) {
        let manifest: Value = match toml::from_str(content) {
            Ok(manifest) => manifest,
            Err(e) => {
                warn!("Failed to parse Cargo.toml: {}", e);
                return;
            }
        };

        let mut names = BTreeSet::new();
        for table in DEPENDENCY_TABLES {
            collect_dependency_names(manifest.get(table), &mut names);
        }
        collect_dependency_names(
            manifest
                .get("workspace")
                .and_then(|workspace| workspace.get("dependencies")),
            &mut names,
        );
        if let Some(targets) = manifest.get("target").and_then(Value::as_table) {
            for target in targets.values() {
                for table in DEPENDENCY_TABLES {
                    collect_dependency_names(target.get(table), &mut names);
                }
            }
        }
        for name in names {
            *self.usage_count.entry(name.to_owned()).or_insert(0) += 1;
        }

        let categories = manifest
            .get("package")
            .and_then(|package| package.get("categories"))
            .and_then(Value::as_array);
        for category in categories.into_iter().flatten().filter_map(Value::as_str) {
            *self.category_stats.entry(category.to_owned()).or_insert(0) += 1;
        }
    }

    /// Returns dependency names with their counts, ordered by descending count and then ascending
    /// name.
    #[must_use]
    pub fn ranked_usage(&self) -> Vec<(&str, usize)> {
        let mut ranked: Vec<(&str, usize)> = self
            .usage_count
            .iter()
            .map(|(name, count)| (name.as_str(), *count))
            .collect();
        ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
        ranked
    }

    /// Prints the twenty most used crates to stdout.
    pub fn print_summary(&self) {
        println!("--- Crate Usage Statistics ---");
        for (name, count) in self.ranked_usage().into_iter().take(20) {
            println!("{name}: {count} usages");
        }
    }
}

/// Tests for dependency counting across Cargo manifests.
#[cfg(test)]
mod tests {
    use super::*;

    /// Dependencies and categories of a manifest are counted.
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

    /// Every dependency table counts once per manifest, honouring renames.
    #[test]
    fn test_all_dependency_tables_renames_and_per_manifest_counting() {
        let mut stats = CrateStats::new();
        stats.process_cargo_toml(
            r#"
            [dependencies]
            serde = "1.0"
            json = { package = "serde_json", version = "1.0" }

            [dev-dependencies]
            serde = "1.0"

            [build-dependencies]
            cc = "1.0"

            [target.'cfg(unix)'.dependencies]
            libc = "0.2"

            [workspace.dependencies]
            anyhow = "1.0"
            "#,
        );

        for name in ["serde", "serde_json", "cc", "libc", "anyhow"] {
            assert_eq!(stats.usage_count.get(name), Some(&1), "{name}");
        }
        assert_eq!(stats.usage_count.get("json"), None);
    }

    /// A manifest that fails to parse changes no counters.
    #[test]
    fn test_invalid_manifest_is_skipped() {
        let mut stats = CrateStats::new();
        stats.process_cargo_toml("this is not [toml");
        assert!(stats.usage_count.is_empty());
    }

    /// Ranking sorts by count descending, then name ascending.
    #[test]
    fn test_ranked_usage_orders_by_count_then_name() {
        let mut stats = CrateStats::new();
        stats.usage_count.insert("b".to_owned(), 2);
        stats.usage_count.insert("a".to_owned(), 2);
        stats.usage_count.insert("c".to_owned(), 5);
        assert_eq!(stats.ranked_usage(), [("c", 5), ("a", 2), ("b", 2)]);
    }
}
