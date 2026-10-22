// TODO Reviewed ??-??-2026
//! Command-line entry point for `book_tool`, a set of maintenance tools for mdBook sources.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use clap::Subcommand;

/// Command-line arguments of `book_tool`.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// The subcommand to run.
    #[command(subcommand)]
    command: Commands,
}

/// The `book_tool` subcommands.
#[derive(Subcommand)]
enum Commands {
    /// Process custom directives (e.g., {{cat rust}}) in Markdown files.
    Directives {
        /// Directories to process.
        #[arg(required = true)]
        directories: Vec<PathBuf>,
        /// Configuration file (default: `config.toml` in the current directory, if present).
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    /// Check for broken links in Markdown files.
    LinkCheck {
        /// Directories to check.
        #[arg(required = true)]
        directories: Vec<PathBuf>,
        /// Also detect and optionally delete unused reference definitions.
        #[arg(short, long)]
        refdefs: bool,
        /// Delete unused reference definitions if --refdefs is set.
        #[arg(short, long, requires = "refdefs")]
        delete: bool,
    },
    /// Audit reference-definition provenance and uses, then optionally delete proven-unused entries.
    RefdefsAudit {
        /// Directories to analyze.
        #[arg(required = true)]
        directories: Vec<PathBuf>,
        /// JSON report path. The report is written before any requested cleanup.
        #[arg(short, long, required = true)]
        report: PathBuf,
        /// Remove only definitions proven unused in every include scope.
        #[arg(long)]
        delete: bool,
        /// Also report and preserve uses found in these Markdown roots, without resolving includes.
        #[arg(long = "also-scan-uses-in", value_name = "DIR")]
        additional_use_directories: Vec<PathBuf>,
        /// Protect generated or otherwise maintained definition files from cleanup.
        #[arg(long = "preserve-definitions-in", value_name = "FILE")]
        protected_definition_files: Vec<PathBuf>,
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
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match &cli.command {
        Commands::Directives {
            directories,
            config,
        } => {
            book_tool::directives::execute(directories.clone(), config.as_deref())?;
        }
        Commands::LinkCheck {
            directories,
            refdefs,
            delete,
        } => {
            book_tool::link_check::execute(directories.clone(), *refdefs, *delete).await?;
        }
        Commands::RefdefsAudit {
            directories,
            report,
            delete,
            additional_use_directories,
            protected_definition_files,
        } => {
            book_tool::refdefs::audit(
                directories,
                report,
                *delete,
                additional_use_directories.clone(),
                protected_definition_files.clone(),
            )?;
        }
        Commands::InlineLinks { directories } => {
            book_tool::inline_links::execute(directories)?;
        }
        Commands::Search { query, directories } => {
            book_tool::search::execute(query, directories)?;
        }
        Commands::ExtractComments {
            directories,
            output,
        } => {
            book_tool::extract_comments::execute(directories, output)?;
        }
    }

    Ok(())
}
