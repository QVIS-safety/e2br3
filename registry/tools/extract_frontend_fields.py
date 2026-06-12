#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


REPO_ROOT = Path(__file__).resolve().parents[2]
FRONTEND_ROOT = REPO_ROOT.parent / "frontend" / "E2BR3-frontend"


class FrontendInventoryError(Exception):
    pass


@dataclass(frozen=True)
class FrontendField:
    key: str
    section: str
    field: str
    file: str
    raw: str


DEFAULT_SOURCE_GLOBS = [
    "../frontend/E2BR3-frontend/components/case-form/sections/SectionC*.tsx",
    "../frontend/E2BR3-frontend/components/case-form/sections/SectionD.tsx",
    "../frontend/E2BR3-frontend/components/case-form/sections/SectionDH.tsx",
    "../frontend/E2BR3-frontend/components/case-form/sections/SectionE.tsx",
    "../frontend/E2BR3-frontend/components/case-form/sections/SectionF.tsx",
    "../frontend/E2BR3-frontend/components/case-form/sections/SectionG.tsx",
    "../frontend/E2BR3-frontend/components/case-form/sections/SectionH.tsx",
]

ALLOWED_FIELD_ROOTS = {
    "caseSummaryInformation",
    "drugReactionAssessments",
    "drugs",
    "literatureReferences",
    "narrative",
    "patientInformation",
    "primarySources",
    "reactions",
    "safetyReportIdentification",
    "studyInformation",
    "testResults",
}
INDEX_PLACEHOLDERS = {
    "activeIndex",
    "assessIndex",
    "doseIndex",
    "drugIndex",
    "index",
    "indicationIndex",
    "selectedIndex",
    "sourceIndex",
    "subIndex",
}


def normalize_field_path(raw: str) -> str:
    value = raw.strip()
    value = re.sub(r"\.\$\{[^}]+\}", "", value)
    value = re.sub(r"\.\d+(?=\.|$)", "", value)
    value = value.replace("`", "").replace('"', "").replace("'", "")
    value = re.sub(r"\.+", ".", value)
    return value.strip(".")


def split_key(key: str) -> tuple[str, str]:
    if "." not in key:
        return key, ""
    section, field = key.split(".", 1)
    return section, field


def is_frontend_field_key(key: str) -> bool:
    if not key or "${" in key or "}" in key:
        return False
    root, field = split_key(key)
    return bool(field) and root in ALLOWED_FIELD_ROOTS


def is_extractable_raw_path(raw: str) -> bool:
    placeholders = set(re.findall(r"\$\{([^}]+)\}", raw))
    return placeholders.issubset(INDEX_PLACEHOLDERS)


FIELD_PATTERNS = [
    re.compile(r"name\s*=\s*[\"']([^\"']+)[\"']"),
    re.compile(r"name\s*=\s*\{\s*`([^`]+)`\s*\}"),
    re.compile(r"register\(\s*[\"']([^\"']+)[\"']"),
    re.compile(r"register\(\s*`([^`]+)`"),
]


def extract_raw_field_paths_from_source(source: str) -> list[str]:
    fields: set[str] = set()
    for pattern in FIELD_PATTERNS:
        for match in pattern.finditer(source):
            raw = match.group(1)
            if raw:
                fields.add(raw.strip())
    fields.update(expand_name_placeholder_paths(source, fields))
    return sorted(fields)


def expand_name_placeholder_paths(source: str, raw_fields: Iterable[str]) -> set[str]:
    tuple_names: set[str] = set()
    for block in re.findall(
        r"\{\s*\[(.*?)\]\.map\(\s*\(\s*\[\s*name\s*,",
        source,
        flags=re.DOTALL,
    ):
        tuple_names.update(re.findall(r"\[\s*[\"']([A-Za-z][A-Za-z0-9_]*)[\"']\s*,", block))
    if not tuple_names:
        return set()

    expanded: set[str] = set()
    for raw in raw_fields:
        if not raw.endswith(".${name}"):
            continue
        prefix = raw.removesuffix(".${name}")
        for name in tuple_names:
            expanded.add(f"{prefix}.{name}")
    return expanded


def extract_field_paths_from_source(source: str) -> list[str]:
    return sorted(
        key
        for key in (
            normalize_field_path(raw)
            for raw in extract_raw_field_paths_from_source(source)
            if is_extractable_raw_path(raw)
        )
        if is_frontend_field_key(key)
    )


def extract_frontend_fields(
    root: Path = REPO_ROOT,
    source_globs: Iterable[str] = DEFAULT_SOURCE_GLOBS,
) -> list[FrontendField]:
    inventory: dict[tuple[str, str, str], FrontendField] = {}
    source_root = root.parent if root.name == "registry" else root
    for source_glob in source_globs:
        matches = sorted(source_root.glob(source_glob))
        if not matches:
            raise FrontendInventoryError(f"frontend source glob matched no files: {source_glob}")
        for source_path in matches:
            try:
                source = source_path.read_text(encoding="utf-8")
            except OSError as exc:
                raise FrontendInventoryError(f"could not read frontend source {source_path}: {exc}") from exc

            for raw_path in extract_raw_field_paths_from_source(source):
                if not is_extractable_raw_path(raw_path):
                    continue
                key = normalize_field_path(raw_path)
                if not is_frontend_field_key(key):
                    continue
                section, field = split_key(key)
                try:
                    relative_file = str(source_path.relative_to(source_root))
                except ValueError:
                    relative_file = str(source_path)
                inventory[(key, relative_file, raw_path)] = FrontendField(
                    key=key,
                    section=section,
                    field=field,
                    file=relative_file,
                    raw=raw_path,
                )

    return sorted(inventory.values(), key=lambda item: (item.key, item.file, item.raw))


def fields_to_json(fields: Iterable[FrontendField]) -> str:
    payload = [
        {
            "key": field.key,
            "section": field.section,
            "field": field.field,
            "file": field.file,
            "raw": field.raw,
        }
        for field in sorted(fields, key=lambda item: (item.key, item.file, item.raw))
    ]
    return json.dumps(payload, indent=2) + "\n"


def main() -> int:
    try:
        fields = extract_frontend_fields()
    except FrontendInventoryError as exc:
        print(f"frontend inventory extraction failed: {exc}", file=sys.stderr)
        return 1
    print(fields_to_json(fields), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
