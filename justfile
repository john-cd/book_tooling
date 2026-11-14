# Purpose: Build, validate, and maintain this Rust workspace with its configured tooling environment.
# Usage: Run just --list from this directory to inspect the available recipes.

# TODO Reviewed ??-??-2026.

# Storage-root overrides (existing environment values win over platform defaults):
# CODE_REPOS_ROOT names the canonical checkout root for tools; CODE_TEMP_ROOT holds temporary files.
# CODE_PACKAGES_ROOT holds package-manager data (including Cargo, npm, and NuGet caches);
# CODE_TOOLS_ROOT holds portable tool installations.
# CODE_TARGET_ROOT stores compiled Cargo target artifacts; CODE_BUILD_ROOT stores intermediate
# Cargo build files (the Windows helper requires this root to be on NTFS).
# CODE_SCCACHE_ROOT selects the compiler cache; SCCACHE_DIR is accepted as a legacy fallback.
set shell := ["bash", "-uc"]

# Recipes run in Bash on every OS: Git Bash on Windows.
set windows-shell := ["C:/Program Files/Git/bin/bash.exe", "-uc"]
set quiet

export CARGO_TERM_QUIET := env("CARGO_TERM_QUIET", "true")

root := replace(source_directory(), "\\", "/")
rustfmt_toolchain_default := trim(replace_regex(read(join(root, "rustfmt-toolchain.toml")), '(?s).*channel\s*=\s*"([^"]+)".*', "$1"))
rustfmt_toolchain_override := env("RUSTFMT_TOOLCHAIN", "")
rustfmt_toolchain := if rustfmt_toolchain_override == "" { rustfmt_toolchain_default } else { rustfmt_toolchain_override }
export RUSTFMT_TOOLCHAIN := rustfmt_toolchain
checkout_name := file_name(root)
worktree_name := if file_name(parent_directory(root)) == ".worktrees" { checkout_name } else { "main" }
windows_cache_home := env("LOCALAPPDATA", join(home_directory(), ".cache"))
unix_cache_home := env("XDG_CACHE_HOME", join(home_directory(), ".cache"))
cache_home := if os() == "windows" { windows_cache_home } else { unix_cache_home }
code_repos_root := env("CODE_REPOS_ROOT", if os_family() == "windows" { "D:/code" } else { join(home_directory(), "code") })
code_temp_root := env("CODE_TEMP_ROOT", if os_family() == "windows" { if path_exists("D:/temp") == "true" { "D:/temp" } else { "C:/temp" } } else { env("TMPDIR", "/tmp") })
code_packages_root := env("CODE_PACKAGES_ROOT", if os_family() == "windows" { "D:/packages" } else { join(home_directory(), ".local/share") })
code_tools_root := env("CODE_TOOLS_ROOT", if os_family() == "windows" { "C:/tools" } else { join(home_directory(), ".local/tools") })
code_target_root := env("CODE_TARGET_ROOT", if os_family() == "windows" { "D:/target" } else { join(cache_home, "cargo-target") })
code_build_root := env("CODE_BUILD_ROOT", if os_family() == "windows" { "D:/code_build" } else { join(cache_home, "cargo-build") })
code_sccache_root := env("CODE_SCCACHE_ROOT", env("SCCACHE_DIR", if os_family() == "windows" { "D:/sccache" } else { join(cache_home, "sccache") }))
target_root := join(code_target_root, "book_tooling")
build_root := join(code_build_root, "book_tooling")
target_dir := join(code_target_root, "book_tooling", tooling_slot)
build_dir := join(code_build_root, "book_tooling", tooling_slot)
# Native Just paths; validate slots before exporting build directories.
tooling_slot := if worktree_name =~ '^(main|[a-j])$' { worktree_name } else { error("Invalid worktree slot: " + worktree_name) }
tooling_main := if file_name(parent_directory(root)) == ".worktrees" { parent_directory(parent_directory(root)) } else { root }
sccache_basedirs := trim(shell('''
set -euo pipefail
root=$1
printf '%s' "$root"
git -C "$root" -c core.fsmonitor=false worktree list --porcelain | while IFS= read -r line; do
    case "$line" in
        "worktree $root/.worktrees/"*)
            slot=${line#"worktree $root/.worktrees/"}
            if [[ $slot =~ ^[a-j]$ ]]; then
                printf '%s%s/.worktrees/%s' "$2" "$root" "$slot"
            fi
            ;;
    esac
done
''', replace(tooling_main, "\\", "/"), if os() == "windows" { ";" } else { ":" }))

export CARGO_TARGET_DIR := target_dir
export CARGO_BUILD_TARGET_DIR := target_dir
host_build_volume := if os() == "windows" { trim(shell('pwsh.exe -NoLogo -NoProfile -File "$1" -BuildRoot "$2"', join(root, "scripts/cargo-host.ps1"), code_build_root)) } else { "ok" }
export CARGO_BUILD_BUILD_DIR := if host_build_volume == "ok" { build_dir } else { error("Cargo build root must use NTFS or ReFS (Rust 1.100.0 or newer)") }
export RUSTC_WRAPPER := "sccache"
export CARGO_BUILD_RUSTC_WRAPPER := "sccache"
export CARGO_INCREMENTAL := "0"
export SCCACHE_DIR := code_sccache_root
export SCCACHE_CACHE_SIZE := "45G"
export SCCACHE_BASEDIRS := sccache_basedirs
export TEMP := if os() == "windows" { code_temp_root } else { env("TEMP", "/tmp") }
export TMP := if os() == "windows" { code_temp_root } else { env("TMP", "/tmp") }
export TMPDIR := if os() == "windows" { code_temp_root } else { env("TMPDIR", "/tmp") }
export DPRINT_CACHE_DIR := env("DPRINT_CACHE_DIR", join(cache_home, "dprint"))

# List the available recipes and task modules.
[no-exit-message]
_default:
    @just --list --unsorted --justfile {{ source_file() }}

# Check Rust formatting with the pinned formatter toolchain.
@format-check:
    echo "Checking Rust formatting with {{ rustfmt_toolchain }}"
    cargo +{{ rustfmt_toolchain }} fmt --all -- --check
    echo "Rust formatting check complete"

# Format all packages and their local path-based dependencies
@format-all:
    echo "Formatting all packages and their local path-based dependencies in $(pwd)"
    cargo +{{ rustfmt_toolchain }} fmt --all
    echo "Rust formatting complete"

# Check all code (all packages, all features)
@check:
    echo "Check all code (all packages, all features) in $(pwd)"
    cargo check --workspace --all-targets --locked --all-features

# `--all-targets` is equivalent to specifying `--lib --bins --tests --benches --examples`.

# Check that every Rust source file under src/ belongs to a compiled module tree.
@orphansources:
    echo "Check for orphan Rust source files in $(pwd)"
    python3 scripts/check_orphan_sources.py || python scripts/check_orphan_sources.py

# Check workspace, Dependabot, editing, toolchain and fixture configuration alignment.
@config-drift:
    echo "Check repository configuration consistency in $(pwd)"
    python3 scripts/check_config_drift.py || python scripts/check_config_drift.py

# Build all packages (all features)
@build:
    echo "Build all packages, all features in $(pwd)"
    cargo build --workspace --all-targets --locked --all-features --timings

# Scan all code for common mistakes (all packages, all features)
@lint-check:
    echo "Scan all code for common mistakes (all packages, all features) in $(pwd)"
    cargo clippy --workspace --all-targets --locked --all-features -- -D warnings

# Check dependency advisories, licenses, duplicate versions, and registry sources.
@deny:
    echo "Check dependency policy in $(pwd)"
    cargo deny --locked check

# Test all code in the workspace (incl. doctests) (all features)
@test-all:
    echo "Test all code in the workspace in $(pwd) (incl. doctests) (all features)"
    -cargo test --doc --workspace --quiet # nextest does not handle doctests
    cargo nextest run --workspace --all-targets --locked --no-fail-fast --status-level all --all-features --failure-output final

# Build and display the `cargo doc` documentation
@doc:
    echo "Build and display the `cargo doc` documentation for $(pwd)"
    cargo clean --doc
    cargo doc --no-deps --workspace --locked --document-private-items
    cd "{{ target_dir }}/doc/" ; (python3 -m http.server 9000 || python -m http.server 9000)

# Uniform task names (short aliases; old names kept as aliases).
alias f := format-all
alias format := format-all
alias fmtall := format-all
alias fa := format-all
alias fc := format-check
alias fmtcheck := format-check
alias c := check
alias cka := check
alias checkall := check
alias ca := check-all
alias b := build
alias ba := build
alias buildall := build
alias br := release
alias build-release := release
alias l := lint
alias lf := lint
alias lc := lint-check
alias clippyall := lint-check
alias la := lint-all
alias t := test-all
alias test := test-all
alias ta := test-all
alias nta := test-all
alias nextestall := test-all
alias d := doc
alias docall := doc
alias de := deny
alias da := deny
alias cl := clean
alias u := update
alias p := prep
alias hi := hooks-install
alias hr := hooks-run
alias osa := orphansources

# Lint with autofix: apply Clippy suggestions
@lint:
  cargo clippy --workspace --all-targets --locked --all-features --fix --allow-dirty --allow-staged -- -D warnings

# Lint with autofix, then lint Markdown.
lint-all: lint markdownlint

# Run all checks without modifying files: format, lint, check, test, deny.
check-all: format-check lint-check check test-all deny orphansources

# Run the local CI gates.
ci: check-all

# Before committing: format everything, lint with autofix, then run all checks.
prep: format-all lint check-all

# Remove build artifacts.
@clean:
  cargo clean

# Update Cargo.lock dependencies.
@update:
  cargo update

# Build release binaries into the configured external artifact directory.
@release:
    echo "Build the tools in $(pwd) in release mode and copy to {{ root }}/../bin"
    cargo +{{ rustfmt_toolchain }} build --bins --locked --all-features --release -Z unstable-options --artifact-dir "{{ build_dir }}/artifacts"

# Forward Cargo arguments with this checkout environment and central output paths.
[positional-arguments]
cargo +args:
    @printf '[cargo] start: %s\n' "$*"
    cargo "$@"
    @printf '[cargo] done: %s\n' "$*"

# Print the active worktree paths and compiler-cache configuration.
build-env:
    @printf 'WORKTREE_DIR=%s\n' {{quote(root)}}
    @printf 'WORKTREE_SLOT=%s\n' {{quote(tooling_slot)}}
    @printf 'REPO_ROOT=%s\n' {{quote(tooling_main)}}
    @printf 'CODE_TARGET_ROOT=%s\n' {{quote(code_target_root)}}
    @printf 'CODE_BUILD_ROOT=%s\n' {{quote(code_build_root)}}
    @printf 'CODE_SCCACHE_ROOT=%s\n' {{quote(code_sccache_root)}}
    @printf 'CODE_TEMP_ROOT=%s\n' {{quote(code_temp_root)}}
    @printf 'BRANCH=%s\n' "$(git -C {{quote(root)}} -c core.fsmonitor=false branch --show-current)"
    @if test -z "$(git -C {{quote(root)}} -c core.fsmonitor=false status --porcelain)"; then echo STATUS=clean; else echo STATUS=dirty; fi
    @printf 'CARGO_TARGET_DIR=%s\n' {{quote(CARGO_TARGET_DIR)}}
    @printf 'CARGO_BUILD_TARGET_DIR=%s\n' {{quote(CARGO_BUILD_TARGET_DIR)}}
    @printf 'CARGO_BUILD_BUILD_DIR=%s\n' {{quote(CARGO_BUILD_BUILD_DIR)}}
    @printf 'RUSTC_WRAPPER=%s\n' {{quote(RUSTC_WRAPPER)}}
    @printf 'CARGO_BUILD_RUSTC_WRAPPER=%s\n' {{quote(CARGO_BUILD_RUSTC_WRAPPER)}}
    @printf 'CARGO_INCREMENTAL=%s\n' {{quote(CARGO_INCREMENTAL)}}
    @printf 'CARGO_BUILD_JOBS=%s\n' {{quote(env("CARGO_BUILD_JOBS", ""))}}
    @printf 'SCCACHE_DIR=%s\n' {{quote(SCCACHE_DIR)}}
    @printf 'SCCACHE_CACHE_SIZE=%s\n' {{quote(SCCACHE_CACHE_SIZE)}}
    @printf 'SCCACHE_BASEDIRS=%s\n' {{quote(SCCACHE_BASEDIRS)}}
    @printf 'TEMP=%s\n' {{quote(TEMP)}}
    @printf 'TMP=%s\n' {{quote(TMP)}}
    @printf 'TMPDIR=%s\n' {{quote(TMPDIR)}}

# Install the repository Git hook shim using prek.
hooks-install:
    prek install

# Run prek against the staged files.
hooks-run:
    prek run

export CODE_REPOS_ROOT := code_repos_root
export CODE_PACKAGES_ROOT := code_packages_root
export CODE_TOOLS_ROOT := code_tools_root
export CODE_TARGET_ROOT := code_target_root
export CODE_BUILD_ROOT := code_build_root
export CODE_TEMP_ROOT := code_temp_root
export CARGO_HOME := env("CARGO_HOME", if os() == "windows" { join(code_packages_root, "cargo") } else { join(home_directory(), ".cargo") })
export NUGET_PACKAGES := env("NUGET_PACKAGES", join(code_packages_root, "nuget"))
export npm_config_cache := env("npm_config_cache", join(code_packages_root, "npm"))

# Configure shared Cargo compiler defaults; checkout output slots remain separate.
configure-cargo-host:
    pwsh -NoLogo -NoProfile -File {{quote(join(root, "scripts/cargo-host.ps1"))}} -BuildRoot {{quote(code_build_root)}} -ConfigureHost

# Link Cargo and rust-analyzer outputs without replacing existing directories.
build-setup:
    pwsh -NoLogo -NoProfile -File {{quote(join(source_directory(), "scripts/cargo-host.ps1"))}} -BuildRoot {{quote(code_build_root)}} -LinkOutputs

# Lint repository Markdown with the pinned markdownlint-cli2.
markdownlint *args='':
    npx --yes markdownlint-cli2@0.24.0 {{ args }}
