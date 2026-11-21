#!/usr/bin/env python3
# TODO Reviewed ??-??-2026.
"""Check repository configuration for cross-file drift.

This read-only check compares settings that must stay aligned across repository
files. It validates Dependabot paths, Cargo workspace members and Rust version
pins, editor and Git line-ending defaults, container toolchain pins, and
documented exceptions for intentional line-ending fixtures.

These checks catch stale or conflicting configuration early, such as a
Dependabot path that no longer exists or a container Rust version that differs
from the repository toolchain. Errors are printed and produce a nonzero exit
status; the script does not modify files. Run it from the repository root with
``just config-drift`` or pass a repository root as the first argument.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path


def read_toml(path: Path) -> dict:
    """Load a TOML file from ``path`` in binary mode, as required by tomllib."""
    with path.open("rb") as stream:
        return tomllib.load(stream)


def read_env(path: Path) -> dict[str, str]:
    """Read simple KEY=VALUE entries, ignoring blank lines and comments."""
    values: dict[str, str] = {}
    for line_number, line in enumerate(
        path.read_text(encoding="utf-8").splitlines(), 1
    ):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        key, separator, value = line.partition("=")
        if not separator or not key.strip():
            raise ValueError(f"{path}:{line_number}: expected KEY=VALUE")
        values[key.strip()] = value.strip().strip("\"'")
    return values


def parse_dependabot_paths(text: str) -> list[tuple[str, str]]:
    """Extract package-ecosystem and directory pairs from Dependabot YAML."""
    entries: list[tuple[str, str]] = []
    current: str | None = None
    in_directories = False
    directories_indent = -1

    # Read only the small YAML structure this check needs; this avoids requiring
    # a YAML dependency just to verify configured paths.
    for line in text.splitlines():
        ecosystem = re.match(r"^\s*-\s+package-ecosystem:\s*([^\s#]+)", line)
        if ecosystem:
            current = ecosystem.group(1).strip("\"'")
            in_directories = False
            directories_indent = -1
            continue
        if current is None:
            continue

        single = re.match(r"^\s+directory:\s*(.+?)\s*(?:#.*)?$", line)
        if single:
            entries.append((current, single.group(1).strip("\"'")))
            in_directories = False
            continue

        list_key = re.match(r"^(\s*)directories:\s*(?:#.*)?$", line)
        if list_key:
            in_directories = True
            directories_indent = len(list_key.group(1))
            continue

        if in_directories:
            directory_item = re.match(r"^(\s*)-\s+([^\s#]+)", line)
            if directory_item and len(directory_item.group(1)) > directories_indent:
                entries.append((current, directory_item.group(2).strip("\"'")))
                continue
            if line.strip() and len(line) - len(line.lstrip()) <= directories_indent:
                in_directories = False

    return entries


def check_dependabot_paths(root: Path, errors: list[str]) -> None:
    """Report invalid or unmatched package directories in Dependabot config."""
    path = root / ".github" / "dependabot.yml"
    if not path.is_file():
        return
    entries = parse_dependabot_paths(path.read_text(encoding="utf-8"))
    if not entries:
        errors.append(".github/dependabot.yml has no package directory entries")
        return

    for ecosystem, configured in entries:
        if not configured.startswith("/") or ".." in Path(configured).parts:
            errors.append(
                f"Dependabot {ecosystem} path must be absolute and repository-relative: {configured}"
            )
            continue
        directory = root.joinpath(*[part for part in configured.split("/") if part])
        if not directory.is_dir():
            errors.append(f"Dependabot {ecosystem} path does not exist: {configured}")
            continue

        required = {
            "cargo": lambda: (directory / "Cargo.toml").is_file(),
            "docker": lambda: any(
                item.name.startswith("Dockerfile") for item in directory.iterdir()
            ),
            "docker-compose": lambda: any(
                item.is_file()
                and (
                    item.name.startswith("compose")
                    or item.name.startswith("docker-compose")
                )
                and item.suffix in {".yaml", ".yml"}
                for item in directory.iterdir()
            ),
            "devcontainers": lambda: any(
                candidate.is_file()
                for candidate in (
                    directory / ".devcontainer.json",
                    directory / ".devcontainer" / "devcontainer.json",
                )
            ),
            "github-actions": lambda: (
                (directory / ".github").is_dir()
                or (directory / "action.yml").is_file()
                or (directory / "action.yaml").is_file()
            ),
            "rust-toolchain": lambda: (directory / "rust-toolchain.toml").is_file(),
            "npm": lambda: (directory / "package.json").is_file(),
        }.get(ecosystem)
        if required is not None and not required():
            errors.append(
                f"Dependabot {ecosystem} path has no matching manifest or config: {configured}"
            )


def check_workspace(root: Path, errors: list[str]) -> None:
    """Validate Cargo workspace members and their effective Rust version pins."""
    manifest = root / "Cargo.toml"
    if not manifest.is_file():
        return
    try:
        cargo_toml = read_toml(manifest)
    except (OSError, tomllib.TOMLDecodeError) as error:
        errors.append(f"Cargo.toml is not valid TOML: {error}")
        return

    workspace = cargo_toml.get("workspace", {})
    for member in workspace.get("members", []):
        matches = list(root.glob(member))
        if not matches:
            errors.append(f"Cargo workspace member pattern matches no paths: {member}")
        for match in matches:
            if match.is_dir() and not (match / "Cargo.toml").is_file():
                errors.append(
                    f"Cargo workspace member has no Cargo.toml: {match.relative_to(root)}"
                )

    command = ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"]
    result = subprocess.run(
        command, cwd=root, capture_output=True, text=True, check=False
    )
    if result.returncode:
        detail = result.stderr.strip() or result.stdout.strip()
        errors.append(f"Cargo metadata rejected the workspace: {detail}")
        return

    try:
        metadata = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        errors.append(f"Cargo metadata did not emit valid JSON: {error}")
        return

    # Cargo metadata resolves inherited package fields, so compare its values
    # with the workspace pin instead of inspecting member manifests directly.
    expected_msrv = workspace.get("package", {}).get("rust-version")
    member_ids = set(metadata.get("workspace_members", []))
    packages = [
        package
        for package in metadata.get("packages", [])
        if package.get("id") in member_ids
    ]
    if expected_msrv and not packages:
        errors.append("Cargo metadata returned no packages for the root workspace")
    for package in packages:
        if package.get("rust_version") != expected_msrv:
            errors.append(
                f"{package.get('manifest_path')}: effective rust-version {package.get('rust_version')!r} "
                f"does not match workspace.package.rust-version {expected_msrv!r}"
            )


def check_editor_and_git(root: Path, errors: list[str]) -> None:
    """Check repository-wide editor defaults and Git line-ending attributes."""
    editorconfig = root / ".editorconfig"
    if editorconfig.is_file():
        global_values: dict[str, str] = {}
        in_global = False
        for line in editorconfig.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line or line.startswith(("#", ";")):
                continue
            if line.startswith("[") and line.endswith("]"):
                in_global = line == "[*]"
                continue
            if in_global and "=" in line:
                key, value = line.split("=", 1)
                global_values[key.strip().lower()] = value.strip().lower()
        for key, expected in {
            "charset": "utf-8",
            "end_of_line": "lf",
            "insert_final_newline": "true",
        }.items():
            if global_values.get(key) != expected:
                errors.append(f".editorconfig [*] must set {key}={expected}")

    attributes = root / ".gitattributes"
    if attributes.is_file():
        rules = [
            line.strip() for line in attributes.read_text(encoding="utf-8").splitlines()
        ]
        if "* text=auto eol=lf" not in rules:
            errors.append(".gitattributes must set '* text=auto eol=lf'")


def check_toolchain_pins(root: Path, errors: list[str]) -> None:
    """Ensure container toolchain pins are valid and referenced by consumers."""
    env_path = root / ".devcontainer" / ".env"
    if not env_path.is_file():
        return
    try:
        values = read_env(env_path)
    except (OSError, ValueError) as error:
        errors.append(f".devcontainer/.env is invalid: {error}")
        return

    rustfmt_path = root / "rustfmt-toolchain.toml"
    if rustfmt_path.is_file() and values.get("RUSTFMT_TOOLCHAIN"):
        try:
            rustfmt = read_toml(rustfmt_path).get("toolchain", {}).get("channel")
        except (OSError, tomllib.TOMLDecodeError) as error:
            errors.append(f"rustfmt-toolchain.toml is invalid: {error}")
            rustfmt = None
        if rustfmt and values["RUSTFMT_TOOLCHAIN"] != rustfmt:
            errors.append(
                "RUSTFMT_TOOLCHAIN in .devcontainer/.env disagrees with rustfmt-toolchain.toml"
            )

    rust_path = root / "rust-toolchain.toml"
    image_pin = values.get("RUST_VERSION") or values.get("RUST_IMAGE_LABEL")
    if rust_path.is_file() and image_pin:
        try:
            stable = read_toml(rust_path).get("toolchain", {}).get("channel")
        except (OSError, tomllib.TOMLDecodeError) as error:
            errors.append(f"rust-toolchain.toml is invalid: {error}")
            stable = None
        if stable and image_pin != stable and not image_pin.startswith(f"{stable}-"):
            errors.append(
                f"container Rust pin {image_pin!r} disagrees with rust-toolchain channel {stable!r}"
            )

    # Limit the reference scan to configuration and tooling that can consume
    # these pins; the env file itself cannot count as its own consumer.
    consumers = [
        path
        for directory in (
            root / ".devcontainer",
            root / ".github",
            root / "scripts",
            root / "templates",
        )
        if directory.is_dir()
        for path in directory.rglob("*")
        if path.is_file()
        and (
            path.name == "Dockerfile"
            or path.name.startswith("Dockerfile.")
            or path.suffix.lower()
            in {
                ".yml",
                ".yaml",
                ".toml",
                ".sh",
                ".ps1",
                ".json",
                ".jsonc",
                ".md",
                ".rs",
            }
        )
    ]
    consumer_text = "\n".join(
        path.read_text(encoding="utf-8", errors="replace") for path in consumers
    )
    for key in values:
        if key.endswith(("_VERSION", "_TOOLCHAIN")) or key in {
            "RUST_VERSION",
            "RUST_IMAGE_LABEL",
        }:
            if not re.search(rf"\b{re.escape(key)}\b", consumer_text):
                errors.append(
                    f".devcontainer/.env pin {key} is not referenced by configuration or tooling"
                )


def check_fixture_exceptions(root: Path, errors: list[str]) -> None:
    """Validate declared line-ending fixtures and their formatter exclusions."""
    policy_path = root / ".config" / "config-drift.toml"
    policy = read_toml(policy_path) if policy_path.is_file() else {}
    exceptions = {
        item.get("path"): item for item in policy.get("fixture_exceptions", [])
    }

    attributes = root / ".gitattributes"
    explicit_paths: set[str] = set()
    if attributes.is_file():
        for line in attributes.read_text(encoding="utf-8").splitlines():
            fields = line.split()
            if len(fields) < 2 or not any(
                field in {"-text", "-eol"} for field in fields[1:]
            ):
                continue
            pattern = fields[0]
            if not any(char in pattern for char in "*?[]"):
                explicit_paths.add(pattern.replace("\\", "/"))

    # Check both sides of the policy: every Git exception needs documentation,
    # and every documented exception must still name an existing fixture.
    for path in sorted(explicit_paths | set(exceptions)):
        entry = exceptions.get(path)
        if entry is None or not str(entry.get("reason", "")).strip():
            errors.append(
                f"intentional fixture exception needs a reason in {policy_path.relative_to(root)}: {path}"
            )
            continue
        fixture = root / path
        if not fixture.is_file():
            errors.append(f"configured fixture exception does not exist: {path}")
            continue
        newline = entry.get("newline")
        if newline == "crlf":
            data = fixture.read_bytes()
            if b"\n" not in data or data.replace(b"\r\n", b"").find(b"\n") >= 0:
                errors.append(
                    f"fixture declared CRLF does not contain only CRLF line endings: {path}"
                )

        prek_path = root / "prek.toml"
        if prek_path.is_file():
            try:
                hooks = read_toml(prek_path).get("repos", [])
            except (OSError, tomllib.TOMLDecodeError) as error:
                errors.append(f"prek.toml is invalid: {error}")
                continue
            for hook in (hook for repo in hooks for hook in repo.get("hooks", [])):
                if hook.get("id") in {"end-of-file-fixer", "mixed-line-ending"}:
                    pattern = hook.get("exclude", "")
                    try:
                        excluded = bool(re.search(pattern, path)) if pattern else False
                    except re.error as error:
                        errors.append(
                            f"prek hook {hook.get('id')} has an invalid exclude regex: {error}"
                        )
                        continue
                    if not excluded:
                        errors.append(
                            f"prek hook {hook.get('id')} must exclude intentional fixture {path}"
                        )


def main() -> int:
    """Run configuration drift checks for the supplied or current repository."""
    root = (
        Path(sys.argv[1]).resolve()
        if len(sys.argv) > 1
        else Path(__file__).resolve().parents[1]
    )
    if not root.is_dir():
        print(f"repository root does not exist: {root}", file=sys.stderr)
        return 2

    errors: list[str] = []
    check_workspace(root, errors)
    check_dependabot_paths(root, errors)
    check_editor_and_git(root, errors)
    check_toolchain_pins(root, errors)
    check_fixture_exceptions(root, errors)

    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print(f"Configuration paths and pins are consistent: {root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
