#!/usr/bin/env python3
"""Exercise release-plz's git-cliff filters and template without registry access."""

import json
from pathlib import Path
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parent.parent
CONFIG_PATH = tomllib.loads((ROOT / "release-plz.toml").read_text())["workspace"]["changelog_config"]
MESSAGES = [
    "chore: release v0.0.36 (#209)",
    "chore(release): prepare v0.1.0",
    "chore(deps): bump actions/checkout from 6 to 7",
    "chore(deps): bump the cargo-dependencies group with 4 updates (#214)",
    "Add CORS to WebFinger error responses (#197)",
    "docs: explain host resolution",
    "refactor: clarify Resource host modeling",
    "test: improve coverage",
    "ci!: require a newer Rust toolchain",
]


def run(*args, cwd):
    return subprocess.check_output(args, cwd=cwd, text=True, stderr=subprocess.PIPE)


with tempfile.TemporaryDirectory(prefix="webfinger-changelog-") as directory:
    temp = Path(directory)
    config = temp / "cliff.toml"
    config.write_text((ROOT / CONFIG_PATH).read_text())
    run("git", "init", "--quiet", cwd=temp)
    run("git", "config", "user.name", "Changelog fixture", cwd=temp)
    run("git", "config", "user.email", "fixture@example.invalid", cwd=temp)
    for message in MESSAGES:
        run("git", "-c", "commit.gpgsign=false", "commit", "--quiet", "--allow-empty", "-m", message, cwd=temp)
    context = json.loads(run("git-cliff", "--config", str(config), "--context", cwd=temp))
    commits = context[0]["commits"]
    assert len(commits) == 5, [commit["message"] for commit in commits]
    # release-plz can supply the same commit through multiple package traversals.
    commits.append(commits[0].copy())
    context_file = temp / "context.json"
    context_file.write_text(json.dumps(context))
    output = run("git-cliff", "--config", str(config), "--from-context", str(context_file), cwd=temp)
    assert output.count("Bump the cargo-dependencies") == 1, output
    assert "Add CORS" in output and "Resource host modeling" in output, output
    assert "Documentation" in output and "Explain host resolution" in output, output
    assert "**Breaking:**" in output and "newer Rust toolchain" in output, output
    assert "release v0.0.36" not in output and "actions/checkout" not in output, output
    assert "Improve coverage" not in output, output
    assert "/compare/" not in output, output
    assert output.endswith("\n\n"), repr(output[-100:])
    context[0]["commits"] = []
    context[0]["version"] = "0.1.1"
    context[0]["timestamp"] = 1791676800
    context_file.write_text(json.dumps(context))
    empty = run("git-cliff", "--config", str(config), "--from-context", str(context_file), cwd=temp)
    assert "## [0.1.1]" in empty and "Shared workspace version bump" in empty, empty
    assert empty.endswith("\n\n"), repr(empty)
print("Changelog filtering, breaking changes, unconventional subjects and deduplication passed")
