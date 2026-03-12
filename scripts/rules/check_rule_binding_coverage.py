#!/usr/bin/env python3
"""Fail when any rule has no enforcement binding."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def repo_root_from_here() -> Path:
    return Path(__file__).resolve().parents[2]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--binding-index-json",
        default="docs/generated/manifests/rule_binding_index.json",
    )
    args = parser.parse_args()

    root = repo_root_from_here()
    path = root / args.binding_index_json
    data = json.loads(path.read_text(encoding="utf-8"))

    missing = []
    for row in data.get("rules", []):
        code = row.get("rule_code")
        bindings = row.get("bindings", [])
        has_enforcement = any(
            isinstance(b, dict) and b.get("binding_kind") == "enforcement"
            for b in bindings
        )
        if not has_enforcement:
            missing.append(code)

    if missing:
        print(f"ERROR: {len(missing)} rules missing enforcement binding.")
        for code in missing[:100]:
            print(f"- {code}")
        return 1

    print("OK: all rules have at least one enforcement binding.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
