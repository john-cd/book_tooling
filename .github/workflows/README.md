# GitHub Actions workflows

| File          | Trigger                        | Purpose                                                                                                                                                                      |
| ------------- | ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ci.yml`      | push / PR to `develop`, manual | Entry point. Calls `rust-ci.yml`.                                                                                                                                            |
| `rust-ci.yml` | `workflow_call`                | Reusable: prek/dprint formatting hooks, build, Clippy, nextest and doctests on Linux, macOS and Windows, plus informational `cargo llvm-cov` coverage (Linux, never a gate). |

Shared building block: [`../actions/setup-rust`](../actions/setup-rust/action.yml) installs the pinned stable
toolchain (1.99.1) and the pinned nightly used only for `rustfmt` (`nightly-2026-10-22`), then restores the Cargo cache.
The same action and the same `rust-ci.yml` shape are used in `mdbook-utils` and `rust_howto`.

The workflow pins `cargo-nextest` 0.9.145, `prek` 0.5.6, and `dprint` 0.64.1. Keep these aligned with
`.config/nextest.toml`, `prek.toml`, and `dprint.jsonc` when updating the tools.

## Conventions

- **Pinning.** Third-party actions are pinned to a full commit SHA with the version in a trailing comment;
  Dependabot (`github-actions` ecosystem) proposes updates. Do not use floating tags such as `@v4`.
- **Toolchain bumps.** Change `toolchain` / `nightly` defaults in `setup-rust/action.yml`, the `+nightly-...`
  in `rust-ci.yml` and the `justfile` together.
- **Shell.** All `run` steps use `bash`, including on Windows, so there is one command line per step.
- **Paths.** `.cargo/config.toml` points at developer drives (`D:/target/...`) and `sccache`. The workflow
  overrides these with `CARGO_TARGET_DIR`, `CARGO_BUILD_BUILD_DIR` and an empty `RUSTC_WRAPPER`.
- **Permissions.** Workflows default to `contents: read`.

## Checking workflows locally

```sh
pip install actionlint-py && actionlint
```
