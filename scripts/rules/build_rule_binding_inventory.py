#!/usr/bin/env python3
"""Build a rule binding inventory across backend/frontend layers.

Outputs:
- JSON: per-rule layer bindings with file/line evidence
- Markdown: summary counts and uncovered rule codes
"""

from __future__ import annotations

import argparse
import json
import re
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, Iterable, List, Set, Tuple


RULE_RE = re.compile(r'"((?:ICH|FDA|MFDS)\.[^"]+)"')
CONST_RULE_RE = re.compile(
    r'pub const ([A-Z0-9_]+):\s*&str\s*=\s*"((?:ICH|FDA|MFDS)\.[^"]+)";',
    re.MULTILINE,
)


@dataclass(frozen=True)
class Evidence:
    layer: str
    file: str
    line: int
    text: str


def repo_root_from_here() -> Path:
    return Path(__file__).resolve().parents[2]


def latest_manifest(path_glob: str, root: Path) -> Path:
    matches = sorted(root.glob(path_glob))
    if not matches:
        raise FileNotFoundError(f"no manifest matches: {path_glob}")
    return matches[-1]


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def manifest_codes(root: Path) -> Tuple[Set[str], Dict[str, str], Dict[str, str]]:
    ich_path = latest_manifest("docs/generated/manifests/ich.rules*.json", root)
    mfds_path = latest_manifest("docs/generated/manifests/mfds.rules*.json", root)
    fda_path = root / "docs/generated/manifests/fda.rules.json"

    ich = load_json(ich_path)
    mfds = load_json(mfds_path)
    fda = load_json(fda_path)

    codes: Set[str] = set()
    source_of_code: Dict[str, str] = {}
    profile_of_code: Dict[str, str] = {}

    for row in ich.get("ich_rules", []):
        code = row.get("code")
        if isinstance(code, str) and code.startswith("ICH."):
            codes.add(code)
            source_of_code[code] = str(ich_path.relative_to(root))
            profile_of_code[code] = "ich"

    for row in mfds.get("mfds_rules", []):
        code = row.get("code")
        if isinstance(code, str) and code.startswith("MFDS."):
            codes.add(code)
            source_of_code[code] = str(mfds_path.relative_to(root))
            profile_of_code[code] = "mfds"

    for row in fda.get("catalog_rules", {}).get("fda", []):
        code = row.get("code")
        if isinstance(code, str) and code.startswith("FDA."):
            codes.add(code)
            source_of_code[code] = str(fda_path.relative_to(root))
            profile_of_code[code] = "fda"

    return codes, source_of_code, profile_of_code


def parse_const_rule_map(path: Path) -> Dict[str, str]:
    mapping: Dict[str, str] = {}
    text = path.read_text(encoding="utf-8")
    for m in CONST_RULE_RE.finditer(text):
        mapping[m.group(1)] = m.group(2)
    return mapping


def find_literal_rule_refs(path: Path) -> List[Tuple[str, int, str]]:
    refs: List[Tuple[str, int, str]] = []
    for idx, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        for m in RULE_RE.finditer(line):
            refs.append((m.group(1), idx, line.strip()))
    return refs


def find_const_refs(path: Path, const_to_code: Dict[str, str]) -> List[Tuple[str, int, str]]:
    refs: List[Tuple[str, int, str]] = []
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    for i, line in enumerate(lines, start=1):
        for const_name, code in const_to_code.items():
            if re.search(rf"\b{re.escape(const_name)}\b", line):
                refs.append((code, i, line.strip()))
    return refs


def add_refs(
    out: Dict[str, List[Evidence]],
    layer: str,
    file_path: Path,
    refs: Iterable[Tuple[str, int, str]],
    code_allowlist: Set[str],
    root: Path,
) -> None:
    try:
        rel = str(file_path.relative_to(root))
    except ValueError:
        rel = str(file_path)
    for code, line, text in refs:
        if code in code_allowlist:
            out[code].append(Evidence(layer=layer, file=rel, line=line, text=text))


def scan_layer_literal(
    out: Dict[str, List[Evidence]],
    layer: str,
    files: Iterable[Path],
    code_allowlist: Set[str],
    root: Path,
) -> None:
    for p in files:
        if not p.exists():
            continue
        add_refs(out, layer, p, find_literal_rule_refs(p), code_allowlist, root)


def scan_layer_const(
    out: Dict[str, List[Evidence]],
    layer: str,
    files: Iterable[Path],
    const_map: Dict[str, str],
    code_allowlist: Set[str],
    root: Path,
) -> None:
    for p in files:
        if not p.exists():
            continue
        add_refs(out, layer, p, find_const_refs(p, const_map), code_allowlist, root)


def dedupe_evidence(items: List[Evidence]) -> List[Evidence]:
    seen = set()
    out: List[Evidence] = []
    for item in items:
        key = (item.layer, item.file, item.line)
        if key in seen:
            continue
        seen.add(key)
        out.append(item)
    return out


def write_outputs(
    root: Path,
    out_json: Path,
    out_md: Path,
    code_set: Set[str],
    profile_by_code: Dict[str, str],
    source_by_code: Dict[str, str],
    bindings: Dict[str, List[Evidence]],
) -> None:
    total = len(code_set)
    covered = 0
    executable_covered = 0
    profile_counts = defaultdict(int)
    uncovered_by_profile = defaultdict(list)
    executable_uncovered_by_profile = defaultdict(list)
    layer_counts = defaultdict(int)
    executable_layer_counts = defaultdict(int)

    executable_layers = {
        "backend_case_semantic",
        "backend_xml_business",
        "backend_xml_business_registry",
        "backend_export_postprocess",
        "backend_export_postprocess_registry",
        "frontend_validation_runtime",
        "frontend_ui_required_flags",
        "frontend_syntax_zod",
    }

    rules_json = []
    for code in sorted(code_set):
        profile = profile_by_code.get(code, "unknown")
        profile_counts[profile] += 1
        ev = dedupe_evidence(bindings.get(code, []))
        layers = sorted({e.layer for e in ev})
        if layers:
            covered += 1
            for layer in layers:
                layer_counts[layer] += 1
        else:
            uncovered_by_profile[profile].append(code)
        executable_layers_for_code = sorted(
            layer for layer in layers if layer in executable_layers
        )
        if executable_layers_for_code:
            executable_covered += 1
            for layer in executable_layers_for_code:
                executable_layer_counts[layer] += 1
        else:
            executable_uncovered_by_profile[profile].append(code)
        rules_json.append(
            {
                "code": code,
                "profile": profile,
                "manifest_source": source_by_code.get(code),
                "enforced_in": layers,
                "executable_enforced_in": executable_layers_for_code,
                "evidence": [
                    {"layer": e.layer, "file": e.file, "line": e.line, "text": e.text}
                    for e in ev[:20]
                ],
            }
        )

    payload = {
        "summary": {
            "total_rule_codes": total,
            "covered_rule_codes": covered,
            "uncovered_rule_codes": total - covered,
            "executable_covered_rule_codes": executable_covered,
            "executable_uncovered_rule_codes": total - executable_covered,
            "profile_counts": dict(profile_counts),
            "layer_coverage_counts": dict(sorted(layer_counts.items())),
            "executable_layer_coverage_counts": dict(sorted(executable_layer_counts.items())),
        },
        "rules": rules_json,
    }
    out_json.parent.mkdir(parents=True, exist_ok=True)
    out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    lines = []
    lines.append("# Rule Binding Inventory")
    lines.append("")
    lines.append(f"- Total rule codes: {total}")
    lines.append(f"- Referenced by >=1 layer: {covered}")
    lines.append(f"- Unreferenced: {total - covered}")
    lines.append(f"- Executably enforced by >=1 layer: {executable_covered}")
    lines.append(f"- Not executably enforced: {total - executable_covered}")
    lines.append("")
    lines.append("## Reference coverage by layer")
    for layer, count in sorted(layer_counts.items()):
        lines.append(f"- {layer}: {count}")
    lines.append("")
    lines.append("## Executable coverage by layer")
    for layer, count in sorted(executable_layer_counts.items()):
        lines.append(f"- {layer}: {count}")
    lines.append("")
    lines.append("## Not executably enforced by profile")
    for profile in sorted(profile_counts.keys()):
        missing = sorted(executable_uncovered_by_profile.get(profile, []))
        lines.append(f"- {profile}: {len(missing)}")
        for code in missing[:50]:
            lines.append(f"  - {code}")
    lines.append("")
    out_md.parent.mkdir(parents=True, exist_ok=True)
    out_md.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--frontend-root",
        default="../frontend/E2BR3-frontend",
        help="Frontend project path (relative to backend repo root).",
    )
    parser.add_argument(
        "--out-json",
        default="docs/generated/manifests/rule_binding_inventory.json",
    )
    parser.add_argument(
        "--out-md",
        default="docs/generated/manifests/rule_binding_inventory.md",
    )
    args = parser.parse_args()

    root = repo_root_from_here()
    frontend = (root / args.frontend_root).resolve()

    codes, source_by_code, profile_by_code = manifest_codes(root)
    bindings: Dict[str, List[Evidence]] = defaultdict(list)

    case_registry = parse_const_rule_map(
        root / "crates/libs/lib-core/src/xml/validate/case_detector_registry.rs"
    )
    export_registry = parse_const_rule_map(
        root / "crates/libs/lib-core/src/xml/validate/export_transform_registry.rs"
    )

    # Backend case semantic validators (code via CASE_RULE_* constants).
    scan_layer_const(
        bindings,
        "backend_case_semantic",
        [
            root / "crates/libs/lib-core/src/xml/ich/validation.rs",
            root / "crates/libs/lib-core/src/xml/fda/validation.rs",
            root / "crates/libs/lib-core/src/xml/mfds/validation.rs",
        ],
        case_registry,
        codes,
        root,
    )

    # Backend XML business checks (XML rule strings used directly).
    scan_layer_literal(
        bindings,
        "backend_xml_business",
        list((root / "crates/libs/lib-core/src/xml/xml_validation").rglob("*.rs")),
        codes,
        root,
    )
    scan_layer_literal(
        bindings,
        "backend_xml_business_registry",
        [root / "crates/libs/lib-core/src/xml/validate/xml_detector_registry.rs"],
        codes,
        root,
    )

    # Backend export postprocess (code via EXPORT_RULE_* constants).
    scan_layer_const(
        bindings,
        "backend_export_postprocess",
        [
            root / "crates/libs/lib-core/src/xml/export_postprocess.rs",
            root / "crates/libs/lib-core/src/xml/export_runtime.rs",
        ],
        export_registry,
        codes,
        root,
    )
    scan_layer_literal(
        bindings,
        "backend_export_postprocess_registry",
        [root / "crates/libs/lib-core/src/xml/validate/export_transform_registry.rs"],
        codes,
        root,
    )

    # Canonical catalog metadata.
    scan_layer_literal(
        bindings,
        "backend_catalog_metadata",
        [root / "crates/libs/lib-core/src/xml/validate/catalog.rs"],
        codes,
        root,
    )

    # Backend tests.
    scan_layer_literal(
        bindings,
        "backend_tests",
        list((root / "crates/libs/lib-core/tests").rglob("*.rs"))
        + list((root / "crates/services/web-server/tests").rglob("*.rs")),
        codes,
        root,
    )

    if frontend.exists():
        # Frontend rule metadata/loading.
        scan_layer_literal(
            bindings,
            "frontend_validation_runtime",
            [
                frontend / "lib/validation/rules.ts",
                frontend / "lib/validation/createGate.ts",
            ],
            codes,
            root,
        )
        # Frontend UI required badges.
        scan_layer_literal(
            bindings,
            "frontend_ui_required_flags",
            list((frontend / "components/case-form/sections").rglob("*.tsx")),
            codes,
            root,
        )
        # Frontend backend issue mapping table.
        scan_layer_literal(
            bindings,
            "frontend_backend_issue_mapping",
            [frontend / "lib/validation/backendIssuePath.ts"],
            codes,
            root,
        )
        # Frontend zod syntax layer.
        scan_layer_literal(
            bindings,
            "frontend_syntax_zod",
            list((frontend / "lib/zod").rglob("*.ts")),
            codes,
            root,
        )
        # Frontend tests/scripts.
        scan_layer_literal(
            bindings,
            "frontend_tests",
            list((frontend / "node-tests").rglob("*.ts"))
            + list((frontend / "__tests__").rglob("*.ts"))
            + list((frontend / "scripts").rglob("*.mjs")),
            codes,
            root,
        )

    write_outputs(
        root,
        root / args.out_json,
        root / args.out_md,
        codes,
        profile_by_code,
        source_by_code,
        bindings,
    )


if __name__ == "__main__":
    main()
