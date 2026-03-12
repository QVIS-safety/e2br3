#!/usr/bin/env python3
"""Triage extracted ICH candidate rows against canonical ich_rules."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Dict, List, Optional, Tuple


SUFFIXES = ("REQUIRED", "RECOMMENDED", "FORBIDDEN", "CONDITIONAL")


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def split_code(code: str) -> Tuple[Optional[str], Optional[str]]:
    if not code.startswith("ICH."):
        return None, None
    body = code[4:]
    for suffix in SUFFIXES:
        needle = f".{suffix}"
        if body.endswith(needle):
            return body[: -len(needle)], suffix
    return body, None


def norm_token(s: str) -> str:
    return re.sub(r"[^A-Z0-9]", "", s.upper())


def build_canonical_indexes(ich_rules: List[dict]) -> Tuple[Dict[str, List[str]], Dict[str, List[str]], set[str]]:
    by_element_exact: Dict[str, List[str]] = {}
    by_element_norm: Dict[str, List[str]] = {}
    canonical_codes: set[str] = set()
    for row in ich_rules:
        code = str(row.get("code", "")).strip()
        if not code:
            continue
        canonical_codes.add(code)
        element, _suffix = split_code(code)
        if not element:
            continue
        by_element_exact.setdefault(element.upper(), []).append(code)
        by_element_norm.setdefault(norm_token(element), []).append(code)
    return by_element_exact, by_element_norm, canonical_codes


def triage_workbook_rows(
    rows: List[dict],
    by_element_exact: Dict[str, List[str]],
    by_element_norm: Dict[str, List[str]],
) -> List[dict]:
    out = []
    for row in rows:
        tag = str(row.get("tag_key", "")).strip()
        if not tag:
            continue
        tag_upper = tag.upper()
        tag_no_ich = tag_upper[4:] if tag_upper.startswith("ICH.") else tag_upper

        severity = str(row.get("severity", "")).strip().lower()
        message = str(row.get("message", "")).strip()
        if tag_upper.startswith("ICH.ACK."):
            status = "non_actionable_ack_row"
            exact = []
            norm = []
        elif severity == "" and message in {"", "-"}:
            status = "non_actionable_data_element_row"
            exact = []
            norm = []
        else:
            exact = by_element_exact.get(tag_no_ich, [])
            norm = by_element_norm.get(norm_token(tag_no_ich), [])
            status = "needs_new_canonical_rule"
        if exact:
            status = "covered_exact_element"
        elif norm:
            status = "covered_normalized_element"

        out.append(
            {
                "source": "workbook_ich_rules",
                "tag_key": tag,
                "sheet": row.get("sheet"),
                "severity": severity,
                "status": status,
                "matched_canonical_codes": sorted(set(exact or norm)),
                "message": message,
            }
        )
    return out


def triage_pdf_rows(
    rows: List[dict],
    canonical_codes: set[str],
    by_element_exact: Dict[str, List[str]],
    by_element_norm: Dict[str, List[str]],
) -> List[dict]:
    out = []
    for row in rows:
        element_id = str(row.get("element_id", "")).strip()
        if not element_id:
            continue
        candidate = row.get("rule_code_candidate")
        conformance = str(row.get("conformance_hint", "")).strip().lower()

        matched: List[str] = []
        status = "needs_new_canonical_rule"
        if isinstance(candidate, str) and candidate in canonical_codes:
            status = "covered_exact_code"
            matched = [candidate]
        else:
            exact = by_element_exact.get(element_id.upper(), [])
            norm = by_element_norm.get(norm_token(element_id), [])
            if exact:
                status = "covered_exact_element"
                matched = sorted(set(exact))
            elif norm:
                status = "covered_normalized_element"
                matched = sorted(set(norm))

        if conformance in {"optional", "unspecified", ""} and status == "needs_new_canonical_rule":
            status = "non_actionable_guidance_row"

        out.append(
            {
                "source": "mfds_pdf_ich_guidance_rules",
                "element_id": element_id,
                "rule_code_candidate": candidate,
                "conformance_hint": conformance,
                "status": status,
                "matched_canonical_codes": matched,
                "source_line": row.get("source_line"),
                "confidence": row.get("confidence"),
            }
        )
    return out


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--ich-manifest-json",
        default="docs/generated/manifests/ich.rules.2026-03-07.json",
        type=Path,
    )
    parser.add_argument(
        "--out-json",
        default="docs/generated/manifests/ich.extracted_candidates.triage.json",
        type=Path,
    )
    parser.add_argument(
        "--out-md",
        default="docs/generated/manifests/ich.extracted_candidates.triage.md",
        type=Path,
    )
    args = parser.parse_args()

    manifest = load_json(args.ich_manifest_json)
    ich_rules = manifest.get("ich_rules", [])
    workbook_rows = manifest.get("workbook_ich_rules", [])
    pdf_rows = manifest.get("mfds_pdf_ich_guidance_rules", [])

    by_exact, by_norm, canonical_codes = build_canonical_indexes(ich_rules)
    workbook_triage = triage_workbook_rows(workbook_rows, by_exact, by_norm)
    pdf_triage = triage_pdf_rows(pdf_rows, canonical_codes, by_exact, by_norm)
    triage_rows = workbook_triage + pdf_triage

    counts: Dict[str, int] = {}
    for row in triage_rows:
        key = str(row["status"])
        counts[key] = counts.get(key, 0) + 1

    payload = {
        "source_manifest": str(args.ich_manifest_json),
        "summary": {
            "canonical_ich_rule_count": len(ich_rules),
            "workbook_ich_row_count": len(workbook_rows),
            "mfds_pdf_ich_guidance_row_count": len(pdf_rows),
            "triage_status_counts": counts,
            "needs_new_canonical_rule_count": counts.get("needs_new_canonical_rule", 0),
        },
        "rows": triage_rows,
    }

    args.out_json.parent.mkdir(parents=True, exist_ok=True)
    args.out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    needs = [r for r in triage_rows if r["status"] == "needs_new_canonical_rule"]
    lines = [
        "# ICH Extracted Candidate Triage",
        "",
        f"- canonical_ich_rule_count: {len(ich_rules)}",
        f"- workbook_ich_row_count: {len(workbook_rows)}",
        f"- mfds_pdf_ich_guidance_row_count: {len(pdf_rows)}",
        f"- needs_new_canonical_rule_count: {len(needs)}",
        "",
        "## Status Counts",
    ]
    for k in sorted(counts.keys()):
        lines.append(f"- {k}: {counts[k]}")
    lines.append("")
    lines.append("## Top 100 Needs-New Candidates")
    for row in needs[:100]:
        ident = row.get("tag_key") or row.get("element_id") or "UNKNOWN"
        lines.append(f"- {row['source']} | {ident} | conformance={row.get('conformance_hint')} | candidate={row.get('rule_code_candidate')}")
    lines.append("")
    args.out_md.parent.mkdir(parents=True, exist_ok=True)
    args.out_md.write_text("\n".join(lines) + "\n", encoding="utf-8")

    print(json.dumps(payload["summary"], ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
