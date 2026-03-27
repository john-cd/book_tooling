use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Process custom directives (e.g., {{cat rust}}) in Markdown files.
    Directives {
        /// Directories to process.
        #[arg(required = true)]
        directories: Vec<PathBuf>,
    },
    /// Check for broken links in Markdown files.
    LinkCheck {
        /// Directories to check.
        #[arg(required = true)]
        directories: Vec<PathBuf>,
    },
    /// Convert inline links to reference-style links in Markdown files.
    InlineLinks {
        /// Directories to process.
        #[arg(required = true)]
        directories: Vec<PathBuf>,
    },
    /// Search for text across Markdown files, ignoring formatting.
    Search {
        /// The text to search for.
        #[arg(required = true)]
        query: String,
        /// Directories to search.
        #[arg(required = true)]
        directories: Vec<PathBuf>,
    },
    /// Extract HTML comments from Markdown files into a single file for auditing.
    ExtractComments {
        /// Directories to search.
        #[arg(required = true)]
        directories: Vec<PathBuf>,
        /// Output file path.
        #[arg(short, long, required = true)]
        output: PathBuf,
    },
    /// Fetch and analyze Rust crate metrics from GitHub, crates.io and lib.rs.
    // TODO HUMAN review scope
    RustRate {
        /// The name of the crate to analyze.
        #[arg(required = true)]
        crate_name: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();

    let cli = Cli::parse();

    match &cli.command {
        Commands::Directives { directories } => {
            book_tool::directives::execute(directories.clone())?;
        }
        Commands::LinkCheck { directories } => {
            book_tool::link_check::execute(directories.clone()).await?;
        }
        Commands::InlineLinks { directories } => {
            book_tool::inline_links::execute(directories.clone())?;
        }
        Commands::Search { query, directories } => {
            book_tool::search::execute(query.clone(), directories.clone())?;
        }
        Commands::ExtractComments { directories, output } => {
            book_tool::extract_comments::execute(directories.clone(), output.clone())?;
        }
        Commands::RustRate { crate_name } => {
            book_tool::rust_rate::execute(crate_name.clone()).await?;
        }
    }

    Ok(())
}
