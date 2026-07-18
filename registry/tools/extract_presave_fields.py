import re
from pathlib import Path

import validate


REPORTER_FRONTEND_FILES = (
    "../frontend/E2BR3-frontend/components/presave/ReporterForm.tsx",
    "../frontend/E2BR3-frontend/lib/types/presave.ts",
)
REPORTER_BACKEND_MODELS = {
    "ReporterPresave": "crates/libs/lib-core/src/model/presave.rs",
}
TECHNICAL_FIELDS = {
    "id",
    "organization_id",
    "deleted",
    "created_at",
    "updated_at",
    "created_by",
    "updated_by",
}


def extract_rust_presave_source(source: str, model: str) -> set[str]:
    return {
        f"{model}.{field}"
        for field in validate.extract_rust_struct_fields(source, model)
        if field not in TECHNICAL_FIELDS
    }


def extract_presave_frontend_source(source: str, section: str) -> set[str]:
    names = set(
        re.findall(
            r'(?:register\(|name\s*=\s*)[^\n]*?["\'`]([A-Za-z][A-Za-z0-9.]*)',
            source,
        )
    )
    names.update(
        re.findall(r"^\s{2}([A-Za-z][A-Za-z0-9]+)\??:\s", source, re.MULTILINE)
    )
    return {f"{section}.{name}" for name in names if name not in {"id", "deleted"}}


def _reporter_type_source(source: str) -> str:
    match = re.search(
        r"export interface ReporterPresaveData\s*\{(?P<body>.*?)^\}",
        source,
        re.MULTILINE | re.DOTALL,
    )
    if not match:
        raise validate.InventoryError("ReporterPresaveData interface not found")
    return match.group("body")


def extract_reporter_frontend(root: Path) -> set[str]:
    fields: set[str] = set()
    for relative in REPORTER_FRONTEND_FILES:
        path = root / relative
        if not path.is_file():
            raise validate.InventoryError(f"presave frontend source not found: {path}")
        source = path.read_text(encoding="utf-8")
        if path.name == "presave.ts":
            source = _reporter_type_source(source)
        fields.update(extract_presave_frontend_source(source, "reporter"))
    return fields


def extract_presave_backend(root: Path, models: dict[str, str]) -> set[str]:
    fields: set[str] = set()
    for model, relative in models.items():
        path = root / relative
        if not path.is_file():
            raise validate.InventoryError(f"presave backend source not found: {path}")
        fields.update(
            extract_rust_presave_source(path.read_text(encoding="utf-8"), model)
        )
    return fields
