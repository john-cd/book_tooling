// Note on old code: This module was moved to `book_tool`.
// The external `directive_lib` has been merged directly into this module.

use anyhow::Result;
use std::path::PathBuf;

// TODO: Declare the modules for the files you moved from `directive_lib/src/`
// and import `Config` and `process_all_directives` into scope. For example:
// pub mod config;
// pub mod process;
// use self::config::Config;
// use self::process::process_all_directives;

pub fn execute(directories: Vec<PathBuf>) -> Result<()> {
    // It assumes a `config.toml` in the current directory.
    let conf = Config::from_file("config.toml").unwrap_or_else(|_| {
        tracing::warn!("Could not load config.toml, using default configuration.");
        Config::default()
    });

    process_all_directives(directories, &conf)?;
    println!("Directive processing complete.");

    Ok(())
}
