#!/usr/bin/env python3
"""
Build ICH-focused manifest JSON from local canonical catalog.

Includes:
- ICH case rules
- ICH XML structural rules (including ICH.XML.*)
- inferred category/severity/phases/conditions aligned with catalog logic
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
from pathlib import Path
from typing import Dict, List, Optional


RULE_BLOCK_RE = re.compile(r"ValidationRuleMetadata\s*\{(.*?)\},", re.S)
CONDITION_BINDING_RE = re.compile(
    r"ConditionBinding\s*\{\s*code:\s*\"([^\"]+)\"\s*,\s*condition:\s*RuleCondition::([A-Za-z0-9_]+)\s*,\s*\}",
    re.S,
)


# ICH-only directives encoded in export_directive_for_code() in catalog.rs.
ICH_EXPORT_DIRECTIVE_MAP: Dict[str, str] = {
    "ICH.XML.MEDDRA.CODE.FORMAT.REQUIRED": "NormalizeInvalidCodeToNullFlavorNi",
    "ICH.XML.COUNTRY.CODE.FORMAT.REQUIRED": "NormalizeInvalidCodeToNullFlavorNi",
    "ICH.XML.XSI_TYPE.NORMALIZE": "NormalizeTypeAttributeToXsiType",
    "ICH.XML.DOCUMENT.TEXT.COMPRESSION.FORBIDDEN": "RemoveDocumentTextCompression",
    "ICH.XML.SUMMARY.LANGUAGE.JA.FORBIDDEN": "RemoveSummaryLanguageJa",
    "ICH.XML.PLACEHOLDER.VALUE.PRUNE": "RemovePlaceholderValueNodes",
    "ICH.XML.PLACEHOLDER.CODESYSTEMVERSION.PRUNE": "RemovePlaceholderCodeSystemVersion",
    "ICH.XML.RACE.NI.PRUNE": "RemoveRaceNiNodes",
    "ICH.XML.RACE.EMPTY.PRUNE": "RemoveRaceEmptyNodes",
    "ICH.XML.GK11.EMPTY.PRUNE": "RemoveEmptyGk11Relationships",
    "ICH.XML.OPTIONAL.PATH.EMPTY.PRUNE": "RemoveOptionalPathEmptyNodes",
    "ICH.XML.STRUCTURAL.EMPTY.PRUNE": "RemoveEmptyStructuralNodes",
}


def parse_condition_bindings(catalog_text: str) -> Dict[str, str]:
    out: Dict[str, str] = {}
    for code, condition in CONDITION_BINDING_RE.findall(catalog_text):
        out[code] = condition
    return out


def is_xml_structure_rule(code: str, section: str) -> bool:
    if section == "xml":
        return True
    if ".NULLFLAVOR." in code:
        return True
    return code in {
        "ICH.C.1.3.CONDITIONAL",
        "ICH.C.1.9.1.CONDITIONAL",
        "ICH.D.7.2.CONDITIONAL",
        "ICH.D.5.SEX.CONDITIONAL",
        "ICH.E.i.4-6.CONDITIONAL",
        "ICH.G.k.4.r.4-8.CONDITIONAL",
    }


def is_export_only_rule(code: str) -> bool:
    if ".PRUNE" in code or ".NORMALIZE" in code:
        return True
    return code in {
        "ICH.XML.DOCUMENT.TEXT.COMPRESSION.FORBIDDEN",
        "ICH.XML.SUMMARY.LANGUAGE.JA.FORBIDDEN",
    }


def infer_severity(code: str, blocking: bool) -> str:
    if code.endswith(".RECOMMENDED") or ".PRUNE" in code or ".NORMALIZE" in code:
        return "info"
    if blocking:
        return "blocking"
    return "warning"


def infer_category(code: str, section: str) -> str:
    if is_xml_structure_rule(code, section):
        return "xml_structure"
    return "case_business"


def infer_phases(code: str, section: str, export_directive: str | None) -> List[str]:
    if export_directive is not None:
        if is_export_only_rule(code):
            return ["export"]
        if is_xml_structure_rule(code, section):
            return ["import", "export"]
        return ["case_validate", "export"]
    if is_xml_structure_rule(code, section):
        return ["import"]
    return ["case_validate"]


def parse_ich_rules(catalog_rs: Path) -> Dict[str, object]:
    text = catalog_rs.read_text(encoding="utf-8")
    conditions = parse_condition_bindings(text)
    rules: List[Dict[str, object]] = []

    for block in RULE_BLOCK_RE.findall(text):
        if "ValidationProfile::Ich" not in block:
            continue
        code_m = re.search(r'code:\s*"([^"]+)"', block)
        section_m = re.search(r'section:\s*"([^"]+)"', block)
        blocking_m = re.search(r"blocking:\s*(true|false)", block)
        message_m = re.search(r'message:\s*"([^"]*)"', block, re.S)
        if not code_m:
            continue

        code = code_m.group(1).strip()
        section = section_m.group(1).strip() if section_m else "unknown"
        blocking = (blocking_m.group(1) == "true") if blocking_m else False
        message = (
            re.sub(r"\s+", " ", message_m.group(1)).strip()
            if message_m
            else code
        )
        export_directive = ICH_EXPORT_DIRECTIVE_MAP.get(code)
        severity = infer_severity(code, blocking)
        rule = {
            "code": code,
            "profile": "ich",
            "section": section,
            "blocking": severity == "blocking",
            "message": message,
            "category": infer_category(code, section),
            "severity": severity,
            "phases": infer_phases(code, section, export_directive),
            "condition": conditions.get(code, "Always"),
            "export_directive": export_directive,
            "is_xml_rule": code.startswith("ICH.XML.") or section == "xml",
        }
        rules.append(rule)

    rules.sort(key=lambda x: str(x["code"]))
    xml_rules = [r for r in rules if r["is_xml_rule"]]
    case_rules = [r for r in rules if not r["is_xml_rule"]]

    return {
        "manifest": "ich",
        "generated_at_utc": dt.datetime.now(dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat(),
        "source": {"catalog_rs": str(catalog_rs)},
        "summary": {
            "ich_rule_count": len(rules),
            "ich_case_rule_count": len(case_rules),
            "ich_xml_rule_count": len(xml_rules),
            "blocking_count": sum(1 for r in rules if r["blocking"]),
            "warning_count": sum(1 for r in rules if r["severity"] == "warning"),
            "info_count": sum(1 for r in rules if r["severity"] == "info"),
            "import_phase_count": sum(1 for r in rules if "import" in r["phases"]),
            "case_validate_phase_count": sum(
                1 for r in rules if "case_validate" in r["phases"]
            ),
            "export_phase_count": sum(1 for r in rules if "export" in r["phases"]),
        },
        "ich_rules": rules,
    }


def _is_mfds_specific_element_id(element_id: str) -> bool:
    eid = element_id.strip().upper()
    return ".KR." in eid or eid.startswith("KR.")


def _to_ich_rule_code_candidate(
    element_id: str,
    conformance_hint: Optional[str],
) -> Optional[str]:
    eid = element_id.strip()
    if not eid:
        return None
    hint = (conformance_hint or "").strip().lower()
    suffix_map = {
        "required": "REQUIRED",
        "recommended": "RECOMMENDED",
        "forbidden": "FORBIDDEN",
        "conditional": "CONDITIONAL",
    }
    suffix = suffix_map.get(hint)
    if suffix is None:
        return None
    return f"ICH.{eid}.{suffix}"


def extract_ich_guidance_rows_from_mfds_pdf_draft(
    mfds_pdf_draft_json: Path,
) -> List[Dict[str, object]]:
    payload = json.loads(mfds_pdf_draft_json.read_text(encoding="utf-8"))
    rows = payload.get("rules", []) if isinstance(payload, dict) else payload
    if not isinstance(rows, list):
        return []

    out: List[Dict[str, object]] = []
    seen = set()
    for row in rows:
        if not isinstance(row, dict):
            continue
        element_id = str(row.get("element_id", "")).strip()
        if not element_id:
            continue
        if _is_mfds_specific_element_id(element_id):
            continue

        track = str(row.get("manifest_track", "")).strip()
        if track and track != "implementable_leaf_candidate":
            continue

        conformance_hint = str(row.get("conformance_hint", "")).strip().lower()
        if conformance_hint in {"unspecified", ""}:
            # Keep only actionable conformance hints for manifest ingestion.
            continue

        key = (element_id.upper(), conformance_hint)
        if key in seen:
            continue
        seen.add(key)

        source_ref = row.get("source_ref", {}) if isinstance(row.get("source_ref"), dict) else {}
        out.append(
            {
                "element_id": element_id,
                "rule_code_candidate": _to_ich_rule_code_candidate(
                    element_id, conformance_hint
                ),
                "conformance_hint": conformance_hint,
                "enforcement_candidate": row.get("enforcement_candidate"),
                "phase_hint": row.get("phase", []),
                "source_kind": row.get("source_kind"),
                "source_line": source_ref.get("line_number_1_based"),
                "source_excerpt": source_ref.get("excerpt"),
                "confidence": row.get("confidence"),
                "status": row.get("status"),
            }
        )
    out.sort(key=lambda x: (str(x["element_id"]), str(x["conformance_hint"])))
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description="Build ICH-specific manifest JSON.")
    parser.add_argument(
        "--catalog-rs",
        default=Path("crates/libs/lib-core/src/xml/validate/catalog.rs"),
        type=Path,
    )
    parser.add_argument("--out-json", required=True, type=Path)
    parser.add_argument(
        "--fda-workbook-json",
        default=None,
        type=Path,
        help="Optional normalized FDA workbook JSON; ICH-profile rows are added under workbook_ich_rules.",
    )
    parser.add_argument(
        "--mfds-pdf-draft-json",
        default=None,
        type=Path,
        help="Optional MFDS PDF draft JSON; non-KR ICH guidance rows are added under mfds_pdf_ich_guidance_rules.",
    )
    args = parser.parse_args()

    manifest = parse_ich_rules(args.catalog_rs)
    if args.fda_workbook_json is not None and args.fda_workbook_json.exists():
        wb = json.loads(args.fda_workbook_json.read_text(encoding="utf-8"))
        wb_rules = wb.get("rules", [])
        if isinstance(wb_rules, list):
            ich_rows = [
                r
                for r in wb_rules
                if isinstance(r, dict)
                and str(r.get("profile", "")).strip().lower() == "ich"
            ]
        else:
            ich_rows = []
        manifest["source"]["fda_workbook_json"] = str(args.fda_workbook_json)
        manifest["workbook_ich_rules"] = ich_rows
        manifest["summary"]["workbook_ich_rule_count"] = len(ich_rows)
        manifest["summary"]["total_ich_rows_catalog_plus_workbook"] = (
            manifest["summary"]["ich_rule_count"] + len(ich_rows)
        )
    if args.mfds_pdf_draft_json is not None and args.mfds_pdf_draft_json.exists():
        pdf_ich_rows = extract_ich_guidance_rows_from_mfds_pdf_draft(
            args.mfds_pdf_draft_json
        )
        manifest["source"]["mfds_pdf_draft_json"] = str(args.mfds_pdf_draft_json)
        manifest["mfds_pdf_ich_guidance_rules"] = pdf_ich_rows
        manifest["summary"]["mfds_pdf_ich_guidance_rule_count"] = len(pdf_ich_rows)
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
