#![allow(missing_docs)]
mod cli;
mod conf;
mod links;
mod refdefs;

use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;

use clap::Parser;
use links::AsyncLinkChecker;
use parse_lib::parse_document;
use tokio::runtime::Handle;
use tracing::error;
use tracing::info;
use tracing::warn;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Install a global tracing subscriber that listens for events
    // and filters based on the value of the RUST_LOG environment variable.
    tracing_subscriber::fmt::init();

    let args = cli::Args::parse();
    let conf = Arc::new(conf::Config::default());
    let scope = core_lib::Scope::default();

    let link_checker = Arc::new(AsyncLinkChecker::new());
    let handle = Handle::current();

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

        let checker = Arc::clone(&link_checker);
        let config = conf.clone();
        let handle_clone = handle.clone();

        core_lib::walk_directory_and_process_files(&dir, &scope, move |p| {
            let checker = Arc::clone(&checker);
            let config = config.clone();
            let handle = handle_clone.clone();

            check_links_in_file(p, &config, &checker, &handle)
        })?;
    }

    info!("DONE");
    Ok(())
}

fn check_links_in_file(
    filepath: &Path,
    conf: &conf::Config,
    checker: &AsyncLinkChecker,
    handle: &Handle,
) -> anyhow::Result<()> {
    core_lib::process_text_file(
        filepath,
        |s: &str| s.contains("[") || s.contains("http"),
        |content: &str| {
            let filename = filepath.file_name().unwrap_or_default().to_string_lossy();
            let doc = match parse_document(&filename, Some(filepath), content) {
                Ok(d) => d,
                Err(e) => {
                    error!("Failed to parse {}: {}", filepath.display(), e);
                    return Cow::Owned(content.to_string());
                }
            };

            let mut current_content = Cow::Borrowed(content);

            if conf.detect_unused_reference_definitions {
                let unused = refdefs::find_unused_refdefs(&doc);
                if !unused.is_empty() {
                    info!(
                        "Found {} unused refdefs in {}",
                        unused.len(),
                        filepath.display()
                    );
                    if conf.delete_unused_reference_definitions {
                        let new_content = refdefs::delete_unused_refdefs(&doc, &unused);
                        current_content = Cow::Owned(new_content);
                    }
                }
            }

            if conf.check_urls {
                let urls = links::extract_urls(&doc);
                for url in urls {
                    if url.scheme() == "http" || url.scheme() == "https" {
                        if !conf.check_external_urls {
                            continue;
                        }

                        // Use the shared handle to block on the async check.
                        let result = handle.block_on(checker.check_url(url.clone()));
                        if !result.success {
                            warn!(
                                "Broken link in {}: {} (Status: {:?}, Error: {:?})",
                                filepath.display(),
                                url,
                                result.status,
                                result.error
                            );
                        }
                    }
                }
            }

            match current_content {
                Cow::Owned(s) => Cow::Owned(s),
                Cow::Borrowed(s) => Cow::Owned(s.to_string()),
            }
        },
    )?;

    Ok(())
}
