#!/usr/bin/env python3
"""Check validator-owned mappings and actual Cargo rule test results."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re

CRATE = Path(__file__).resolve().parents[1]
ROOT = CRATE.parents[2]
from rule_inventory import discover_business_rule_codes


def check_catalog(crate: Path = CRATE, root: Path = ROOT) -> set[str]:
    catalog = json.loads((crate / "rules.json").read_text())
    if catalog["schemaVersion"] != 2:
        raise ValueError("unsupported validator rule catalog schema")
    rules = catalog["rules"]
    codes = [rule["ruleCode"] for rule in rules]
    implemented = discover_business_rule_codes(root)
    if len(codes) != len(set(codes)) or set(codes) != implemented:
        raise ValueError(
            f"rule catalog differs from implemented baseline: "
            f"missing={sorted(implemented - set(codes))}, "
            f"extra={sorted(set(codes) - implemented)}, duplicates={len(codes) - len(set(codes))}"
        )
    tests = set()
    for rule in rules:
        name = "rule_" + re.sub(r"[^a-z0-9]", "_", rule["ruleCode"].lower())
        full_name = f"{rule['testModule']}::{name}"
        if rule["testName"] != name or full_name in tests:
            raise ValueError(f"invalid/duplicate test binding: {rule['ruleCode']}")
        tests.add(full_name)
        if "scenarioId" in rule or "mutationTarget" in rule:
            raise ValueError(f"rule tests must not bind fuzzer scenarios: {rule['ruleCode']}")
        if rule["expectedIssueCode"] not in {rule["ruleCode"], "ICH.MEDDRA.VERSION.UNAVAILABLE"}:
            raise ValueError(f"invalid current issue-code binding: {rule['ruleCode']}")
        paths = rule["expectedIssuePaths"]
        if not paths or len(paths) != len(set(paths)) or any(not isinstance(p, str) or not p for p in paths):
            raise ValueError(f"rule requires unique tested field paths: {rule['ruleCode']}")
        module = re.fullmatch(
            r"case::sections::[cdefgh]::rule_tests::(ich|fda|mfds)",
            rule["testModule"],
        )
        if module is None or module[1] != rule["ruleCode"].split(".", 1)[0].lower():
            raise ValueError(f"invalid Rust test module: {rule['ruleCode']}")

    editor_fields = set()
    editor_pages = set()
    for path in sorted((root / "registry/editor-contracts").glob("*.json")):
        if path.name == "schema.json":
            continue
        page = json.loads(path.read_text())
        editor_pages.add(page["pageId"])
        for field in page["fields"]:
            if "businessValidation" in field:
                raise ValueError(f"businessValidation belongs in validator: {path}:{field['code']}")
            editor_fields.add((page["pageId"], field["authority"], field["code"], field["frontendPath"]))
    seen = set()
    mapped_pages = set()
    target_pairs = set()
    target_fields = set()
    rules_by_code = {rule["ruleCode"]: rule for rule in rules}
    for path in sorted((crate / "field-rules").glob("*.json")):
        page = json.loads(path.read_text())
        if page["pageId"] in mapped_pages:
            raise ValueError(f"duplicate validator field mapping page: {path}")
        mapped_pages.add(page["pageId"])
        for target in page["fields"]:
            field_path = target["fieldPath"]
            key = (page["pageId"], field_path)
            if key in target_fields:
                raise ValueError(f"duplicate field mapping: {key}")
            target_fields.add(key)
            codes_for_field = target["ruleCodes"]
            if not codes_for_field or len(codes_for_field) != len(set(codes_for_field)):
                raise ValueError(f"field mapping requires unique rules: {field_path}")
            for code in codes_for_field:
                if code not in rules_by_code or field_path not in rules_by_code[code]["expectedIssuePaths"]:
                    raise ValueError(f"field/rule mapping mismatch: {field_path}: {code}")
                if (code, field_path) in target_pairs:
                    raise ValueError(f"duplicate field/rule mapping: {field_path}: {code}")
                target_pairs.add((code, field_path))
        for field in page["editorMappings"]:
            key = (page["pageId"], field["authority"], field["code"], field["frontendPath"])
            if key in seen or key not in editor_fields:
                raise ValueError(f"duplicate/orphan validator field mapping: {key}")
            seen.add(key)
            stage = field["businessValidation"]
            status = stage["status"]
            if status == "verified":
                if not stage.get("issuePath"):
                    raise ValueError(f"verified mapping requires issuePath: {key}")
            elif status not in {"blocked", "not_applicable"} or not stage.get("reason"):
                raise ValueError(f"invalid validator field mapping status/reason: {key}")
    if mapped_pages != editor_pages:
        raise ValueError("validator field mapping pages differ from editor pages")
    expected_pairs = {(rule["ruleCode"], path) for rule in rules for path in rule["expectedIssuePaths"]}
    if target_pairs != expected_pairs:
        raise ValueError(f"field mappings do not cover rule paths: {sorted(expected_pairs - target_pairs)}")
    return tests


def check_test_log(text: str, expected: set[str]) -> None:
    results: dict[str, list[str]] = {}
    for full_name, result in re.findall(r"^test (\S+) \.\.\. (\S+)", text, re.MULTILINE):
        name = full_name.rsplit("::", 1)[-1]
        if name.startswith("rule_"):
            results.setdefault(full_name, []).append(result)
    if set(results) != expected:
        raise ValueError(f"Cargo rule test coverage mismatch: missing={sorted(expected - set(results))}, "
                         f"extra={sorted(set(results) - expected)}")
    bad = {name: statuses for name, statuses in results.items() if statuses != ["ok"]}
    if bad:
        raise ValueError(f"rule tests must execute and pass exactly once: {bad}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--test-log", type=Path, help="Cargo output; requires every rule test to pass")
    args = parser.parse_args()
    expected = check_catalog()
    if args.test_log is not None:
        check_test_log(args.test_log.read_text(), expected)
    print(f"validator rule tests: {len(expected)} bindings checked"
          + ("; every Rust rule test passed" if args.test_log is not None else ""))


if __name__ == "__main__":
    main()
