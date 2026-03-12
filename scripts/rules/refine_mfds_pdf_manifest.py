#!/usr/bin/env python3
"""
Refine MFDS PDF draft manifest into implementable leaf candidates.

Input:
- JSON created by `extract_mfds_pdf_manifest.py`

Output:
- Refined JSON and markdown summary for implementation planning.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
from pathlib import Path
from typing import Dict, List


LEAF_RE = re.compile(r"^[NCDEFGH]\.(?:\d+|[a-z])(?:\.[0-9a-z]+)*$")
GROUP_CONTAINER_RE = re.compile(r"^[NCDEFGH]\.[0-9]+\.r\.[0-9]+$")


def is_container_id(element_id: str) -> bool:
    parts = element_id.split(".")
    if len(parts) <= 2:
        return True
    if element_id.endswith(".r"):
        return True
    if GROUP_CONTAINER_RE.match(element_id):
        return True
    return False


def priority_from_hint(hint: str) -> str:
    if hint in {"required", "conditional_required"}:
        return "high"
    if hint == "conditional":
        return "medium"
    return "low"


def refine_rules(rules: List[Dict[str, object]]) -> List[Dict[str, object]]:
    candidates: List[Dict[str, object]] = []
    for rec in rules:
        element_id = str(rec.get("element_id", "")).strip()
        if not element_id:
            continue
        if not LEAF_RE.match(element_id):
            continue
        if is_container_id(element_id):
            continue

        out = dict(rec)
        hint = str(out.get("conformance_hint", "unspecified"))
        out["implementation_priority"] = priority_from_hint(hint)
        out["manifest_track"] = "implementable_leaf_candidate"
        candidates.append(out)

    best: Dict[str, Dict[str, object]] = {}
    for rec in candidates:
        eid = str(rec["element_id"])
        prev = best.get(eid)
        if prev is None:
            best[eid] = rec
            continue
        prev_conf = float(prev.get("confidence", 0.0))
        cur_conf = float(rec.get("confidence", 0.0))
        if cur_conf > prev_conf:
            best[eid] = rec
            continue
        if cur_conf == prev_conf:
            prev_line = int(prev["source_ref"]["line_number_1_based"])
            cur_line = int(rec["source_ref"]["line_number_1_based"])
            if cur_line < prev_line:
                best[eid] = rec

    return sorted(best.values(), key=lambda item: str(item["element_id"]))


def summarize(rules: List[Dict[str, object]]) -> Dict[str, int]:
    return {
        "total_implementable_candidates": len(rules),
        "high_priority": sum(
            1 for r in rules if r.get("implementation_priority") == "high"
        ),
        "medium_priority": sum(
            1 for r in rules if r.get("implementation_priority") == "medium"
        ),
        "low_priority": sum(
            1 for r in rules if r.get("implementation_priority") == "low"
        ),
    }


def to_markdown(
    source_manifest: Path, refined_rules: List[Dict[str, object]], summary: Dict[str, int]
) -> str:
    lines: List[str] = []
    lines.append("# MFDS PDF Implementable Manifest Draft")
    lines.append("")
    lines.append(
        f"- Generated at (UTC): `{dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()}`"
    )
    lines.append(f"- Source manifest: `{source_manifest}`")
    lines.append("")
    lines.append("## Summary")
    lines.append("")
    for key, value in summary.items():
        lines.append(f"- {key.replace('_', ' ')}: `{value}`")
    lines.append("")
    lines.append("## First 60 Candidate IDs")
    lines.append("")
    for rec in refined_rules[:60]:
        lines.append(
            f"- `{rec['element_id']}` ({rec.get('conformance_hint', 'unspecified')}, {rec['implementation_priority']})"
        )
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Refine MFDS PDF draft manifest into implementable candidates."
    )
    parser.add_argument("--in-json", required=True, type=Path)
    parser.add_argument("--out-json", required=True, type=Path)
    parser.add_argument("--out-md", required=True, type=Path)
    args = parser.parse_args()

    payload = json.loads(args.in_json.read_text(encoding="utf-8"))
    source_rules = payload.get("rules", [])
    if not isinstance(source_rules, list):
        raise ValueError("input JSON missing 'rules' list")

    refined = refine_rules(source_rules)
    summary = summarize(refined)

    out_payload = {
        "generated_at_utc": dt.datetime.now(dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat(),
        "source_manifest": str(args.in_json),
        "summary": summary,
        "rules": refined,
    }

    args.out_json.parent.mkdir(parents=True, exist_ok=True)
    args.out_md.parent.mkdir(parents=True, exist_ok=True)
    args.out_json.write_text(
        json.dumps(out_payload, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    args.out_md.write_text(
        to_markdown(args.in_json, refined, summary),
        encoding="utf-8",
    )

    print(
        json.dumps(
            {
                "out_json": str(args.out_json),
                "out_md": str(args.out_md),
                "summary": summary,
            },
            ensure_ascii=False,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
