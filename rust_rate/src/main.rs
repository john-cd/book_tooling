//! Command-line entry point that analyzes crate usage on GitHub and summarizes a crate from
//! crates.io.
//!
//! TODO Reviewed ??-??-2026

use anyhow::Context as _;
use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use futures::future::join_all;
use rust_rate::crates_io;
use rust_rate::crawler::GithubCrawler;
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
    },
}

/// Searches GitHub for manifests that mention `crate_name`, then prints usage statistics.
async fn run_stats(crate_name: &str, limit: usize, token_env: &str) -> Result<()> {
    crates_io::validate_crate_name(crate_name)?;
    let token = std::env::var(token_env).ok();
    let crawler = GithubCrawler::new(token.as_deref())?;
    let analyzer = PatternAnalyzer::new()?;
    let mut stats = CrateStats::new();

    let items = crawler
        .search_cargo_tomls(crate_name)
        .await
        .context("GitHub search failed (rate limit or missing token?)")?;
    info!("Found {} manifests; analyzing {limit}", items.len());

    let downloads = items.iter().take(limit).map(|item| {
        let crawler = &crawler;
        async move {
            info!("Processing {} in {}", item.path, item.repository.full_name);
            crawler.download_content(&item.url).await
        }
    });

    for result in join_all(downloads).await {
        match result {
            Ok(content) => {
                stats.process_cargo_toml(&content);
                for pattern in analyzer.analyze_source(&content, crate_name)? {
                    info!("  Found pattern: {pattern}");
                }
            }
            Err(error) => warn!("Skipping a manifest: {error:#}"),
        }
    }

    stats.print_summary();
    Ok(())
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
        } => run_stats(&crate_name, limit, &token_env).await,
    }
}
