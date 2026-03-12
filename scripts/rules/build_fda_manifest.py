#!/usr/bin/env python3
"""
Build FDA-only manifest JSON from available local sources.

Includes:
- Extracted FDA workbook rows (normalized JSON)
- Catalog-backed runtime FDA profile rules
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
from pathlib import Path
from typing import Dict, List, Tuple


RULE_BLOCK_RE = re.compile(r"ValidationRuleMetadata\s*\{(.*?)\},", re.S)
CONDITION_BINDING_RE = re.compile(
    r'ConditionBinding\s*\{\s*code:\s*"([^"]+)"\s*,\s*condition:\s*RuleCondition::([A-Za-z0-9_]+)\s*,\s*\}',
    re.S,
)


def parse_condition_bindings(catalog_text: str) -> Dict[str, str]:
    out: Dict[str, str] = {}
    for code, condition in CONDITION_BINDING_RE.findall(catalog_text):
        out[code] = condition
    return out


def parse_catalog_fda_rules(catalog_rs: Path) -> List[Dict[str, object]]:
    text = catalog_rs.read_text(encoding="utf-8")
    conditions = parse_condition_bindings(text)
    fda_rules: List[Dict[str, object]] = []

    for block in RULE_BLOCK_RE.findall(text):
        profile_m = re.search(r"profile:\s*ValidationProfile::(\w+)", block)
        code_m = re.search(r'code:\s*"([^"]+)"', block)
        section_m = re.search(r'section:\s*"([^"]+)"', block)
        blocking_m = re.search(r"blocking:\s*(true|false)", block)
        message_m = re.search(r'message:\s*"([^"]*)"', block, re.S)
        if not profile_m or not code_m:
            continue

        profile = profile_m.group(1)
        if profile != "Fda":
            continue

        code = code_m.group(1).strip()
        section = section_m.group(1).strip() if section_m else "unknown"
        blocking = (blocking_m.group(1) == "true") if blocking_m else False
        message = (
            re.sub(r"\s+", " ", message_m.group(1)).strip()
            if message_m
            else code
        )
        rec = {
            "code": code,
            "profile": "fda" if profile == "Fda" else "ich",
            "section": section,
            "blocking": blocking,
            "message": message,
            "condition": conditions.get(code, "Always"),
        }

        fda_rules.append(rec)

    fda_rules.sort(key=lambda r: str(r["code"]))
    return fda_rules


def unique_tag_keys(workbook_rules: List[Dict[str, object]]) -> List[str]:
    keys = {
        str(r.get("tag_key", "")).strip()
        for r in workbook_rules
        if str(r.get("tag_key", "")).strip()
    }
    return sorted(keys)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Build consolidated FDA rules manifest JSON."
    )
    parser.add_argument(
        "--workbook-json",
        default=Path("docs/generated/manifests/fda.core_regional_rules.extracted.2026-03-07.json"),
        type=Path,
    )
    parser.add_argument(
        "--catalog-rs",
        default=Path("crates/libs/lib-core/src/xml/validate/catalog.rs"),
        type=Path,
    )
    parser.add_argument(
        "--out-json",
        default=Path("docs/generated/manifests/fda.rules.json"),
        type=Path,
    )
    args = parser.parse_args()

    wb = json.loads(args.workbook_json.read_text(encoding="utf-8"))
    workbook_rules_all = wb.get("rules", [])
    if not isinstance(workbook_rules_all, list):
        raise ValueError("workbook json missing 'rules' list")
    workbook_rules = [
        r
        for r in workbook_rules_all
        if isinstance(r, dict)
        and str(r.get("profile", "")).strip().lower() == "fda"
    ]

    catalog_fda_rules = parse_catalog_fda_rules(args.catalog_rs)
    wb_tag_keys = unique_tag_keys(workbook_rules)

    manifest = {
        "manifest": "fda",
        "generated_at_utc": dt.datetime.now(dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat(),
        "sources": {
            "workbook_json": str(args.workbook_json),
            "catalog_rs": str(args.catalog_rs),
        },
        "summary": {
            "workbook_row_count": len(workbook_rules),
            "workbook_profile_counts_original": wb.get("counts_by_profile", {}),
            "workbook_profile_counts_included": {"fda": len(workbook_rules)},
            "workbook_unique_tag_key_count": len(wb_tag_keys),
            "catalog_fda_rule_count": len(catalog_fda_rules),
            "catalog_total_rules": len(catalog_fda_rules),
        },
        "workbook_rules": workbook_rules,
        "workbook_unique_tag_keys": wb_tag_keys,
        "catalog_rules": {"fda": catalog_fda_rules},
    }

    args.out_json.parent.mkdir(parents=True, exist_ok=True)
    args.out_json.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    print(
        json.dumps(
            {
                "out_json": str(args.out_json),
                **manifest["summary"],
            },
            ensure_ascii=False,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
