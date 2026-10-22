//! crates.io summary workflow.
//!
//! TODO Reviewed ??-??-2026

use anyhow::Context as _;
use anyhow::Result;
use serde::Deserialize;

/// Base URL of the public crates.io API.
pub const CRATES_IO_API: &str = "https://crates.io/api/v1";

/// The subset of the crates.io `crate` object that the summary reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateSummary {
    /// Crate name as published.
    pub name: String,
    /// One-line description, when the crate has one.
    pub description: Option<String>,
    /// Repository URL, when the crate declares one.
    pub repository: Option<String>,
    /// All-time download count.
    pub downloads: u64,
    /// Downloads over the recent window reported by crates.io.
    pub recent_downloads: u64,
}

/// The envelope of the crates.io `GET /crates/{name}` response.
#[derive(Debug, Deserialize)]
struct CrateResponse {
    /// The crate record; absent when crates.io returns an unexpected document.
    #[serde(rename = "crate")]
    krate: Option<CrateRecord>,
}

/// The `crate` object of the crates.io response.
#[derive(Debug, Deserialize)]
struct CrateRecord {
    /// Crate name as published.
    name: String,
    /// One-line description.
    description: Option<String>,
    /// Repository URL.
    repository: Option<String>,
    /// All-time download count.
    #[serde(default)]
    downloads: u64,
    /// Recent download count.
    #[serde(default)]
    recent_downloads: Option<u64>,
}

/// Checks that `name` is a valid crates.io crate name before it is placed in a URL.
///
/// # Errors
///
/// Returns an error if the name is empty, longer than 64 characters, starts with a character
/// other than an ASCII letter, or contains anything but ASCII letters, digits, `-` and `_`.
pub fn validate_crate_name(name: &str) -> Result<()> {
    let valid = name.len() <= 64
        && name.starts_with(|c: char| c.is_ascii_alphabetic())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    anyhow::ensure!(valid, "'{name}' is not a valid crate name");
    Ok(())
}

/// Fetches the crates.io summary of `crate_name` from the API rooted at `api_base`.
///
/// # Errors
///
/// Returns an error if the name is invalid, the request fails, crates.io returns an error
/// status, or the response lacks crate data.
pub async fn fetch_summary(api_base: &str, crate_name: &str) -> Result<CrateSummary> {
    validate_crate_name(crate_name)?;
    let client = reqwest::Client::builder()
        .user_agent("book_tool_rust_rate/1.0")
        .build()
        .context("Failed to build the crates.io HTTP client")?;
    let url = format!("{}/crates/{crate_name}", api_base.trim_end_matches('/'));

    let response = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("Failed to request {url}"))?;
    anyhow::ensure!(
        response.status().is_success(),
        "Failed to fetch crate data: HTTP {}",
        response.status()
    );
    let body: CrateResponse = response
        .json()
        .await
        .context("Failed to decode the crates.io response")?;
    let record = body
        .krate
        .context("Failed to parse crate data from the API response.")?;

    Ok(CrateSummary {
        name: record.name,
        description: record.description,
        repository: record.repository,
        downloads: record.downloads,
        recent_downloads: record.recent_downloads.unwrap_or(0),
    })
}

/// Fetches and prints a small crates.io summary for a single crate.
///
/// # Errors
///
/// Returns an error if the name is invalid, the request fails, crates.io returns an error
/// status, or the response lacks crate data.
pub async fn execute(crate_name: &str) -> Result<()> {
    println!("Fetching statistics for crate '{crate_name}'...");
    let summary = fetch_summary(CRATES_IO_API, crate_name).await?;

    println!("Crate: {}", summary.name);
    println!(
        "Description: {}",
        summary.description.as_deref().unwrap_or("N/A")
    );
    println!(
        "Repository: {}",
        summary.repository.as_deref().unwrap_or("N/A")
    );
    println!("Total Downloads: {}", summary.downloads);
    println!("Recent Downloads: {}", summary.recent_downloads);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Typical crate names pass validation.
    #[test]
    fn valid_crate_names_are_accepted() {
        for name in ["anyhow", "serde_json", "tree-sitter", "a1"] {
            validate_crate_name(name).expect("valid crate name");
        }
    }

    /// Empty, path-like, non-ASCII and over-long names fail validation.
    #[test]
    fn invalid_crate_names_are_rejected() {
        for name in ["", "1abc", "a/b", "a b", "a?b=c", "..", "caf\u{e9}"] {
            validate_crate_name(name).expect_err("invalid crate name");
        }
        validate_crate_name(&"a".repeat(65)).expect_err("name is too long");
    }
}
