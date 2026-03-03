#!/usr/bin/env python3
"""
Import FDA E2B(R3) Excel rule workbooks into a normalized JSON snapshot.

No third-party dependencies are required (parses .xlsx as ZIP/XML).
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import zipfile
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, Iterable, List, Optional
from xml.etree import ElementTree as ET


NS_MAIN = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
NS_REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
NS_PKG_REL = "http://schemas.openxmlformats.org/package/2006/relationships"


def _nstag(ns: str, tag: str) -> str:
    return f"{{{ns}}}{tag}"


def _str_norm(v: Optional[str]) -> str:
    return (v or "").strip()


def _header_key(v: str) -> str:
    s = v.strip().lower()
    s = re.sub(r"[^a-z0-9]+", "_", s)
    s = re.sub(r"_+", "_", s).strip("_")
    return s


def _first_non_empty(*values: str) -> str:
    for v in values:
        if v.strip():
            return v.strip()
    return ""


def _sheet_kind(name: str) -> str:
    n = name.lower()
    if "forward" in n:
        return "forward_compat"
    if "regional" in n or "core" in n:
        return "core_regional"
    return "unknown"


def _infer_profile(sheet_name: str, tag_id: str, region_col: str) -> str:
    for raw in (region_col, tag_id, sheet_name):
        s = raw.upper()
        if "FDA" in s:
            return "fda"
        if "MFDS" in s:
            return "mfds"
        if "ICH" in s:
            return "ich"
    if re.match(r"^(?:ACK|[A-Z])\.", tag_id.strip(), re.IGNORECASE):
        return "ich"
    return "unknown"


def _canonical_tag_key(tag_id: str, profile: str) -> str:
    t = tag_id.strip().replace(" ", "")
    t = t.rstrip(".,;:")

    if not t:
        return ""

    up = t.upper()
    if profile == "fda" and not up.startswith("FDA."):
        t = f"FDA.{t}"
    elif profile == "ich" and not up.startswith("ICH."):
        t = f"ICH.{t}"
    elif profile == "mfds" and not up.startswith("MFDS."):
        t = f"MFDS.{t}"

    # FDA.D.11.r.1 is a repeated Race row; collapse to field-level base tag.
    t = re.sub(r"^FDA\.D\.11\.r\.\d+[a-z]?$", "FDA.D.11", t, flags=re.IGNORECASE)
    return t.upper()


@dataclass
class ParsedSheet:
    name: str
    rows: List[List[str]]


def parse_shared_strings(zf: zipfile.ZipFile) -> List[str]:
    path = "xl/sharedStrings.xml"
    if path not in zf.namelist():
        return []
    root = ET.fromstring(zf.read(path))
    out: List[str] = []
    for si in root.findall(f".//{_nstag(NS_MAIN, 'si')}"):
        parts: List[str] = []
        for t in si.findall(f".//{_nstag(NS_MAIN, 't')}"):
            parts.append(t.text or "")
        out.append("".join(parts))
    return out


def parse_workbook_sheets(zf: zipfile.ZipFile) -> Dict[str, str]:
    wb = ET.fromstring(zf.read("xl/workbook.xml"))
    rels = ET.fromstring(zf.read("xl/_rels/workbook.xml.rels"))

    rid_to_target: Dict[str, str] = {}
    for r in rels.findall(f".//{_nstag(NS_PKG_REL, 'Relationship')}"):
        rid = r.attrib.get("Id", "")
        tgt = r.attrib.get("Target", "")
        if not rid or not tgt:
            continue
        rid_to_target[rid] = f"xl/{tgt}" if not tgt.startswith("xl/") else tgt

    out: Dict[str, str] = {}
    for sheet in wb.findall(f".//{_nstag(NS_MAIN, 'sheet')}"):
        name = sheet.attrib.get("name", "").strip()
        rid = sheet.attrib.get(_nstag(NS_REL, "id"), "").strip()
        if name and rid and rid in rid_to_target:
            out[name] = rid_to_target[rid]
    return out


def parse_sheet_rows(
    zf: zipfile.ZipFile, sheet_path: str, shared_strings: List[str]
) -> List[List[str]]:
    def col_idx(cell_ref: str) -> int:
        letters = "".join(ch for ch in cell_ref if ch.isalpha()).upper()
        if not letters:
            return -1
        out = 0
        for ch in letters:
            out = out * 26 + (ord(ch) - ord("A") + 1)
        return out - 1

    root = ET.fromstring(zf.read(sheet_path))
    rows: List[List[str]] = []
    for row in root.findall(f".//{_nstag(NS_MAIN, 'row')}"):
        cells_by_idx: Dict[int, str] = {}
        max_idx = -1
        for cell in row.findall(f"{_nstag(NS_MAIN, 'c')}"):
            cell_type = cell.attrib.get("t", "")
            idx = col_idx(cell.attrib.get("r", ""))
            if idx < 0:
                continue
            v = cell.find(f"{_nstag(NS_MAIN, 'v')}")
            is_node = cell.find(f"{_nstag(NS_MAIN, 'is')}")
            text = ""
            if cell_type == "s" and v is not None and v.text is not None:
                idx = int(v.text)
                if 0 <= idx < len(shared_strings):
                    text = shared_strings[idx]
            elif cell_type == "inlineStr" and is_node is not None:
                t = is_node.find(f".//{_nstag(NS_MAIN, 't')}")
                text = t.text if t is not None and t.text else ""
            elif v is not None and v.text is not None:
                text = v.text
            cidx = col_idx(cell.attrib.get("r", ""))
            cells_by_idx[cidx] = _str_norm(text)
            max_idx = max(max_idx, cidx)
        if max_idx >= 0:
            values = [cells_by_idx.get(i, "") for i in range(max_idx + 1)]
        else:
            values = []
        if any(values):
            rows.append(values)
    return rows


def find_header_index(rows: List[List[str]]) -> int:
    expected_headers = {
        "source",
        "header element",
        "data element number",
        "data element",
        "data element name",
        "e2b tag id",
        "tag id",
        "business rule",
        "rejection, if not met",
        "warning, if not met",
        "severity",
        "message",
    }
    best_idx = 0
    best_score = -1
    for i, row in enumerate(rows[:120]):
        raw = [v.strip().lower() for v in row if v.strip()]
        exact = sum(1 for v in raw if v in expected_headers)
        if exact >= 2:
            return i
        fuzzy = sum(
            1
            for v in raw
            if any(
                marker in v
                for marker in (
                    "data element",
                    "tag",
                    "business rule",
                    "rejection",
                    "warning",
                    "severity",
                    "message",
                )
            )
        )
        score = exact * 100 + fuzzy
        if score > best_score:
            best_score = score
            best_idx = i
    return best_idx


def as_dict_rows(rows: List[List[str]]) -> Iterable[Dict[str, str]]:
    if not rows:
        return []
    header_idx = find_header_index(rows)
    header = [_header_key(v) for v in rows[header_idx]]
    out: List[Dict[str, str]] = []
    for row in rows[header_idx + 1 :]:
        rec: Dict[str, str] = {}
        for i, key in enumerate(header):
            if not key:
                continue
            rec[key] = row[i] if i < len(row) else ""
        if any(v.strip() for v in rec.values()):
            out.append(rec)
    return out


def extract_rule_record(sheet: str, row: Dict[str, str]) -> Optional[Dict[str, str]]:
    tag_id = _first_non_empty(
        row.get("e2b_tag_id", ""),
        row.get("tag_id", ""),
        row.get("tag", ""),
        row.get("data_element_number", ""),
        row.get("data_element", ""),
    )
    if not tag_id:
        return None
    if tag_id.strip().lower() in {"(header)", "header"}:
        return None
    if not re.match(r"^(?:FDA|ICH|MFDS|ACK|[A-Z])\.", tag_id.strip(), re.IGNORECASE):
        return None

    severity = _first_non_empty(
        row.get("severity", ""),
        row.get("rule_severity", ""),
        row.get("level", ""),
    )
    if not severity:
        if _str_norm(row.get("rejection_if_not_met", "")):
            severity = "rejection"
        elif _str_norm(row.get("warning_if_not_met", "")):
            severity = "warning"
    region = _first_non_empty(
        row.get("region", ""),
        row.get("profile", ""),
        row.get("jurisdiction", ""),
        row.get("source", ""),
    )
    message = _first_non_empty(
        row.get("validation_detail_message", ""),
        row.get("message", ""),
        row.get("rule_description", ""),
        row.get("business_rule", ""),
        row.get("rejection_if_not_met", ""),
        row.get("warning_if_not_met", ""),
        row.get("ich_business_rules", ""),
        row.get("post_market_business_rule", ""),
        row.get("pre_market_business_rule", ""),
    )

    profile = _infer_profile(sheet, tag_id, region)
    canonical_tag = _canonical_tag_key(tag_id, profile)
    scope = "core" if profile == "ich" else "profile_overlay"

    return {
        "profile": profile,
        "scope": scope,
        "sheet": sheet,
        "sheet_kind": _sheet_kind(sheet),
        "tag_id_raw": tag_id,
        "tag_key": canonical_tag,
        "severity": severity,
        "message": message,
    }


def import_rules(xlsx_path: Path) -> Dict[str, object]:
    with zipfile.ZipFile(xlsx_path, "r") as zf:
        shared = parse_shared_strings(zf)
        sheets = parse_workbook_sheets(zf)
        records: List[Dict[str, str]] = []
        for sheet_name, sheet_path in sheets.items():
            rows = parse_sheet_rows(zf, sheet_path, shared)
            for row in as_dict_rows(rows):
                rec = extract_rule_record(sheet_name, row)
                if rec is not None:
                    records.append(rec)

    records.sort(
        key=lambda r: (
            r["profile"],
            r["tag_key"],
            r["sheet"],
            r["severity"],
            r["message"],
        )
    )

    deduped: List[Dict[str, str]] = []
    seen = set()
    for r in records:
        key = (
            r["profile"],
            r["tag_key"],
            r["severity"],
            r["message"],
            r["sheet_kind"],
        )
        if key in seen:
            continue
        seen.add(key)
        deduped.append(r)

    by_profile: Dict[str, int] = {}
    for r in deduped:
        by_profile[r["profile"]] = by_profile.get(r["profile"], 0) + 1

    return {
        "source_file": str(xlsx_path),
        "record_count": len(deduped),
        "counts_by_profile": by_profile,
        "rules": deduped,
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--xlsx", required=True, help="Path to FDA Excel workbook")
    ap.add_argument(
        "--out",
        required=True,
        help="Output JSON path for normalized snapshot",
    )
    args = ap.parse_args()

    xlsx_path = Path(args.xlsx)
    out_path = Path(args.out)
    if not xlsx_path.exists():
        print(f"missing xlsx file: {xlsx_path}", file=sys.stderr)
        return 2

    snapshot = import_rules(xlsx_path)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(snapshot, indent=2, ensure_ascii=True) + "\n")
    print(
        f"wrote {snapshot['record_count']} normalized rows -> {out_path}",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
