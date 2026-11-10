//! Command-line entry point that analyzes crate usage on GitHub and summarizes a crate from
//! crates.io.
//!
//! TODO Reviewed ??-??-2026.

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use futures::future::join_all;
use rust_rate::crates_io;
use rust_rate::crawler::CrawlerLimits;
use rust_rate::crawler::GITHUB_API;
use rust_rate::crawler::GithubCrawler;
use rust_rate::crawler::GithubItem;
use rust_rate::patterns::PatternAnalyzer;
use rust_rate::stats::CrateStats;
use tracing::info;
use tracing::warn;

/// Command-line arguments.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The workflow to run.
    #[command(subcommand)]
    command: Commands,
}

/// The workflows `rust_rate` offers.
#[derive(Subcommand)]
enum Commands {
    /// Print the crates.io summary of a crate.
    CratesIo {
        /// Name of the crate, for example `anyhow`.
        crate_name: String,
    },
    /// Sample GitHub `Cargo.toml` files that mention a crate and print usage statistics.
    Stats {
        /// Name of the crate, for example `anyhow`.
        crate_name: String,
        /// Maximum number of manifests to download and analyze.
        #[arg(short, long, default_value_t = 5)]
        limit: usize,
        /// Environment variable that holds a GitHub token (raises the API rate limit).
        #[arg(long, default_value = "GITHUB_TOKEN")]
        token_env: String,
        /// Root URL of the GitHub REST API (override for GitHub Enterprise or a test server).
        #[arg(long, default_value = GITHUB_API)]
        api_base: String,
        /// Maximum time in milliseconds for one request, including reading its response body.
        #[arg(long, default_value_t = 30_000)]
        request_timeout_ms: u64,
        /// Maximum time in milliseconds to establish a connection.
        #[arg(long, default_value_t = 10_000)]
        connect_timeout_ms: u64,
        /// Maximum bytes accepted from one code-search response.
        #[arg(long, default_value_t = 16 * 1024 * 1024)]
        max_search_response_bytes: usize,
        /// Maximum bytes accepted from one downloaded file response.
        #[arg(long, default_value_t = 16 * 1024 * 1024)]
        max_content_response_bytes: usize,
    },
}

/// Searches GitHub for manifests that mention `crate_name`, tallies their dependencies, then
/// downloads Rust sources that import the crate and reports the code patterns found in them.
async fn run_stats(
    crate_name: &str,
    limit: usize,
    token_env: &str,
    api_base: &str,
    limits: CrawlerLimits,
) -> Result<()> {
    crates_io::validate_crate_name(crate_name)?;
    let token = std::env::var(token_env).ok();
    let crawler = GithubCrawler::new_with_limits(token.as_deref(), limits)?.with_api_base(api_base);
    let analyzer = PatternAnalyzer::new()?;
    let mut stats = CrateStats::new();

    let manifests = crawler
        .search_cargo_tomls(crate_name)
        .await
        .context("GitHub search failed (rate limit or missing token?)")?;
    info!("Found {} manifests; analyzing {limit}", manifests.len());
    for result in download_all(&crawler, &manifests, limit).await {
        match result {
            Ok(content) => stats.process_cargo_toml(&content),
            Err(error) => warn!("Skipping a manifest: {error:#}"),
        }
    }

    let sources = crawler
        .search_rust_sources(crate_name)
        .await
        .context("GitHub source search failed (rate limit or missing token?)")?;
    info!("Found {} source files; analyzing {limit}", sources.len());
    for result in download_all(&crawler, &sources, limit).await {
        match result {
            Ok(content) => {
                for pattern in analyzer.analyze_source(&content, crate_name)? {
                    info!("  Found pattern: {pattern}");
                }
            }
            Err(error) => warn!("Skipping a source file: {error:#}"),
        }
    }

    stats.print_summary();
    Ok(())
}

/// Downloads at most `limit` of the `items` concurrently, returning one result per item in order.
async fn download_all(
    crawler: &GithubCrawler,
    items: &[GithubItem],
    limit: usize,
) -> Vec<Result<String>> {
    join_all(items.iter().take(limit).map(|item| async move {
        info!("Processing {} in {}", item.path, item.repository.full_name);
        crawler.download_content(&item.url).await
    }))
    .await
}

/// Parses the command line, initializes tracing, and runs the selected workflow.
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::CratesIo { crate_name } => crates_io::execute(&crate_name).await,
        Commands::Stats {
            crate_name,
            limit,
            token_env,
            api_base,
            request_timeout_ms,
            connect_timeout_ms,
            max_search_response_bytes,
            max_content_response_bytes,
        } => {
            let limits = CrawlerLimits {
                request_timeout: Duration::from_millis(request_timeout_ms),
                connect_timeout: Duration::from_millis(connect_timeout_ms),
                max_search_response_bytes,
                max_content_response_bytes,
            };
            run_stats(&crate_name, limit, &token_env, &api_base, limits).await
        }
    }
}
