use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Result;
use parse_lib::link_extractor::LinkExtractor;
use parse_lib::visit::Visitor;
use url::Url;

use crate::common::walk::Scope;
use crate::common::walk::walk_directory_and_process_files;
use crate::links::AsyncLinkChecker;

pub async fn execute(
    directories: Vec<PathBuf>,
    check_refdefs: bool,
    delete_unused: bool,
) -> Result<()> {
    let checker = AsyncLinkChecker::new();
    let mut all_links = HashSet::new();

    let scope = Scope::default();

    for dir in &directories {
        walk_directory_and_process_files(dir, &scope, |path| {
            let content = std::fs::read_to_string(path)?;
            let filename = path.file_name().unwrap_or_default().to_string_lossy();
            if let Ok(doc) = parse_lib::parse_document(&filename, Some(path), &content) {
                if check_refdefs {
                    let unused = crate::refdefs::find_unused_refdefs(&doc);
                    if !unused.is_empty() {
                        tracing::info!(
                            "Found {} unused refdefs in {}",
                            unused.len(),
                            path.display()
                        );
                        if delete_unused {
                            let new_content = crate::refdefs::delete_unused_refdefs(&doc, &unused);
                            std::fs::write(path, new_content)?;
                        }
                    }
                }

                let mut extractor = LinkExtractor::new();
                extractor.visit_document(&doc);
                for link in extractor.external_links {
                    if let Ok(url) = Url::parse(link) {
                        if url.scheme() == "http" || url.scheme() == "https" {
                            all_links.insert(url);
                        }
                    }
                }
            }
            Ok(())
        })?;
    }

    println!(
        "Found {} unique external links. Verifying...",
        all_links.len()
    );
    let mut broken_links = 0;
    for url in all_links {
        let result = checker.check_url(url.clone()).await;
        if !result.success {
            broken_links += 1;
            tracing::error!("BROKEN: {} (Error: {:?})", url, result.error);
        } else {
            tracing::info!("OK: {}", url);
        }
    }

    if broken_links > 0 {
        anyhow::bail!("Found {} broken links.", broken_links);
    } else {
        println!("All links verified successfully.");
    }
    Ok(())
}
