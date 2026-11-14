//! Detects code patterns in Rust source files.
//!
//! TODO Reviewed ??-??-2026.

use std::collections::HashMap;

use anyhow::Context as _;
use regex::Regex;

/// Detects simple crate-specific usage patterns in Rust source files.
pub struct PatternAnalyzer {
    /// Compiled regular expressions keyed by crate name.
    patterns: HashMap<String, Regex>,
}

impl PatternAnalyzer {
    /// Compiles the built-in usage patterns for Serde, Tokio, and anyhow.
    ///
    /// # Errors
    /// Returns an error identifying any built-in regular expression that fails to compile.
    pub fn new() -> anyhow::Result<Self> {
        let mut patterns = HashMap::new();
        // Example patterns for common crates
        patterns.insert(
            "serde".to_owned(),
            Regex::new(r"#\[derive\([^)]*\bSerialize\b[^)]*\)\]")
                .context("Failed to compile serde pattern")?,
        );
        patterns.insert(
            "tokio".to_owned(),
            Regex::new(r"#\[tokio::main(?:\(.*\))?\]")
                .context("Failed to compile tokio pattern")?,
        );
        patterns.insert(
            "anyhow".to_owned(),
            Regex::new(r"->\s*(?:anyhow::)?Result<[^{;]*>\s*\{")
                .context("Failed to compile anyhow pattern")?,
        );

        Ok(Self { patterns })
    }

    /// Analyzes a Rust source file for crate-specific patterns and import statements.
    ///
    /// # Errors
    ///
    /// Returns an error if the import pattern for `crate_name` cannot be compiled.
    pub fn analyze_source(&self, content: &str, crate_name: &str) -> anyhow::Result<Vec<String>> {
        let mut results = Vec::new();
        if let Some(re) = self.patterns.get(crate_name) {
            for cap in re.captures_iter(content) {
                results.push(cap[0].to_string());
            }
        }

        // Also look for imports
        let escaped_crate_name = regex::escape(crate_name);
        let import_re = Regex::new(&format!(r"use\s+{escaped_crate_name}::"))
            .context("Failed to compile import pattern")?;
        if import_re.is_match(content) {
            results.push(format!("Import of {crate_name}"));
        }

        Ok(results)
    }
}

/// Tests for matching Rust usage patterns.
#[cfg(test)]
mod tests {
    use super::*;

    /// Imports and `Result` returns are detected for `anyhow`.
    #[test]
    fn test_analyze_source() {
        let analyzer = PatternAnalyzer::new().expect("create analyzer");
        let content = "
            use anyhow::Result;
            /// Provides an entry point containing an explicit result type for the pattern fixture.
            fn main() -> Result<()> {
                Ok(())
            }
        ";
        let patterns = analyzer
            .analyze_source(content, "anyhow")
            .expect("analyze source");
        assert!(patterns.contains(&"Import of anyhow".to_owned()));
        assert!(patterns.iter().any(|pattern| pattern.contains("Result")));
    }

    /// Regex metacharacters in a crate name are escaped.
    #[test]
    fn test_analyze_source_escapes_dynamic_crate_name() {
        let analyzer = PatternAnalyzer::new().expect("create analyzer");
        let patterns = analyzer
            .analyze_source("use foo[bar::Thing;", "foo[bar")
            .expect("analyze source with escaped crate name");

        assert!(patterns.contains(&"Import of foo[bar".to_owned()));
    }

    /// Verifies serde and anyhow patterns are specific.
    #[test]
    fn test_serde_and_anyhow_patterns_are_specific() {
        let analyzer = PatternAnalyzer::new().expect("create analyzer");
        let serde = analyzer
            .analyze_source(
                "#[derive(Debug, Serialize, Deserialize)]\nstruct A;\n#[derive(Debug)]\nstruct B; // Serialize",
                "serde",
            )
            .expect("analyze source");
        assert_eq!(serde, ["#[derive(Debug, Serialize, Deserialize)]"]);

        let anyhow = analyzer
            .analyze_source(
                "fn a() -> anyhow::Result<()> {\n}\nfn b() -> Option<Result<u8>> { None }",
                "anyhow",
            )
            .expect("analyze source");
        assert_eq!(anyhow, ["-> anyhow::Result<()> {"]);
    }

    /// The tokio pattern matches both bare and parameterized attribute forms.
    #[test]
    fn tokio_main_attribute_is_detected_with_and_without_arguments() {
        let analyzer = PatternAnalyzer::new().expect("create analyzer");
        let found = analyzer
            .analyze_source(
                "#[tokio::main]\nasync fn a() {}\n#[tokio::main(flavor = \"current_thread\")]\nasync fn b() {}\n#[tokio::test]\nasync fn c() {}",
                "tokio",
            )
            .expect("analyze source");
        assert_eq!(
            found,
            [
                "#[tokio::main]",
                "#[tokio::main(flavor = \"current_thread\")]"
            ]
        );
    }

    /// Unknown crates only report imports, and empty or unrelated sources report nothing.
    #[test]
    fn unknown_crates_and_empty_sources_report_only_imports() {
        let analyzer = PatternAnalyzer::new().expect("create analyzer");
        assert!(
            analyzer
                .analyze_source("", "serde")
                .expect("empty")
                .is_empty()
        );
        assert!(
            analyzer
                .analyze_source("fn main() {}", "rand")
                .expect("unrelated")
                .is_empty()
        );
        let found = analyzer
            .analyze_source("use   rand::Rng;", "rand")
            .expect("import");
        assert_eq!(found, ["Import of rand"]);
    }

    /// An import of a different crate with a shared prefix is not a match.
    #[test]
    fn import_detection_requires_the_path_separator() {
        let analyzer = PatternAnalyzer::new().expect("create analyzer");
        let found = analyzer
            .analyze_source("use serde_json::Value;", "serde")
            .expect("analyze");
        assert!(found.is_empty(), "{found:?}");
    }

    /// Malformed Rust is still scanned lexically instead of failing.
    #[test]
    fn syntactically_invalid_source_is_scanned_without_error() {
        let analyzer = PatternAnalyzer::new().expect("create analyzer");
        let found = analyzer
            .analyze_source("use anyhow::{{{ fn ->->", "anyhow")
            .expect("analyze");
        assert_eq!(found, ["Import of anyhow"]);
    }

    /// Multiple matches of the same pattern are all reported in source order.
    #[test]
    fn repeated_matches_are_reported_in_order() {
        let analyzer = PatternAnalyzer::new().expect("create analyzer");
        let found = analyzer
            .analyze_source(
                "#[derive(Serialize)]\nstruct A;\n#[derive(Clone, Serialize)]\nstruct B;",
                "serde",
            )
            .expect("analyze");
        assert_eq!(
            found,
            ["#[derive(Serialize)]", "#[derive(Clone, Serialize)]"]
        );
    }
}
