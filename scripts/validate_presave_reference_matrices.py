#!/usr/bin/env python3
"""Validate canonical E2BR3 presave reference matrices."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
INDEX = ROOT / "docs/reference-matrices/presave/index.json"
ALLOWED_CATEGORIES = {
    "referenceImportedToCase",
    "referencePreserveOnly",
    "localSystemOnly",
    "removed",
}
FORBIDDEN_CATEGORY_WORDS = {
    "",
    "unknown",
    "maybe",
    "tbd",
    "todo",
    "fallback",
    "ambiguous",
}
ALLOWED_STATUSES = {"aligned", "partially_aligned", "not_aligned"}
REQUIRED_HEADER = [
    "field",
    "reference evidence",
    "local frontend",
    "local backend/BMC",
    "case target",
    "category",
    "action",
]
ONE_SOURCE_HEADER = [
    "field",
    "reference evidence",
    "canonical frontend source",
    "canonical backend source",
    "allowed read aliases",
    "allowed write keys",
    "case import target",
    "duplicate sources found",
    "category",
    "decision",
    "tests required",
]
ONE_SOURCE_MATRIX_IDS = {
    "narrative-h",
    "product-dg",
    "reporter-c2r",
    "sender-c3",
    "study-c5",
}


def split_row(line: str) -> list[str]:
    return [cell.strip().strip("`") for cell in line.strip().strip("|").split("|")]


def is_separator(cells: list[str]) -> bool:
    return bool(cells) and all(re.fullmatch(r":?-{3,}:?", cell.strip()) for cell in cells)


def find_matrix_rows(path: Path) -> tuple[list[str], list[list[str]]]:
    lines = path.read_text(encoding="utf-8").splitlines()
    for idx, line in enumerate(lines):
        if not line.startswith("|"):
            continue
        header = split_row(line)
        lowered = [cell.lower() for cell in header]
        if "category" not in lowered:
            continue
        rows: list[list[str]] = []
        for row_line in lines[idx + 1 :]:
            if not row_line.startswith("|"):
                break
            cells = split_row(row_line)
            if is_separator(cells):
                continue
            rows.append(cells)
        return header, rows
    raise ValueError("missing markdown table with a category column")


def require_coverage(path: Path, text: str) -> list[str]:
    errors: list[str] = []
    if "coverage check" not in text.lower():
        errors.append(f"{path}: missing Coverage Check section")
    if not re.search(r"(uncategorized fields:\s*0|zero uncategorized fields)", text, re.I):
        errors.append(f"{path}: coverage must state zero uncategorized fields")
    if not re.search(r"(ambiguous fields:\s*0|zero ambiguous fields)", text, re.I):
        errors.append(f"{path}: coverage must state zero ambiguous fields")
    return errors


def validate_matrix(matrix_id: str, path: Path) -> list[str]:
    errors: list[str] = []
    text = path.read_text(encoding="utf-8")
    header, rows = find_matrix_rows(path)
    required_header = (
        ONE_SOURCE_HEADER if matrix_id in ONE_SOURCE_MATRIX_IDS else REQUIRED_HEADER
    )
    if header != required_header:
        errors.append(f"{path}: matrix header must be {required_header!r}")
    category_index = [cell.lower() for cell in header].index("category")

    if not rows:
        errors.append(f"{path}: matrix table has no data rows")

    for row_num, row in enumerate(rows, start=1):
        if category_index >= len(row):
            errors.append(f"{path}: row {row_num} has no category cell")
            continue
        category = row[category_index].strip().strip("`")
        if category.lower() in FORBIDDEN_CATEGORY_WORDS:
            errors.append(f"{path}: row {row_num} has forbidden category {category!r}")
        elif category not in ALLOWED_CATEGORIES:
            errors.append(f"{path}: row {row_num} has unsupported category {category!r}")

    errors.extend(require_coverage(path, text))
    return errors


def main() -> int:
    errors: list[str] = []
    try:
        index = json.loads(INDEX.read_text(encoding="utf-8"))
    except Exception as exc:  # noqa: BLE001 - report exact JSON/file failure.
        print(f"{INDEX}: {exc}", file=sys.stderr)
        return 1

    ids: set[str] = set()
    for entry in index.get("matrices", []):
        matrix_id = entry.get("id")
        if not matrix_id:
            errors.append("index.json: matrix entry missing id")
            continue
        if matrix_id in ids:
            errors.append(f"index.json: duplicate matrix id {matrix_id}")
        ids.add(matrix_id)

        status = entry.get("status")
        if status not in ALLOWED_STATUSES:
            errors.append(f"index.json: {matrix_id} has invalid status {status!r}")

        matrix_path = ROOT / entry.get("path", "")
        if not matrix_path.is_file():
            errors.append(f"index.json: {matrix_id} path does not exist: {matrix_path}")
            continue
        errors.extend(validate_matrix(matrix_id, matrix_path))

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    print(f"Validated {len(ids)} presave reference matrices.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
