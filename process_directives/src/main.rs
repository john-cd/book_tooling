#![allow(missing_docs)]
mod cli;

use clap::Parser;
use directive_lib::process_all_directives;

fn main() -> anyhow::Result<()> {
    // Install a global tracing subscriber that listens for events
    // and filters based on the value of the RUST_LOG environment variable.
    tracing_subscriber::fmt::init();

    // Process command-line arguments to retrieve the directory to process:
    let args = cli::Args::parse();

    // TODO Use default configuration for now.
    let conf = directive_lib::Config::default();

    process_all_directives(args.directories, &conf)?;

    println!("DONE");
    Ok(())
}
