#!/usr/bin/env python3
"""
Audit MFDS E2B(R3) rule coverage against local catalog/runtime rules.

Parses MFDS Excel workbooks directly as ZIP/XML (no third-party deps),
extracts KR regional element IDs, and diffs them against local MFDS rules.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import zipfile
from pathlib import Path
from typing import Dict, Iterable, List, Set
from xml.etree import ElementTree as ET


NS_MAIN = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
NS_REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
NS_PKG_REL = "http://schemas.openxmlformats.org/package/2006/relationships"


def nstag(ns: str, tag: str) -> str:
    return f"{{{ns}}}{tag}"


def parse_shared_strings(zf: zipfile.ZipFile) -> List[str]:
    path = "xl/sharedStrings.xml"
    if path not in zf.namelist():
        return []
    root = ET.fromstring(zf.read(path))
    out: List[str] = []
    for si in root.findall(f".//{nstag(NS_MAIN, 'si')}"):
        parts = []
        for t in si.findall(f".//{nstag(NS_MAIN, 't')}"):
            parts.append(t.text or "")
        out.append("".join(parts))
    return out


def parse_workbook_sheets(zf: zipfile.ZipFile) -> Dict[str, str]:
    wb = ET.fromstring(zf.read("xl/workbook.xml"))
    rels = ET.fromstring(zf.read("xl/_rels/workbook.xml.rels"))
    rid_to_target: Dict[str, str] = {}
    for rel in rels.findall(f".//{nstag(NS_PKG_REL, 'Relationship')}"):
        rid = rel.attrib.get("Id", "")
        target = rel.attrib.get("Target", "")
        if rid and target:
            rid_to_target[rid] = (
                f"xl/{target}" if not target.startswith("xl/") else target
            )

    out: Dict[str, str] = {}
    for sheet in wb.findall(f".//{nstag(NS_MAIN, 'sheet')}"):
        name = sheet.attrib.get("name", "").strip()
        rid = sheet.attrib.get(nstag(NS_REL, "id"), "").strip()
        if name and rid and rid in rid_to_target:
            out[name] = rid_to_target[rid]
    return out


def parse_sheet_rows(
    zf: zipfile.ZipFile, sheet_path: str, shared: List[str]
) -> List[Dict[str, str]]:
    root = ET.fromstring(zf.read(sheet_path))
    rows: List[Dict[str, str]] = []
    for row in root.findall(f".//{nstag(NS_MAIN, 'row')}"):
        rec: Dict[str, str] = {}
        for cell in row.findall(f"{nstag(NS_MAIN, 'c')}"):
            ref = cell.attrib.get("r", "")
            col = "".join(ch for ch in ref if ch.isalpha())
            if not col:
                continue
            cell_type = cell.attrib.get("t", "")
            v = cell.find(f"{nstag(NS_MAIN, 'v')}")
            text = ""
            if cell_type == "s" and v is not None and (v.text or "").isdigit():
                idx = int(v.text or "0")
                if 0 <= idx < len(shared):
                    text = shared[idx]
            elif cell_type == "inlineStr":
                is_node = cell.find(f"{nstag(NS_MAIN, 'is')}")
                if is_node is not None:
                    text = "".join(
                        t.text or ""
                        for t in is_node.findall(f".//{nstag(NS_MAIN, 't')}")
                    )
            elif v is not None and v.text is not None:
                text = v.text
            rec[col] = text.strip()
        if rec:
            rows.append(rec)
    return rows


def load_workbook_rows(xlsx: Path) -> Dict[str, List[Dict[str, str]]]:
    with zipfile.ZipFile(xlsx, "r") as zf:
        shared = parse_shared_strings(zf)
        sheets = parse_workbook_sheets(zf)
        out: Dict[str, List[Dict[str, str]]] = {}
        for sheet_name, sheet_path in sheets.items():
            if sheet_path in zf.namelist():
                out[sheet_name] = parse_sheet_rows(zf, sheet_path, shared)
        return out


def extract_core_kr_ids(core_rows: Dict[str, List[Dict[str, str]]]) -> Set[str]:
    out: Set[str] = set()
    for row in core_rows.get("ICSR", []):
        source = (row.get("A") or "").strip().upper()
        elem = (row.get("C") or "").strip()
        if source == "KR" and elem and elem != "-":
            out.add(elem)
    return out


def extract_individual_kr_ids(
    individual_rows: Dict[str, List[Dict[str, str]]]
) -> Set[str]:
    out: Set[str] = set()
    kr_pat = re.compile(r"\.KR(?:\.|$)", re.IGNORECASE)
    for rows in individual_rows.values():
        for row in rows:
            elem = (row.get("B") or "").strip()
            if not elem or elem.lower() == "element id":
                continue
            if kr_pat.search(elem):
                out.add(elem)
    return out


def extract_internal_mfds_profile_rules(catalog_rs: Path) -> Set[str]:
    text = catalog_rs.read_text(encoding="utf-8")
    # ValidationRuleMetadata { code: "...", profile: ValidationProfile::Mfds, ... }
    pat = re.compile(
        r"ValidationRuleMetadata\s*\{[^}]*?code:\s*\"([^\"]+)\"[^}]*?"
        r"profile:\s*ValidationProfile::Mfds[^}]*?\}",
        re.S,
    )
    return {m.group(1) for m in pat.finditer(text)}


def extract_internal_mfds_elements_from_codes(codes: Iterable[str]) -> Set[str]:
    out: Set[str] = set()
    for code in codes:
        if not code.startswith("MFDS."):
            continue
        elem = code.split("MFDS.", 1)[1]
        elem = re.sub(
            r"\.(REQUIRED|RECOMMENDED|FORBIDDEN|CONDITIONAL)$", "", elem
        )
        out.add(elem)
    return out


def normalize_mfds_element_id(elem: str) -> str:
    """Normalize known MFDS element-id aliases to a single canonical key."""
    e = elem.strip()
    # Workbook variants observed for the same substance KR fields.
    if e == "G.k.2.3.r.KR.1a":
        return "G.k.2.3.r.1.KR.1a"
    if e == "G.k.2.3.r.KR.1b":
        return "G.k.2.3.r.1.KR.1b"
    return e


def extract_runtime_mfds_rule_constants(mfds_validation_rs: Path) -> Set[str]:
    text = mfds_validation_rs.read_text(encoding="utf-8")
    return set(re.findall(r"CASE_RULE_MFDS_[A-Z0-9_]+", text))


def to_markdown(report: Dict[str, object]) -> str:
    lines: List[str] = []
    lines.append("# MFDS Rule Coverage Audit")
    lines.append("")
    lines.append(f"- Generated at (UTC): `{report['generated_at_utc']}`")
    lines.append(
        f"- Core KR element IDs found: `{report['core_kr_count']}`"
    )
    lines.append(
        f"- Individual KR element IDs found: `{report['individual_kr_count']}`"
    )
    lines.append(
        "- Unique KR element IDs (official): "
        f"`{report['official_unique_kr_count']}`"
    )
    lines.append(
        f"- Internal MFDS profile rules: `{report['internal_mfds_rule_count']}`"
    )
    lines.append(
        "- Missing KR element IDs not covered by internal MFDS rules: "
        f"`{report['missing_kr_count']}`"
    )
    lines.append("")
    lines.append("## Missing KR Element IDs")
    lines.append("")
    missing = report.get("missing_kr_elements_not_covered_by_internal_rules", [])
    if not missing:
        lines.append("- None")
    else:
        for elem in missing:
            lines.append(f"- `{elem}`")
    lines.append("")
    lines.append("## Internal MFDS Profile Rules")
    lines.append("")
    for code in report.get("internal_mfds_profile_rule_codes", []):
        lines.append(f"- `{code}`")
    lines.append("")
    lines.append("## Source Files")
    lines.append("")
    lines.append(f"- Core workbook: `{report['core_xlsx']}`")
    lines.append(f"- Individual workbook: `{report['individual_xlsx']}`")
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Audit MFDS official KR element coverage vs local rules."
    )
    parser.add_argument("--core-xlsx", required=True, type=Path)
    parser.add_argument("--individual-xlsx", required=True, type=Path)
    parser.add_argument(
        "--catalog-rs",
        default=Path("crates/libs/lib-core/src/xml/validate/catalog.rs"),
        type=Path,
    )
    parser.add_argument(
        "--mfds-validation-rs",
        default=Path("crates/libs/lib-core/src/xml/mfds/validation.rs"),
        type=Path,
    )
    parser.add_argument("--out-json", required=True, type=Path)
    parser.add_argument("--out-md", required=True, type=Path)
    args = parser.parse_args()

    core_rows = load_workbook_rows(args.core_xlsx)
    individual_rows = load_workbook_rows(args.individual_xlsx)

    core_kr = extract_core_kr_ids(core_rows)
    individual_kr = extract_individual_kr_ids(individual_rows)
    official_kr = sorted(
        {normalize_mfds_element_id(elem) for elem in (core_kr | individual_kr)}
    )

    internal_codes = sorted(extract_internal_mfds_profile_rules(args.catalog_rs))
    internal_elements = extract_internal_mfds_elements_from_codes(internal_codes)
    internal_elements_upper = {
        normalize_mfds_element_id(x).upper() for x in internal_elements
    }

    missing_kr = sorted(
        elem for elem in official_kr if elem.upper() not in internal_elements_upper
    )
    runtime_consts = sorted(
        extract_runtime_mfds_rule_constants(args.mfds_validation_rs)
    )

    report: Dict[str, object] = {
        "generated_at_utc": dt.datetime.now(dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat(),
        "core_xlsx": str(args.core_xlsx),
        "individual_xlsx": str(args.individual_xlsx),
        "core_kr_count": len(core_kr),
        "core_kr_elements": sorted(core_kr),
        "individual_kr_count": len(individual_kr),
        "individual_kr_elements": sorted(individual_kr),
        "official_unique_kr_count": len(official_kr),
        "official_unique_kr_elements": official_kr,
        "internal_mfds_rule_count": len(internal_codes),
        "internal_mfds_profile_rule_codes": internal_codes,
        "internal_mfds_element_count": len(internal_elements),
        "internal_mfds_elements": sorted(internal_elements),
        "runtime_mfds_constants_count": len(runtime_consts),
        "runtime_mfds_constants": runtime_consts,
        "missing_kr_count": len(missing_kr),
        "missing_kr_elements_not_covered_by_internal_rules": missing_kr,
    }

    args.out_json.parent.mkdir(parents=True, exist_ok=True)
    args.out_md.parent.mkdir(parents=True, exist_ok=True)
    args.out_json.write_text(
        json.dumps(report, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    args.out_md.write_text(to_markdown(report), encoding="utf-8")

    print(
        json.dumps(
            {
                "official_unique_kr_count": report["official_unique_kr_count"],
                "internal_mfds_rule_count": report["internal_mfds_rule_count"],
                "missing_kr_count": report["missing_kr_count"],
                "out_json": str(args.out_json),
                "out_md": str(args.out_md),
            },
            ensure_ascii=False,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
