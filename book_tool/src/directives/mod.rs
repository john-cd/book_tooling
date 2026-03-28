mod crate_blocks;
use crate_blocks::*;

pub mod conf;
pub use conf::*;

mod links_or_badges;
use links_or_badges::*;

mod examples;
use examples::*;

use anyhow::Result;
use std::path::PathBuf;
use crate::common::walk::{Scope, walk_directory_and_process_files};
use crate::common::process_text_file;

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

/// Process all directives in all files within the specified directories.
pub fn process_all_directives<I>(directories: I, conf: &Config) -> anyhow::Result<()>
where
    I: IntoIterator<Item = std::path::PathBuf>,
{
    use anyhow::Context;

    // Extensions to process and excluded files.
    let scope = Scope {
        included_extensions: conf.included_extensions.iter().map(|s| s.into()).collect(),
        excluded_filenames: conf.excluded_filenames.iter().map(|s| s.into()).collect(),
    };

    for directory in directories.into_iter() {
        let dir = directory.as_path().canonicalize()
            .with_context(|| format!("Failed to canonicalize directory: {}", directory.as_path().display()))?;
        tracing::info!("Processing directory {}", dir.display());
        walk_directory_and_process_files(&dir, &scope, |f| {
            process_all_directives_in_file(f, conf)
        })?;
    }
    Ok(())
}

/// Given a file, process all directives found in it.
fn process_all_directives_in_file(
    filepath: &std::path::Path,
    conf: &Config,
) -> anyhow::Result<()> {
    use parse_lib::parse_document;
    use parse_lib::markdown_renderer::MarkdownRenderer;
    use parse_lib::visit::Visitor;
    use std::borrow::Cow;

    process_text_file(
        filepath,
        |s: &str| s.contains("{{") || s.contains("[["), // Fast path check
        |s: &str| {
            match parse_document("file", Some(filepath), s) {
                Ok(doc) => {
                    let mut renderer = MarkdownRenderer::new(|directive| {
                        use parse_lib::ast::DirectiveData;
                        match directive {
                            DirectiveData::CrateBlock { .. } if conf.process_crate_block_directives => {
                                render_crate_block(directive)
                            }
                            DirectiveData::ExampleBlock { .. } if conf.process_example_directives => {
                                render_example(directive)
                            }
                            DirectiveData::Link { .. } | DirectiveData::Badge { .. } if conf.process_link_and_badge_directives => {
                                render_link_or_badge(directive)
                            }
                            _ => directive.to_string()
                        }
                    });

                    if conf.process_wikilinks {
                        renderer = renderer.with_wikilink_resolver(|wikilink| {
                            let display_text = wikilink.display.unwrap_or(wikilink.target);
                            let link_label = wikilink.target.replace(' ', "-");
                            let after = wikilink.immediately_after.unwrap_or("");
                            format!("[{}{}][{}]", display_text, after, link_label)
                        });
                    }

                    renderer.visit_document(&doc);
                    Cow::Owned(renderer.markdown)
                }
                Err(e) => {
                    tracing::warn!("Failed to parse {}: {}", filepath.display(), e);
                    Cow::Borrowed(s)
                }
            }
        },
    )
}
