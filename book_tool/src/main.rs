use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use clap::Subcommand;

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
        /// Also detect and optionally delete unused reference definitions.
        #[arg(short, long)]
        refdefs: bool,
        /// Delete unused reference definitions if --refdefs is set.
        #[arg(short, long, requires = "refdefs")]
        delete: bool,
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
        Commands::Directives { directories } => {
            book_tool::directives::execute(directories.clone())?;
        }
        Commands::LinkCheck {
            directories,
            refdefs,
            delete,
        } => {
            book_tool::link_check::execute(directories.clone(), *refdefs, *delete).await?;
        }
        Commands::InlineLinks { directories } => {
            book_tool::inline_links::execute(directories.clone())?;
        }
        Commands::Search { query, directories } => {
            book_tool::search::execute(query.clone(), directories.clone())?;
        }
        Commands::ExtractComments {
            directories,
            output,
        } => {
            book_tool::extract_comments::execute(directories.clone(), output.clone())?;
        }
    }

    Ok(())
}
