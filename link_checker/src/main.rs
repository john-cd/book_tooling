mod cli;
mod conf;
mod links;
mod refdefs;

use std::path::Path;
use clap::Parser;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Install a global tracing subscriber that listens for events
    // and filters based on the value of the RUST_LOG environment variable.
    tracing_subscriber::fmt::init();

    // Process command-line arguments to retrieve the directory to process:
    let args = cli::Args::parse();

    let _conf = conf::Config::default();
    let scope = core_lib::Scope::default();

    for directory in &args.directories {
        let dir = directory.as_path().canonicalize()?;
        info!("Processing {}", dir.display());
        core_lib::walk_directory_and_process_files(&dir, &scope, |p| {
            // Use a block to handle the async call in a sync closure
            // In a real app we might want to collect all paths and then process async
            tokio::task::block_in_place(|| {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(check_links_in_file(p))
            })
        })?;
    }
    info!("DONE");
    Ok(())
}

async fn check_links_in_file(filepath: &Path) -> anyhow::Result<()> {
    info!("Checking file: {}", filepath.display());

    // Read the file content
    let content = std::fs::read_to_string(filepath)?;

    // Find all links
    let links = links::check_links(&content, true);

    if links.is_empty() {
        return Ok(());
    }

    info!("Found {} links in {}", links.len(), filepath.display());

    for link in links {
        links::validate_link(&link).await;
    }

    Ok(())
}
