#!/usr/bin/env python3
"""
Extract a draft MFDS rule manifest from the Korean E2B(R3) guidance PDF.

This script is intentionally conservative:
- It uses `pdftotext -layout` for stable line extraction.
- It extracts element-id headed blocks (e.g., C.1.1, D.8.r.2a).
- It infers conformance hints from nearby lines (적합성 + 필수/선택/조건).
- It emits provenance for each record (line number + excerpt + source file).

Output is a *draft* manifest for review, not a legal-grade parser.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import subprocess
from pathlib import Path
from typing import Dict, List, Optional


ELEMENT_HEADING_RE = re.compile(
    r"^\s*([NCDEFGH]\.(?:\d+|[a-z])(?:\.[0-9a-z]+)*)\b", re.IGNORECASE
)


def run_pdftotext(pdf_path: Path, txt_path: Path) -> None:
    cmd = ["pdftotext", "-layout", str(pdf_path), str(txt_path)]
    subprocess.run(cmd, check=True)


def normalize_element_id(value: str) -> str:
    # Keep stable catalog-like style: leading section letter upper-case, others as-is.
    raw = value.strip()
    if not raw:
        return raw
    return raw[0].upper() + raw[1:]


def infer_conformance(window: str) -> Optional[str]:
    if "적합성" not in window:
        return None
    has_required = "필수" in window
    has_optional = "선택" in window
    has_conditional = "조건" in window
    if has_required and has_conditional:
        return "conditional_required"
    if has_required:
        return "required"
    if has_optional:
        return "optional"
    if has_conditional:
        return "conditional"
    return "unknown"


def confidence_from_conformance(conformance: Optional[str]) -> float:
    if conformance is None:
        return 0.35
    if conformance in {"required", "optional", "conditional_required"}:
        return 0.75
    if conformance == "conditional":
        return 0.65
    return 0.4


def extract_records(lines: List[str], source_pdf: Path) -> List[Dict[str, object]]:
    records: List[Dict[str, object]] = []
    seen: set[str] = set()

    for idx, line in enumerate(lines):
        match = ELEMENT_HEADING_RE.match(line)
        if not match:
            continue
        raw_id = match.group(1)
        element_id = normalize_element_id(raw_id)

        # De-duplicate repeated ToC headings and wrapped page artifacts.
        dedupe_key = f"{element_id}:{idx // 5}"
        if dedupe_key in seen:
            continue
        seen.add(dedupe_key)

        window_lines = lines[idx : idx + 14]
        window = "\n".join(window_lines)
        conformance = infer_conformance(window)

        excerpt = " ".join(part.strip() for part in window_lines[:4]).strip()
        excerpt = re.sub(r"\s+", " ", excerpt)
        if len(excerpt) > 220:
            excerpt = excerpt[:220] + "..."

        records.append(
            {
                "rule_id": f"DRAFT.MFDS.{element_id.upper()}",
                "authority": "mfds",
                "profile": "ich_or_mfds_guidance",
                "element_id": element_id,
                "phase": ["case_validate", "xml_import", "xml_export", "submit_gate"],
                "conformance_hint": conformance or "unspecified",
                "enforcement_candidate": (
                    "required"
                    if conformance in {"required", "conditional_required"}
                    else "optional_or_manual_review"
                ),
                "source_kind": "pdf_guidance",
                "source_ref": {
                    "file": str(source_pdf),
                    "line_number_1_based": idx + 1,
                    "excerpt": excerpt,
                },
                "confidence": confidence_from_conformance(conformance),
                "status": "draft_review_required",
            }
        )
    return records


def collapse_best_record_per_element(
    records: List[Dict[str, object]],
) -> List[Dict[str, object]]:
    best: Dict[str, Dict[str, object]] = {}
    for rec in records:
        eid = str(rec["element_id"])
        prev = best.get(eid)
        if prev is None:
            best[eid] = rec
            continue

        prev_score = float(prev.get("confidence", 0.0))
        cur_score = float(rec.get("confidence", 0.0))
        if cur_score > prev_score:
            best[eid] = rec
            continue

        # Tie-break: keep earlier line as usually more canonical heading.
        prev_line = int(prev["source_ref"]["line_number_1_based"])
        cur_line = int(rec["source_ref"]["line_number_1_based"])
        if cur_score == prev_score and cur_line < prev_line:
            best[eid] = rec
    return sorted(best.values(), key=lambda r: str(r["element_id"]))


def build_summary(manifest: List[Dict[str, object]]) -> Dict[str, int]:
    out = {
        "total_rules": len(manifest),
        "required_like": 0,
        "optional_like": 0,
        "unspecified": 0,
    }
    for rec in manifest:
        hint = str(rec.get("conformance_hint", "unspecified"))
        if hint in {"required", "conditional_required"}:
            out["required_like"] += 1
        elif hint in {"optional", "conditional"}:
            out["optional_like"] += 1
        else:
            out["unspecified"] += 1
    return out


def to_markdown(
    source_pdf: Path,
    txt_path: Path,
    manifest: List[Dict[str, object]],
    summary: Dict[str, int],
) -> str:
    lines: List[str] = []
    lines.append("# MFDS PDF Manifest Draft")
    lines.append("")
    lines.append(
        f"- Generated at (UTC): `{dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()}`"
    )
    lines.append(f"- Source PDF: `{source_pdf}`")
    lines.append(f"- Extracted text: `{txt_path}`")
    lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append(f"- Total element rules: `{summary['total_rules']}`")
    lines.append(f"- Required-like: `{summary['required_like']}`")
    lines.append(f"- Optional/conditional-like: `{summary['optional_like']}`")
    lines.append(f"- Unspecified: `{summary['unspecified']}`")
    lines.append("")
    lines.append("## Notes")
    lines.append("")
    lines.append(
        "- This is a parser-derived draft from PDF text and must be reviewed against official machine-readable sources."
    )
    lines.append("- `conformance_hint` is inferred from nearby `적합성` lines and can be noisy.")
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Extract draft MFDS rule manifest from guidance PDF."
    )
    parser.add_argument("--pdf", required=True, type=Path)
    parser.add_argument(
        "--txt-out",
        required=True,
        type=Path,
        help="Path for pdftotext output.",
    )
    parser.add_argument("--out-json", required=True, type=Path)
    parser.add_argument("--out-md", required=True, type=Path)
    args = parser.parse_args()

    run_pdftotext(args.pdf, args.txt_out)
    lines = args.txt_out.read_text(encoding="utf-8", errors="ignore").splitlines()

    raw_records = extract_records(lines, args.pdf)
    manifest = collapse_best_record_per_element(raw_records)
    summary = build_summary(manifest)

    payload = {
        "generated_at_utc": dt.datetime.now(dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat(),
        "source_pdf": str(args.pdf),
        "text_dump": str(args.txt_out),
        "summary": summary,
        "rules": manifest,
    }

    args.out_json.parent.mkdir(parents=True, exist_ok=True)
    args.out_md.parent.mkdir(parents=True, exist_ok=True)
    args.out_json.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    args.out_md.write_text(
        to_markdown(args.pdf, args.txt_out, manifest, summary),
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
