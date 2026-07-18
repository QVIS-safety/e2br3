#!/usr/bin/env python3
from __future__ import annotations

import json
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import extract_frontend_fields


ROOT = Path(__file__).resolve().parents[1]

AUTHORITIES = {"ICH", "FDA", "MFDS"}
SECTIONS = {"N", "C", "D", "E", "F", "G", "H"}
ROW_STATUSES = {
    "complete",
    "backend_missing",
    "frontend_missing",
    "intentionally_unmapped",
    "not_applicable",
    "conflict",
}
MAPPING_STATUSES = {"mapped", "missing", "not_applicable", "conflict"}
ALLOWED_ROW_FIELDS = {
    "id",
    "e2br3_code",
    "label",
    "section",
    "authority",
    "status",
    "backend",
    "frontend",
    "evidence",
    "action",
    "notes",
    "local_only",
}
DICTIONARY_KINDS = {"element", "group"}
DICTIONARY_CONFORMANCES = {"mandatory", "conditional_mandatory", "optional", "required"}
ALLOWED_VALUE_CONSTRAINT_KINDS = {
    "code_set",
    "boolean",
    "true_marker",
    "numeric",
    "format",
    "vocabulary",
    "descriptive",
}
ALLOWED_VALUE_CONSTRAINT_FIELDS = {
    "kind",
    "values",
    "numeric_shape",
    "format_name",
    "vocabulary_scope",
    "identifier_profile",
    "enforcement",
}
NUMERIC_SHAPES = {"decimal", "integer", "dotted_version"}
FORMAT_NAMES = {"e2b_datetime", "base64", "ich_identifier"}
VOCABULARY_SCOPES = {
    "all",
    "time",
    "gestation",
    "dose",
    "frequency",
    "dose_form",
    "route",
    "item_seq",
}
VOCABULARY_RECEIVERS = {"KR", "FR"}
IDENTIFIER_PROFILES = {"mpid", "phpid", "substance_id"}
CONSTRAINT_ENFORCEMENTS = {"case_validate", "representation_enforced"}
DICTIONARY_VOCABULARIES = {
    "MedDRA",
    "WHODrug",
    "ISO3166",
    "ISO639",
    "sex",
    "UCUM",
    "EDQM",
    "MFDS_PRODUCT",
}
ALLOWED_DICTIONARY_ENTRY_FIELDS = {
    "code",
    "name",
    "name_kr",
    "section",
    "kind",
    "conformance",
    "data_type",
    "max_length",
    "allowed_values",
    "allowed_value_constraint",
    "null_flavors",
    "oid",
    "profiles",
    "xpath",
    "hl7_data_type",
    "hl7_component",
    "vocabulary",
    "vocabulary_variants",
    "fda_severity",
    "fda_error_id",
    "condition_text",
    "notes",
}
FDA_SEVERITIES = {"rejection", "warning"}
BACKEND_MODELS = {
    "SafetyReportIdentification": "crates/libs/lib-core/src/model/safety_report.rs",
    "SenderInformation": "crates/libs/lib-core/src/model/safety_report.rs",
    "PrimarySource": "crates/libs/lib-core/src/model/safety_report.rs",
    "LiteratureReference": "crates/libs/lib-core/src/model/safety_report.rs",
    "DocumentsHeldBySender": "crates/libs/lib-core/src/model/safety_report.rs",
    "StudyInformation": "crates/libs/lib-core/src/model/safety_report.rs",
    "StudyRegistrationNumber": "crates/libs/lib-core/src/model/safety_report.rs",
    "StudyFdaCrossReportedInd": "crates/libs/lib-core/src/model/safety_report.rs",
    "Case": "crates/libs/lib-core/src/model/case.rs",
    "SourceDocument": "crates/libs/lib-core/src/model/case.rs",
    "OtherCaseIdentifier": "crates/libs/lib-core/src/model/case_identifiers.rs",
    "LinkedReportNumber": "crates/libs/lib-core/src/model/case_identifiers.rs",
    "ReceiverInformation": "crates/libs/lib-core/src/model/receiver.rs",
    "PatientInformation": "crates/libs/lib-core/src/model/patient.rs",
    "PatientIdentifier": "crates/libs/lib-core/src/model/patient.rs",
    "MedicalHistoryEpisode": "crates/libs/lib-core/src/model/patient.rs",
    "PastDrugHistory": "crates/libs/lib-core/src/model/patient.rs",
    "PatientDeathInformation": "crates/libs/lib-core/src/model/patient.rs",
    "ReportedCauseOfDeath": "crates/libs/lib-core/src/model/patient.rs",
    "AutopsyCauseOfDeath": "crates/libs/lib-core/src/model/patient.rs",
    "ParentInformation": "crates/libs/lib-core/src/model/patient.rs",
    "ParentMedicalHistory": "crates/libs/lib-core/src/model/parent_history.rs",
    "ParentPastDrugHistory": "crates/libs/lib-core/src/model/parent_history.rs",
    "Reaction": "crates/libs/lib-core/src/model/reaction.rs",
    "TestResult": "crates/libs/lib-core/src/model/test_result.rs",
    "DrugInformation": "crates/libs/lib-core/src/model/drug.rs",
    "DrugActiveSubstance": "crates/libs/lib-core/src/model/drug.rs",
    "DosageInformation": "crates/libs/lib-core/src/model/drug.rs",
    "DrugIndication": "crates/libs/lib-core/src/model/drug.rs",
    "DrugDeviceCharacteristic": "crates/libs/lib-core/src/model/drug.rs",
    "DrugReactionAssessment": "crates/libs/lib-core/src/model/drug_reaction_assessment.rs",
    "RelatednessAssessment": "crates/libs/lib-core/src/model/drug_reaction_assessment.rs",
    "NarrativeInformation": "crates/libs/lib-core/src/model/narrative.rs",
    "SenderDiagnosis": "crates/libs/lib-core/src/model/narrative.rs",
    "CaseSummaryInformation": "crates/libs/lib-core/src/model/narrative.rs",
    "MessageHeader": "crates/libs/lib-core/src/model/message_header.rs",
}
IGNORED_BACKEND_FIELDS = {
    "id",
    "case_id",
    "deleted",
    "drug_id",
    "reaction_id",
    "patient_id",
    "parent_id",
    "death_info_id",
    "narrative_id",
    "study_information_id",
    "drug_reaction_assessment_id",
    "message_date_format",
    "message_format_release",
    "message_format_version",
    "receiver_organization",
    "created_at",
    "updated_at",
    "created_by",
    "updated_by",
    "sequence_number",
    "source_sender_presave_id",
    "source_reporter_presave_id",
    "source_study_presave_id",
    "source_product_presave_id",
    "source_narrative_presave_id",
    "source_patient_presave_id",
    "version",
}
# Plumbing that is only plumbing on one model. Kept out of IGNORED_BACKEND_FIELDS so
# generic names like `status` stay tracked everywhere else.
IGNORED_BACKEND_FIELDS_BY_MODEL = {
    "Case": {
        "organization_id",
        "dg_prd_key",
        "status",
        "review_receivers_json",
        "workflow_routes_json",
        "workflow_status",
        "workflow_assigned_role",
        "workflow_assigned_user_id",
        "workflow_due_at",
        "workflow_description",
        "workflow_updated_at",
        "submitted_by",
        "submitted_at",
        "raw_xml",
        "dirty_c",
        "dirty_d",
        "dirty_e",
        "dirty_f",
        "dirty_g",
        "dirty_h",
    },
}


@dataclass
class ValidationResult:
    errors: list[str] = field(default_factory=list)

    @property
    def ok(self) -> bool:
        return not self.errors

    def add(self, message: str) -> None:
        self.errors.append(message)


class InventoryError(Exception):
    pass


def extract_rust_struct_fields(source: str, struct_name: str) -> list[str]:
    marker = f"pub struct {struct_name}"
    start = source.find(marker)
    if start == -1:
        raise InventoryError(f"could not find Rust struct {struct_name}")

    brace_start = source.find("{", start)
    if brace_start == -1:
        raise InventoryError(f"could not find body for Rust struct {struct_name}")

    depth = 0
    end = None
    for index in range(brace_start, len(source)):
        char = source[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                end = index
                break

    if end is None:
        raise InventoryError(f"could not parse body for Rust struct {struct_name}")

    body = source[brace_start + 1 : end]
    fields: list[str] = []
    for line in body.splitlines():
        stripped = line.strip()
        if not stripped.startswith("pub "):
            continue
        name = stripped.removeprefix("pub ").split(":", 1)[0].strip()
        if name:
            fields.append(name)
    return fields


def backend_key(model: str, field_name: str) -> str:
    return f"{model}.{field_name}"


def should_ignore_backend_field(model: str, field_name: str) -> bool:
    if field_name in IGNORED_BACKEND_FIELDS or field_name.endswith("_null_flavor"):
        return True
    return field_name in IGNORED_BACKEND_FIELDS_BY_MODEL.get(model, frozenset())


def iter_backend_model_fields(root: Path, backend_models: dict[str, str]):
    source_root = root if (root / "crates").exists() else root.parent
    for model_name, relative_path in sorted(backend_models.items()):
        source_path = source_root / relative_path
        try:
            source = source_path.read_text(encoding="utf-8")
        except FileNotFoundError as exc:
            raise InventoryError(f"{source_path}: configured backend source file does not exist") from exc

        for field_name in extract_rust_struct_fields(source, model_name):
            yield model_name, field_name


def extract_backend_inventory(root: Path, backend_models: dict[str, str]) -> set[str]:
    return {
        backend_key(model_name, field_name)
        for model_name, field_name in iter_backend_model_fields(root, backend_models)
        if not should_ignore_backend_field(model_name, field_name)
    }


def extract_backend_null_flavor_columns(root: Path, backend_models: dict[str, str]) -> set[str]:
    """Real `*_null_flavor` columns, which rows may map but are never required to.

    Most nullFlavors are in-band: the base field carries either a value or the flavor
    token, and the frontend API layer splits them apart at save time, so the base
    field's row already accounts for the column. A few fields instead have their own
    dedicated nullFlavor input and column; those rows map here so joins resolve.
    """
    return {
        backend_key(model_name, field_name)
        for model_name, field_name in iter_backend_model_fields(root, backend_models)
        if field_name.endswith("_null_flavor")
    }


def load_json(path: Path, result: ValidationResult) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        result.add(f"{path}: file does not exist")
    except json.JSONDecodeError as exc:
        result.add(f"{path}: invalid JSON at line {exc.lineno}, column {exc.colno}: {exc.msg}")
    return None


def validate_dictionary_entry(entry: Any, source: Path, result: ValidationResult) -> None:
    if not isinstance(entry, dict):
        result.add(f"{source}: each dictionary entry must be an object")
        return

    code = str(entry.get("code", "<missing code>"))
    for key in ("code", "name", "section", "kind"):
        if not entry.get(key):
            result.add(f"{source}: entry {code}: missing required field {key}")
    for key in entry:
        if key not in ALLOWED_DICTIONARY_ENTRY_FIELDS:
            result.add(f"{source}: entry {code}: unsupported field {key}")

    section = entry.get("section")
    if section is not None and section not in SECTIONS:
        result.add(f"{source}: entry {code}: invalid section {section!r}")

    kind = entry.get("kind")
    if kind is not None and kind not in DICTIONARY_KINDS:
        result.add(f"{source}: entry {code}: invalid kind {kind!r}; expected one of {sorted(DICTIONARY_KINDS)}")

    conformance = entry.get("conformance")
    if kind == "element" and not conformance:
        result.add(f"{source}: entry {code}: conformance is required for element entries")
    if conformance is not None and conformance not in DICTIONARY_CONFORMANCES:
        result.add(
            f"{source}: entry {code}: invalid conformance {conformance!r}; expected one of {sorted(DICTIONARY_CONFORMANCES)}"
        )

    severity = entry.get("fda_severity")
    if severity is not None and severity not in FDA_SEVERITIES:
        result.add(
            f"{source}: entry {code}: invalid fda_severity {severity!r};"
            f" expected one of {sorted(FDA_SEVERITIES)}"
        )

    vocabulary = entry.get("vocabulary")
    if vocabulary is not None and vocabulary not in DICTIONARY_VOCABULARIES:
        result.add(
            f"{source}: entry {code}: invalid vocabulary {vocabulary!r};"
            f" expected one of {sorted(DICTIONARY_VOCABULARIES)}"
        )

    vocabulary_variants = entry.get("vocabulary_variants")
    if vocabulary is not None and vocabulary_variants is not None:
        result.add(
            f"{source}: entry {code}: cannot combine vocabulary with vocabulary_variants"
        )
    if vocabulary_variants is not None:
        if not isinstance(vocabulary_variants, list) or not vocabulary_variants:
            result.add(
                f"{source}: entry {code}: vocabulary_variants must be a non-empty array"
            )
        else:
            seen_receivers: set[str] = set()
            for index, variant in enumerate(vocabulary_variants):
                if not isinstance(variant, dict):
                    result.add(
                        f"{source}: entry {code}: vocabulary variant {index} must be an object"
                    )
                    continue
                unsupported = set(variant) - {"receiver", "vocabulary", "vocabulary_scope"}
                for key in sorted(unsupported):
                    result.add(
                        f"{source}: entry {code}: unsupported vocabulary variant field {key}"
                    )
                receiver = variant.get("receiver")
                if receiver not in VOCABULARY_RECEIVERS:
                    result.add(
                        f"{source}: entry {code}: invalid vocabulary receiver {receiver!r};"
                        f" expected one of {sorted(VOCABULARY_RECEIVERS)}"
                    )
                elif receiver in seen_receivers:
                    result.add(
                        f"{source}: entry {code}: duplicate vocabulary receiver {receiver!r}"
                    )
                else:
                    seen_receivers.add(receiver)
                variant_vocabulary = variant.get("vocabulary")
                if variant_vocabulary not in DICTIONARY_VOCABULARIES:
                    result.add(
                        f"{source}: entry {code}: invalid vocabulary {variant_vocabulary!r};"
                        f" expected one of {sorted(DICTIONARY_VOCABULARIES)}"
                    )
                vocabulary_scope = variant.get("vocabulary_scope")
                if vocabulary_scope not in VOCABULARY_SCOPES:
                    result.add(
                        f"{source}: entry {code}: invalid vocabulary_scope {vocabulary_scope!r};"
                        f" expected one of {sorted(VOCABULARY_SCOPES)}"
                    )

    allowed_value_constraint = entry.get("allowed_value_constraint")
    if allowed_value_constraint is not None:
        if not entry.get("allowed_values"):
            result.add(
                f"{source}: entry {code}: allowed_value_constraint requires"
                " allowed_values source text"
            )
        if not isinstance(allowed_value_constraint, dict):
            result.add(
                f"{source}: entry {code}: allowed_value_constraint must be an object"
            )
        else:
            for key in allowed_value_constraint:
                if key not in ALLOWED_VALUE_CONSTRAINT_FIELDS:
                    result.add(
                        f"{source}: entry {code}: unsupported allowed_value_constraint field {key}"
                    )

            constraint_kind = allowed_value_constraint.get("kind")
            if constraint_kind not in ALLOWED_VALUE_CONSTRAINT_KINDS:
                result.add(
                    f"{source}: entry {code}: invalid allowed_value_constraint kind"
                    f" {constraint_kind!r}; expected one of"
                    f" {sorted(ALLOWED_VALUE_CONSTRAINT_KINDS)}"
                )

            values = allowed_value_constraint.get("values")
            values_valid = (
                isinstance(values, list)
                and bool(values)
                and all(isinstance(value, str) and bool(value) for value in values)
                and len(values) == len(set(values))
            )
            if values is not None and not values_valid:
                result.add(
                    f"{source}: entry {code}: allowed_value_constraint values must be"
                    " a non-empty array of unique, non-empty strings"
                )
            if constraint_kind == "code_set" and not values_valid:
                result.add(
                    f"{source}: entry {code}: code_set allowed_value_constraint"
                    " requires values"
                )

            enforcement = allowed_value_constraint.get("enforcement")
            if constraint_kind == "descriptive":
                if enforcement is not None:
                    result.add(
                        f"{source}: entry {code}: descriptive allowed_value_constraint"
                        " cannot declare enforcement"
                    )
            elif enforcement not in CONSTRAINT_ENFORCEMENTS:
                result.add(
                    f"{source}: entry {code}: non-descriptive allowed_value_constraint"
                    " requires enforcement"
                )

            numeric_shape = allowed_value_constraint.get("numeric_shape")
            if constraint_kind == "numeric":
                if numeric_shape not in NUMERIC_SHAPES:
                    result.add(
                        f"{source}: entry {code}: numeric allowed_value_constraint"
                        " requires numeric_shape"
                    )
            elif numeric_shape is not None:
                result.add(
                    f"{source}: entry {code}: numeric_shape requires numeric kind"
                )

            format_name = allowed_value_constraint.get("format_name")
            if constraint_kind == "format":
                if format_name not in FORMAT_NAMES:
                    result.add(
                        f"{source}: entry {code}: format allowed_value_constraint"
                        " requires format_name"
                    )
            elif format_name is not None:
                result.add(
                    f"{source}: entry {code}: format_name requires format kind"
                )

            vocabulary_scope = allowed_value_constraint.get("vocabulary_scope")
            identifier_profile = allowed_value_constraint.get("identifier_profile")
            if constraint_kind == "vocabulary":
                if vocabulary_scope is not None and identifier_profile is not None:
                    result.add(
                        f"{source}: entry {code}: identifier_profile cannot be"
                        " combined with vocabulary_scope"
                    )
                elif vocabulary_scope not in VOCABULARY_SCOPES and identifier_profile not in IDENTIFIER_PROFILES:
                    result.add(
                        f"{source}: entry {code}: vocabulary allowed_value_constraint"
                        " requires vocabulary_scope or identifier_profile"
                    )
            elif vocabulary_scope is not None or identifier_profile is not None:
                result.add(
                    f"{source}: entry {code}: vocabulary metadata requires vocabulary kind"
                )

    profiles = entry.get("profiles")
    if profiles is not None:
        if not isinstance(profiles, dict):
            result.add(f"{source}: entry {code}: profiles must be an object")
        else:
            for profile, value in profiles.items():
                if value not in DICTIONARY_CONFORMANCES:
                    result.add(
                        f"{source}: entry {code}: invalid profile conformance {value!r} for {profile};"
                        f" expected one of {sorted(DICTIONARY_CONFORMANCES)}"
                    )


def load_dictionaries(root: Path, result: ValidationResult) -> dict[str, dict[str, Any]]:
    """Load registry/dictionary/*.json as {authority: {code: entry}}."""
    dictionaries: dict[str, dict[str, Any]] = {}
    dictionary_dir = root / "dictionary"
    if not dictionary_dir.is_dir():
        return dictionaries

    seen_codes: dict[str, Path] = {}
    for path in sorted(dictionary_dir.glob("*.json")):
        payload = load_json(path, result)
        if payload is None:
            continue
        if not isinstance(payload, dict):
            result.add(f"{path}: dictionary file must contain a JSON object")
            continue

        authority = payload.get("authority")
        if authority not in AUTHORITIES:
            result.add(f"{path}: invalid authority {authority!r}; expected one of {sorted(AUTHORITIES)}")
            continue

        entries = payload.get("entries")
        if not isinstance(entries, list) or not entries:
            result.add(f"{path}: entries must be a non-empty list")
            continue

        for entry in entries:
            validate_dictionary_entry(entry, path, result)
            if not isinstance(entry, dict):
                continue
            code = entry.get("code")
            if not isinstance(code, str):
                continue
            if code in seen_codes:
                result.add(f"{path}: duplicate dictionary code {code}; first seen in {seen_codes[code]}")
            seen_codes[code] = path
            dictionaries.setdefault(authority, {})[code] = entry

    validate_rules_files(dictionary_dir / "rules", set(seen_codes), result)
    return dictionaries


def validate_rules_files(rules_dir: Path, known_codes: set[str], result: ValidationResult) -> None:
    if not rules_dir.is_dir():
        return
    for path in sorted(rules_dir.glob("*.json")):
        payload = load_json(path, result)
        if payload is None:
            continue
        if not isinstance(payload, dict):
            result.add(f"{path}: rules file must contain a JSON object")
            continue
        if payload.get("authority") not in AUTHORITIES:
            result.add(f"{path}: invalid authority {payload.get('authority')!r}; expected one of {sorted(AUTHORITIES)}")
        if not payload.get("source"):
            result.add(f"{path}: missing required field source")
        rules = payload.get("rules")
        if not isinstance(rules, dict) or not rules:
            result.add(f"{path}: rules must be a non-empty object")
            continue
        for code, text in rules.items():
            if code not in known_codes:
                result.add(f"{path}: rule for unknown code {code}; every rule must reference a dictionary element")
            if not isinstance(text, str) or not text.strip():
                result.add(f"{path}: rule for {code} must be a non-empty string")


def validate_mapping(row_id: str, name: str, value: Any, result: ValidationResult) -> None:
    if not isinstance(value, dict):
        result.add(f"{row_id}: {name} must be an object")
        return

    status = value.get("status")
    if status not in MAPPING_STATUSES:
        result.add(f"{row_id}: {name}.status must be one of {sorted(MAPPING_STATUSES)}")
        return

    evidence = value.get("evidence", "")
    if status == "mapped" and not evidence:
        result.add(f"{row_id}: {name}.evidence is required when status is mapped")
    if status == "mapped":
        required_by_side = {
            "backend": ("model", "field"),
            "frontend": ("section", "field"),
        }
        for key in required_by_side.get(name, ()):
            if not value.get(key):
                result.add(f"{row_id}: {name}.{key} is required when status is mapped")


def validate_row(row: Any, source: Path, result: ValidationResult) -> None:
    if not isinstance(row, dict):
        result.add(f"{source}: each registry row must be an object")
        return

    row_id = str(row.get("id", "<missing id>"))
    required = [
        "id",
        "e2br3_code",
        "label",
        "section",
        "authority",
        "status",
        "backend",
        "frontend",
    ]
    for key in required:
        if key not in row:
            result.add(f"{row_id}: missing required field {key}")
    for key in row:
        if key not in ALLOWED_ROW_FIELDS:
            result.add(f"{row_id}: unsupported field {key}")

    authority = row.get("authority")
    if authority not in AUTHORITIES:
        result.add(f"{row_id}: invalid authority {authority!r}; expected one of {sorted(AUTHORITIES)}")

    section = row.get("section")
    if section not in SECTIONS:
        result.add(f"{row_id}: invalid section {section!r}; expected one of {sorted(SECTIONS)}")

    status = row.get("status")
    if status not in ROW_STATUSES:
        result.add(f"{row_id}: invalid status {status!r}; expected one of {sorted(ROW_STATUSES)}")

    for name in ("backend", "frontend"):
        validate_mapping(row_id, name, row.get(name), result)

    if status == "complete":
        for name in ("backend", "frontend"):
            value = row.get(name)
            if isinstance(value, dict) and value.get("status") != "mapped":
                result.add(f"{row_id}: complete rows require {name}.status to be mapped")

    backend_status = row.get("backend", {}).get("status") if isinstance(row.get("backend"), dict) else None
    frontend_status = row.get("frontend", {}).get("status") if isinstance(row.get("frontend"), dict) else None
    if status == "backend_missing" and backend_status != "missing":
        result.add(f"{row_id}: backend_missing rows require backend.status to be missing")
    if status == "frontend_missing" and frontend_status != "missing":
        result.add(f"{row_id}: frontend_missing rows require frontend.status to be missing")
    if status == "conflict" and "conflict" not in {backend_status, frontend_status}:
        result.add(f"{row_id}: conflict rows require backend.status or frontend.status to be conflict")

    if "local_only" in row and not isinstance(row.get("local_only"), bool):
        result.add(f"{row_id}: local_only must be a boolean")

    e2br3_code = row.get("e2br3_code")
    if authority == "MFDS" and isinstance(e2br3_code, str) and ".KR." not in e2br3_code:
        result.add(f"{row_id}: MFDS rows must use a KR regional E2BR3 code")
    if authority == "FDA" and isinstance(e2br3_code, str) and e2br3_code.startswith("FDA.") is False:
        result.add(f"{row_id}: FDA rows must use an FDA regional E2BR3 code")
    if authority == "ICH" and isinstance(e2br3_code, str) and (e2br3_code.startswith("FDA.") or ".KR." in e2br3_code):
        result.add(f"{row_id}: ICH rows must not use FDA or KR regional E2BR3 codes")


def validate_registry(
    root: Path = ROOT,
    backend_models: dict[str, str] | None = None,
    validate_backend_inventory: bool = True,
    validate_frontend_inventory: bool = False,
    validate_dictionary_membership: bool = False,
    frontend_source_globs: list[str] | None = None,
    validate_presave_registry_rows: bool = False,
    validate_presave_inventory: bool = False,
) -> ValidationResult:
    result = ValidationResult()
    if backend_models is None:
        backend_models = BACKEND_MODELS

    dictionaries = load_dictionaries(root, result)

    index_path = root / "index.json"
    index = load_json(index_path, result)
    if not isinstance(index, dict):
        return result

    sections = index.get("sections")
    if not isinstance(sections, list) or not sections:
        result.add(f"{index_path}: sections must be a non-empty list")
        return result

    seen_ids: dict[str, Path] = {}
    seen_codes: dict[str, Path] = {}
    seen_backend: dict[str, str] = {}
    seen_frontend: dict[str, str] = {}
    case_rows_by_code: dict[str, dict[str, Any]] = {}
    row_authorities: list[tuple[str, str, str, bool]] = []
    for section_file in sections:
        if not isinstance(section_file, str):
            result.add(f"{index_path}: section entries must be strings")
            continue
        source = root / section_file
        rows = load_json(source, result)
        if rows is None:
            continue
        if not isinstance(rows, list):
            result.add(f"{source}: section file must contain a JSON array")
            continue

        for row in rows:
            validate_row(row, source, result)
            if not isinstance(row, dict):
                continue
            row_id = row.get("id")
            if isinstance(row_id, str):
                if row_id in seen_ids:
                    result.add(f"{row_id}: duplicate id in {source}; first seen in {seen_ids[row_id]}")
                seen_ids[row_id] = source
            code = row.get("e2br3_code")
            if isinstance(code, str):
                if code in seen_codes:
                    result.add(f"{row_id}: duplicate e2br3_code {code} in {source}; first seen in {seen_codes[code]}")
                seen_codes[code] = source
                case_rows_by_code.setdefault(code, row)
                authority = row.get("authority")
                if isinstance(row_id, str) and isinstance(authority, str):
                    row_authorities.append((row_id, code, authority, row.get("local_only") is True))

            backend = row.get("backend")
            if isinstance(backend, dict) and backend.get("status") == "mapped":
                key = f"{backend.get('model')}.{backend.get('field')}"
                if key in seen_backend:
                    result.add(f"{row_id}: duplicate backend mapping {key}; first seen in {seen_backend[key]}")
                seen_backend[key] = row_id

            frontend = row.get("frontend")
            if isinstance(frontend, dict) and frontend.get("status") == "mapped":
                key = f"{frontend.get('section')}.{frontend.get('field')}"
                if key in seen_frontend:
                    result.add(f"{row_id}: duplicate frontend mapping {key}; first seen in {seen_frontend[key]}")
                seen_frontend[key] = row_id

    if validate_dictionary_membership:
        if not dictionaries:
            result.add("dictionary membership validation requested but no dictionary files were loaded")
        for row_id, code, authority, local_only in row_authorities:
            authority_codes = dictionaries.get(authority)
            if authority_codes is None:
                continue
            if local_only:
                if code in authority_codes:
                    result.add(
                        f"{row_id}: local_only row uses e2br3_code {code} which is defined in the {authority} dictionary"
                    )
                continue
            if code not in authority_codes:
                result.add(f"{row_id}: e2br3_code {code} is not defined in the {authority} dictionary")
        for authority, entries in sorted(dictionaries.items()):
            for code, entry in sorted(entries.items()):
                if (
                    entry.get("kind") == "element"
                    and entry.get("conformance") == "mandatory"
                    and code not in seen_codes
                ):
                    result.add(f"missing registry row for mandatory {authority} element {code}")

    if validate_backend_inventory:
        try:
            source_backend = extract_backend_inventory(root, backend_models)
            support_columns = extract_backend_null_flavor_columns(root, backend_models)
        except InventoryError as exc:
            result.add(str(exc))
            return result

        scoped_models = set(backend_models)
        registry_backend = {
            key for key in seen_backend if key.split(".", 1)[0] in scoped_models
        }
        for key in sorted(source_backend - registry_backend):
            result.add(f"missing backend mapping: {key}")
        for key in sorted(registry_backend - source_backend - support_columns):
            result.add(f"unknown backend mapping: {key}")

    if validate_frontend_inventory:
        try:
            source_frontend = {
                field.key
                for field in (
                    extract_frontend_fields.extract_frontend_fields(
                        root=root,
                        source_globs=frontend_source_globs,
                    )
                    if frontend_source_globs is not None
                    else extract_frontend_fields.extract_frontend_fields_ast(root=root)
                )
            }
        except extract_frontend_fields.FrontendInventoryError as exc:
            result.add(str(exc))
            return result

        source_roots = {key.split(".", 1)[0] for key in source_frontend}
        registry_frontend = {
            key for key in seen_frontend if key.split(".", 1)[0] in source_roots
        }
        for key in sorted(source_frontend - registry_frontend):
            result.add(f"missing frontend mapping: {key}")
        for key in sorted(registry_frontend - source_frontend):
            result.add(f"unknown frontend mapping: {key}")

    if validate_presave_registry_rows or validate_presave_inventory:
        import extract_presave_fields
        import presave_registry

        presaves = presave_registry.load_presave_registry(root, result)
        expected_transfers: set[tuple[str, str]] = set()
        for row in presaves.rows:
            if row.get("status") == "not_applicable" and row.get("local_only") is True:
                continue
            code = row["e2br3_code"]
            case_row = case_rows_by_code.get(code)
            if case_row is None:
                result.add(f"missing case registry join: {code}")
                continue
            backend = row.get("backend", {})
            case_backend = case_row.get("backend", {})
            if backend.get("status") == "mapped" and case_backend.get("status") == "mapped":
                expected_transfers.add(
                    (
                        f"{backend['model']}.{backend['field']}",
                        f"{case_backend['model']}.{case_backend['field']}",
                    )
                )

        if validate_presave_inventory:
            try:
                source_frontend = extract_presave_fields.extract_reporter_frontend(root)
                source_backend = extract_presave_fields.extract_presave_backend(
                    root, extract_presave_fields.REPORTER_BACKEND_MODELS
                )
                source_transfers = extract_presave_fields.extract_reporter_transfers(root)
            except (InventoryError, extract_frontend_fields.FrontendInventoryError) as exc:
                result.add(str(exc))
                return result

            registry_frontend = set(presaves.frontend_keys.values())
            registry_backend = set(presaves.backend_keys.values())
            for key in sorted(source_frontend - registry_frontend):
                result.add(f"missing presave frontend mapping: {key}")
            for key in sorted(registry_frontend - source_frontend):
                result.add(f"unknown presave frontend mapping: {key}")
            for key in sorted(source_backend - registry_backend):
                result.add(f"missing presave backend mapping: {key}")
            for key in sorted(registry_backend - source_backend):
                result.add(f"unknown presave backend mapping: {key}")
            for pair in sorted(expected_transfers - source_transfers):
                result.add(f"missing presave-to-case assignment: {pair[0]} -> {pair[1]}")
            for source, actual in sorted(source_transfers):
                expected = {target for candidate, target in expected_transfers if candidate == source}
                if expected and actual not in expected:
                    result.add(
                        f"wrong presave-to-case target: {source} -> {actual}; "
                        f"expected {sorted(expected)[0]}"
                    )

    return result


def main() -> int:
    strict_backend_inventory = "--strict-backend-inventory" in sys.argv[1:]
    strict_frontend_inventory = "--strict-frontend-inventory" in sys.argv[1:]
    strict_dictionary = "--strict-dictionary" in sys.argv[1:]
    strict_presave_registry = "--strict-presave-registry" in sys.argv[1:]
    strict_presave_inventory = "--strict-presave-inventory" in sys.argv[1:]
    result = validate_registry(
        validate_backend_inventory=strict_backend_inventory,
        validate_frontend_inventory=strict_frontend_inventory,
        validate_dictionary_membership=strict_dictionary,
        validate_presave_registry_rows=(
            strict_presave_registry or strict_presave_inventory
        ),
        validate_presave_inventory=strict_presave_inventory,
    )
    if result.ok:
        print("registry validation passed")
        return 0

    print("registry validation failed", file=sys.stderr)
    for error in result.errors:
        print(f"- {error}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
