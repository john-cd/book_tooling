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

    files.into_iter().try_for_each(|file| {
        tracing::info!("Processing {}.", file.display());
        f(&file).with_context(|| format!("Error while processing file: {}", file.display()))
    })?;

    Ok(())
}
