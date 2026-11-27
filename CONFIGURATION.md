<!-- TODO Reviewed ??-??-2026. -->

# Repository configuration

Reviewed 2026-11-01. This is the consistency plan and reference for `book_tooling`.
Keep this file, README, contributor guidance, and configuration comments synchronized.

## Consistency plan

1. Share LF/UTF-8 editing defaults and language formatting conventions.
2. Pin nightly Rustfmt to `nightly-2026-11-16` and align lint baselines while preserving stricter rules, domain limits, and teaching examples.
   A dated nightly keeps Rustfmt output consistent across developer machines and CI as the nightly channel moves. When bumping it, update the formatter defaults and run the repository's formatting check with the new toolchain.
3. Enumerate maintained dependency roots and use predictable monthly grouped updates.
4. Isolate checkout outputs, align dependency compiler flags, and make CI caches configuration-aware.
5. Validate format/lint and representative builds; measure cold/warm time and output size before
   promoting further optimization changes. Keep platform and integration blockers visible in TODO.

`just config-drift` runs the `book_tool config-drift` Rust command to check Cargo workspace/member
and Dependabot paths, the shared LF editor/Git defaults, container/toolchain pin agreement, and
documented fixture exceptions. `just orphansources` runs `book_tool orphan-sources` to verify
reachable Rust modules and report orphaned workspace sources. Cargo metadata,
prek, and dprint validate the relevant manifests and structured files; CI also runs actionlint and
`docker compose config` for their respective schemas. Keep exceptions in
`.config/config-drift.toml` with a reason and matching hook exclusions.

## File ownership

| Files                                       | Convention                                                                                                                                                                   |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `.editorconfig`, `.gitattributes`           | UTF-8 and LF, four-space Rust/Python, two-space configuration; preserve Markdown hard breaks and binary attributes.                                                          |
| `.gitignore`, `.dockerignore`, `.aiexclude` | Exclude outputs, compiler caches, environments, and worktrees; keep maintained configuration and lockfiles reviewable. Ignore files are not a security boundary.             |
| `AGENTS.md`, `CONTRIBUTING.md`, `README.md` | Keep repository-specific commands, ownership, and examples; link this reference.                                                                                             |
| `Cargo.toml`, `clippy.toml`, `rustfmt.toml` | Workspace lint inheritance and a common baseline; preserve explicit exceptions and package compatibility requirements.                                                       |
| Python project files                        | No Python project is currently tracked. If one is added, use Ruff with 100 columns and LF plus a committed `uv.lock` and `.python-version`.                                  |
| `prek.toml`                                 | Keep the existing repository hook set and pinned hook revisions. Run the configured formatter and linter rather than duplicating policy in hooks.                            |
| `deny.toml`, `nextest.toml`                 | Keep dependency-license exceptions, test filters, and resource/time limits local. Nextest normally discovers `.config/nextest.toml`; a Cargo profile is selected separately. |
| `.github/`, `.devcontainer/`, `Dockerfile*` | Preserve deployment/platform behavior; align package coverage and cache inputs. Keep compiler/Python pins compatible with manifests and lockfiles.                           |

## Dependency updates

Dependabot runs monthly on Monday at 6:00 a.m. Pacific with grouped minor/patch updates. These are
the maintained roots; workspace members share their root manifest/lockfile. Immutable parser,
test-book, and malformed-manifest fixtures are excluded intentionally. Revisit this table and
`.github/dependabot.yml` whenever a manifest, Dockerfile, chart, or local action is added.
There is no tracked Python project or uv workspace, so no Python dependency-update root is configured. If one is added, list its lockfile root here and in Dependabot.

| Ecosystem        | Roots                              |
| ---------------- | ---------------------------------- |
| `cargo`          | `/`, `/parse_lib/fuzz`             |
| `docker`         | `/.devcontainer`                   |
| `docker-compose` | `/.devcontainer`                   |
| `devcontainers`  | `/`                                |
| `github-actions` | `/`, `/.github/actions/setup-rust` |
| `rust-toolchain` | `/`                                |

GitHub Actions uses `/` for workflows and explicit directories for nested composite actions.
Dependabot can update supported toolchain files, container images/features, charts, and packages;
it does not update arbitrary version strings in shell scripts or custom formatter-pin files.

## CI caches

Rust-cache entries include toolchain/environment inputs plus workflow, job, architecture, and
Cargo manifest/config/lockfile hashes. The validate and coverage jobs add labels for their Cargo
profiles and default feature sets. Save Rust artifact caches only on pushes to the default branch;
pull requests restore caches. CI maps the Cargo workspace to `.ci/target/book_tooling/main` and
caches `.ci/build/book_tooling/main` separately. Keep specialized target, feature, and release jobs
isolated. sccache is installed by the shared setup action, and incremental compilation stays disabled,
including when using GitHub caches. An artifact cache does not make Rust incremental compilation
compatible with sccache. If a Python workflow is added, key its setup-uv cache by `uv.lock` and retain
`--locked` for commands.
Container builds retain their existing cargo-chef/BuildKit strategy; when cooking dependencies,
use the same compiler, features, profile, target, flags, and build paths as the application build.

## Cargo profiles and output paths

`.cargo/config.toml` owns shared development profiles. Child Cargo configs keep only workspace
environment/target requirements and package tuning for independent dependency graphs. Cargo
config discovery starts at the command's working directory; run from the owning workspace,
especially when that workspace has a child config. Config values override manifest profiles,
and environment/command-line overrides take precedence over config values.

| Profile                                | Workspace code                               | Dependencies and build tools                                                                 |
| -------------------------------------- | -------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `dev` / `test`                         | Level 0, no symbols, incremental disabled    | Level 1 by default, selected compute-heavy dependencies at level 3, no symbols.              |
| `debug`                                | Level 2 symbols for workspace crates         | Same optimized dependency settings as `dev`, with dependency symbols disabled.               |
| `ci`                                   | Inherits `dev`, incremental disabled         | Same flags as `dev` for dependency cache reuse. Use release jobs for performance validation. |
| `tools`                                | Level 2 for repository tools                 | Same dependency settings as `dev`.                                                           |
| `release` / other specialized profiles | Preserve existing shipping/profiling choices | Preserve existing shipping/profiling choices.                                                |

`test` inherits `dev`. Package overrides take precedence over build overrides; macro parsing
crates are tuned explicitly. Optimization can increase cold compile time, and size optimization
is not guaranteed to make every crate smaller. Matching profile flags improves cache eligibility;
compiler versions, target/features, source paths, native tools, and rustflags also affect hits.
The built-in `debug` profile keeps full symbols on workspace crates while the shared dependency override keeps third-party dependencies symbol-free.

Run `just build-setup` once per checkout. It creates ignored `target`, `build`, and `.sccache`
links and refuses to replace existing files, directories, or links to another slot. PowerShell 7
is required by `scripts/cargo-host.ps1`. Windows uses junctions and NTFS intermediates; Unix
uses native cache roots and symlinks. Existing Rust How-To setup recipes also provide native
shell setup. Do not copy platform-specific links between Windows and Linux checkouts.

The existing `just cargo <subcommand> ...` runner sets absolute
`CODE_TARGET_ROOT/<repo>/<slot>` and `CODE_BUILD_ROOT/<repo>/<slot>` paths. The local Cargo config
names the checkout's `target` and `build` links, which point to those same per-slot roots.
`cargo.targetDir = null` prevents rust-analyzer from allocating another target tree. Older Rust versions failed to build through ReFS-to-NTFS junctions with Windows I/O errors; Rust 1.100.0 includes the upstream fix. With Rust 1.100.0 or newer, direct ReFS build output is supported. Use the Just runner or set absolute `CARGO_TARGET_DIR` and `CARGO_BUILD_BUILD_DIR` values for direct Cargo commands. Inherited `CARGO_*` variables can override these settings; launch the editor without another checkout\x27s output variables. Multiple independent workspaces share a checkout\x27s target slot and Cargo locking.

Build scripts/macros are optimized because they execute during compilation. Cargo gives
package overrides precedence over build overrides, so unspecified external build dependencies
use the size-optimized wildcard and explicitly tuned macro parsing crates use level 3. sccache caches
eligible library compilations, not linking of binaries, dynamic libraries, or procedural macros.
Keep the shared 45 GiB cache and stable per-slot directories. Inspect running Cargo/rustc
processes before restarting sccache. Clean only demonstrated stale/corrupt content, never by
recursively deleting a checkout link or another active slot.

Select profiles with `just cargo build --profile debug` or `--profile tools`; ordinary
build/test uses `dev`/`test`. Select a Cargo profile in nextest with `--cargo-profile ci`, independently
of nextest's test-runner `--profile ci`.

## Linkers

| Platform         | Policy                                                                                                                                                                                                          |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Windows MSVC     | Rust-bundled `rust-lld.exe` in the maintained repos; preserve stack flags. Rust How-To retains its measured default-linker exception. MSVC libraries/SDK are still required.                                    |
| x86-64 GNU/Linux | Rust 1.90+ already defaults to LLD. mold 3.0 is a Rust rewrite and remains an optional, measured Linux choice. For AArch64/other triples, explicitly install/configure the appropriate linker before switching. |
| macOS            | Use the modern Apple linker shipped with Xcode 15+. The sold repository is archived and recommends Apple's linker.                                                                                              |
| Experiments      | Compare mold and Wild on representative native Linux binaries. Wild supports Linux architectures and WebAssembly, with Mach-O/Windows support still on its roadmap; do not assume universal drop-in support.    |

Changing linker flags invalidates compiler cache keys. Benchmark cold and warm links with native
dependencies, debug symbols, and release/LTO settings before adopting a new default.

## Validation evidence

Windows x86_64-pc-windows-msvc, develop base `a639fc4` plus `37d6ef2` (2026-11-01):

- `cargo check --workspace --all-targets --locked` and all-features Clippy passed with direct Cargo
  after `37d6ef2`. The direct debugging-profile check passed using the host's absolute build fallback.
- CI, tools, and debug profile checks passed with verbose flags inspected and the runner's
  absolute per-slot environment. `cargo test --workspace --locked --no-run` passed in 1m10s with
  that environment; `rust-lld.exe` linked the Windows test executables. The warm dev check completed
  in 0.64s. `sccache` reported 61.69% hits during the test-profile build (256 C/C++ hits; zero
  Rust hits for that new profile).
- `rustc` invocations confirmed dependencies use `opt-level=s`, no debug symbols, and `strip=debuginfo`
  in regular profiles; named hash/collection/regex crates use `opt-level=3`; build scripts use
  `opt-level=3`. The debug profile used full symbols. Rust-bundled `rust-lld.exe` linked the
  Windows test executables successfully.
- Historical validation created checkout links at `D:/target/book_tooling/f`, `D:/code_build/book_tooling/f`, and `D:/cache/sccache`. D: is ReFS. An earlier Rust version failed through the ReFS junction; Rust 1.100.0 or newer fixes that upstream issue. The D: ReFS path change has not yet been revalidated by a build on this checkout. Use the Just runner or absolute Cargo environment paths for builds. An isolated setup probe passed root overrides, repeatability, existing-directory
  refusal, and mismatched-link refusal.
- The `just` wrapper could not start Git Bash in the managed shell, so setup and builds used the
  equivalent PowerShell setup script and explicit runner environment. Linux/macOS CI, an isolated
  cold-versus-warm rebuild comparison, and comparative linker benchmarks remain unverified. Do not
  claim cross-platform performance gains from these Windows checks.
- On 2026-11-01 at `51aad9c`, a Windows x86_64-pc-windows-msvc build of
  `cargo build --workspace --all-targets --locked --profile ci --quiet` completed in 348.166 s
  with fresh target and intermediate directories under slot `f` and `CARGO_INCREMENTAL=0`. The
  outputs contained 75,351,859 target bytes and 734,167,032 intermediate bytes. Repeating the same
  command against those outputs completed as a Cargo no-op in 1.475 s; this is not a warm rebuild.
  The shared sccache v0.19.0 daemon reported a 0% Rust hit rate at capture, but its counters and
  cache were shared with other work, so this is not an isolated Rust-cache measurement. The
  compiler-cache and cold-versus-warm rebuild comparison remains open.
- On 2026-11-05 the manual `Measure` workflow ([run 41959102270](https://github.com/john-cd/book_tooling/actions/runs/41959102270))
  built the workspace at develop `b4437a1` on GitHub-hosted runners with `RUSTC_WRAPPER` disabled
  (no compiler cache, so the numbers are isolated from sccache) and `CARGO_INCREMENTAL=0`. Warm
  means a rebuild after `touch parse_lib/src/lib.rs` with dependencies already built. Each OS used
  its default linker; no alternative linker was compared.

  | Runner                                   | `ci` cold | `ci` warm | `ci` target | `release` cold | `release` warm | Both profiles' target |
  | ---------------------------------------- | --------- | --------- | ----------- | -------------- | -------------- | --------------------- |
  | ubuntu-latest (x86_64-unknown-linux-gnu) | 148 s     | 8 s       | 623,888 KiB | 160 s          | 30 s           | 1,122,052 KiB         |
  | macos-latest (aarch64-apple-darwin)      | 47 s      | 3 s       | 592,576 KiB | 49 s           | 10 s           | 1,090,780 KiB         |
  | windows-latest (x86_64-pc-windows-msvc)  | 460 s     | 20 s      | 675,506 KiB | 446 s          | 81 s           | 1,264,112 KiB         |

  Both release builds and all `ci` builds linked successfully on all three platforms, and a
  repeated `ci` build was a Cargo no-op (0 s). Runner hardware varies between runs, so treat the
  figures as order-of-magnitude comparisons. Rerun with `gh workflow run measure.yml`.

## References

- [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) and
  [configuration precedence](https://doc.rust-lang.org/cargo/reference/config.html).
- [sccache Rust limitations](https://github.com/mozilla/sccache/blob/main/docs/Rust.md).
- [Dependabot options](https://docs.github.com/en/code-security/reference/supply-chain-security/dependabot-options-reference).
- [Rust 1.90 LLD default](https://blog.rust-lang.org/2025/09/01/rust-lld-on-1.90.0-stable/),
  [LLD platforms](https://lld.llvm.org/), [mold](https://github.com/rui314/mold),
  [sold recommendation](https://github.com/bluewhalesystems/sold), and [Wild support](https://github.com/wild-linker/wild).
- [rust-analyzer target directory](https://rust-analyzer.github.io/book/configuration.html#rust-analyzercargotargetdir).
