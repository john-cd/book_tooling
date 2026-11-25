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
toolchain and the nightly from [`../../rustfmt-toolchain.toml`](../../rustfmt-toolchain.toml), used only for `rustfmt`,
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

<!-- workflow-operations-reference:start -->

<!-- TODO Reviewed ??-??-2026. -->

## Workflow operations reference

The workflow YAML files own execution behavior. Their inline comments explain the purpose and
the boundaries that must survive future refactoring. This reference records the current job map;
repository-level Actions enable/disable settings remain independent of these tracked definitions.

### Changes in this review

- container.yml: Added branch-scoped concurrency with obsolete-run cancellation.
- fuzz.yml: Added branch-scoped concurrency with obsolete-run cancellation.
- measure.yml: Added branch-scoped concurrency with obsolete-run cancellation.

### [CI](ci.yml)

Events: `push, pull_request, workflow_dispatch`. Branch/path filters and manual defaults are defined in the linked file.

| Job | Runs on / reuse | Dependencies | Guard | Main checks or side effects |
| --- | --- | --- | --- | --- |
| changes | "ubuntu-latest" | [] | Successful dependencies (default) | actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1; dorny/paths-filter@0e4a8c6effa4802afeda77dc8d303f8176d7dfad |
| rust | "./.github/workflows/rust-ci.yml" | "changes" | ${{ github.event_name == 'workflow_dispatch' \|\| needs.changes.outputs.code == 'true' }} |  |
| documentation | "ubuntu-latest" | "changes" | needs.changes.outputs.code != 'true' | actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1; dprint/check@371a959e3c77f34a76a03d2c50df2e3f2302d016 |

### [Container](container.yml)

Events: `workflow_dispatch`. Branch/path filters and manual defaults are defined in the linked file.

| Job | Runs on / reuse | Dependencies | Guard | Main checks or side effects |
| --- | --- | --- | --- | --- |
| build | "ubuntu-latest" | [] | Successful dependencies (default) | Check out; Set up Docker Buildx; Build image; Check pinned tools |

### [Fuzz](fuzz.yml)

Events: `workflow_dispatch`. Branch/path filters and manual defaults are defined in the linked file.

| Job | Runs on / reuse | Dependencies | Guard | Main checks or side effects |
| --- | --- | --- | --- | --- |
| fuzz | "ubuntu-latest" | [] | Successful dependencies (default) | Check out; Set up Rust; Install cargo-fuzz; Fuzz round_trip; Upload crashes |

### [Measure](measure.yml)

Events: `workflow_dispatch`. Branch/path filters and manual defaults are defined in the linked file.

| Job | Runs on / reuse | Dependencies | Guard | Main checks or side effects |
| --- | --- | --- | --- | --- |
| measure | "${{ matrix.os }}" | [] | Successful dependencies (default) | Check out; Set up Rust; Measure |

### [Rust CI (reusable)](rust-ci.yml)

Events: `workflow_call`. Branch/path filters and manual defaults are defined in the linked file.

| Job | Runs on / reuse | Dependencies | Guard | Main checks or side effects |
| --- | --- | --- | --- | --- |
| validate | "${{ matrix.os }}" | [] | Successful dependencies (default) | Code checkout; Set up Rust (pinned stable + pinned nightly rustfmt); Install workspace tools (cargo-nextest, prek, dprint, cargo-deny, just); Run repository hooks; Check configuration paths and cross-file pins; Validate GitHub Actions workflows; Validate devcontainer Compose configuration; Build; Clippy; Run tests; Run doctests |
| coverage | "ubuntu-latest" | [] | Successful dependencies (default) | Code checkout; Set up Rust (pinned stable + llvm-tools); Install cargo-llvm-cov; Measure coverage |
| msrv | "ubuntu-latest" | [] | Successful dependencies (default) | actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1; dtolnay/rust-toolchain@683b588c487a69d01f6c6ddf40f036ff682cc2c8; Check declared minimum version |

### Caches, permissions and failures

- Keep package downloads and compiler outputs separate. Match cache keys to compiler, OS, dependency locks, build profiles and features; a cache is an optimization, not a prerequisite for correctness.
- Matrix jobs start on separate machines. Build/test commands in one job reuse local outputs; moving them to separate jobs requires an explicit compatible artifact handoff.
- Read-only validation and live publication have different lifecycles. Preserve existing credentials, event/ref conditions, environments and non-cancelling deployment groups.
- Timeouts bound hung or unexpectedly expensive runs. Diagnose a timeout before increasing it; never convert a failed check into an unconditional success.
- A queued/disabled/billing-blocked run is not a passed check. Record hosted results separately from local workflow parsing/format validation.

### Local validation

```sh
actionlint
git diff --check
```

Use the repository formatter for edited YAML/Markdown. Run the commands named by the affected
jobs when their executable behavior changes; platform/container/credential-dependent checks
must be reported at the scope actually executed. Do not run publishing commands merely to test
a comment or validation-only change.

<!-- workflow-operations-reference:end -->
