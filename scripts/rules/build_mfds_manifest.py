#!/usr/bin/env python3
"""
Build MFDS-specific manifest JSON from local canonical catalog + official KR sources.

Outputs a single JSON focused on MFDS-only rules so it can be separated from ICH/FDA manifests.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
from pathlib import Path
from typing import Dict, List, Set

from audit_mfds_rule_coverage import (
    extract_core_kr_ids,
    extract_individual_kr_ids,
    load_workbook_rows,
    normalize_mfds_element_id,
)


RULE_BLOCK_RE = re.compile(r"ValidationRuleMetadata\s*\{(.*?)\},", re.S)


def parse_mfds_validation_metadata(catalog_rs: Path) -> List[Dict[str, object]]:
    text = catalog_rs.read_text(encoding="utf-8")
    out: List[Dict[str, object]] = []

    for block in RULE_BLOCK_RE.findall(text):
        if "ValidationProfile::Mfds" not in block:
            continue
        code_m = re.search(r'code:\s*"([^"]+)"', block)
        section_m = re.search(r'section:\s*"([^"]+)"', block)
        blocking_m = re.search(r"blocking:\s*(true|false)", block)
        message_m = re.search(r'message:\s*"([^"]*)"', block, re.S)
        if not code_m:
            continue
        code = code_m.group(1).strip()
        out.append(
            {
                "code": code,
                "profile": "mfds",
                "section": section_m.group(1).strip() if section_m else "unknown",
                "blocking": (blocking_m.group(1) == "true") if blocking_m else False,
                "message": (
                    re.sub(r"\s+", " ", message_m.group(1)).strip()
                    if message_m
                    else code
                ),
            }
        )
    return sorted(out, key=lambda x: str(x["code"]))


def element_id_from_mfds_code(code: str) -> str | None:
    if not code.startswith("MFDS."):
        return None
    elem = code.split("MFDS.", 1)[1]
    elem = re.sub(r"\.(REQUIRED|RECOMMENDED|FORBIDDEN|CONDITIONAL)$", "", elem)
    return normalize_mfds_element_id(elem)


def build_manifest(
    core_xlsx: Path,
    individual_xlsx: Path,
    catalog_rs: Path,
) -> Dict[str, object]:
    core_rows = load_workbook_rows(core_xlsx)
    individual_rows = load_workbook_rows(individual_xlsx)

    core_kr = extract_core_kr_ids(core_rows)
    individual_kr = extract_individual_kr_ids(individual_rows)
    official_kr_elements = sorted(
        {normalize_mfds_element_id(x) for x in (core_kr | individual_kr)}
    )

    rules = parse_mfds_validation_metadata(catalog_rs)
    rule_elements: Set[str] = set()
    for rule in rules:
        eid = element_id_from_mfds_code(str(rule["code"]))
        if eid:
            rule_elements.add(eid)

    official_kr_upper = {x.upper() for x in official_kr_elements}
    rule_elements_upper = {x.upper() for x in rule_elements}
    missing_official_kr = sorted(
        x for x in official_kr_elements if x.upper() not in rule_elements_upper
    )
    local_only_elements = sorted(
        x for x in rule_elements if x.upper() not in official_kr_upper
    )

    return {
        "manifest": "mfds",
        "generated_at_utc": dt.datetime.now(dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat(),
        "sources": {
            "catalog_rs": str(catalog_rs),
            "core_xlsx": str(core_xlsx),
            "individual_xlsx": str(individual_xlsx),
        },
        "official_kr_elements": official_kr_elements,
        "official_kr_count": len(official_kr_elements),
        "mfds_rules": rules,
        "mfds_rule_count": len(rules),
        "mfds_rule_elements": sorted(rule_elements),
        "mfds_rule_element_count": len(rule_elements),
        "missing_official_kr_elements_in_mfds_rules": missing_official_kr,
        "missing_official_kr_count": len(missing_official_kr),
        "local_only_mfds_elements_not_in_official_kr_set": local_only_elements,
        "local_only_count": len(local_only_elements),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Build MFDS-specific manifest JSON.")
    parser.add_argument("--core-xlsx", required=True, type=Path)
    parser.add_argument("--individual-xlsx", required=True, type=Path)
    parser.add_argument(
        "--catalog-rs",
        default=Path("crates/libs/lib-core/src/xml/validate/catalog.rs"),
        type=Path,
    )
    parser.add_argument("--out-json", required=True, type=Path)
    args = parser.parse_args()

    manifest = build_manifest(args.core_xlsx, args.individual_xlsx, args.catalog_rs)
    args.out_json.parent.mkdir(parents=True, exist_ok=True)
    args.out_json.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    print(
        json.dumps(
            {
                "out_json": str(args.out_json),
                "mfds_rule_count": manifest["mfds_rule_count"],
                "official_kr_count": manifest["official_kr_count"],
                "missing_official_kr_count": manifest[
                    "missing_official_kr_count"
                ],
            },
            ensure_ascii=False,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
