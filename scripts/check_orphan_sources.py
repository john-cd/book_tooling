#!/usr/bin/env python3

# TODO Reviewed ??-??-2026.

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

MODULE_DECLARATION = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;"
)


def workspace_members(root: Path) -> list[Path]:
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    metadata = json.loads(result.stdout)
    workspace_ids = set(metadata["workspace_members"])
    members = [
        Path(package["manifest_path"]).parent
        for package in metadata["packages"]
        if package["id"] in workspace_ids
    ]
    return sorted(members)


def crate_roots(src_dir: Path) -> list[Path]:
    roots: list[Path] = []
    for entry_name in ("lib.rs", "main.rs"):
        entry = src_dir / entry_name
        if entry.exists():
            roots.append(entry)

    bin_dir = src_dir / "bin"
    if not bin_dir.exists():
        return roots

    for child in sorted(bin_dir.iterdir()):
        if child.is_file() and child.suffix == ".rs":
            roots.append(child)
        elif child.is_dir():
            main_rs = child / "main.rs"
            if main_rs.exists():
                roots.append(main_rs)

    return roots


def module_search_dir(module_file: Path, is_root: bool) -> Path:
    if is_root or module_file.name == "mod.rs":
        return module_file.parent
    return module_file.parent / module_file.stem


def resolve_child_module(module_file: Path, module_name: str, is_root: bool) -> Path | None:
    base_dir = module_search_dir(module_file, is_root)
    flat_file = base_dir / f"{module_name}.rs"
    nested_file = base_dir / module_name / "mod.rs"

    if flat_file.exists():
        return flat_file
    if nested_file.exists():
        return nested_file
    return None


def walk_reachable_sources(crate_dir: Path) -> tuple[set[Path], list[str]]:
    src_dir = crate_dir / "src"
    reachable: set[Path] = set()
    errors: list[str] = []
    pending = [(root, True) for root in crate_roots(src_dir)]

    while pending:
        module_file, is_root = pending.pop()
        module_file = module_file.resolve()

        if module_file in reachable:
            continue

        reachable.add(module_file)
        for line_number, line in enumerate(module_file.read_text(encoding="utf-8").splitlines(), start=1):
            match = MODULE_DECLARATION.match(line)
            if not match:
                continue

            module_name = match.group(1)
            child = resolve_child_module(module_file, module_name, is_root)
            if child is None:
                errors.append(
                    f"{module_file.relative_to(crate_dir)}:{line_number} declares module `{module_name}` "
                    f"but neither {module_search_dir(module_file, is_root).joinpath(f'{module_name}.rs').relative_to(crate_dir)} "
                    f"nor {module_search_dir(module_file, is_root).joinpath(module_name, 'mod.rs').relative_to(crate_dir)} exists"
                )
                continue

            pending.append((child, False))

    return reachable, errors


def main() -> int:
    repo_root = Path(__file__).resolve().parent.parent
    failures: list[str] = []

    for crate_dir in workspace_members(repo_root):
        src_dir = crate_dir / "src"
        if not src_dir.exists():
            continue

        reachable, resolution_errors = walk_reachable_sources(crate_dir)
        failures.extend(f"{crate_dir.name}: {error}" for error in resolution_errors)

        all_sources = {path.resolve() for path in src_dir.rglob("*.rs")}
        orphaned = sorted(path.relative_to(crate_dir) for path in all_sources - reachable)
        failures.extend(f"{crate_dir.name}: orphan source {path}" for path in orphaned)

    if failures:
        print("orphan source check failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1

    print("No orphan Rust source files found under workspace src/ trees.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
