use std::collections::HashMap;

use regex::Regex;

/// Detects simple crate-specific usage patterns in Rust source files.
pub struct PatternAnalyzer {
    patterns: HashMap<String, Regex>,
}

impl Default for PatternAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternAnalyzer {
    /// Builds a pattern analyzer with a small built-in library of regular expressions.
    pub fn new() -> Self {
        let mut patterns = HashMap::new();
        // Example patterns for common crates
        patterns.insert(
            "serde".to_string(),
            Regex::new(r"#\[derive\(.*Serialize.*\)\b").unwrap(),
        );
        patterns.insert(
            "tokio".to_string(),
            Regex::new(r"#\[tokio::main\]").unwrap(),
        );
        patterns.insert(
            "anyhow".to_string(),
            Regex::new(r"Result<.*>\s*\{").unwrap(),
        );

        Self { patterns }
    }

    /// Analyzes a Rust source file for crate-specific patterns and import statements.
    pub fn analyze_source(&self, content: &str, crate_name: &str) -> Vec<String> {
        let mut results = Vec::new();
        if let Some(re) = self.patterns.get(crate_name) {
            for cap in re.captures_iter(content) {
                results.push(cap[0].to_string());
            }
        }

        // Also look for imports
        let import_re = Regex::new(&format!(r"use\s+{}::", crate_name)).unwrap();
        if import_re.is_match(content) {
            results.push(format!("Import of {}", crate_name));
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_source() {
        let analyzer = PatternAnalyzer::new();
        let content = r#"
            use anyhow::Result;
            fn main() -> Result<()> {
                Ok(())
            }
        "#;
        let patterns = analyzer.analyze_source(content, "anyhow");
        assert!(patterns.contains(&"Import of anyhow".to_string()));
        assert!(patterns.iter().any(|p| p.contains("Result")));
    }
}
