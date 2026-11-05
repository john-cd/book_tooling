//! Walks directories and runs a function on each file that matches a `Scope`.
//!
//! TODO Reviewed ??-??-2026

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::bail;
use walkdir::WalkDir;

/// Defines what files should be processed based on their extension and filename.
#[derive(Debug, Clone)]
pub struct Scope {
    /// Extensions to process (e.g., "md", "txt").
    pub included_extensions: Vec<OsString>,
    /// File names to exclude (e.g., "SUMMARY.md").
    pub excluded_filenames: Vec<OsString>,
}

impl Default for Scope {
    /// Returns the default scope: Markdown files except `refs.incl.md` and `SUMMARY.md`.
    fn default() -> Self {
        Self {
            included_extensions: vec![OsString::from("md")],
            excluded_filenames: vec![OsString::from("refs.incl.md"), OsString::from("SUMMARY.md")],
        }
    }
}

impl Scope {
    /// Returns the default scope adjusted by command-line style overrides.
    ///
    /// A non-empty `extensions` list replaces the default extensions (leading dots are
    /// ignored); `excluded_filenames` are added to the default exclusions.
    #[must_use]
    pub fn with_overrides<S: AsRef<str>>(extensions: &[S], excluded_filenames: &[S]) -> Self {
        let mut scope = Self::default();
        if !extensions.is_empty() {
            scope.included_extensions = extensions
                .iter()
                .map(|extension| OsString::from(extension.as_ref().trim_start_matches('.')))
                .collect();
        }
        scope.excluded_filenames.extend(
            excluded_filenames
                .iter()
                .map(|name| OsString::from(name.as_ref())),
        );
        scope
    }
}

/// Normalizes a path to a string usable as a map key; lowercased on Windows, where paths are
/// case-insensitive.
fn path_identity(path: &Path) -> String {
    let identity = path.to_string_lossy().replace('\\', "/");
    #[cfg(windows)]
    {
        identity.to_lowercase()
    }
    #[cfg(not(windows))]
    {
        identity
    }
}

/// Returns the files under `root` that `scope` includes and that are not already in `seen`, in
/// depth-first file-name order.
fn collect_files(root: &Path, scope: &Scope, seen: &mut HashSet<String>) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root).sort_by_file_name() {
        let entry = entry.with_context(|| format!("Failed to traverse {}", root.display()))?;
        if entry.file_type().is_dir() || entry.file_type().is_symlink() {
            continue;
        }

        let path = entry.path();
        let Some(file_name) = path.file_name() else {
            continue;
        };
        if scope.excluded_filenames.contains(&file_name.to_os_string())
            || !path.extension().is_some_and(|extension| {
                scope
                    .included_extensions
                    .contains(&extension.to_os_string())
            })
        {
            continue;
        }

        let canonical = path
            .canonicalize()
            .with_context(|| format!("Failed to resolve file {}", path.display()))?;
        if seen.insert(path_identity(&canonical)) {
            files.push(path.to_path_buf());
        }
    }
    Ok(files)
}

/// Walks multiple directories and processes each matching file once, including when roots overlap.
///
/// # Errors
///
/// Returns an error if a directory cannot be traversed or if `process` fails for a file.
pub fn walk_directories_and_process_files<I, F>(
    directories: I,
    scope: &Scope,
    mut f: F,
) -> Result<()>
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
    F: FnMut(&Path) -> Result<()> + Sync + Send,
{
    let mut seen = HashSet::new();
    let mut files = Vec::new();
    for directory in directories {
        let root = directory.as_ref();
        tracing::debug!(directory = %root.display(), "Walking directory");
        files.extend(collect_files(root, scope, &mut seen)?);
    }

    for file in files {
        tracing::info!("Processing {}.", file.display());
        f(&file).with_context(|| format!("Error while processing file: {}", file.display()))?;
    }
    Ok(())
}

/// Walks Markdown files, parses them, and invokes `on_document` for every valid document.
///
/// Parse failures are reported and accumulated. The walk returns an error with scanned,
/// processed, and failed counts after leaving all malformed source files untouched.
///
/// # Errors
///
/// Returns an error if a directory cannot be traversed, a file cannot be read, or `process` fails
/// for a file.
pub fn walk_parsed_markdown_files<I, F, E>(
    directories: I,
    scope: &Scope,
    mut on_document: F,
    mut on_parse_error: E,
) -> Result<()>
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
    F: for<'input> FnMut(&Path, &parse_lib::Document<'input>) -> Result<()> + Sync + Send,
    E: FnMut(&Path, &anyhow::Error) + Sync + Send,
{
    let mut scanned = 0_usize;
    let mut processed = 0_usize;
    let mut failed_paths = Vec::new();

    walk_directories_and_process_files(directories, scope, |path| {
        scanned += 1;
        tracing::trace!(path = %path.display(), "Reading Markdown file during parsed walk");
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) => {
                let error = anyhow::Error::new(error)
                    .context(format!("Failed to read Markdown file {}", path.display()));
                on_parse_error(path, &error);
                failed_paths.push(path.display().to_string());
                return Ok(());
            }
        };

        match parse_lib::parse_document("file", None, &content) {
            Ok(doc) => {
                tracing::trace!(path = %path.display(), "Parsed Markdown file during walk");
                on_document(path, &doc)?;
                processed += 1;
            }
            Err(error) => {
                on_parse_error(path, &anyhow::Error::new(error));
                failed_paths.push(path.display().to_string());
            }
        }
        Ok(())
    })?;

    if !failed_paths.is_empty() {
        let paths = failed_paths.join(", ");
        eprintln!(
            "Markdown processing incomplete: scanned {scanned}, processed {processed}, failed {} file(s): {paths}",
            failed_paths.len()
        );
        bail!(
            "Markdown processing incomplete after scanning {scanned} file(s): processed {processed}, failed {}",
            failed_paths.len()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    /// A file reachable from two overlapping roots is processed once.
    #[test]
    fn overlapping_roots_process_each_file_once() -> Result<()> {
        let temp = tempdir()?;
        let nested = temp.path().join("nested");
        std::fs::create_dir_all(&nested)?;
        let chapter = nested.join("chapter.md");
        std::fs::write(&chapter, "# Chapter\n")?;
        let nested = nested.canonicalize()?;
        let mut processed = Vec::new();

        walk_directories_and_process_files(
            [temp.path(), nested.as_path()],
            &Scope::default(),
            |path| {
                processed.push(path.canonicalize()?);
                Ok(())
            },
        )?;

        assert_eq!(processed, [chapter.canonicalize()?]);
        Ok(())
    }

    /// Files are visited depth-first in file-name order regardless of the filesystem's directory
    /// order.
    #[test]
    fn traversal_order_is_deterministic() -> Result<()> {
        let temp = tempdir()?;
        std::fs::create_dir_all(temp.path().join("a"))?;
        for name in ["b.md", "a/c.md", "a/b.md", "z.md", "a.md"] {
            std::fs::write(temp.path().join(name), "# Title\n")?;
        }
        let mut visited = Vec::new();

        walk_directories_and_process_files([temp.path()], &Scope::default(), |path| {
            visited.push(
                path.strip_prefix(temp.path())?
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
            Ok(())
        })?;

        assert_eq!(visited, ["a/b.md", "a/c.md", "a.md", "b.md", "z.md"]);
        Ok(())
    }

    /// A missing root fails the walk instead of looking empty.
    #[test]
    fn traversal_failure_is_reported() -> Result<()> {
        let temp = tempdir()?;
        let missing = temp.path().join("missing");

        let error = walk_directories_and_process_files([&missing], &Scope::default(), |_| Ok(()))
            .expect_err("missing roots must fail instead of looking empty");

        assert!(error.to_string().contains("traverse"));
        Ok(())
    }

    /// Overrides replace the extensions and extend the exclusions.
    #[test]
    fn scope_overrides_replace_extensions_and_extend_exclusions() {
        let none: [&str; 0] = [];
        let default = Scope::with_overrides(&none, &none);
        assert_eq!(
            default.included_extensions,
            Scope::default().included_extensions
        );
        assert_eq!(
            default.excluded_filenames,
            Scope::default().excluded_filenames
        );

        let custom = Scope::with_overrides(&[".txt", "markdown"], &["skip.md"]);
        assert_eq!(
            custom.included_extensions,
            vec![OsString::from("txt"), OsString::from("markdown")]
        );
        assert!(
            custom
                .excluded_filenames
                .contains(&OsString::from("skip.md"))
        );
        assert!(
            custom
                .excluded_filenames
                .contains(&OsString::from("SUMMARY.md"))
        );
    }

    /// A custom scope selects files by extension and skips excluded names.
    #[test]
    fn custom_scope_filters_walked_files() {
        let temp = tempfile::tempdir().expect("temp dir");
        for name in ["a.txt", "b.md", "skip.txt"] {
            std::fs::write(temp.path().join(name), "x").expect("write");
        }
        let scope = Scope::with_overrides(&["txt"], &["skip.txt"]);
        let mut seen = Vec::new();
        walk_directories_and_process_files([temp.path()], &scope, |path| {
            seen.push(path.file_name().expect("file name").to_owned());
            Ok(())
        })
        .expect("walk");
        assert_eq!(seen, vec![OsString::from("a.txt")]);
    }
}
