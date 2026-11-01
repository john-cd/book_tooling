# Repository configuration

Reviewed 2026-11-01. This is the consistency plan and reference for `book_tooling`.
Keep this file, README, contributor guidance, and configuration comments synchronized.

## Consistency plan

1. Share LF/UTF-8 editing defaults and language formatting conventions.
2. Pin nightly Rustfmt to `nightly-2026-10-22` and align lint baselines while preserving stricter rules, domain limits, and teaching examples.
3. Enumerate maintained dependency roots and use predictable monthly grouped updates.
4. Isolate checkout outputs, align dependency compiler flags, and make CI caches configuration-aware.
5. Validate format/lint and representative builds; measure cold/warm time and output size before
   promoting further optimization changes. Keep platform and integration blockers visible in TODO.

## File ownership

| Files                                       | Convention                                                                                                                                                                   |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `.editorconfig`, `.gitattributes`           | UTF-8 and LF, four-space Rust/Python, two-space configuration; preserve Markdown hard breaks and binary attributes.                                                          |
| `.gitignore`, `.dockerignore`, `.aiexclude` | Exclude outputs, compiler caches, environments, and worktrees; keep maintained configuration and lockfiles reviewable. Ignore files are not a security boundary.             |
| `AGENTS.md`, `CONTRIBUTING.md`, `README.md` | Keep repository-specific commands, ownership, and examples; link this reference.                                                                                             |
| `Cargo.toml`, `clippy.toml`, `rustfmt.toml` | Workspace lint inheritance and a common baseline; preserve explicit exceptions and package compatibility requirements.                                                       |
| `pyproject.toml`, `uv.lock`                 | Ruff uses 100 columns and LF; explicit default E4/E7/E9/F lint selection. Keep project Python pins and locked resolution.                                                    |
| `prek.toml`                                 | Keep the existing repository hook set and pinned hook revisions. Run the configured formatter and linter rather than duplicating policy in hooks.                            |
| `deny.toml`, `nextest.toml`                 | Keep dependency-license exceptions, test filters, and resource/time limits local. Nextest normally discovers `.config/nextest.toml`; a Cargo profile is selected separately. |
| `.github/`, `.devcontainer/`, `Dockerfile*` | Preserve deployment/platform behavior; align package coverage and cache inputs. Keep compiler/Python pins compatible with manifests and lockfiles.                           |

## Dependency updates

Dependabot runs monthly on Monday at 6:00 a.m. Pacific with grouped minor/patch updates. These are
the maintained roots; workspace members share their root manifest/lockfile. Immutable parser,
test-book, and malformed-manifest fixtures are excluded intentionally. Revisit this table and
`.github/dependabot.yml` whenever a manifest, Dockerfile, chart, or local action is added.
Python uv workspace members share the root lock; they do not need overlapping update entries.

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
Cargo-config/lockfile hashes. Save Rust artifact caches only on pushes to the default branch;
pull requests restore caches. CI sets matching `CODE_*` and `CARGO_*` paths under `.ci/target/<repo>/main` and
`.ci/build/<repo>/main`. Rust-cache workspace maps use those same absolute paths,
including intermediate artifacts, so direct Cargo and task runners reuse one output tree.
Keep specialized coverage, target, feature, and release jobs isolated. sccache is pinned where
installed, and incremental compilation stays disabled, including when using GitHub caches.
An artifact cache does not make Rust incremental compilation compatible with sccache.
Python workflows use setup-uv caching keyed by `uv.lock`, and commands retain `--locked`.
Container builds retain their existing cargo-chef/BuildKit strategy; when cooking dependencies,
use the same compiler, features, profile, target, flags, and build paths as the application build.

## Cargo profiles and output paths

`.cargo/config.toml` owns shared development profiles. Child Cargo configs keep only workspace
environment/target requirements and package tuning for independent dependency graphs. Cargo
config discovery starts at the command's working directory; run from the owning workspace,
especially when that workspace has a child config. Config values override manifest profiles,
and environment/command-line overrides take precedence over config values.

| Profile                                | Workspace code                                           | Dependencies and build tools                                                                                                                 |
| -------------------------------------- | -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `dev` / `test`                         | `opt-level = 0`, line tables, LTO off, 256 codegen units | `opt-level = "s"`, no debug info, 16 units; selected hashing, collections, math, regex, parallelism, allocator and macro crates use level 3. |
| `debugging`                            | Level 0 and full symbols                                 | Level 0 and full symbols, including build scripts/procedural macros; intentionally uses a separate cache variant.                            |
| `ci`                                   | Inherits `dev`, incremental disabled                     | Same flags as `dev` for dependency cache reuse. Use release jobs for performance validation.                                                 |
| `tools`                                | Level 2 for repeatedly executed repository tools         | Same dependency flags as `dev`; build overrides request level 3 with no debug info; named package overrides take precedence.                 |
| `release` / other specialized profiles | Preserve existing shipping/profiling choices             | Preserve existing shipping/profiling choices.                                                                                                |

`test` inherits `dev`. Package overrides take precedence over build overrides; macro parsing
crates are tuned explicitly. Optimization can increase cold compile time, and size optimization
is not guaranteed to make every crate smaller. Matching profile flags improves cache eligibility;
compiler versions, target/features, source paths, native tools, and rustflags also affect hits.
The `debugging` profile intentionally rebuilds dependencies to make them debuggable.

Run `just build-setup` once per checkout. It creates ignored `target`, `build`, and `.sccache`
links and refuses to replace existing files, directories, or links to another slot. PowerShell 7
is required by `scripts/cargo-host.ps1`. Windows uses junctions and NTFS intermediates; Unix
uses native cache roots and symlinks. Existing Rust How-To setup recipes also provide native
shell setup. Do not copy platform-specific links between Windows and Linux checkouts.

The existing `just cargo <subcommand> ...` runner sets absolute `CODE_TARGET_ROOT/<repo>/<slot>`
and `CODE_BUILD_ROOT/<repo>/<slot>` paths. Direct Cargo and rust-analyzer use the checkout-local
`target` link. The local config intentionally leaves `build-dir` unset, so direct Cargo uses the
absolute per-workspace fallback from `CARGO_HOME/config.toml`; this avoids traversing a build link
from a ReFS checkout to NTFS on Windows. `cargo.targetDir = null` prevents rust-analyzer from
allocating another target tree. Inherited `CARGO_*` variables can override these settings; launch
the editor without another checkout's output variables. Multiple independent workspaces share a
checkout's target slot and Cargo locking.

Build scripts/macros are optimized because they execute during compilation. Cargo gives
package overrides precedence over build overrides, so unspecified external build dependencies
use the size-optimized wildcard and explicitly tuned macro parsing crates use level 3. sccache caches
eligible library compilations, not linking of binaries, dynamic libraries, or procedural macros.
Keep the shared 35 GiB cache and stable per-slot directories. Inspect running Cargo/rustc
processes before restarting sccache. Clean only demonstrated stale/corrupt content, never by
recursively deleting a checkout link or another active slot.

Select profiles with `just cargo build --profile debugging` or `--profile tools`; ordinary
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
- CI, tools, and debugging profile checks passed with verbose flags inspected and the runner's
  absolute per-slot environment. `cargo test --workspace --locked --no-run` passed in 1m10s with
  that environment; `rust-lld.exe` linked the Windows test executables. The warm dev check completed
  in 0.64s. `sccache` reported 61.69% hits during the test-profile build (256 C/C++ hits; zero
  Rust hits for that new profile).
- `rustc` invocations confirmed dependencies use `opt-level=s`, no debug symbols, and `strip=debuginfo`
  in regular profiles; named hash/collection/regex crates use `opt-level=3`; build scripts use
  `opt-level=3`. The debugging profile used full symbols. Rust-bundled `rust-lld.exe` linked the
  Windows test executables successfully.
- `build-setup` created and then revalidated the checkout links at `D:/target/book_tooling/f`,
  `C:/code_build/book_tooling/f`, and `D:/sccache`. C: is NTFS and D: is ReFS. A debug check through
  the ReFS-to-NTFS `build` junction failed with Windows I/O errors; it passed when Cargo received
  the absolute NTFS `CARGO_BUILD_BUILD_DIR`. The local config now leaves `build-dir` unset so direct
  Cargo's host-configured absolute per-workspace fallback. A direct debug check and default dev
  check passed after this change. An isolated setup probe passed root overrides, repeatability,
  existing-directory refusal, and mismatched-link refusal.
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
- The reusable CI workflow has Linux, macOS, and Windows runners, but no passing hosted Linux/macOS
  or release-linker run is recorded for this revision. Those platform and release checks remain
  open; the local Windows build does not establish their linker or SDK behavior.

## References

- [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) and
  [configuration precedence](https://doc.rust-lang.org/cargo/reference/config.html).
- [sccache Rust limitations](https://github.com/mozilla/sccache/blob/main/docs/Rust.md).
- [Dependabot options](https://docs.github.com/en/code-security/reference/supply-chain-security/dependabot-options-reference).
- [Rust 1.90 LLD default](https://blog.rust-lang.org/2025/09/01/rust-lld-on-1.90.0-stable/),
  [LLD platforms](https://lld.llvm.org/), [mold](https://github.com/rui314/mold),
  [sold recommendation](https://github.com/bluewhalesystems/sold), and [Wild support](https://github.com/wild-linker/wild).
- [rust-analyzer target directory](https://rust-analyzer.github.io/book/configuration.html#rust-analyzercargotargetdir).
