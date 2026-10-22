//! Detects code patterns in Rust source files.
//!
//! TODO Reviewed ??-??-2026

use std::collections::HashMap;

use anyhow::Context as _;
use regex::Regex;

/// Detects simple crate-specific usage patterns in Rust source files.
pub struct PatternAnalyzer {
    /// Compiled regular expressions keyed by crate name.
    patterns: HashMap<String, Regex>,
}

impl PatternAnalyzer {
    /// Builds a pattern analyzer with a small built-in library of regular expressions.
    ///
    /// # Errors
    ///
    /// Returns an error if a built-in pattern fails to compile.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_source() {
        let analyzer = PatternAnalyzer::new().expect("create analyzer");
        let content = "
            use anyhow::Result;
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

    #[test]
    fn test_analyze_source_escapes_dynamic_crate_name() {
        let analyzer = PatternAnalyzer::new().expect("create analyzer");
        let patterns = analyzer
            .analyze_source("use foo[bar::Thing;", "foo[bar")
            .expect("analyze source with escaped crate name");

        assert!(patterns.contains(&"Import of foo[bar".to_owned()));
    }

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
}
