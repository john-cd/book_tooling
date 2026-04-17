use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Result;
use parse_lib::ast::AutolinkData;
use parse_lib::ast::InlineImageData;
use parse_lib::ast::InlineLinkData;
use parse_lib::ast::ReferenceDefinitionData;
use parse_lib::visit::Visitor;
use url::Url;

use crate::common::walk::Scope;
use crate::common::walk::walk_directory_and_process_files;
use crate::links::AsyncLinkChecker;

fn collect_active_external_links(
    doc: &parse_lib::Document,
    unused_refdefs: Option<&HashSet<String>>,
) -> HashSet<Url> {
    struct ActiveLinkCollector<'a> {
        urls: HashSet<Url>,
        unused_refdefs: Option<&'a HashSet<String>>,
    }

    impl ActiveLinkCollector<'_> {
        fn add_url(&mut self, url: &str) {
            if let Ok(parsed) = Url::parse(url)
                && matches!(parsed.scheme(), "http" | "https")
            {
                self.urls.insert(parsed);
            }
        }
    }

    impl<'ast> Visitor<'ast> for ActiveLinkCollector<'_> {
        fn visit_inline_link(&mut self, data: &'ast InlineLinkData<'ast>) {
            self.add_url(data.url);
        }

        fn visit_autolink(&mut self, data: &'ast AutolinkData<'ast>) {
            self.add_url(data.url);
        }

        fn visit_inline_image(&mut self, data: &'ast InlineImageData<'ast>) {
            self.add_url(data.url);
        }

        fn visit_reference_definition(&mut self, data: &'ast ReferenceDefinitionData<'ast>) {
            if self
                .unused_refdefs
                .is_some_and(|unused| unused.contains(data.label))
            {
                return;
            }

            self.add_url(data.url);
        }
    }

    let mut collector = ActiveLinkCollector {
        urls: HashSet::new(),
        unused_refdefs,
    };
    collector.visit_document(doc);
    collector.urls
}

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
                let unused = check_refdefs.then(|| crate::refdefs::find_unused_refdefs(&doc));

                if check_refdefs {
                    let unused = unused.as_ref().expect("unused refdefs should be available");
                    if !unused.is_empty() {
                        tracing::info!(
                            "Found {} unused refdefs in {}",
                            unused.len(),
                            path.display()
                        );
                        if delete_unused {
                            let new_content = crate::refdefs::delete_unused_refdefs(&doc, unused);
                            std::fs::write(path, new_content)?;
                        }
                    }
                }

                all_links.extend(collect_active_external_links(&doc, unused.as_ref()));
            }
            Ok(())
        })?;
    }

    println!(
        "Found {} unique external links. Verifying...",
        all_links.len()
    );

    let mut results = checker.check_urls(all_links).await;
    results.sort_by(|(left, _), (right, _)| left.as_str().cmp(right.as_str()));

    let mut broken_links = 0;
    for (url, result) in results {
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
