#!/usr/bin/env python3
"""Generate enum-based rule binding index from inventory + manifests."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Dict, List


RULE_PHASE_ENUM = ["import", "case_validate", "export"]
BINDING_KIND_ENUM = ["enforcement", "mapping", "metadata", "test"]
SURFACE_ENUM = [
    "case_validation",
    "xml_validation",
    "xml_export",
    "frontend_create_gate",
    "frontend_ui_required",
    "frontend_zod",
    "frontend_path_mapping",
    "catalog_metadata",
    "test_backend",
    "test_frontend",
]

LAYER_TO_BINDING_KIND = {
    "backend_case_semantic": "enforcement",
    "backend_xml_business": "enforcement",
    "backend_xml_business_registry": "enforcement",
    "backend_export_postprocess": "enforcement",
    "backend_export_postprocess_registry": "enforcement",
    "frontend_validation_runtime": "enforcement",
    "frontend_ui_required_flags": "enforcement",
    "frontend_syntax_zod": "enforcement",
    "frontend_backend_issue_mapping": "mapping",
    "backend_catalog_metadata": "metadata",
    "backend_tests": "test",
    "frontend_tests": "test",
}

LAYER_TO_SURFACE = {
    "backend_case_semantic": "case_validation",
    "backend_xml_business": "xml_validation",
    "backend_xml_business_registry": "xml_validation",
    "backend_export_postprocess": "xml_export",
    "backend_export_postprocess_registry": "xml_export",
    "frontend_validation_runtime": "frontend_create_gate",
    "frontend_ui_required_flags": "frontend_ui_required",
    "frontend_syntax_zod": "frontend_zod",
    "frontend_backend_issue_mapping": "frontend_path_mapping",
    "backend_catalog_metadata": "catalog_metadata",
    "backend_tests": "test_backend",
    "frontend_tests": "test_frontend",
}


def repo_root_from_here() -> Path:
    return Path(__file__).resolve().parents[2]


def latest_manifest(path_glob: str, root: Path) -> Path:
    matches = sorted(root.glob(path_glob))
    if not matches:
        raise FileNotFoundError(f"no manifest matches: {path_glob}")
    return matches[-1]


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def build_phase_map(root: Path) -> Dict[str, List[str]]:
    """Rule phases from manifests.

    - ICH: from ich.rules*.json `phases` field
    - FDA/MFDS: current manifests are case-level business rules => case_validate
    """
    phases: Dict[str, List[str]] = {}

    ich_path = latest_manifest("docs/generated/manifests/ich.rules*.json", root)
    ich = load_json(ich_path)
    for row in ich.get("ich_rules", []):
        code = row.get("code")
        if not isinstance(code, str):
            continue
        row_phases = row.get("phases")
        if isinstance(row_phases, list):
            norm = [p for p in row_phases if p in RULE_PHASE_ENUM]
            phases[code] = sorted(set(norm)) if norm else ["case_validate"]
        else:
            phases[code] = ["case_validate"]

    fda_path = root / "docs/generated/manifests/fda.rules.json"
    fda = load_json(fda_path)
    for row in fda.get("catalog_rules", {}).get("fda", []):
        code = row.get("code")
        if isinstance(code, str):
            phases.setdefault(code, ["case_validate"])

    mfds_path = latest_manifest("docs/generated/manifests/mfds.rules*.json", root)
    mfds = load_json(mfds_path)
    for row in mfds.get("mfds_rules", []):
        code = row.get("code")
        if isinstance(code, str):
            phases.setdefault(code, ["case_validate"])

    return phases


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--inventory-json",
        default="docs/generated/manifests/rule_binding_inventory.json",
    )
    parser.add_argument(
        "--out-json",
        default="docs/generated/manifests/rule_binding_index.json",
    )
    args = parser.parse_args()

    root = repo_root_from_here()
    inventory = load_json(root / args.inventory_json)
    phase_map = build_phase_map(root)

    rules_out = []
    for row in inventory.get("rules", []):
        code = row.get("code")
        if not isinstance(code, str):
            continue
        profile = row.get("profile", "unknown")
        manifest_source = row.get("manifest_source")
        evidence = row.get("evidence", [])
        bindings = []
        for ev in evidence:
            layer = ev.get("layer")
            if layer not in LAYER_TO_BINDING_KIND:
                continue
            bindings.append(
                {
                    "binding_kind": LAYER_TO_BINDING_KIND[layer],
                    "surface": LAYER_TO_SURFACE[layer],
                    "layer": layer,
                    "file": ev.get("file"),
                    "line": ev.get("line"),
                }
            )
        dedup = {}
        for b in bindings:
            key = (b["binding_kind"], b["surface"], b["layer"], b["file"], b["line"])
            dedup[key] = b
        bindings = sorted(
            dedup.values(),
            key=lambda b: (b["binding_kind"], b["surface"], b["file"] or "", b["line"] or 0),
        )

        rules_out.append(
            {
                "rule_code": code,
                "profile": profile,
                "phases": phase_map.get(code, ["case_validate"]),
                "manifest_source": manifest_source,
                "bindings": bindings,
            }
        )

    payload = {
        "enums": {
            "rule_phase": RULE_PHASE_ENUM,
            "binding_kind": BINDING_KIND_ENUM,
            "surface": SURFACE_ENUM,
        },
        "rules": sorted(rules_out, key=lambda r: r["rule_code"]),
    }

    out = root / args.out_json
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
