<!-- TODO Reviewed ??-??-2026. -->

# GitHub Actions workflows

| File            | Trigger                        | Purpose                                                                                                                                                                      |
| --------------- | ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ci.yml`        | push / PR to `develop`, manual | Entry point. Calls `rust-ci.yml`.                                                                                                                                            |
| `rust-ci.yml`   | `workflow_call`                | Reusable: prek/dprint formatting hooks, build, Clippy, nextest and doctests on Linux, macOS and Windows, plus informational `cargo llvm-cov` coverage (Linux, never a gate). |
| `fuzz.yml`      | manual                         | Runs the `parse_lib` `round_trip` libFuzzer target on Linux for a chosen number of seconds (default 600); never a gate.                                                      |
| `container.yml` | manual                         | Builds the `.devcontainer` image and checks the pinned tool versions; never a gate.                                                                                          |
| `measure.yml`   | manual                         | Times cold and warm `ci` and `release` builds and reports output sizes on Linux, macOS and Windows, without a compiler cache; never a gate.                                  |

Shared building block: [`../actions/setup-rust`](../actions/setup-rust/action.yml) installs the pinned stable
toolchain and the nightly from [`../../rustfmt-toolchain.toml`](../../rustfmt-toolchain), used only for `rustfmt`,
then restores the Cargo cache. Set `RUSTFMT_TOOLCHAIN` to override the pin for local commands and container builds.
Related Rust CI workflows are maintained in `mdbook-utils` and `rust_howto`; keep toolchain,
cache, and validation changes aligned with each repository's configuration.

The setup action pins sccache 0.19.0. CI pins cargo-nextest 0.9.151, prek 0.5.9, dprint 0.64.3, cargo-deny 0.20.2, and Just 1.59.0; keep these aligned with `.devcontainer/.env` and `CONTRIBUTING.md`. `.config/nextest.toml` also pins the nextest version.

## Conventions

- **Pinning.** Third-party actions are pinned to a full commit SHA with the version in a trailing comment;
  Dependabot (`github-actions` ecosystem) proposes updates. Do not use floating tags such as `@v4`.
- **Toolchain bumps.** Change `rustfmt-toolchain.toml`; the action, Dockerfile, Just recipes, and CI hooks read that file.
- **Shell.** All `run` steps use `bash`, including on Windows, so there is one command line per step.
- **Paths and caches.** `.cargo/config.toml` uses Cargo's portable checkout-local output defaults and `sccache`. Local
  `just` recipes use platform cache directories; CI maps its workspace to `.ci/target/book_tooling/main` and caches
  `.ci/build/book_tooling/main` separately. The reusable action installs sccache and disables incremental compilation.
  Validate and coverage jobs have explicit cache labels for their Cargo profiles and feature sets.
- **Permissions.** Workflows default to `contents: read`.

## Checking workflows locally

```sh
actionlint
zizmor --offline --strict-collection --collect all -- .github
```

The pinned actionlint release predates GitHub's `$/` self-repository syntax. `.github/actionlint.yaml`
ignores only its two unsupported-format diagnostics; all other workflow errors remain visible. Install
the standalone `actionlint` binary and `zizmor` using the versions managed by your development environment.

## CI cost and local checks

Recognized root documentation edits run dprint without recompiling the workspace. Unknown paths,
test fixtures, Rust files, manifests and workflow changes retain the full CI lane. Compilation,
Clippy, Nextest and doctests stay in one job so they share build outputs. Cross-platform manual
runs execute repository hooks and configuration checks only on Linux; build/tests run on each OS.

[Trusted Windows checks](https://github.com/john-cd/local_runners/actions/workflows/rust-repositories.yml)
use the existing J-RAZER runner, manually dispatched by the owner, and always resolve current
`develop`. Static mode checks formatting/workflows. Full mode runs Clippy, tests and doctests,
requires 15 GiB free on D:, uses two Cargo workers and isolated persistent runner output roots.
Public PR checks remain hosted; this local lane neither publishes nor receives deployment secrets.
