import os
import re
from dataclasses import dataclass
from pathlib import Path

import validate


@dataclass(frozen=True)
class PresaveSectionConfig:
    frontend_section: str
    interface_name: str
    form_file: str
    backend_models: tuple[str, ...]
    transfer_files: tuple[str, ...]


PRESAVE_MODEL_FILE = "crates/libs/lib-core/src/model/presave.rs"
PRESAVE_TYPE_FILE = "lib/types/presave.ts"
PRESAVE_SECTIONS = {
    "sender": PresaveSectionConfig(
        "sender",
        "SenderPresaveData",
        "components/presave/SenderForm.tsx",
        ("SenderPresave", "SenderPresaveGateway", "SenderPresaveResponsiblePerson"),
        ("app/(protected)/[authority]/case/[id]/detail/SD/hooks/useSenderPresaveImport.ts",),
    ),
    "receiver": PresaveSectionConfig(
        "receiver",
        "ReceiverPresaveData",
        "components/presave/ReceiverForm.tsx",
        ("ReceiverPresave", "ReceiverPresaveConsignee", "ReceiverPresaveRoute"),
        ("app/(protected)/[authority]/case/[id]/detail/SD/hooks/useReceiverPresaveImport.ts",),
    ),
    "product": PresaveSectionConfig(
        "product",
        "ProductPresaveData",
        "components/presave/ProductForm.tsx",
        ("ProductPresave", "ProductPresaveSubstance"),
        (
            "app/(protected)/[authority]/case/[id]/detail/DG/components/SectionG.tsx",
            "app/(protected)/[authority]/case/[id]/detail/DG/hooks/useSectionGDrugs.ts",
        ),
    ),
    "reporter": PresaveSectionConfig(
        "reporter",
        "ReporterPresaveData",
        "components/presave/ReporterForm.tsx",
        ("ReporterPresave",),
        ("app/(protected)/[authority]/case/[id]/detail/RP/model/rpModel.ts",),
    ),
    "study": PresaveSectionConfig(
        "study",
        "StudyPresaveData",
        "components/presave/StudyForm.tsx",
        (
            "StudyPresave",
            "StudyPresaveRegistrationNumber",
            "StudyPresaveFdaCrossReportedInd",
            "StudyPresaveProduct",
            "StudyPresaveReporter",
        ),
        ("app/(protected)/[authority]/case/[id]/detail/SI/hooks/useStudyImport.ts",),
    ),
    "narrative": PresaveSectionConfig(
        "narrative",
        "NarrativePresaveData",
        "components/presave/NarrativeForm.tsx",
        ("NarrativePresave",),
        ("app/(protected)/[authority]/case/[id]/detail/NR/NRPage.tsx",),
    ),
}


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
    "reporterTitleNullFlavor": "reporter_title_null_flavor",
    "reporterGivenNameNullFlavor": "reporter_given_name_null_flavor",
    "reporterMiddleNameNullFlavor": "reporter_middle_name_null_flavor",
    "reporterFamilyNameNullFlavor": "reporter_family_name_null_flavor",
    "reporterOrganizationNullFlavor": "organization_null_flavor",
    "reporterDepartmentNullFlavor": "department_null_flavor",
    "reporterStreetNullFlavor": "street_null_flavor",
    "reporterCityNullFlavor": "city_null_flavor",
    "reporterStateNullFlavor": "state_null_flavor",
    "reporterPostcodeNullFlavor": "postcode_null_flavor",
    "reporterTelephoneNullFlavor": "telephone_null_flavor",
    "reporterCountryNullFlavor": "country_code_null_flavor",
    "qualificationNullFlavor": "qualification_null_flavor",
}
PRIMARY_SOURCE_FRONTEND_TO_BACKEND = dict(REPORTER_FRONTEND_TO_BACKEND)
REPORTER_TRANSFER_FILE = (
    "../frontend/E2BR3-frontend/app/(protected)/[authority]/case/[id]/detail/"
    "RP/model/rpModel.ts"
)


def resolve_frontend_path(root: Path, relative: str) -> Path:
    explicit_root = os.environ.get("E2BR3_FRONTEND_ROOT")
    if explicit_root:
        return Path(explicit_root) / relative
    repo_root = root if (root / "registry").exists() else root.parent
    candidates = (
        repo_root.parent / "frontend" / "E2BR3-frontend" / relative,
        repo_root / "frontend" / "E2BR3-frontend" / relative,
    )
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    return candidates[-1]


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


def _interface_source(source: str, interface_name: str) -> str:
    match = re.search(
        rf"export interface {re.escape(interface_name)}\s*\{{(?P<body>.*?)^\}}",
        source,
        re.MULTILINE | re.DOTALL,
    )
    if not match:
        raise validate.InventoryError(f"{interface_name} interface not found")
    return match.group("body")


def extract_presave_frontend(root: Path, section: str) -> set[str]:
    config = PRESAVE_SECTIONS[section]
    form_path = resolve_frontend_path(root, config.form_file)
    type_path = resolve_frontend_path(root, PRESAVE_TYPE_FILE)
    for path in (form_path, type_path):
        if not path.is_file():
            raise validate.InventoryError(f"presave frontend source not found: {path}")
    type_source = _interface_source(
        type_path.read_text(encoding="utf-8"), config.interface_name
    )
    type_fields = extract_presave_frontend_source(type_source, section)
    form_fields = extract_presave_frontend_source(
        form_path.read_text(encoding="utf-8"), section
    )
    return type_fields & (form_fields | type_fields)


def extract_reporter_frontend(root: Path) -> set[str]:
    return extract_presave_frontend(root, "reporter")


def extract_presave_backend(root: Path, models: dict[str, str]) -> set[str]:
    repo_root = root if (root / "registry").exists() else root.parent
    fields: set[str] = set()
    for model, relative in models.items():
        path = repo_root / relative
        if not path.is_file():
            raise validate.InventoryError(f"presave backend source not found: {path}")
        fields.update(
            extract_rust_presave_source(path.read_text(encoding="utf-8"), model)
        )
    return fields


def extract_section_backend(root: Path, section: str) -> set[str]:
    config = PRESAVE_SECTIONS[section]
    return extract_presave_backend(
        root, {model: PRESAVE_MODEL_FILE for model in config.backend_models}
    )


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
    if "normalizePrimarySourceValue" in source and "primarySourceForRegulatoryPurposes" in source:
        pairs.add(
            (
                "ReporterPresave.primary_source_regulatory",
                "PrimarySource.primary_source_regulatory",
            )
        )
    return pairs


def extract_reporter_transfers(root: Path) -> set[tuple[str, str]]:
    path = resolve_frontend_path(
        root, PRESAVE_SECTIONS["reporter"].transfer_files[0]
    )
    if not path.is_file():
        raise validate.InventoryError(f"reporter transfer source not found: {path}")
    return extract_reporter_transfer_source(path.read_text(encoding="utf-8"))


def extract_presave_transfers(root: Path, section: str) -> set[tuple[str, str]]:
    if section == "reporter":
        return extract_reporter_transfers(root)
    return set()
