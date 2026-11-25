//! Cross-file consistency checks for repository configuration.
//!
//! TODO Reviewed ??-??-2026.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitCode;

use anyhow::Context as _;
use anyhow::Result;
use regex::Regex;
use serde_json::Value as JsonValue;
use toml::Value as TomlValue;
use walkdir::WalkDir;

/// Runs the repository configuration checks and prints their diagnostics.
///
/// Returns success, failure, or the original status 2 for a missing repository root.
///
/// # Errors
///
/// Returns an error when a required file cannot be read or traversed, or when a
/// configuration file outside the script's explicitly handled parse paths is invalid.
pub fn execute(root: &Path) -> Result<ExitCode> {
    if !root.is_dir() {
        eprintln!("repository root does not exist: {}", root.display());
        return Ok(ExitCode::from(2));
    }
    let root = root
        .canonicalize()
        .with_context(|| format!("failed to resolve repository root {}", root.display()))?;
    let errors = check(&root)?;
    if errors.is_empty() {
        println!(
            "Configuration paths and pins are consistent: {}",
            root.display()
        );
        Ok(ExitCode::SUCCESS)
    } else {
        for error in errors {
            eprintln!("ERROR: {error}");
        }
        Ok(ExitCode::FAILURE)
    }
}

/// Collects every configuration consistency diagnostic for one repository root.
fn check(root: &Path) -> Result<Vec<String>> {
    let mut errors = Vec::new();
    check_workspace(root, &mut errors)?;
    check_dependabot_paths(root, &mut errors)?;
    check_editor_and_git(root, &mut errors)?;
    check_toolchain_pins(root, &mut errors)?;
    check_fixture_exceptions(root, &mut errors)?;
    Ok(errors)
}

/// Reads and parses a TOML file with path context.
fn read_toml(path: &Path) -> Result<TomlValue> {
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("failed to parse TOML in {}", path.display()))
}

/// Extracts package-ecosystem and directory pairs from the small Dependabot YAML
/// structure used by the repository policy.
fn parse_dependabot_paths(text: &str) -> Result<Vec<(String, String)>> {
    let ecosystem_regex = Regex::new(r"^\s*-\s+package-ecosystem:\s*([^\s#]+)")?;
    let directory_regex = Regex::new(r"^\s+directory:\s*(.+?)\s*(?:#.*)?$")?;
    let directories_regex = Regex::new(r"^(\s*)directories:\s*(?:#.*)?$")?;
    let directory_item_regex = Regex::new(r"^(\s*)-\s+([^\s#]+)")?;
    let mut entries = Vec::new();
    let mut current: Option<String> = None;
    let mut in_directories = false;
    let mut directories_indent = 0;

    for line in text.lines() {
        if let Some(captures) = ecosystem_regex.captures(line) {
            current = captures
                .get(1)
                .map(|value| value.as_str().trim_matches(['"', '\'']).to_owned());
            in_directories = false;
            directories_indent = 0;
            continue;
        }
        let Some(ecosystem) = current.as_ref() else {
            continue;
        };

        if let Some(captures) = directory_regex.captures(line) {
            if let Some(directory) = captures.get(1) {
                entries.push((
                    ecosystem.clone(),
                    directory.as_str().trim_matches(['"', '\'']).to_owned(),
                ));
            }
            in_directories = false;
            continue;
        }
        if let Some(captures) = directories_regex.captures(line) {
            in_directories = true;
            directories_indent = captures.get(1).map_or(0, |indent| indent.as_str().len());
            continue;
        }
        if in_directories
            && let Some(captures) = directory_item_regex.captures(line)
            && captures.get(1).map_or(0, |value| value.as_str().len()) > directories_indent
            && let Some(directory) = captures.get(2)
        {
            entries.push((
                ecosystem.clone(),
                directory.as_str().trim_matches(['"', '\'']).to_owned(),
            ));
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if in_directories && !line.trim().is_empty() && indent <= directories_indent {
            in_directories = false;
        }
    }
    Ok(entries)
}

/// Validates configured Dependabot package paths and ecosystem-specific manifests.
fn check_dependabot_paths(root: &Path, errors: &mut Vec<String>) -> Result<()> {
    let path = root.join(".github/dependabot.yml");
    if !path.is_file() {
        return Ok(());
    }
    let text =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let entries = parse_dependabot_paths(&text)?;
    if entries.is_empty() {
        errors.push(".github/dependabot.yml has no package directory entries".to_owned());
        return Ok(());
    }

    for (ecosystem, configured) in entries {
        let path_parts = Path::new(&configured).components().collect::<Vec<_>>();
        if !configured.starts_with('/') || path_parts.iter().any(|part| part.as_os_str() == "..") {
            errors.push(format!(
                "Dependabot {ecosystem} path must be absolute and repository-relative: {configured}"
            ));
            continue;
        }
        let directory = root.join(
            configured
                .trim_start_matches('/')
                .replace('/', std::path::MAIN_SEPARATOR_STR),
        );
        if !directory.is_dir() {
            errors.push(format!(
                "Dependabot {ecosystem} path does not exist: {configured}"
            ));
            continue;
        }

        let has_config = match ecosystem.as_str() {
            "cargo" => directory.join("Cargo.toml").is_file(),
            "docker" => directory
                .read_dir()
                .with_context(|| format!("failed to read {}", directory.display()))?
                .filter_map(std::result::Result::ok)
                .any(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with("Dockerfile")
                }),
            "docker-compose" => directory
                .read_dir()
                .with_context(|| format!("failed to read {}", directory.display()))?
                .filter_map(std::result::Result::ok)
                .any(|entry| {
                    let name = entry.file_name();
                    let name = name.to_string_lossy();
                    entry.path().is_file()
                        && (name.starts_with("compose") || name.starts_with("docker-compose"))
                        && matches!(
                            entry.path().extension().and_then(|value| value.to_str()),
                            Some("yaml" | "yml")
                        )
                }),
            "devcontainers" => {
                directory.join(".devcontainer.json").is_file()
                    || directory.join(".devcontainer/devcontainer.json").is_file()
            }
            "github-actions" => {
                directory.join(".github").is_dir()
                    || directory.join("action.yml").is_file()
                    || directory.join("action.yaml").is_file()
            }
            "rust-toolchain" => directory.join("rust-toolchain.toml").is_file(),
            "npm" => directory.join("package.json").is_file(),
            _ => true,
        };
        if !has_config {
            errors.push(format!(
                "Dependabot {ecosystem} path has no matching manifest or config: {configured}"
            ));
        }
    }
    Ok(())
}

/// Expands one Cargo member glob using path components beneath the workspace root.
fn member_matches(root: &Path, pattern: &str) -> Result<Vec<PathBuf>> {
    let normalized = pattern.replace('\\', "/");
    let parts = normalized
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let mut matches = Vec::new();
    expand_member_pattern(&parts, 0, root.to_path_buf(), &mut matches)?;
    matches.sort();
    matches.dedup();
    Ok(matches)
}

/// Recursively expands a single Cargo member glob component.
fn expand_member_pattern(
    parts: &[&str],
    index: usize,
    current: PathBuf,
    matches: &mut Vec<PathBuf>,
) -> Result<()> {
    if index >= parts.len() {
        if current.exists() {
            matches.push(current);
        }
        return Ok(());
    }

    let Some(part) = parts.get(index).copied() else {
        return Ok(());
    };
    if part == "**" {
        expand_member_pattern(parts, index + 1, current.clone(), matches)?;
        if current.is_dir() {
            for entry in fs::read_dir(&current)
                .with_context(|| format!("failed to read {}", current.display()))?
            {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    expand_member_pattern(parts, index, entry.path(), matches)?;
                }
            }
        }
        return Ok(());
    }

    if has_glob_meta(part) {
        if current.is_dir() {
            let matcher = glob_component_regex(part)?;
            for entry in fs::read_dir(&current)
                .with_context(|| format!("failed to read {}", current.display()))?
            {
                let entry = entry?;
                if matcher.is_match(&entry.file_name().to_string_lossy()) {
                    expand_member_pattern(parts, index + 1, entry.path(), matches)?;
                }
            }
        }
    } else {
        let next = if Path::new(part).is_absolute() {
            PathBuf::from(part)
        } else {
            current.join(part)
        };
        expand_member_pattern(parts, index + 1, next, matches)?;
    }
    Ok(())
}

/// Returns whether a path component uses supported Cargo glob syntax.
fn has_glob_meta(part: &str) -> bool {
    part.contains(['*', '?', '['])
}

/// Converts one Cargo glob path component into an anchored regular expression.
fn glob_component_regex(pattern: &str) -> Result<Regex> {
    let mut regex = String::from("^");
    let mut chars = pattern.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '*' => regex.push_str(".*"),
            '?' => regex.push('.'),
            '[' => {
                regex.push('[');
                if chars.peek() == Some(&'!') {
                    chars.next();
                    regex.push('^');
                } else if chars.peek() == Some(&'^') {
                    regex.push('\\');
                }
                let mut closed = false;
                for class_character in chars.by_ref() {
                    if class_character == ']' {
                        closed = true;
                        regex.push(']');
                        break;
                    }
                    if class_character == '\\' {
                        regex.push_str("\\\\");
                    } else {
                        regex.push(class_character);
                    }
                }
                if !closed {
                    regex.push_str("\\[");
                }
            }
            other => regex.push_str(&regex::escape(&other.to_string())),
        }
    }
    regex.push('$');
    Regex::new(&regex)
        .with_context(|| format!("invalid Cargo workspace member pattern {pattern:?}"))
}

/// Checks workspace member paths and compares effective package MSRVs with the workspace pin.
fn check_workspace(root: &Path, errors: &mut Vec<String>) -> Result<()> {
    let manifest = root.join("Cargo.toml");
    if !manifest.is_file() {
        return Ok(());
    }
    let cargo_toml = match read_toml(&manifest) {
        Ok(value) => value,
        Err(error) => {
            errors.push(format!("Cargo.toml is not valid TOML: {error:#}"));
            return Ok(());
        }
    };
    let workspace = cargo_toml.get("workspace").and_then(TomlValue::as_table);
    if let Some(members) = workspace
        .and_then(|table| table.get("members"))
        .and_then(TomlValue::as_array)
    {
        for member in members.iter().filter_map(TomlValue::as_str) {
            let matches = member_matches(root, member)?;
            if matches.is_empty() {
                errors.push(format!(
                    "Cargo workspace member pattern matches no paths: {member}"
                ));
            }
            for matched in matches {
                if matched.is_dir() && !matched.join("Cargo.toml").is_file() {
                    let relative = matched.strip_prefix(root).unwrap_or(&matched);
                    errors.push(format!(
                        "Cargo workspace member has no Cargo.toml: {}",
                        relative.display()
                    ));
                }
            }
        }
    }

    let result = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1", "--locked"])
        .current_dir(root)
        .output()
        .context("failed to run cargo metadata")?;
    if !result.status.success() {
        let detail = String::from_utf8_lossy(&result.stderr).trim().to_owned();
        let detail = if detail.is_empty() {
            String::from_utf8_lossy(&result.stdout).trim().to_owned()
        } else {
            detail
        };
        errors.push(format!("Cargo metadata rejected the workspace: {detail}"));
        return Ok(());
    }
    let metadata: JsonValue = match serde_json::from_slice(&result.stdout) {
        Ok(value) => value,
        Err(error) => {
            errors.push(format!("Cargo metadata did not emit valid JSON: {error}"));
            return Ok(());
        }
    };

    let expected_msrv = workspace
        .and_then(|table| table.get("package"))
        .and_then(TomlValue::as_table)
        .and_then(|table| table.get("rust-version"))
        .and_then(TomlValue::as_str);
    let workspace_ids = metadata
        .get("workspace_members")
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(JsonValue::as_str)
        .collect::<BTreeSet<_>>();
    let packages = metadata
        .get("packages")
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .filter(|package| {
            package
                .get("id")
                .and_then(JsonValue::as_str)
                .is_some_and(|id| workspace_ids.contains(id))
        });

    let mut package_count = 0_usize;
    for package in packages {
        package_count += 1_usize;
        let actual = package.get("rust_version").and_then(JsonValue::as_str);
        if let Some(expected) = expected_msrv
            && actual != Some(expected)
        {
            errors.push(format!(
                "{}: effective rust-version {:?} does not match workspace.package.rust-version {:?}",
                package.get("manifest_path").and_then(JsonValue::as_str).unwrap_or("<unknown manifest>"),
                actual,
                expected
            ));
        }
    }
    if expected_msrv.is_some() && package_count == 0_usize {
        errors.push("Cargo metadata returned no packages for the root workspace".to_owned());
    }
    Ok(())
}

/// Reads simple `KEY=VALUE` entries, ignoring blank lines and comments.
fn read_env(path: &Path) -> Result<BTreeMap<String, String>> {
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut values = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            anyhow::bail!("{}:{}: expected KEY=VALUE", path.display(), index + 1);
        };
        let key = key.trim();
        if key.is_empty() {
            anyhow::bail!("{}:{}: expected KEY=VALUE", path.display(), index + 1);
        }
        values.insert(
            key.to_owned(),
            value.trim().trim_matches(['"', '\'']).to_owned(),
        );
    }
    Ok(values)
}

/// Checks repository-wide editor defaults and Git line-ending attributes.
fn check_editor_and_git(root: &Path, errors: &mut Vec<String>) -> Result<()> {
    let editorconfig = root.join(".editorconfig");
    if editorconfig.is_file() {
        let text = fs::read_to_string(&editorconfig)
            .with_context(|| format!("failed to read {}", editorconfig.display()))?;
        let mut global_values = BTreeMap::new();
        let mut in_global = false;
        for line in text.lines().map(str::trim) {
            if line.is_empty() || line.starts_with(['#', ';']) {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                in_global = line == "[*]";
            } else if in_global && let Some((key, value)) = line.split_once('=') {
                global_values.insert(key.trim().to_lowercase(), value.trim().to_lowercase());
            }
        }
        for (key, expected) in [
            ("charset", "utf-8"),
            ("end_of_line", "lf"),
            ("insert_final_newline", "true"),
        ] {
            if global_values.get(key).map(String::as_str) != Some(expected) {
                errors.push(format!(".editorconfig [*] must set {key}={expected}"));
            }
        }
    }

    let attributes = root.join(".gitattributes");
    if attributes.is_file() {
        let text = fs::read_to_string(&attributes)
            .with_context(|| format!("failed to read {}", attributes.display()))?;
        let has_default = text
            .lines()
            .map(str::trim)
            .any(|line| line == "* text=auto eol=lf");
        if !has_default {
            errors.push(".gitattributes must set '* text=auto eol=lf'".to_owned());
        }
    }
    Ok(())
}

/// Ensures container toolchain pins are valid and consumed by relevant configuration.
fn check_toolchain_pins(root: &Path, errors: &mut Vec<String>) -> Result<()> {
    let env_path = root.join(".devcontainer/.env");
    if !env_path.is_file() {
        return Ok(());
    }
    let values = match read_env(&env_path) {
        Ok(values) => values,
        Err(error) => {
            errors.push(format!(".devcontainer/.env is invalid: {error:#}"));
            return Ok(());
        }
    };
    check_rustfmt_pin(root, &values, errors);
    check_container_rust_pin(root, &values, errors);
    let consumer_text = pin_consumer_text(root)?;
    check_pin_references(&values, &consumer_text, errors)?;
    Ok(())
}

/// Checks the Rustfmt toolchain pin against `rustfmt-toolchain.toml`.
fn check_rustfmt_pin(root: &Path, values: &BTreeMap<String, String>, errors: &mut Vec<String>) {
    let rustfmt_path = root.join("rustfmt-toolchain.toml");
    if rustfmt_path.is_file()
        && let Some(pin) = values.get("RUSTFMT_TOOLCHAIN")
    {
        match read_toml(&rustfmt_path) {
            Ok(toml) => {
                if let Some(channel) = toml
                    .get("toolchain")
                    .and_then(|value| value.get("channel"))
                    .and_then(TomlValue::as_str)
                    && pin != channel
                {
                    errors.push("RUSTFMT_TOOLCHAIN in .devcontainer/.env disagrees with rustfmt-toolchain.toml".to_owned());
                }
            }
            Err(error) => errors.push(format!("rustfmt-toolchain.toml is invalid: {error:#}")),
        }
    }
}

/// Checks the container Rust pin against `rust-toolchain.toml`.
fn check_container_rust_pin(
    root: &Path,
    values: &BTreeMap<String, String>,
    errors: &mut Vec<String>,
) {
    let rust_path = root.join("rust-toolchain.toml");
    let image_pin = values
        .get("RUST_VERSION")
        .or_else(|| values.get("RUST_IMAGE_LABEL"));
    if rust_path.is_file()
        && let Some(image_pin) = image_pin
    {
        match read_toml(&rust_path) {
            Ok(toml) => {
                if let Some(stable) = toml
                    .get("toolchain")
                    .and_then(|value| value.get("channel"))
                    .and_then(TomlValue::as_str)
                    && image_pin != stable
                    && !image_pin.starts_with(&format!("{stable}-"))
                {
                    errors.push(format!("container Rust pin {image_pin:?} disagrees with rust-toolchain channel {stable:?}"));
                }
            }
            Err(error) => errors.push(format!("rust-toolchain.toml is invalid: {error:#}")),
        }
    }
}

/// Reads the selected configuration and tooling files that may consume version pins.
fn pin_consumer_text(root: &Path) -> Result<String> {
    let mut consumers = Vec::new();
    for directory in
        [".devcontainer", ".github", "scripts", "templates"].map(|name| root.join(name))
    {
        if !directory.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&directory) {
            let entry =
                entry.with_context(|| format!("failed to traverse {}", directory.display()))?;
            if !entry.path().is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy();
            let extension = entry
                .path()
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if name == "Dockerfile"
                || name.starts_with("Dockerfile.")
                || [
                    "yml", "yaml", "toml", "sh", "ps1", "json", "jsonc", "md", "rs",
                ]
                .contains(&extension.as_str())
            {
                consumers.push(entry.into_path());
            }
        }
    }
    let mut consumer_text = String::new();
    for path in consumers {
        let bytes =
            fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
        consumer_text.push_str(&String::from_utf8_lossy(&bytes));
        consumer_text.push('\n');
    }
    Ok(consumer_text)
}

/// Reports version-like environment pins absent from the relevant consumer files.
fn check_pin_references(
    values: &BTreeMap<String, String>,
    consumer_text: &str,
    errors: &mut Vec<String>,
) -> Result<()> {
    for key in values.keys().filter(|key| {
        key.ends_with("_VERSION")
            || key.ends_with("_TOOLCHAIN")
            || ["RUST_VERSION", "RUST_IMAGE_LABEL"].contains(&key.as_str())
    }) {
        let key_regex = Regex::new(&format!(r"\b{}\b", regex::escape(key)))?;
        if !key_regex.is_match(consumer_text) {
            errors.push(format!(
                ".devcontainer/.env pin {key} is not referenced by configuration or tooling"
            ));
        }
    }
    Ok(())
}

/// Checks that Git line-ending exceptions are documented, real, and excluded from hooks.
fn check_fixture_exceptions(root: &Path, errors: &mut Vec<String>) -> Result<()> {
    let policy_path = root.join(".config/config-drift.toml");
    let policy = if policy_path.is_file() {
        read_toml(&policy_path)?
    } else {
        TomlValue::Table(toml::map::Map::new())
    };
    let mut exceptions = BTreeMap::new();
    if let Some(items) = policy
        .get("fixture_exceptions")
        .and_then(TomlValue::as_array)
    {
        for item in items {
            if let Some(path) = item.get("path").and_then(TomlValue::as_str) {
                exceptions.insert(path.to_owned(), item);
            }
        }
    }

    let explicit_paths = git_line_ending_exceptions(root)?;
    let mut all_exception_paths = explicit_paths;
    all_exception_paths.extend(exceptions.keys().cloned());
    for path in all_exception_paths {
        if let Some(entry) = exceptions.get(&path) {
            check_fixture_exception(root, &path, entry, errors)?;
        } else {
            errors.push(format!(
                "intentional fixture exception needs a reason in .config/config-drift.toml: {path}"
            ));
        }
    }
    Ok(())
}

/// Collects literal `.gitattributes` paths that disable text or EOL conversion.
fn git_line_ending_exceptions(root: &Path) -> Result<BTreeSet<String>> {
    let mut explicit_paths = BTreeSet::new();
    let attributes = root.join(".gitattributes");
    if attributes.is_file() {
        let text = fs::read_to_string(&attributes)
            .with_context(|| format!("failed to read {}", attributes.display()))?;
        for line in text.lines() {
            let mut fields = line.split_whitespace();
            let Some(pattern) = fields.next() else {
                continue;
            };
            if !fields.any(|field| matches!(field, "-text" | "-eol")) {
                continue;
            }
            if !pattern
                .chars()
                .any(|character| matches!(character, '*' | '?' | '[' | ']'))
            {
                explicit_paths.insert(pattern.replace('\\', "/"));
            }
        }
    }
    Ok(explicit_paths)
}

/// Validates one documented fixture exception and its declared newline format.
fn check_fixture_exception(
    root: &Path,
    path: &str,
    entry: &TomlValue,
    errors: &mut Vec<String>,
) -> Result<()> {
    if entry
        .get("reason")
        .and_then(TomlValue::as_str)
        .is_none_or(|reason| reason.trim().is_empty())
    {
        errors.push(format!(
            "intentional fixture exception needs a reason in .config/config-drift.toml: {path}"
        ));
        return Ok(());
    }
    let fixture = root.join(path);
    if !fixture.is_file() {
        errors.push(format!(
            "configured fixture exception does not exist: {path}"
        ));
        return Ok(());
    }
    if entry.get("newline").and_then(TomlValue::as_str) == Some("crlf") {
        let data =
            fs::read(&fixture).with_context(|| format!("failed to read {}", fixture.display()))?;
        let has_lf = data.contains(&b'\n');
        let has_bare_lf = data
            .windows(2)
            .any(|pair| matches!(pair, [previous, next] if *next == b'\n' && *previous != b'\r'));
        if !has_lf || has_bare_lf {
            errors.push(format!(
                "fixture declared CRLF does not contain only CRLF line endings: {path}"
            ));
        }
    }
    check_prek_exclusions(root, path, errors);
    Ok(())
}

/// Verifies formatting hooks exclude each intentional fixture exception.
fn check_prek_exclusions(root: &Path, path: &str, errors: &mut Vec<String>) {
    let prek_path = root.join("prek.toml");
    if !prek_path.is_file() {
        return;
    }
    let hooks = match read_toml(&prek_path) {
        Ok(value) => value,
        Err(error) => {
            errors.push(format!("prek.toml is invalid: {error:#}"));
            return;
        }
    };
    let repos = hooks
        .get("repos")
        .and_then(TomlValue::as_array)
        .into_iter()
        .flatten();
    for hook in repos.flat_map(|repo| {
        repo.get("hooks")
            .and_then(TomlValue::as_array)
            .into_iter()
            .flatten()
    }) {
        let id = hook.get("id").and_then(TomlValue::as_str).unwrap_or("");
        if !["end-of-file-fixer", "mixed-line-ending"].contains(&id) {
            continue;
        }
        let pattern = hook
            .get("exclude")
            .and_then(TomlValue::as_str)
            .unwrap_or("");
        let excluded = if pattern.is_empty() {
            false
        } else {
            match Regex::new(pattern) {
                Ok(pattern) => pattern.is_match(path),
                Err(error) => {
                    errors.push(format!(
                        "prek hook {id} has an invalid exclude regex: {error}"
                    ));
                    continue;
                }
            }
        };
        if !excluded {
            errors.push(format!(
                "prek hook {id} must exclude intentional fixture {path}"
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn template_dockerfiles_count_as_pin_consumers() -> Result<()> {
        let temp = tempdir()?;
        let root = temp.path();
        let devcontainer = root.join(".devcontainer");
        fs::create_dir_all(&devcontainer)?;
        fs::write(
            devcontainer.join(".env"),
            "CARGO_CHEF_VERSION=0.1.78\nUNUSED_VERSION=1\n",
        )?;
        let template = root.join("templates/dockerfile");
        fs::create_dir_all(&template)?;
        fs::write(template.join("Dockerfile"), "ARG CARGO_CHEF_VERSION\n")?;
        let mut errors = Vec::new();
        check_toolchain_pins(root, &mut errors)?;
        assert_eq!(
            errors,
            [".devcontainer/.env pin UNUSED_VERSION is not referenced by configuration or tooling"]
        );
        Ok(())
    }

    #[test]
    fn rust_image_pin_matches_full_toolchain_version() -> Result<()> {
        let temp = tempdir()?;
        let root = temp.path();
        let devcontainer = root.join(".devcontainer");
        fs::create_dir_all(&devcontainer)?;
        fs::write(
            root.join("rust-toolchain.toml"),
            "[toolchain]\nchannel = \"1.100.1\"\n",
        )?;
        fs::write(
            devcontainer.join("compose.yml"),
            "image: rust:${RUST_VERSION}\n",
        )?;
        for (pin, should_pass) in [
            ("1.100.1", true),
            ("1.100.1-slim", true),
            ("1.100.10", false),
        ] {
            fs::write(devcontainer.join(".env"), format!("RUST_VERSION={pin}\n"))?;
            let mut errors = Vec::new();
            check_toolchain_pins(root, &mut errors)?;
            assert_eq!(errors.is_empty(), should_pass, "pin {pin}: {errors:?}");
            if !should_pass {
                assert!(errors[0].contains("disagrees with rust-toolchain"));
            }
        }
        Ok(())
    }

    #[test]
    fn dependabot_paths_extract_single_and_multiple_directories() -> Result<()> {
        let parsed = parse_dependabot_paths(
            "updates:\n  - package-ecosystem: cargo\n    directory: /tools/cli\n  - package-ecosystem: docker\n    directories:\n      - /images/a\n      - '/images/b'\n",
        )?;
        assert_eq!(
            parsed,
            [
                ("cargo".to_owned(), "/tools/cli".to_owned()),
                ("docker".to_owned(), "/images/a".to_owned()),
                ("docker".to_owned(), "/images/b".to_owned())
            ]
        );
        Ok(())
    }

    #[test]
    fn workspace_member_patterns_expand_globs() -> Result<()> {
        let temp = tempdir()?;
        fs::create_dir_all(temp.path().join("crates/alpha"))?;
        fs::create_dir_all(temp.path().join("crates/beta"))?;
        let matches = member_matches(temp.path(), "crates/*")?;
        assert_eq!(matches.len(), 2);
        assert!(matches.iter().any(|path| path.ends_with("alpha")));
        assert!(matches.iter().any(|path| path.ends_with("beta")));
        Ok(())
    }

    #[test]
    fn fixture_exceptions_require_policy_and_real_crlf_files() -> Result<()> {
        let temp = tempdir()?;
        let root = temp.path();
        fs::create_dir_all(root.join(".config"))?;
        fs::create_dir_all(root.join("fixtures"))?;
        fs::write(root.join("fixtures/data.txt"), b"first\r\nsecond\r\n")?;
        fs::write(
            root.join(".gitattributes"),
            "* text=auto eol=lf\nfixtures/data.txt -text\n",
        )?;
        fs::write(
            root.join(".config/config-drift.toml"),
            "[[fixture_exceptions]]\npath = \"fixtures/data.txt\"\nreason = \"Preserved bytes\"\nnewline = \"crlf\"\n",
        )?;
        fs::write(
            root.join("prek.toml"),
            "[[repos.hooks]]\nid = \"end-of-file-fixer\"\nexclude = 'fixtures/data\\.txt'\n",
        )?;
        let mut errors = Vec::new();
        check_fixture_exceptions(root, &mut errors)?;
        assert!(errors.is_empty(), "{errors:?}");
        fs::write(root.join("fixtures/data.txt"), b"first\r\nsecond\n")?;
        check_fixture_exceptions(root, &mut errors)?;
        assert_eq!(
            errors,
            ["fixture declared CRLF does not contain only CRLF line endings: fixtures/data.txt"]
        );
        Ok(())
    }

    #[test]
    fn missing_fixture_policy_is_reported_for_git_exceptions() -> Result<()> {
        let temp = tempdir()?;
        fs::write(temp.path().join(".gitattributes"), "fixture.bin -text\n")?;
        let mut errors = Vec::new();
        check_fixture_exceptions(temp.path(), &mut errors)?;
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("needs a reason"));
        Ok(())
    }
}
