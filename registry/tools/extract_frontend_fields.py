#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import subprocess
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
    "../frontend/E2BR3-frontend/app/(protected)/*/case/*/detail/**/*.tsx",
]

ALLOWED_FIELD_ROOTS = {
    "caseSummaryInformation",
    "drugReactionAssessments",
    "drugs",
    "literatureReferences",
    "messageHeader",
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

# Field names composed from an object-array `.map` callback, e.g. a
# `SERIOUSNESS_CRITERIA = [{ name: "criteria..." }] as const` list rendered via
# `name={`reactions.${i}.seriousness.${criterion.name}`}`. The concrete field
# name lives on the array element and never appears literally at the `name=`
# call site, so it is resolved here from the `as const` source array.
OBJECT_NAME_TEMPLATE = re.compile(r"`([^`]*?)\$\{[A-Za-z_]\w*\.name\}`")
OBJECT_ARRAY_AS_CONST = re.compile(r"=\s*\[(.*?)\]\s*as const", re.DOTALL)
OBJECT_ARRAY_NAME_ENTRY = re.compile(r"\bname:\s*[\"']([A-Za-z][A-Za-z0-9_]*)[\"']")


def expand_object_name_map_paths(source: str) -> set[str]:
    prefixes = {prefix.rstrip(".") for prefix in OBJECT_NAME_TEMPLATE.findall(source)}
    if not prefixes:
        return set()

    names: set[str] = set()
    for array_body in OBJECT_ARRAY_AS_CONST.findall(source):
        names.update(OBJECT_ARRAY_NAME_ENTRY.findall(array_body))
    if not names:
        return set()

    return {f"{prefix}.{name}" for prefix in prefixes if prefix for name in names}


def extract_raw_field_paths_from_source(source: str) -> list[str]:
    fields: set[str] = set()
    for pattern in FIELD_PATTERNS:
        for match in pattern.finditer(source):
            raw = match.group(1)
            if raw:
                fields.add(raw.strip())
    fields.update(expand_name_placeholder_paths(source, fields))
    fields.update(expand_object_name_map_paths(source))
    return sorted(fields)


def extract_frontend_fields_ast(root: Path = REPO_ROOT) -> list[FrontendField]:
    repo_root = root if (root / "registry").exists() else root.parent
    frontend_root = repo_root.parent / "frontend" / "E2BR3-frontend"
    script = Path(__file__).with_suffix(".mjs")
    completed = subprocess.run(
        ["node", str(script), str(frontend_root)],
        check=False,
        capture_output=True,
        text=True,
    )
    if completed.returncode:
        raise FrontendInventoryError(completed.stderr.strip() or "AST frontend extraction failed")
    payload = json.loads(completed.stdout)
    return [
        FrontendField(
            key=item["key"],
            section=split_key(item["key"])[0],
            field=split_key(item["key"])[1],
            file=item["file"],
            raw=item["key"],
        )
        for item in payload
    ]


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
