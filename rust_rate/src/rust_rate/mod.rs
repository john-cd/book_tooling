// TODO Reviewed ??-??-2026
use anyhow::Result;

/// Fetches and prints a small crates.io summary for a single crate.
pub async fn execute(crate_name: String) -> Result<()> {
    println!("Fetching statistics for crate '{}'...", crate_name);

    let url = format!("https://crates.io/api/v1/crates/{}", crate_name);
    let client = reqwest::Client::builder()
        .user_agent("book_tool_rust_rate/1.0")
        .build()?;

    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        let data: serde_json::Value = response.json().await?;
        if let Some(crate_data) = data.get("crate") {
            let downloads = crate_data
                .get("downloads")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let recent_downloads = crate_data
                .get("recent_downloads")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let repository = crate_data
                .get("repository")
                .and_then(|v| v.as_str())
                .unwrap_or("N/A");
            let description = crate_data
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("N/A");

            println!("Crate: {}", crate_name);
            println!("Description: {}", description);
            println!("Repository: {}", repository);
            println!("Total Downloads: {}", downloads);
            println!("Recent Downloads: {}", recent_downloads);
        } else {
            anyhow::bail!("Failed to parse crate data from the API response.");
        }
    } else {
        anyhow::bail!("Failed to fetch crate data: HTTP {}", response.status());
    }

    Ok(())
}
