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
REPORTER_FRONTEND_TO_BACKEND = {
    "reporterTitle": "reporter_title",
    "reporterGivenName": "reporter_given_name",
    "reporterMiddleName": "reporter_middle_name",
    "reporterFamilyName": "reporter_family_name",
    "reporterOrganization": "organization",
    "reporterDepartment": "department",
    "reporterStreet": "street",
    "reporterCity": "city",
    "reporterState": "state",
    "reporterPostcode": "postcode",
    "reporterTelephone": "telephone",
    "reporterCountry": "country_code",
    "qualification": "qualification",
    "qualificationKr1": "qualification_kr1",
    "primarySourceForRegulatoryPurposes": "primary_source_regulatory",
    "reporterNameNullFlavor": "reporter_name_null_flavor",
    "reporterAddressNullFlavor": "reporter_address_null_flavor",
    "reporterCountryNullFlavor": "country_code_null_flavor",
    "qualificationNullFlavor": "qualification_null_flavor",
}
PRIMARY_SOURCE_FRONTEND_TO_BACKEND = dict(REPORTER_FRONTEND_TO_BACKEND)
REPORTER_TRANSFER_FILE = (
    "../frontend/E2BR3-frontend/app/(protected)/[authority]/case/[id]/detail/"
    "RP/model/rpModel.ts"
)


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


def extract_reporter_transfer_source(source: str) -> set[tuple[str, str]]:
    pairs: set[tuple[str, str]] = set()
    for match in re.finditer(
        r"(?P<target>[A-Za-z][A-Za-z0-9]*)\s*:\s*[^,\n]*?"
        r"data\.(?P<source>[A-Za-z][A-Za-z0-9]*)",
        source,
    ):
        source_field = REPORTER_FRONTEND_TO_BACKEND.get(match.group("source"))
        target_field = PRIMARY_SOURCE_FRONTEND_TO_BACKEND.get(match.group("target"))
        if source_field and target_field:
            pairs.add(
                (
                    f"ReporterPresave.{source_field}",
                    f"PrimarySource.{target_field}",
                )
            )
    return pairs


def extract_reporter_transfers(root: Path) -> set[tuple[str, str]]:
    path = root / REPORTER_TRANSFER_FILE
    if not path.is_file():
        raise validate.InventoryError(f"reporter transfer source not found: {path}")
    return extract_reporter_transfer_source(path.read_text(encoding="utf-8"))
