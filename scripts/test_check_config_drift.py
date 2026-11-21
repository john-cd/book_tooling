# TODO Reviewed ??-??-2026.
"""Regression tests for container toolchain drift detection."""

import tempfile
import unittest
from pathlib import Path

from check_config_drift import check_toolchain_pins


class ToolchainPinTests(unittest.TestCase):
    """Checks version matching independently of the current workspace pins."""

    def test_template_dockerfiles_count_as_pin_consumers(self):
        """Recognizes Dockerfile names without relying on a file extension."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            devcontainer = root / ".devcontainer"
            devcontainer.mkdir()
            (devcontainer / ".env").write_text(
                "CARGO_CHEF_VERSION=0.1.78\nUNUSED_VERSION=1\n", encoding="utf-8"
            )
            template = root / "templates" / "dockerfile"
            template.mkdir(parents=True)
            (template / "Dockerfile").write_text(
                "ARG CARGO_CHEF_VERSION\n", encoding="utf-8"
            )
            errors: list[str] = []
            check_toolchain_pins(root, errors)
            self.assertEqual(
                errors,
                [
                    ".devcontainer/.env pin UNUSED_VERSION is not referenced by configuration or tooling"
                ],
            )

    def test_rust_image_must_match_the_full_toolchain_version(self):
        """Allows a distribution suffix and rejects neighboring version prefixes."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            devcontainer = root / ".devcontainer"
            devcontainer.mkdir()
            (root / "rust-toolchain.toml").write_text(
                '[toolchain]\nchannel = "1.100.1"\n', encoding="utf-8"
            )
            (devcontainer / "compose.yml").write_text(
                "image: rust:${RUST_VERSION}\n", encoding="utf-8"
            )
            for pin, should_pass in (
                ("1.100.1", True),
                ("1.100.1-slim", True),
                ("1.100.10", False),
            ):
                with self.subTest(pin=pin):
                    (devcontainer / ".env").write_text(
                        f"RUST_VERSION={pin}\n", encoding="utf-8"
                    )
                    errors: list[str] = []
                    check_toolchain_pins(root, errors)
                    if should_pass:
                        self.assertEqual(errors, [])
                    else:
                        self.assertEqual(len(errors), 1)
                        self.assertIn("disagrees with rust-toolchain", errors[0])
