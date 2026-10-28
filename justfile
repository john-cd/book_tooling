# TODO Reviewed ??-??-2026

set shell := ["bash", "-uc"]

# Recipes run in Bash on every OS: Git Bash on Windows.
set windows-shell := ["C:/Program Files/Git/bin/bash.exe", "-uc"]

root := replace(source_directory(), "\\", "/")
rustfmt_toolchain_default := trim(replace_regex(read(join(root, "rustfmt-toolchain.toml")), '(?s).*channel\s*=\s*"([^"]+)".*', "$1"))
rustfmt_toolchain_override := env("RUSTFMT_TOOLCHAIN", "")
rustfmt_toolchain := if rustfmt_toolchain_override == "" { rustfmt_toolchain_default } else { rustfmt_toolchain_override }
worktree_name := env("BOOK_TOOLING_WORKTREE_NAME", "main")
windows_cache_home := env("LOCALAPPDATA", join(home_directory(), ".cache"))
macos_cache_home := env("XDG_CACHE_HOME", join(join(home_directory(), "Library"), "Caches"))
unix_cache_home := env("XDG_CACHE_HOME", join(home_directory(), ".cache"))
default_cache_home := if os() == "windows" { windows_cache_home } else if os() == "macos" { macos_cache_home } else { unix_cache_home }
cache_home := env("BOOK_TOOLING_CACHE_HOME", default_cache_home)
build_root := join(cache_home, "book_tooling")
target_dir := env("CARGO_TARGET_DIR", join(join(build_root, "target"), worktree_name))
build_dir := env("CARGO_BUILD_BUILD_DIR", join(join(build_root, "build"), worktree_name))
export CARGO_TARGET_DIR := target_dir
export CARGO_BUILD_BUILD_DIR := build_dir
export RUSTC_WRAPPER := env("RUSTC_WRAPPER", "sccache")
export CARGO_INCREMENTAL := env("CARGO_INCREMENTAL", "0")
export SCCACHE_DIR := env("SCCACHE_DIR", join(cache_home, "sccache"))
export DPRINT_CACHE_DIR := env("DPRINT_CACHE_DIR", join(cache_home, "dprint"))

[no-exit-message]
_default:
    @just --list --unsorted --justfile {{ source_file() }}

alias fa := fmtall

@fmtcheck:
    cargo +{{ rustfmt_toolchain }} fmt --all -- --check

# Format all packages and their local path-based dependencies
@fmtall:
    echo "Formatting all packages and their local path-based dependencies in $(pwd)"
    cargo +{{ rustfmt_toolchain }} fmt --all

alias cka := checkall

# Check all code (all packages, all features)
@checkall:
    echo "Check all code (all packages, all features) in $(pwd)"
    cargo check --workspace --all-targets --locked --all-features

# `--all-targets` is equivalent to specifying `--lib --bins --tests --benches --examples`.

alias osa := orphansources

# Check that every Rust source file under src/ belongs to a compiled module tree.
@orphansources:
    echo "Check for orphan Rust source files in $(pwd)"
    python3 scripts/check_orphan_sources.py || python scripts/check_orphan_sources.py

alias ba := buildall

# Build all packages (all features)
@buildall:
    echo "Build all packages, all features in $(pwd)"
    cargo build --workspace --all-targets --locked --all-features --timings

alias ca := clippyall

# Scan all code for common mistakes (all packages, all features)
@clippyall:
    echo "Scan all code for common mistakes (all packages, all features) in $(pwd)"
    cargo clippy --workspace --all-targets --locked --all-features -- -D warnings

alias da := deny

# Check dependency advisories, licenses, duplicate versions, and registry sources.
@deny:
    echo "Check dependency policy in $(pwd)"
    cargo deny --locked check

alias nta := nextestall

# Test all code in the workspace (incl. doctests) (all features)
@nextestall:
    echo "Test all code in the workspace in $(pwd) (incl. doctests) (all features)"
    -cargo test --doc --workspace --quiet # nextest does not handle doctests
    cargo nextest run --workspace --all-targets --locked --no-fail-fast --status-level all --all-features --failure-output final

# Build and display the `cargo doc` documentation
@docall:
    echo "Build and display the `cargo doc` documentation for $(pwd)"
    cargo clean --doc
    cargo doc --no-deps --workspace --locked --document-private-items
    cd "{{ target_dir }}/doc/" ; (python3 -m http.server 9000 || python -m http.server 9000)

@release:
    echo "Build the tools in $(pwd) in release mode and copy to {{ root }}/../bin"
    cargo +{{ rustfmt_toolchain }} build --bins --locked --all-features --release -Z unstable-options --artifact-dir "{{ root }}/../bin"
