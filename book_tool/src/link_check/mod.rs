use anyhow::Result;
use core_lib::walk_directory_and_process_files;
use crate::verifier::LinkVerifier;
use parse_lib::{link_extractor::LinkExtractor, visit::Visitor};
use std::collections::HashSet;
use std::path::PathBuf;

pub async fn execute(directories: Vec<PathBuf>) -> Result<()> {
    let verifier = LinkVerifier::new();
    let mut all_links = HashSet::new();

    let scope = core_lib::Scope::default();

    for dir in &directories {
        walk_directory_and_process_files(dir, &scope, |path| {
            let content = std::fs::read_to_string(path)?;
            if let Ok(doc) = parse_lib::parse_document("doc", Some(path), &content) {
                let mut extractor = LinkExtractor::new();
                extractor.visit_document(&doc);
                all_links.extend(extractor.external_links);
            }
            Ok(())
        })?;
    }

    println!("Found {} unique external links. Verifying...", all_links.len());
    let results = verifier.verify_links(&all_links).await;

    let mut broken_links = 0;
    for (url, is_valid) in results {
        if !is_valid {
            broken_links += 1;
            tracing::error!("BROKEN: {}", url);
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
