//! Expands custom directives and wikilinks in Markdown files.

mod crate_blocks;
use crate_blocks::*;

/// Directive-processing configuration types.
mod conf;
pub use conf::*;

mod links_or_badges;
use links_or_badges::*;

mod examples;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use examples::*;

use crate::common::Scope;
use crate::common::walk_parsed_markdown_files;

/// Processes directives in all configured Markdown files under the given directories.
///
/// The command loads `config.toml` from the current working directory when present,
/// and otherwise falls back to [`Config::default`]. Files are rewritten in place.
pub fn execute(directories: Vec<PathBuf>) -> Result<()> {
    tracing::info!(
        directory_count = directories.len(),
        "Starting directive processing workflow"
    );

    let config_path = Path::new("config.toml");
    let conf = match config_path.try_exists()? {
        true => match Config::from_file(config_path) {
            Ok(conf) => {
                tracing::info!(path = %config_path.display(), "Loaded directive configuration");
                conf
            }
            Err(error) => {
                tracing::error!(
                    path = %config_path.display(),
                    error = %error,
                    "Failed to load directive configuration"
                );
                return Err(error);
            }
        },
        false => {
            tracing::info!("No config.toml found; using default directive configuration");
            Config::default()
        }
    };

    process_all_directives(directories, &conf)?;
    tracing::info!("Completed directive processing workflow");
    println!("Directive processing complete.");

    Ok(())
}

/// Processes directives in all matching files within the specified directories.
///
/// The provided [`Config`] controls which directive families and file filters are active.
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

    let directories = directories
        .into_iter()
        .map(|directory| {
            let dir = directory.as_path().canonicalize().with_context(|| {
                format!(
                    "Failed to canonicalize directory: {}",
                    directory.as_path().display()
                )
            })?;
            tracing::info!("Processing directory {}", dir.display());
            Ok(dir)
        })
        .collect::<Result<Vec<_>>>()?;

    walk_parsed_markdown_files(
        &directories,
        &scope,
        |path, doc| process_all_directives_in_file(path, doc, conf),
        crate::common::warn_and_continue_on_parse_failure,
    )?;

    Ok(())
}

/// Given a file, process all directives found in it.
fn process_all_directives_in_file(
    filepath: &std::path::Path,
    doc: &parse_lib::Document<'_>,
    conf: &Config,
) -> anyhow::Result<()> {
    use parse_lib::markdown_renderer::MarkdownRenderer;
    use parse_lib::visit::Visitor;

    use crate::common::replace_text_file_atomically;

    if !doc.source.contains("{{") && !doc.source.contains("[[") {
        return Ok(());
    }

    let mut renderer = MarkdownRenderer::new(|directive| {
        use parse_lib::ast::DirectiveData;
        match directive {
            DirectiveData::CrateBlock { .. } if conf.process_crate_block_directives => {
                render_crate_block(directive)
            }
            DirectiveData::ExampleBlock { .. } if conf.process_example_directives => {
                render_example(directive)
            }
            DirectiveData::Link { .. } | DirectiveData::Badge { .. }
                if conf.process_link_and_badge_directives =>
            {
                render_link_or_badge(directive)
            }
            _ => directive.to_string(),
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

    renderer.visit_document(doc);
    if renderer.markdown != doc.source {
        replace_text_file_atomically(filepath, &renderer.markdown)?;
    }

    Ok(())
}
