#!/usr/bin/env python3
"""Verify shared versions after release-plz updates a materialized checkout."""

import argparse
from pathlib import Path
import tomllib

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("root", nargs="?", type=Path, default=Path(__file__).resolve().parent.parent)
root = parser.parse_args().root.resolve()
workspace = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]
version = workspace["package"]["version"]
lockfile = tomllib.loads((root / "Cargo.lock").read_text())
packages = {}

for member in workspace["members"]:
    manifest = tomllib.loads((root / member / "Cargo.toml").read_text())
    package = manifest["package"]
    name = package["name"]
    assert package["version"] == {"workspace": True}, f"{name} must inherit the workspace version"
    locked = [entry["version"] for entry in lockfile["package"] if entry["name"] == name]
    assert locked == [version], f"{name}: lockfile has {locked}, expected {version}"
    if package.get("publish") is not False:
        headings = [line for line in (root / member / "CHANGELOG.md").read_text().splitlines()
                    if line.startswith("## ")]
        assert headings and headings[0].startswith(f"## [{version}] - "), f"{name}: stale release notes"
    packages[member] = name

for name, dependency in workspace["dependencies"].items():
    if isinstance(dependency, dict) and dependency.get("path") in packages:
        assert dependency["version"] == version, f"{name}: internal requirement differs from {version}"

print(f"All {len(packages)} crates inherit {version}; lockfile, internal requirements and publishable notes agree")
