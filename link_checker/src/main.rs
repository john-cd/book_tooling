// [finish](https://github.com/john-cd/rust_howto/issues/1422)

mod cli;
mod conf;
mod links;
mod refdefs;

use std::path::Path;
use clap::Parser;
use tracing::{info, error};
use std::sync::Arc;
use links::asynchro::LinkValidator;
use conf::Config;
use std::sync::Mutex;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = cli::Args::parse();
    let conf = Arc::new(Config::default());
    let scope = core_lib::Scope::default();

    let validator = Arc::new(LinkValidator::new(conf.follow_redirects));

    // Collect all files to process first.
    let files = Arc::new(Mutex::new(Vec::new()));
    for directory in &args.directories {
        if !directory.exists() {
            warn!(
                "Directory {} does not exist, skipping.",
                directory.display()
            );
            continue;
        }
        let dir = directory.as_path().canonicalize()?;
        info!("Walking {}", dir.display());
        let f = Arc::clone(&files);
        core_lib::walk_directory_and_process_files(&dir, &scope, move |p| {
            f.lock().unwrap().push(p.to_path_buf());
            Ok(())
        })?;
    }

    // Process files asynchronously.
    let files_to_process = files.lock().unwrap().clone();
    for filepath in files_to_process {
        process_file(&filepath, &validator, &conf).await?;
    }

    info!("DONE");
    Ok(())
}

async fn process_file(filepath: &Path, validator: &LinkValidator, conf: &Config) -> anyhow::Result<()> {
    info!("Processing file: {}", filepath.display());

    let content = std::fs::read_to_string(filepath)?;

    if conf.check_urls {
        check_links_in_file(filepath, &content, validator).await?;
    }

    if conf.detect_unused_reference_definitions {
        let unused = refdefs::detect_unused_reference_definitions(&content);
        if !unused.is_empty() {
            info!("Found {} unused reference definitions in {}", unused.len(), filepath.display());
        }

        if conf.delete_unused_reference_definitions {
            let new_content = refdefs::delete_unused_reference_definitions(&content);
            if let std::borrow::Cow::Owned(new_content) = new_content {
                std::fs::write(filepath, new_content)?;
                info!("Deleted unused reference definitions in {}", filepath.display());
            }
        }
    }

    Ok(())
}

async fn check_links_in_file(filepath: &Path, content: &str, validator: &LinkValidator) -> anyhow::Result<()> {
    let links = links::check_links(content, true);

    if links.is_empty() {
        return Ok(());
    }

    info!("Found {} links in {}", links.len(), filepath.display());

    let urls: Vec<_> = links.into_iter().map(|l| l.url).collect();
    let results = validator.validate_all(urls).await;

    for result in results {
        if !result.success {
            error!("Link FAILED ({}): {}", result.status, result.url);
        } else {
            info!("Link OK: {}", result.url);
        }
    }

    Ok(())
}
