//! Expands custom directives and wikilinks in Markdown files.
//!
//! TODO Reviewed ??-??-2026.

/// Implements crate blocks behavior for the directives component, keeping the related rules and
/// data transformations at one boundary. The parent module calls these routines to provide execute,
/// process markdown, process markdown with example resolver, validate no unexpanded directives
/// behavior.
mod crate_blocks;
use crate_blocks::render_badge_set;
use crate_blocks::render_category_block;
use crate_blocks::render_crate_block;

/// Directive-processing configuration types.
mod conf;

pub use conf::*;

/// Implements links or badges behavior for the directives component, keeping the related rules and
/// data transformations at one boundary. The parent module calls these routines to provide execute,
/// process markdown, process markdown with example resolver, validate no unexpanded directives
/// behavior.
mod links_or_badges;
use links_or_badges::render_link_or_badge;

/// Implements examples behavior for the directives component, keeping the related rules and data
/// transformations at one boundary. The parent module calls these routines to provide execute,
/// process markdown, process markdown with example resolver, validate no unexpanded directives
/// behavior.
mod examples;

use std::ops::Range;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use examples::render_example;

use crate::common::Scope;
use crate::common::walk_parsed_markdown_files;

/// Processes directives in all configured Markdown files under the given directories.
///
/// The command loads `config_path` when given, otherwise `config.toml` from the current
/// working directory when present, and otherwise falls back to [`Config::default`].
/// Files are rewritten in place.
///
/// # Errors
///
/// Returns an error if an explicitly named configuration file is missing, the configuration
/// cannot be loaded, or a directory cannot be processed.
pub fn execute(directories: Vec<PathBuf>, config_path: Option<&Path>) -> Result<()> {
    tracing::info!(
        directory_count = directories.len(),
        "Starting directive processing workflow"
    );

    if let Some(explicit) = config_path {
        anyhow::ensure!(
            explicit.try_exists()?,
            "Configuration file not found: {}",
            explicit.display()
        );
    }
    let config_path = config_path.unwrap_or_else(|| Path::new("config.toml"));
    let conf = if config_path.try_exists()? {
        match Config::from_file(config_path) {
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
        }
    } else {
        tracing::info!("No config.toml found; using default directive configuration");
        Config::default()
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
///
/// # Errors
///
/// Returns an error if the Markdown cannot be parsed or a directive cannot be expanded.
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
///
/// # Errors
///
/// Returns an error if the Markdown cannot be parsed, a directive cannot be expanded, or
/// `resolve_example` fails.
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
    use parse_lib::visit::Visitor as _;

    let document = parse_lib::parse_document("<in-memory Markdown>", None, markdown)
        .context("Failed to parse in-memory Markdown")?;
    let mut errors = Vec::new();
    let rendered = {
        let mut renderer = MarkdownRenderer::new(|directive| match directive {
            DirectiveData::CrateBlock { .. } if conf.process_crate_block_directives => {
                render_crate_block(directive)
            }
            DirectiveData::BadgeSet { .. } if conf.process_crate_block_directives => {
                render_badge_set(directive)
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
            DirectiveData::Link { .. }
            | DirectiveData::Badge { .. }
            | DirectiveData::CrateBlock { .. }
            | DirectiveData::BadgeSet { .. }
            | DirectiveData::CategoryBlock { .. }
            | DirectiveData::ExampleBlock { .. } => directive.to_string(),
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
///
/// # Errors
///
/// Returns an error that lists the directives still present outside code.
pub fn validate_no_unexpanded_directives(markdown: &str) -> Result<()> {
    use parse_lib::ast::CodeSpanData;
    use parse_lib::ast::FencedCodeBlockData;
    use parse_lib::ast::HtmlCommentData;
    use parse_lib::visit::Visitor;

    /// Collects source ranges that must be excluded from directive scanning.
    struct ProtectedSpanCollector<'s> {
        /// Original Markdown source borrowed by the parsed document.
        source: &'s str,
        /// Ranges occupied by code spans, fenced blocks, and HTML comments.
        spans: Vec<Range<usize>>,
    }

    impl ProtectedSpanCollector<'_> {
        /// Records the byte range of `text` when it borrows from the scanned source.
        fn add_span(&mut self, text: &str) {
            if let Some(span) = parse_lib::source_span(self.source, text) {
                self.spans.push(span);
            }
        }
    }

    impl<'a> Visitor<'a> for ProtectedSpanCollector<'_> {
        /// Protects the lines of a fenced code block from the directive scan.
        fn visit_fenced_code_block(&mut self, data: &'a FencedCodeBlockData<'a>) {
            for line in &data.lines {
                self.add_span(line);
            }
        }

        /// Protects the content of an inline code span from the directive scan.
        fn visit_code_span(&mut self, data: &'a CodeSpanData<'a>) {
            self.add_span(data.content);
        }

        /// Protects the lines of an HTML comment from the directive scan.
        fn visit_html_comment(&mut self, data: &'a HtmlCommentData<'a>) {
            for line in &data.lines {
                self.add_span(line);
            }
        }
    }

    let document = parse_lib::parse_document("<in-memory Markdown>", None, markdown)
        .context("Failed to inspect in-memory Markdown for unresolved directives")?;
    let mut collector = ProtectedSpanCollector {
        source: markdown,
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

/// Appends every `{{#...}}` directive in `text` that is not a native mdBook helper to `unresolved`.
fn collect_unresolved_directive_text(text: &str, unresolved: &mut Vec<String>) {
    /// Native mdBook helper names that are valid outside code blocks.
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
                unresolved.push(fragment.to_owned());
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
///
/// # Errors
///
/// Returns an error if a directory cannot be traversed or a file cannot be processed.
pub fn process_all_directives<I>(directories: I, conf: &Config) -> anyhow::Result<()>
where
    I: IntoIterator<Item = std::path::PathBuf>,
{
    // Extensions to process and excluded files.
    let scope = Scope {
        included_extensions: conf
            .included_extensions
            .iter()
            .map(std::convert::Into::into)
            .collect(),
        excluded_filenames: conf
            .excluded_filenames
            .iter()
            .map(std::convert::Into::into)
            .collect(),
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

/// Tests for directive resolution and source preservation.
#[cfg(test)]
mod tests {
    use anyhow::anyhow;

    use super::*;

    /// In-memory processing expands custom directives but leaves native helpers and code alone.
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
        assert!(rendered.contains("[anyhow][c~anyhow~docs]\u{2197}{{hi:anyhow}}"));
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

    /// A failing example resolver aborts processing with the resolver error.
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

    /// Crate and category directives expand to the badges the book already uses.
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

    /// Directives inside table cells expand, and expanding again changes nothing.
    #[test]
    fn crate_and_category_directives_expand_inside_tables_idempotently() {
        let config = Config::default();
        let source = concat!(
            "| Recipe | Crates | Categories |\n",
            "|--------|--------|------------|\n",
            "| [Sort values][ex~sort-values] | {{#crate std}} | ",
            "{{#categories algorithms=Algorithms}} |\n\n",
            "[ex~sort-values]: sorting.md#sort-values\n"
        );
        let expected = concat!(
            "| Recipe | Crates | Categories |\n",
            "|--------|--------|------------|\n",
            "| [Sort values][ex~sort-values] | ",
            "[![std][c~std~docs~badge]][c~std~docs]{{hi:std}} | ",
            "[![cat~algorithms][cat~algorithms~badge]][cat~algorithms]",
            "{{hi:Algorithms}} |\n\n",
            "[ex~sort-values]: sorting.md#sort-values\n"
        );

        let rendered = process_markdown(source, &config).expect("table directives render");
        assert_eq!(rendered, expected);
        assert_eq!(
            process_markdown(&rendered, &config).expect("second processing succeeds"),
            expected
        );
    }

    /// A badge set expands to the four crate badges, one index entry and the category badges.
    #[test]
    fn badge_set_renders_crate_badges_then_categories() {
        let rendered = process_markdown(
            "{{#badges ndarray data-structures science=Science}}\n",
            &Config::default(),
        )
        .expect("render badge set");

        assert_eq!(
            rendered,
            concat!(
                "[![ndarray][c~ndarray~docs~badge]][c~ndarray~docs] ",
                "[![ndarray~crates.io][c~ndarray~crates.io~badge]][c~ndarray~crates.io] ",
                "[![ndarray~repo][c~ndarray~repo~badge]][c~ndarray~repo] ",
                "[![ndarray~lib.rs][c~ndarray~lib.rs~badge]][c~ndarray~lib.rs]{{hi:ndarray}} ",
                "[![cat~data-structures][cat~data-structures~badge]][cat~data-structures]",
                "{{hi:data-structures}} ",
                "[![cat~science][cat~science~badge]][cat~science]{{hi:Science}}\n"
            )
        );
        validate_no_unexpanded_directives(&rendered).expect("badge set is fully expanded");
    }

    /// An unindexed badge set omits only the crate index term and keeps category titles.
    #[test]
    fn badge_set_can_omit_the_crate_index_entry() {
        let rendered =
            process_markdown("{{#badges ndarray= science=Science}}\n", &Config::default())
                .expect("render unindexed badge set");

        assert_eq!(
            rendered,
            concat!(
                "[![ndarray][c~ndarray~docs~badge]][c~ndarray~docs] ",
                "[![ndarray~crates.io][c~ndarray~crates.io~badge]][c~ndarray~crates.io] ",
                "[![ndarray~repo][c~ndarray~repo~badge]][c~ndarray~repo] ",
                "[![ndarray~lib.rs][c~ndarray~lib.rs~badge]][c~ndarray~lib.rs] ",
                "[![cat~science][cat~science~badge]][cat~science]{{hi:Science}}\n"
            )
        );
        validate_no_unexpanded_directives(&rendered).expect("badge set is fully expanded");
    }

    /// A badge set omits a category index term when its title is empty.
    #[test]
    fn badge_set_omits_empty_category_title_index_entry() {
        let rendered = process_markdown("{{#badges ndarray science=}}", &Config::default())
            .expect("render badge set with an empty category title");

        assert_eq!(
            rendered,
            concat!(
                "[![ndarray][c~ndarray~docs~badge]][c~ndarray~docs] ",
                "[![ndarray~crates.io][c~ndarray~crates.io~badge]][c~ndarray~crates.io] ",
                "[![ndarray~repo][c~ndarray~repo~badge]][c~ndarray~repo] ",
                "[![ndarray~lib.rs][c~ndarray~lib.rs~badge]][c~ndarray~lib.rs]",
                "{{hi:ndarray}} ",
                "[![cat~science][cat~science~badge]][cat~science]"
            )
        );
    }

    /// A category-only directive omits an index term when its title is empty.
    #[test]
    fn category_block_omits_empty_title_index_entry() {
        assert_eq!(
            process_markdown("{{#categories science=}}", &Config::default())
                .expect("render category with an empty title"),
            "[![cat~science][cat~science~badge]][cat~science]"
        );
    }

    /// The unresolved-directive check skips code and native mdBook helpers.
    #[test]
    fn unresolved_check_ignores_code_and_native_mdbook_helpers() {
        let source = concat!(
            "{{#include section.md}}\n\n",
            "```rust\n{{#crate code_example}}\n```\n"
        );
        validate_no_unexpanded_directives(source).expect("input should parse");
    }

    /// Unknown and unterminated directives are reported.
    #[test]
    fn unresolved_check_reports_unknown_and_malformed_directives() {
        let error = validate_no_unexpanded_directives("{{#unknown thing}} {{#example}}")
            .expect_err("unresolved custom directives are errors");

        assert!(error.to_string().contains("#unknown"));
        assert!(error.to_string().contains("#example"));
    }

    /// Checks that disabling directive families preserves native helpers and surrounding Markdown.
    #[test]
    fn disabled_families_preserve_native_helpers_and_unrelated_source() {
        let source = concat!(
            "# Original heading\n\n",
            "{{docs serde}} {{#crate serde}} {{#include refs.incl.md}}\n",
            "`{{!web https://example.com}}`\n",
            "```md\n{{#categories filesystem}}\n```\n"
        );
        let config = Config {
            process_crate_block_directives: false,
            process_example_directives: false,
            process_link_and_badge_directives: false,
            process_wikilinks: false,
            ..Config::default()
        };

        assert_eq!(
            process_markdown(source, &config).expect("render with custom families disabled"),
            source
        );
    }
}
