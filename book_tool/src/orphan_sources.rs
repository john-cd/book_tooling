//! Resolves Rust module declarations and reports unreferenced workspace sources.
//!
//! TODO Reviewed ??-??-2026.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitCode;
use std::sync::OnceLock;

use anyhow::Context as _;
use anyhow::Result;
use regex::Regex;
use serde_json::Value;
use walkdir::WalkDir;

/// Checks each Cargo workspace member for unresolved and unreachable Rust sources.
///
/// # Errors
///
/// Returns an error when Cargo metadata, workspace paths, or Rust source files cannot be read.
pub fn execute(root: &Path) -> Result<ExitCode> {
    if !root.is_dir() {
        eprintln!("workspace root does not exist: {}", root.display());
        return Ok(ExitCode::from(2));
    }
    let root = root
        .canonicalize()
        .with_context(|| format!("failed to resolve workspace root {}", root.display()))?;
    let failures = scan_workspace(&root)?;
    if failures.is_empty() {
        println!("No orphan Rust source files found under workspace src/ trees.");
        Ok(ExitCode::SUCCESS)
    } else {
        eprintln!("orphan source check failed:");
        for failure in failures {
            eprintln!("- {failure}");
        }
        Ok(ExitCode::FAILURE)
    }
}

/// Collects orphan and module-resolution diagnostics for all workspace packages.
fn scan_workspace(root: &Path) -> Result<Vec<String>> {
    let members = workspace_package_dirs(root)?;
    let mut failures = Vec::new();
    for crate_dir in members {
        check_crate_sources(&crate_dir, &mut failures)?;
    }
    Ok(failures)
}

/// Discovers workspace package directories using Cargo's metadata response.
fn workspace_package_dirs(root: &Path) -> Result<Vec<PathBuf>> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(root)
        .output()
        .context("failed to run cargo metadata")?;
    if !output.status.success() {
        anyhow::bail!(
            "Cargo metadata rejected workspace {}: {}",
            root.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let metadata: Value =
        serde_json::from_slice(&output.stdout).context("Cargo metadata returned invalid JSON")?;
    let workspace_ids = metadata
        .get("workspace_members")
        .and_then(Value::as_array)
        .context("Cargo metadata omitted workspace_members")?
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    let packages = metadata
        .get("packages")
        .and_then(Value::as_array)
        .context("Cargo metadata omitted packages")?;

    let mut members = packages
        .iter()
        .filter(|package| {
            package
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| workspace_ids.contains(id))
        })
        .map(|package| {
            package
                .get("manifest_path")
                .and_then(Value::as_str)
                .map(PathBuf::from)
                .context("Cargo metadata package omitted manifest_path")
                .map(|manifest| {
                    manifest
                        .parent()
                        .map(Path::to_path_buf)
                        .context("Cargo manifest path has no parent")
                })
                .and_then(std::convert::identity)
        })
        .collect::<Result<Vec<_>>>()?;
    members.sort();
    Ok(members)
}

/// Checks one package's reachable module tree and reports its unreferenced sources.
fn check_crate_sources(crate_dir: &Path, failures: &mut Vec<String>) -> Result<()> {
    let crate_dir = crate_dir
        .canonicalize()
        .with_context(|| format!("failed to resolve workspace member {}", crate_dir.display()))?;
    let src_dir = crate_dir.join("src");
    if !src_dir.exists() {
        return Ok(());
    }
    let (reachable, resolution_errors) = walk_reachable_sources(&crate_dir)?;
    let crate_name = crate_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<crate>");
    failures.extend(
        resolution_errors
            .into_iter()
            .map(|error| format!("{crate_name}: {error}")),
    );

    let mut all_sources = BTreeSet::new();
    for entry in WalkDir::new(&src_dir) {
        let entry = entry.with_context(|| format!("failed to traverse {}", src_dir.display()))?;
        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "rs")
            && entry.path().is_file()
        {
            all_sources.insert(
                entry
                    .path()
                    .canonicalize()
                    .with_context(|| format!("failed to resolve {}", entry.path().display()))?,
            );
        }
    }
    let mut orphans = all_sources
        .difference(&reachable)
        .cloned()
        .collect::<Vec<_>>();
    orphans.sort();
    for orphan in orphans {
        let relative = orphan.strip_prefix(&crate_dir).with_context(|| {
            format!(
                "resolved source {} is outside crate {}",
                orphan.display(),
                crate_dir.display()
            )
        })?;
        failures.push(format!(
            "{crate_name}: orphan source {}",
            relative.display()
        ));
    }
    Ok(())
}

/// Returns the compiled module-declaration pattern used by the source walker.
fn module_declaration_regex() -> Result<&'static Regex> {
    static MODULE_DECLARATION: OnceLock<std::result::Result<Regex, regex::Error>> = OnceLock::new();
    match MODULE_DECLARATION.get_or_init(|| {
        Regex::new(r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;")
    }) {
        Ok(regex) => Ok(regex),
        Err(error) => Err(anyhow::anyhow!(error.to_string())),
    }
}

/// Finds library, binary, and nested binary crate roots under `src`.
fn crate_roots(src_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut roots = Vec::new();
    for entry_name in ["lib.rs", "main.rs"] {
        let entry = src_dir.join(entry_name);
        if entry.exists() {
            roots.push(entry);
        }
    }
    let bin_dir = src_dir.join("bin");
    if !bin_dir.exists() {
        return Ok(roots);
    }
    let mut children = fs::read_dir(&bin_dir)
        .with_context(|| format!("failed to read {}", bin_dir.display()))?
        .collect::<std::io::Result<Vec<_>>>()?;
    children.sort_by_key(std::fs::DirEntry::file_name);
    for child in children {
        let path = child.path();
        if path.is_file() && path.extension().is_some_and(|extension| extension == "rs") {
            roots.push(path);
        } else if path.is_dir() {
            let main_rs = path.join("main.rs");
            if main_rs.exists() {
                roots.push(main_rs);
            }
        }
    }
    Ok(roots)
}

/// Returns the directory Rust uses to resolve children of a source module.
fn module_search_dir(module_file: &Path, is_root: bool) -> PathBuf {
    let parent = module_file.parent().unwrap_or_else(|| Path::new("."));
    if is_root || module_file.file_name().is_some_and(|name| name == "mod.rs") {
        parent.to_path_buf()
    } else {
        parent.join(module_file.file_stem().unwrap_or_default())
    }
}

/// Resolves one `mod name;` declaration using flat-file-first Rust lookup.
fn resolve_child_module(module_file: &Path, module_name: &str, is_root: bool) -> Option<PathBuf> {
    let base_dir = module_search_dir(module_file, is_root);
    let flat_file = base_dir.join(format!("{module_name}.rs"));
    let nested_file = base_dir.join(module_name).join("mod.rs");
    if flat_file.exists() {
        Some(flat_file)
    } else if nested_file.exists() {
        Some(nested_file)
    } else {
        None
    }
}

/// Walks declared file modules and returns reachable canonical sources plus unresolved uses.
fn walk_reachable_sources(crate_dir: &Path) -> Result<(BTreeSet<PathBuf>, Vec<String>)> {
    let crate_dir = crate_dir
        .canonicalize()
        .with_context(|| format!("failed to resolve crate directory {}", crate_dir.display()))?;
    let src_dir = crate_dir.join("src");
    let mut reachable = BTreeSet::new();
    let mut errors = Vec::new();
    let mut pending = crate_roots(&src_dir)?
        .into_iter()
        .map(|path| (path, true))
        .collect::<Vec<_>>();

    while let Some((module_file, is_root)) = pending.pop() {
        let module_file = module_file
            .canonicalize()
            .with_context(|| format!("failed to resolve source {}", module_file.display()))?;
        if !reachable.insert(module_file.clone()) {
            continue;
        }
        let content = fs::read_to_string(&module_file)
            .with_context(|| format!("failed to read Rust source {}", module_file.display()))?;
        for (index, line) in content.lines().enumerate() {
            let Some(captures) = module_declaration_regex()?.captures(line) else {
                continue;
            };
            let module_name = captures.get(1).map_or("", |value| value.as_str());
            if let Some(child) = resolve_child_module(&module_file, module_name, is_root) {
                pending.push((child, false));
                continue;
            }
            let base_dir = module_search_dir(&module_file, is_root);
            let flat_file = base_dir.join(format!("{module_name}.rs"));
            let nested_file = base_dir.join(module_name).join("mod.rs");
            let relative_file = module_file.strip_prefix(&crate_dir).with_context(|| {
                format!(
                    "module source {} is outside crate {}",
                    module_file.display(),
                    crate_dir.display()
                )
            })?;
            let relative_flat = flat_file.strip_prefix(&crate_dir).unwrap_or(&flat_file);
            let relative_nested = nested_file.strip_prefix(&crate_dir).unwrap_or(&nested_file);
            errors.push(format!(
                "{}:{} declares module `{module_name}` but neither {} nor {} exists",
                relative_file.display(),
                index + 1,
                relative_flat.display(),
                relative_nested.display()
            ));
        }
    }
    Ok((reachable, errors))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn root_children_and_nested_modules_resolve_with_rust_paths() -> Result<()> {
        let temp = tempdir()?;
        let crate_dir = temp.path();
        fs::create_dir_all(crate_dir.join("src/flat"))?;
        fs::create_dir_all(crate_dir.join("src/nested"))?;
        fs::write(crate_dir.join("src/lib.rs"), "mod flat;\npub mod nested;\n")?;
        fs::write(crate_dir.join("src/flat.rs"), "mod inner;\n")?;
        fs::write(crate_dir.join("src/flat/inner.rs"), "// reachable\n")?;
        fs::write(crate_dir.join("src/nested/mod.rs"), "// reachable\n")?;
        fs::write(crate_dir.join("src/unused.rs"), "// orphan\n")?;
        let (reachable, errors) =
            walk_reachable_sources(crate_dir).unwrap_or_else(|error| panic!("{error:#}"));
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(reachable.len(), 4);
        assert!(reachable.iter().any(|path| path.ends_with("inner.rs")));
        Ok(())
    }

    #[test]
    fn missing_child_reports_module_line_and_both_candidate_paths() -> Result<()> {
        let temp = tempdir()?;
        let crate_dir = temp.path();
        fs::create_dir_all(crate_dir.join("src"))?;
        fs::write(crate_dir.join("src/lib.rs"), "// line one\nmod missing;\n")?;
        let (_, errors) = walk_reachable_sources(crate_dir)?;
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("src\\lib.rs:2") || errors[0].contains("src/lib.rs:2"));
        assert!(errors[0].contains("missing.rs"));
        assert!(errors[0].contains("missing\\mod.rs") || errors[0].contains("missing/mod.rs"));
        Ok(())
    }

    #[test]
    fn crate_roots_include_direct_bin_files_and_nested_bin_mains() -> Result<()> {
        let temp = tempdir()?;
        let src = temp.path().join("src");
        fs::create_dir_all(src.join("bin/tool"))?;
        fs::write(src.join("lib.rs"), "")?;
        fs::write(src.join("bin/direct.rs"), "")?;
        fs::write(src.join("bin/tool/main.rs"), "")?;
        fs::write(src.join("bin/ignored.md"), "")?;
        let roots = crate_roots(&src)?;
        assert_eq!(roots.len(), 3);
        assert!(roots.iter().any(|path| path.ends_with("direct.rs")));
        assert!(roots.iter().any(|path| path.ends_with("tool/main.rs")));
        Ok(())
    }
}
