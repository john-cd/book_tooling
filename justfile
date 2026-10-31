# TODO Reviewed ??-??-2026

set shell := ["bash", "-uc"]

# Recipes run in Bash on every OS: Git Bash on Windows.
set windows-shell := ["C:/Program Files/Git/bin/bash.exe", "-uc"]

root := replace(source_directory(), "\\", "/")
rustfmt_toolchain_default := trim(replace_regex(read(join(root, "rustfmt-toolchain.toml")), '(?s).*channel\s*=\s*"([^"]+)".*', "$1"))
rustfmt_toolchain_override := env("RUSTFMT_TOOLCHAIN", "")
rustfmt_toolchain := if rustfmt_toolchain_override == "" { rustfmt_toolchain_default } else { rustfmt_toolchain_override }
checkout_name := file_name(root)
worktree_name := if checkout_name == "book_tooling" { "main" } else { checkout_name }
sccache_base := if file_name(parent_directory(root)) == ".worktrees" { parent_directory(parent_directory(root)) } else { root }
inherited_sccache_bases := env("SCCACHE_BASEDIRS", "")
sccache_base_separator := if os() == "windows" { ";" } else { ":" }
windows_cache_home := env("LOCALAPPDATA", join(home_directory(), ".cache"))
unix_cache_home := env("XDG_CACHE_HOME", join(home_directory(), ".cache"))
cache_home := if os() == "windows" { windows_cache_home } else { unix_cache_home }
target_root := if os() == "windows" { "D:/target/book_tooling" } else { join(cache_home, "cargo-target/book_tooling") }
build_root := if os() == "windows" { "C:/code_build/book_tooling" } else { join(cache_home, "cargo-build/book_tooling") }
target_dir := join(target_root, worktree_name)
build_dir := join(build_root, worktree_name)
export CARGO_TARGET_DIR := target_dir
export CARGO_BUILD_TARGET_DIR := target_dir
export CARGO_BUILD_BUILD_DIR := build_dir
export RUSTC_WRAPPER := "sccache"
export CARGO_BUILD_RUSTC_WRAPPER := "sccache"
export CARGO_INCREMENTAL := "0"
export SCCACHE_DIR := if os() == "windows" { "D:/sccache" } else { join(cache_home, "sccache") }
export SCCACHE_CACHE_SIZE := "20G"
export SCCACHE_BASEDIRS := if inherited_sccache_bases == "" { sccache_base } else { inherited_sccache_bases + sccache_base_separator + sccache_base }
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
    cargo +{{ rustfmt_toolchain }} build --bins --locked --all-features --release -Z unstable-options --artifact-dir "{{ build_dir }}/artifacts"

[positional-arguments]
cargo +args:
    cargo "$@"
