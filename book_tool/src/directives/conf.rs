//! Configuration of the directives subcommand, loaded from a TOML file.
//!
//! TODO Reviewed ??-??-2026

use std::default::Default;
use std::path::Path;

use anyhow::Context as _;
use serde::Deserialize;
use serde::Serialize;

/// Configuration for directive processing.
///
/// When deserializing, missing fields are filled from [`Default`]. The same
/// shape is accepted from `config.toml` through [`Config::from_file`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "kebab-case")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Each flag independently toggles one directive family in config.toml."
)]
pub struct Config {
    /// If true, process crate and category block directives e.g.
    /// `{{#crate my_crate}}`
    /// `{{#categories filesystem,api-bindings}}`
    /// into several badges for the crate, its categories, etc...
    pub process_crate_block_directives: bool,

    /// If true, process example directives, e.g.
    /// `{{#example some_example}}`
    /// into a fenced code block that includes an example file.
    pub process_example_directives: bool,

    /// If true, convert crate-level links and badges directives, e.g.,
    /// `{{crate xyz}}`, `{{docs xyz}}`, `{{github xyz}}`, `{{lib.rs xyz}}`,
    /// `{{crates.io xyz}}`, `{{web xyz}}`
    /// and
    /// `{{!crate xyz}}`, etc.
    ///  into Markdown links to or badges for the crate's corresponding web pages.
    ///
    /// Also convert category links and badges directives, e.g.,
    ///   `{{cat: accessibility}}`
    ///   `{{!cat mathematics}}`
    /// into links to or badges for the category.
    pub process_link_and_badge_directives: bool,

    /// If true, process wikilinks, e.g.
    /// `[[file | title1]]`
    /// and convert them to markdown links to internal pages.
    pub process_wikilinks: bool,

    /// Extensions to process. If `None`, uses default ("md").
    pub included_extensions: Vec<String>,

    /// File names to exclude. If `None`, uses default ("refs.incl.md", "SUMMARY.md").
    pub excluded_filenames: Vec<String>,
}

impl Default for Config {
    /// Returns the default configuration: every directive family enabled, Markdown files only.
    fn default() -> Self {
        Self {
            process_crate_block_directives: true,
            process_example_directives: true,
            process_link_and_badge_directives: true,
            process_wikilinks: true,
            included_extensions: vec!["md".to_owned()],
            excluded_filenames: vec!["refs.incl.md".to_owned(), "SUMMARY.md".to_owned()],
        }
    }
}

impl Config {
    /// Loads directive-processing configuration from a TOML file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or is not valid configuration TOML.
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_config(contents: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("config.toml");
        std::fs::write(&path, contents).expect("write config");
        (dir, path)
    }

    #[test]
    fn empty_file_yields_defaults() {
        let (_dir, path) = write_config("");
        assert_eq!(Config::from_file(&path).expect("loads"), Config::default());
    }

    #[test]
    fn partial_file_fills_missing_fields_from_defaults() {
        let (_dir, path) = write_config(
            "process-wikilinks = false\nincluded-extensions = [\"md\", \"markdown\"]\n",
        );
        let config = Config::from_file(&path).expect("loads");
        assert!(!config.process_wikilinks);
        assert_eq!(config.included_extensions, ["md", "markdown"]);
        assert!(config.process_example_directives);
        assert_eq!(
            config.excluded_filenames,
            Config::default().excluded_filenames
        );
    }

    #[test]
    fn unrecognized_keys_are_ignored() {
        let (_dir, path) = write_config("not-a-real-key = 1\nprocess-wikilinks = false\n");
        let config = Config::from_file(&path).expect("unknown keys are ignored");
        assert!(!config.process_wikilinks);
    }

    #[test]
    fn malformed_toml_reports_path() {
        let (_dir, path) = write_config("process-wikilinks = [unclosed\n");
        let err = Config::from_file(&path).expect_err("must fail");
        assert!(
            format!("{err:#}").contains("Failed to parse config file"),
            "{err:#}"
        );
    }

    #[test]
    fn wrong_value_type_is_an_error() {
        let (_dir, path) = write_config("process-wikilinks = \"yes\"\n");
        Config::from_file(&path).expect_err("string is not a bool");
    }

    #[test]
    fn missing_file_reports_read_failure() {
        let dir = tempfile::tempdir().expect("temp dir");
        let err = Config::from_file(dir.path().join("absent.toml")).expect_err("must fail");
        assert!(
            format!("{err:#}").contains("Failed to read config file"),
            "{err:#}"
        );
    }

    #[test]
    fn serialization_round_trips() {
        let config = Config::default();
        let text = toml::to_string(&config).expect("serializes");
        assert_eq!(toml::from_str::<Config>(&text).expect("parses"), config);
    }
}
