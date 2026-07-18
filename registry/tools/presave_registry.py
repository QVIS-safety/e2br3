from dataclasses import dataclass
from pathlib import Path
from typing import Any

import validate


@dataclass(frozen=True)
class PresaveRegistry:
    rows: tuple[dict[str, Any], ...]
    by_code: dict[str, dict[str, Any]]
    backend_keys: dict[str, str]
    frontend_keys: dict[str, str]


def load_presave_registry(
    root: Path, result: validate.ValidationResult
) -> PresaveRegistry:
    index_path = root / "presaves" / "index.json"
    index = validate.load_json(index_path, result)
    rows: list[dict[str, Any]] = []
    by_code: dict[str, dict[str, Any]] = {}
    backend_keys: dict[str, str] = {}
    frontend_keys: dict[str, str] = {}

    if not isinstance(index, dict) or not isinstance(index.get("sections"), list):
        result.add(f"{index_path}: sections must be a non-empty list")
        return PresaveRegistry((), {}, {}, {})

    for relative in index["sections"]:
        if not isinstance(relative, str):
            result.add(f"{index_path}: section entries must be strings")
            continue
        source = root / "presaves" / relative
        payload = validate.load_json(source, result)
        if not isinstance(payload, list):
            result.add(f"{source}: section file must contain a JSON array")
            continue
        for row in payload:
            validate.validate_row(row, source, result)
            if not isinstance(row, dict) or not isinstance(
                row.get("e2br3_code"), str
            ):
                continue
            code = row["e2br3_code"]
            if code in by_code:
                result.add(f"{row.get('id')}: duplicate presave e2br3_code {code}")
                continue
            by_code[code] = row
            rows.append(row)
            backend = row.get("backend", {})
            frontend = row.get("frontend", {})
            if isinstance(backend, dict) and backend.get("status") == "mapped":
                backend_keys[code] = f"{backend['model']}.{backend['field']}"
            if isinstance(frontend, dict) and frontend.get("status") == "mapped":
                frontend_keys[code] = f"{frontend['section']}.{frontend['field']}"

    return PresaveRegistry(tuple(rows), by_code, backend_keys, frontend_keys)
