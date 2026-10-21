//! Expands custom directives and wikilinks in Markdown files.

mod crate_blocks;
use crate_blocks::*;

/// Directive-processing configuration types.
mod conf;
pub use conf::*;

mod links_or_badges;
use links_or_badges::*;

mod examples;
use std::ops::Range;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
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

/// Expands configured directives and wikilinks in an in-memory Markdown document.
///
/// This is the render-time API for adapters such as mdBook preprocessors. Unlike
/// [`process_all_directives`], it never reads or rewrites authoring files.
/// Use [`process_markdown_with_example_resolver`] when examples need resolution
/// relative to a chapter or another caller-specific root.
pub fn process_markdown(markdown: &str, conf: &Config) -> Result<String> {
    process_markdown_with_example_resolver(markdown, conf, |name| {
        let name = name.trim();
        anyhow::ensure!(!name.is_empty(), "example name cannot be empty");
        let directive = parse_lib::ast::DirectiveData::ExampleBlock { name };
        Ok(render_example(&directive))
    })
}

/// Expands directives in-memory, using `resolve_example` for each example block.
///
/// The callback returns the Markdown replacement for an example directive. It
/// can resolve a path in the caller's context and return a contextual error when
/// the example cannot be found. Any callback failure aborts the transformation.
pub fn process_markdown_with_example_resolver<F>(
    markdown: &str,
    conf: &Config,
    mut resolve_example: F,
) -> Result<String>
where
    F: FnMut(&str) -> Result<String>,
{
    use parse_lib::ast::DirectiveData;
    use parse_lib::markdown_renderer::MarkdownRenderer;
    use parse_lib::visit::Visitor;

    let document = parse_lib::parse_document("<in-memory Markdown>", None, markdown)
        .context("Failed to parse in-memory Markdown")?;
    let mut errors = Vec::new();
    let rendered = {
        let mut renderer = MarkdownRenderer::new(|directive| match directive {
            DirectiveData::CrateBlock { .. } if conf.process_crate_block_directives => {
                render_crate_block(directive)
            }
            DirectiveData::CategoryBlock { .. } if conf.process_crate_block_directives => {
                render_category_block(directive)
            }
            DirectiveData::ExampleBlock { name } if conf.process_example_directives => {
                match resolve_example(name) {
                    Ok(replacement) => replacement,
                    Err(error) => {
                        errors.push(error.context(format!("Failed to resolve example '{name}'")));
                        directive.to_string()
                    }
                }
            }
            DirectiveData::Link { .. } | DirectiveData::Badge { .. }
                if conf.process_link_and_badge_directives =>
            {
                render_link_or_badge(directive)
            }
            _ => directive.to_string(),
        });

        if conf.process_wikilinks {
            renderer = renderer.with_wikilink_resolver(|wikilink| {
                let display_text = wikilink.display.unwrap_or(wikilink.target);
                let link_label = wikilink.target.replace(' ', "-");
                let after = wikilink.immediately_after.unwrap_or("");
                format!("[{display_text}{after}][{link_label}]")
            });
        }

        renderer.visit_document(&document);
        renderer.markdown
    };

    if !errors.is_empty() {
        let details = errors
            .into_iter()
            .map(|error| format!("- {error:#}"))
            .collect::<Vec<_>>()
            .join("\n");
        anyhow::bail!("Markdown contains unresolved directives:\n{details}");
    }

    Ok(rendered)
}

/// Fails when custom `{{#...}}` directives remain outside code after processing.
/// Native mdBook helpers such as `{{#include ...}}` are allowed and left intact.
pub fn validate_no_unexpanded_directives(markdown: &str) -> Result<()> {
    use parse_lib::ast::CodeSpanData;
    use parse_lib::ast::FencedCodeBlockData;
    use parse_lib::ast::HtmlCommentData;
    use parse_lib::visit::Visitor;

    struct ProtectedSpanCollector {
        source_base: usize,
        source_len: usize,
        spans: Vec<Range<usize>>,
    }

    impl ProtectedSpanCollector {
        fn add_span(&mut self, text: &str) {
            let Some(start) = (text.as_ptr() as usize).checked_sub(self.source_base) else {
                return;
            };
            let Some(end) = start.checked_add(text.len()) else {
                return;
            };
            if end <= self.source_len {
                self.spans.push(start..end);
            }
        }
    }

    impl<'a> Visitor<'a> for ProtectedSpanCollector {
        fn visit_fenced_code_block(&mut self, data: &'a FencedCodeBlockData<'a>) {
            for line in &data.lines {
                self.add_span(line);
            }
        }

        fn visit_code_span(&mut self, data: &'a CodeSpanData<'a>) {
            self.add_span(data.content);
        }

        fn visit_html_comment(&mut self, data: &'a HtmlCommentData<'a>) {
            for line in &data.lines {
                self.add_span(line);
            }
        }
    }

    let document = parse_lib::parse_document("<in-memory Markdown>", None, markdown)
        .context("Failed to inspect in-memory Markdown for unresolved directives")?;
    let mut collector = ProtectedSpanCollector {
        source_base: markdown.as_ptr() as usize,
        source_len: markdown.len(),
        spans: Vec::new(),
    };
    collector.visit_document(&document);
    collector.spans.sort_by_key(|range| range.start);

    let mut unresolved = Vec::new();
    let mut cursor = 0;
    for range in collector.spans {
        if range.start > cursor
            && let Some(text) = markdown.get(cursor..range.start)
        {
            collect_unresolved_directive_text(text, &mut unresolved);
        }
        cursor = cursor.max(range.end);
    }
    if cursor < markdown.len()
        && let Some(text) = markdown.get(cursor..)
    {
        collect_unresolved_directive_text(text, &mut unresolved);
    }

    if unresolved.is_empty() {
        return Ok(());
    }

    anyhow::bail!(
        "Unexpanded custom directives remain outside code: {}",
        unresolved.join(", ")
    )
}

fn collect_unresolved_directive_text(text: &str, unresolved: &mut Vec<String>) {
    const NATIVE_MDBOOK_HELPERS: [&str; 3] = ["include", "rustdoc_include", "playground"];
    let mut offset = 0;

    while let Some(rest) = text.get(offset..) {
        let Some(relative_start) = rest.find("{{#") else {
            break;
        };
        let start = offset + relative_start;
        let body_start = start + 3;
        let Some(body) = text.get(body_start..) else {
            break;
        };
        let close = body.find("}}");
        let body_end = close.map_or(text.len(), |index| body_start + index);
        let Some(directive_body) = text.get(body_start..body_end) else {
            break;
        };
        let name = directive_body
            .trim_start()
            .split(|character: char| character.is_whitespace() || character == ':')
            .next()
            .unwrap_or_default();
        if !NATIVE_MDBOOK_HELPERS.contains(&name) {
            let fragment_end = close.map_or(text.len(), |_| body_end.saturating_add(2));
            if let Some(fragment) = text.get(start..fragment_end) {
                unresolved.push(fragment.to_string());
            }
        }
        if close.is_none() {
            break;
        }
        offset = body_end.saturating_add(2);
    }
}

/// Processes directives in all matching files within the specified directories.
///
/// The provided [`Config`] controls which directive families and file filters are active.
pub fn process_all_directives<I>(directories: I, conf: &Config) -> anyhow::Result<()>
where
    I: IntoIterator<Item = std::path::PathBuf>,
{
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
    use crate::common::replace_text_file_atomically;

    if !doc.source.contains("{{") && !doc.source.contains("[[") {
        return Ok(());
    }

    let markdown = process_markdown(doc.source, conf)
        .with_context(|| format!("Failed to process Markdown file {}", filepath.display()))?;
    if markdown != doc.source {
        replace_text_file_atomically(filepath, &markdown)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;

    use super::*;

    #[test]
    fn in_memory_processing_expands_directives_and_preserves_native_helpers_and_code() {
        let source = concat!(
            "# Keep this heading\n\n",
            "{{#include chapter.incl.md}}\n\n",
            "{{docs anyhow}} {{!docs anyhow}}\n\n",
            "{{#crate anyhow filesystem}}\n\n",
            "{{#categories filesystem=Filesystem}}\n\n",
            "{{#example ../../../crates/cats/filesystem/examples/read_write/read_file.rs:example}}\n\n",
            "```rust\n{{#crate untouched}}\n{{#example untouched}}\n```\n"
        );
        let config = Config::default();
        let rendered = process_markdown_with_example_resolver(source, &config, |name| {
            Ok(format!("{{{{#include {name}}}}}"))
        })
        .expect("in-memory transformation succeeds");

        assert!(rendered.starts_with("# Keep this heading\n\n{{#include chapter.incl.md}}"));
        assert!(rendered.contains("[anyhow][c~anyhow~docs]↗{{hi:anyhow}}"));
        assert!(rendered.contains("[![anyhow][c~anyhow~docs~badge]][c~anyhow~docs]"));
        assert!(
            rendered.contains("[![cat~filesystem][cat~filesystem~badge]][cat~filesystem]"),
            "rendered output: {rendered}"
        );
        assert!(rendered.contains(
            "[![anyhow][c~anyhow~docs~badge]][c~anyhow~docs]{{hi:anyhow}} [![cat~filesystem][cat~filesystem~badge]][cat~filesystem]{{hi:filesystem}}"
        ));
        assert!(rendered.contains("{{hi:Filesystem}}"));
        assert!(rendered.contains(
            "[![cat~filesystem][cat~filesystem~badge]][cat~filesystem]{{hi:Filesystem}}"
        ));
        assert!(rendered.contains(
            "{{#include ../../../crates/cats/filesystem/examples/read_write/read_file.rs:example}}"
        ));
        assert!(rendered.contains("```rust\n{{#crate untouched}}\n{{#example untouched}}\n```"));
    }

    #[test]
    fn in_memory_processing_returns_example_resolution_errors() {
        let error = process_markdown_with_example_resolver(
            "{{#example missing.rs:example}}",
            &Config::default(),
            |name| Err(anyhow!("no such example: {name}")),
        )
        .expect_err("missing examples are reported");

        assert!(error.to_string().contains("unresolved directives"));
        assert!(format!("{error:#}").contains("no such example: missing.rs:example"));
    }

    #[test]
    fn crate_and_category_directives_preserve_existing_book_badges() {
        let config = Config::default();
        let source = "{{#crate std}} {{#categories filesystem=Filesystem}}";
        let expected = "[![std][c~std~docs~badge]][c~std~docs]{{hi:std}} [![cat~filesystem][cat~filesystem~badge]][cat~filesystem]{{hi:Filesystem}}";

        let rendered = process_markdown(source, &config).expect("directives render");
        assert_eq!(rendered, expected);
        assert_eq!(
            process_markdown(&rendered, &config).expect("repeat render is a no-op"),
            expected
        );
    }

    #[test]
    fn unresolved_check_ignores_code_and_native_mdbook_helpers() {
        let source = concat!(
            "{{#include section.md}}\n\n",
            "```rust\n{{#crate code_example}}\n```\n"
        );
        assert!(validate_no_unexpanded_directives(source).is_ok());
    }

    #[test]
    fn unresolved_check_reports_unknown_and_malformed_directives() {
        let error = validate_no_unexpanded_directives("{{#unknown thing}} {{#example}}")
            .expect_err("unresolved custom directives are errors");

        assert!(error.to_string().contains("#unknown"));
        assert!(error.to_string().contains("#example"));
    }
}

