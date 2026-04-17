use anyhow::Result;
use futures::future::join_all;
use rust_rate::crawler::GithubCrawler;
use rust_rate::patterns::PatternAnalyzer;
use rust_rate::rust_rate::execute;
use rust_rate::stats::CrateStats;
use tracing::info;
use tracing::warn;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("rust_rate tool started.");

    // Retrieve GitHub token from environment variable if available
    let token = std::env::var("GITHUB_TOKEN").ok();
    let crawler = GithubCrawler::new(token);
    let mut stats = CrateStats::new();
    let analyzer = PatternAnalyzer::new();

    // Call execute to demonstrate integration and fix dead code
    if let Err(e) = execute("anyhow".to_string()).await {
        warn!("crates.io execute failed: {}", e);
    }

    // Example: Search for Cargo.toml files containing "anyhow"
    let items = match crawler.search_cargo_tomls("anyhow").await {
        Ok(items) => items,
        Err(e) => {
            warn!("Search failed (likely rate limit or missing token): {}", e);
            return Ok(());
        }
    };

    let download_futures = items.iter().take(5).map(|item| {
        let crawler = &crawler;
        async move {
            info!("Processing {} in {}", item.path, item.repository.full_name);
            crawler.download_content(&item.url).await
        }
    });

    let results = join_all(download_futures).await;

    for content in results.into_iter().flatten() {
        stats.process_cargo_toml(&content);

        // If we found a repository, let's try to find a .rs file in it for pattern analysis
        // This is a simplified example: in a real tool, we would crawl the repo structure.
        // For now, let's just analyze the Cargo.toml if it were a .rs file just to demonstrate.
        let patterns = analyzer.analyze_source(&content, "anyhow");
        for p in patterns {
            info!("  Found pattern: {}", p);
        }
    }

    stats.print_summary();

    Ok(())
}
