use std::ffi::OsString;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use walkdir::WalkDir;

/// Defines what files should be processed based on their extension and filename.
#[derive(Debug)]
pub struct Scope {
    /// Extensions to process (e.g., "md", "txt").
    pub included_extensions: Vec<OsString>,
    /// File names (i.e. final component of a `Path`) to exclude (e.g., "SUMMARY.md").
    pub excluded_filenames: Vec<OsString>,
}

impl Default for Scope {
    fn default() -> Self {
        Self {
            included_extensions: vec![OsString::from("md")],
            excluded_filenames: vec![OsString::from("refs.incl.md"), OsString::from("SUMMARY.md")],
        }
    }
}

/// Walks through a directory and processes all files matching the specified scope.
///
/// # Arguments
///
/// * `root` - The directory to walk through.
/// * `scope` - The criteria for including/excluding files.
/// * `f` - A function that processes one file at a time.
///
/// # Errors
///
/// Returns an error if directory traversal fails or if the processing function `f` returns an error.
pub fn walk_directory_and_process_files<F>(root: &Path, scope: &Scope, mut f: F) -> Result<()>
where
    F: FnMut(&Path) -> Result<()> + Sync + Send,
{
    tracing::debug!(
        root = %root.display(),
        included_extensions = ?scope.included_extensions,
        excluded_filenames = ?scope.excluded_filenames,
        "Scanning directory for matching files"
    );

    let files = WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e: &walkdir::DirEntry| {
            let p = e.path();
            let file_name = p.file_name();
            let ext = p.extension();

            p.is_file()
                && file_name.is_some_and(|f_n: &std::ffi::OsStr| {
                    !scope.excluded_filenames.contains(&f_n.to_os_string())
                })
                && ext.is_some_and(|ext: &std::ffi::OsStr| {
                    scope.included_extensions.contains(&ext.to_os_string())
                })
        })
        .map(|e: walkdir::DirEntry| e.path().to_path_buf())
        .collect::<Vec<_>>();

    tracing::debug!(
        root = %root.display(),
        file_count = files.len(),
        "Collected matching files for directory walk"
    );

    files.into_iter().try_for_each(|file| {
        tracing::info!("Processing {}.", file.display());
        f(&file).with_context(|| format!("Error while processing file: {}", file.display()))
    })?;

    Ok(())
}

/// Walks multiple directories and processes all files matching the provided scope.
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
    for directory in directories {
        tracing::debug!(directory = %directory.as_ref().display(), "Walking directory");
        walk_directory_and_process_files(directory.as_ref(), scope, &mut f)?;
    }

    Ok(())
}

/// Walks Markdown files, parses them, and invokes `on_document` for each successfully parsed AST.
///
/// Parse failures are delegated to `on_parse_error` and do not stop the directory walk.
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
    walk_directories_and_process_files(directories, scope, |path| {
        tracing::trace!(path = %path.display(), "Reading Markdown file during parsed walk");
        let content = std::fs::read_to_string(path)?;

        match parse_lib::parse_document("file", None, &content) {
            Ok(doc) => {
                tracing::trace!(path = %path.display(), "Parsed Markdown file during walk");
                on_document(path, &doc)
            }
            Err(error) => {
                on_parse_error(path, &error);
                Ok(())
            }
        }
    })
}
