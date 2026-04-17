use std::collections::HashMap;

use regex::Regex;

pub struct PatternAnalyzer {
    patterns: HashMap<String, Regex>,
}

impl Default for PatternAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternAnalyzer {
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

    /// Analyze a Rust source file for patterns.
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
