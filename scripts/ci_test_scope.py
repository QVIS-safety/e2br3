#!/usr/bin/env python3
"""Select validator and remaining backend CI tests from the complete Git diff."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess

VALIDATOR = "crates/libs/validator/"
# Includes transitive Rust dependencies and the validator's test/tool inputs.
SHARED_PREFIXES = (
    "crates/libs/lib-core/",
    "crates/libs/lib-auth/",
    "crates/libs/lib-utils/",
    "crates/libs/input-contracts/",
    "registry/editor-contracts/",
    "registry/sections/",
    "registry/vocabularies/",
    ".cargo/",
)
SHARED_FILES = {
    "Cargo.toml", "Cargo.lock", "rust-toolchain", "rust-toolchain.toml",
    "rustfmt.toml", ".rustfmt.toml", ".github/workflows/ci.yml",
    "scripts/ci_test_scope.py", "scripts/tests/test_ci_test_scope.py",
}


def select_tests(paths: list[str]) -> dict[str, bool]:
    return {
        "validator": any(
            path.startswith((VALIDATOR, *SHARED_PREFIXES)) or path in SHARED_FILES
            for path in paths
        ),
        "backend": any(not path.startswith(VALIDATOR) for path in paths),
    }


def changed_paths(base: str, head: str, pull_request: bool, root: Path) -> list[str]:
    def git(*args: str) -> bytes:
        return subprocess.check_output(["git", *args], cwd=root)

    if base == "0" * 40:
        if pull_request:
            raise ValueError("pull request requires a real base commit")
        raw = git("ls-tree", "-r", "--name-only", "-z", head)
    else:
        if pull_request:
            base = git("merge-base", base, head).decode().strip()
        # Both sides of moves matter when files cross the validator boundary.
        raw = git("diff", "--no-ext-diff", "--no-renames", "--name-only", "-z", base, head, "--")
    return [os.fsdecode(path) for path in raw.split(b"\0") if path]


def main() -> None:
    event_name = os.environ["GITHUB_EVENT_NAME"]
    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text())
    if event_name == "pull_request":
        base = event["pull_request"]["base"]["sha"]
        head = event["pull_request"]["head"]["sha"]
    elif event_name == "push":
        base, head = event["before"], event["after"]
    else:
        raise ValueError(f"unsupported CI event: {event_name}")
    paths = changed_paths(base, head, event_name == "pull_request", Path.cwd())
    selected = select_tests(paths)
    with Path(os.environ["GITHUB_OUTPUT"]).open("a") as output:
        for suite, enabled in selected.items():
            output.write(f"{suite}={str(enabled).lower()}\n")
    print(json.dumps({"base": base, "head": head, "paths": paths, "tests": selected}, indent=2))


if __name__ == "__main__":
    main()
