#![allow(missing_docs)]
#![allow(unused)]

mod crate_blocks;
use crate_blocks::*;

pub mod conf;
pub use conf::*;

mod links_or_badges;
use links_or_badges::*;

mod examples;
use examples::*;

/// Process all directives in all files within the specified directories.
///
/// This function walks through each directory, identifies files in scope,
/// and applies transformations based on the provided configuration.
///
/// # Arguments
///
/// * `directories` - An iterator over the directories to scan for files.
/// * `conf` - The configuration determining which directives to process.
///
/// # Errors
///
/// Returns an error if directory canonicalization fails or if any file processing error occurs.
pub fn process_all_directives<I>(directories: I, conf: &Config) -> anyhow::Result<()>
where
    I: IntoIterator<Item = std::path::PathBuf>,
{
    use anyhow::Context;

    // Extensions to process and excluded files.
    let scope = core_lib::Scope {
        included_extensions: conf.included_extensions.iter().map(|s| s.into()).collect(),
        excluded_filenames: conf.excluded_filenames.iter().map(|s| s.into()).collect(),
    };

    for directory in directories.into_iter() {
        let dir = directory.as_path().canonicalize()
            .with_context(|| format!("Failed to canonicalize directory: {}", directory.as_path().display()))?;
        tracing::info!("Processing directory {}", dir.display());
        core_lib::walk_directory_and_process_files(&dir, &scope, |f| {
            process_all_directives_in_file(f, conf)
        })?;
    }
    Ok(())
}

/// Given a file, process all directives found in it.
///
/// # Arguments
///
/// * `filepath` - The path to the file to process.
/// * `conf` - The configuration determining which directives to process.
fn process_all_directives_in_file(
    filepath: &std::path::Path,
    conf: &Config,
) -> anyhow::Result<()> {
    use parse_lib::parse_document;
    use parse_lib::markdown_renderer::MarkdownRenderer;
    use parse_lib::visit::Visitor;
    use std::borrow::Cow;

    // Read a text file in memory, test if its contents should be processed, and if true, update its contents.
    core_lib::process_text_file(
        filepath,
        |s: &str| s.contains("{{") || s.contains("[["), // Fast path check
        |s: &str| {
            match parse_document("file", Some(filepath), s) {
                Ok(doc) => {
                    let mut renderer = MarkdownRenderer::new(|directive| {
                        if conf.process_crate_block_directives && directive.name == "crate" {
                            return render_crate_block(directive);
                        }
                        if conf.process_example_directives && directive.name == "example" {
                            return render_example(directive);
                        }
                        if conf.process_link_and_badge_directives {
                            let is_badge = directive.name.starts_with('!');
                            let kind_str = if is_badge { &directive.name[1..] } else { directive.name };
                            if matches!(kind_str, "cat" | "crate" | "docs" | "github" | "lib.rs" | "crates.io" | "web") {
                                return render_link_or_badge(directive);
                            }
                        }

                        // Unhandled directives fallback: Best-effort reconstruction.
                        // Link and badge directives do not use the '#' prefix.
                        let prefix = if matches!(directive.name, "example" | "crate" | "include") { "#" } else { "" };
                        let space = if directive.content.is_empty() { "" } else { " " };
                        format!("{{{{{}{}{}{}}}}}", prefix, directive.name, space, directive.content)
                    });

                    if conf.process_wikilinks {
                        renderer = renderer.with_wikilink_resolver(|wikilink| {
                            // This logic is migrated from the old regex-based `wikilinks.rs`
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

#[cfg(test)]
mod tests {
    use super::*;
    use parse_lib::{markdown_renderer::MarkdownRenderer, parse_document, visit::Visitor};

    /// Test harness that runs the full AST parsing and directive rendering pipeline.
    fn run_pipeline_test(input: &str, expected: &str) {
        let conf = Config::default();
        run_pipeline_test_with_conf(input, expected, &conf);
    }

    /// Test harness that accepts a custom configuration.
    fn run_pipeline_test_with_conf(input: &str, expected: &str, conf: &Config) {
        let doc = parse_document("test.md", None, input).expect("Failed to parse test input");

        let mut renderer = MarkdownRenderer::new(|directive| {
            if conf.process_crate_block_directives && directive.name == "crate" {
                return render_crate_block(directive);
            }
            if conf.process_example_directives && directive.name == "example" {
                return render_example(directive);
            }
            if conf.process_link_and_badge_directives {
                let is_badge = directive.name.starts_with('!');
                let kind_str = if is_badge { &directive.name[1..] } else { directive.name };
                if matches!(
                    kind_str,
                    "cat" | "crate" | "docs" | "github" | "lib.rs" | "crates.io" | "web"
                ) {
                    return render_link_or_badge(directive);
                }
            }
            let prefix = if matches!(directive.name, "example" | "crate" | "include") { "#" } else { "" };
            let space = if directive.content.is_empty() { "" } else { " " };
            format!("{{{{{}{}{}{}}}}}", prefix, directive.name, space, directive.content)
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

        // Trim whitespace because the renderer adds newlines after block elements.
        assert_eq!(renderer.markdown.trim(), expected.trim());
    }

    #[test]
    fn test_integration_link_directive() {
        run_pipeline_test("Link to {{cat rust}}.", "Link to [rust][cat~rust]↗{{hi:rust}}.");
    }

    #[test]
    fn test_integration_wikilink() {
        run_pipeline_test("Link to [[a page]].", "Link to [a page][a-page].");
    }

    #[test]
    fn test_integration_skips_code_span() {
        let input = "This is a real one {{cat foo}}. This is not: `{{cat bar}}`.";
        let expected = "This is a real one [foo][cat~foo]↗{{hi:foo}}. This is not: `{{cat bar}}`.";
        run_pipeline_test(input, expected);
    }

    #[test]
    fn test_integration_directive_is_disabled() {
        let mut conf = Config::default();
        conf.process_link_and_badge_directives = false;
        let input = "This should not be processed: {{cat foo}}";
        let expected = "This should not be processed: {{cat foo}}";
        run_pipeline_test_with_conf(input, expected, &conf);
    }

    #[test]
    fn test_integration_directive_in_blockquote() {
        let input = "\
> This is a blockquote.
> Here is a directive: {{cat rust}}
> And some more text.";
        let expected = "\
> This is a blockquote.
> Here is a directive: [rust][cat~rust]↗{{hi:rust}}
> And some more text.";
        run_pipeline_test(input, expected);
    }

    #[test]
    fn test_integration_directive_in_nested_list_in_blockquote() {
        let input = "\
> - item 1
>
>   - nested: {{cat rust}}";
        let expected = "\
> - item 1
>
>   - nested: [rust][cat~rust]↗{{hi:rust}}";
        run_pipeline_test(input, expected);
    }
}
