# GitHub Actions workflows

| File          | Trigger                        | Purpose                                                                                                                                                                      |
| ------------- | ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ci.yml`      | push / PR to `develop`, manual | Entry point. Calls `rust-ci.yml`.                                                                                                                                            |
| `rust-ci.yml` | `workflow_call`                | Reusable: prek/dprint formatting hooks, build, Clippy, nextest and doctests on Linux, macOS and Windows, plus informational `cargo llvm-cov` coverage (Linux, never a gate). |

Shared building block: [`../actions/setup-rust`](../actions/setup-rust/action.yml) installs the pinned stable
toolchain (1.99.1) and the nightly from [`../../rustfmt-toolchain`](../../rustfmt-toolchain), used only for `rustfmt`,
then restores the Cargo cache. Set `RUSTFMT_TOOLCHAIN` to override the pin for local commands and container builds.
The same action and the same `rust-ci.yml` shape are used in `mdbook-utils` and `rust_howto`.

The workflow pins cargo-nextest 0.9.145, prek 0.5.6, dprint 0.64.1, and cargo-deny 0.20.2. Keep these aligned with the nextest, prek, dprint, and cargo-deny configuration files when updating the tools.

## Conventions

- **Pinning.** Third-party actions are pinned to a full commit SHA with the version in a trailing comment;
  Dependabot (`github-actions` ecosystem) proposes updates. Do not use floating tags such as `@v4`.
- **Toolchain bumps.** Change `rustfmt-toolchain`; the action, Dockerfile, Just recipes, and CI hooks read that file.
- **Shell.** All `run` steps use `bash`, including on Windows, so there is one command line per step.
- **Paths.** `.cargo/config.toml` uses Cargo's portable checkout-local output defaults and `sccache`. Local
  `just` recipes use platform cache directories; the workflow overrides the output paths with
  `CARGO_TARGET_DIR` and `CARGO_BUILD_BUILD_DIR`, and disables the wrapper with an empty `RUSTC_WRAPPER`.
- **Permissions.** Workflows default to `contents: read`.

## Checking workflows locally

```sh
pip install actionlint-py && actionlint
```
