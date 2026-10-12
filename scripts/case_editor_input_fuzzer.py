#!/usr/bin/env python3
"""Seeded case-editor input-contract/save/audit fuzzer.

One synthetic case per run. Each owner row is created once, then one field is
mutated at a time so audit deltas stay attributable to a single input.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import random
import re
import subprocess
import sys
import time
import unicodedata
import uuid
import urllib.parse
from dataclasses import asdict, dataclass
from decimal import Decimal, InvalidOperation
from pathlib import Path
from typing import Any

from rbac_rls_blackbox import ApiClient, commit_sha, guard_target, response_summary
from audit_trail_consistency_fuzzer import (
    AUDIT_FIELD_ALIASES,
    audit_field_key,
    audit_key_matches,
    audit_log_complete,
)


DEFAULT_CONTRACT = Path(__file__).resolve().parents[2] / "frontend-origin-dev/lib/case-editor/generated/editorContracts.json"
DEFAULT_NULL_FLAVOR_PAIRS = Path(__file__).resolve().parents[2] / "frontend-origin-dev/lib/case-save/pages/null-flavor-pairs.ts"
ROW_PAGES = {"AE": "reaction", "DG": "drug", "DH": "pastDrugHistory", "LB": "testResult", "LR": "literatureReference"}
DIRECT_LIFECYCLE_OWNERS = {
    "CI": "linkedReports", "RP": "primarySources", "SI": "studyRegistrationNumbers",
    "DM": "medicalHistoryEpisodes", "NR": "caseSummaryInformation",
}
DIRECT_LIFECYCLE_OWNER_OVERRIDES = {
    "CI": {"documentsHeldBySender", "otherCaseIdentifiers", "sourceDocuments"},
    "SI": {"fdaCrossReportedIndNumbers"},
    "DM": {"reportedCauses", "autopsyCauses", "parentMedicalHistory", "parentPastDrugs"},
    "NR": {"senderDiagnoses"},
}
DG_NESTED_LIFECYCLE_ENDPOINTS = {
    "activeSubstances": "active-substances",
    "indications": "indications",
    "drugReactionAssessments": "reaction-assessments",
    "fdaDevices": "devices",
    "dosageInformation": "dosages",
    "relatednessAssessments": "relatedness",
    "fdaDeviceFollowUpTypes": "codes",
    "fdaDeviceProblemCodes": "codes",
    "fdaDeviceRemedialActions": "codes",
}
DG_NESTED_CREATE_FIELDS = {
    "activeSubstances": {"substance_name", "substance_termid", "substance_termid_version",
                         "substance_termid_code_system", "mfds_version", "mfds_id",
                         "strength_value", "strength_unit"},
    "indications": {"indication_text", "indication_text_null_flavor",
                    "indication_meddra_version", "indication_meddra_code"},
    "drugReactionAssessments": {"administration_start_interval_value",
                                "administration_start_interval_unit",
                                "last_dose_interval_value", "last_dose_interval_unit",
                                "recurrence_action", "reaction_recurred",
                                "dechallenge_result", "expectedness"},
    "fdaDevices": {"malfunction", "device_brand_name", "device_brand_name_null_flavor",
                   "common_device_name", "common_device_name_null_flavor",
                   "device_product_code", "manufacturer_name", "manufacturer_address",
                   "manufacturer_city", "manufacturer_state", "manufacturer_country",
                   "device_usage", "device_lot_number", "operator_of_device"},
    "dosageInformation": {"dose_value", "dose_unit", "number_of_units", "frequency_unit",
                          "first_administration_date", "first_administration_date_raw",
                          "last_administration_date", "last_administration_date_raw",
                          "duration_value", "duration_unit", "continuing", "batch_lot_number",
                          "dosage_text", "dose_form", "dose_form_null_flavor", "dose_form_termid",
                          "dose_form_termid_version", "route_of_administration",
                          "route_of_administration_null_flavor", "route_termid", "route_termid_version",
                          "route_termid_code_system", "parent_route", "parent_route_null_flavor",
                          "parent_route_termid", "parent_route_termid_version",
                          "parent_route_termid_code_system", "first_administration_date_null_flavor",
                          "last_administration_date_null_flavor"},
    "relatednessAssessments": {"source_of_assessment", "method_of_assessment",
                               "method_of_assessment_kr1", "result_of_assessment",
                               "result_of_assessment_kr1", "result_of_assessment_kr1_null_flavor",
                               "result_of_assessment_kr2"},
    "fdaDeviceFollowUpTypes": {"element", "value_code"},
    "fdaDeviceProblemCodes": {"element", "value_code"},
    "fdaDeviceRemedialActions": {"element", "value_code"},
}
DG_NESTED_REQUIRED_EDIT_SEEDS = {
    # FDA device manufacturer is an app-local free-text field and is absent from
    # the portable generated constraint rows. It is still part of the real UI/model.
    "fdaDevices": ("manufacturer_name", "Fuzz device manufacturer"),
}
LIFECYCLE_TEXT_PATHS = {
    "reaction": "primarySourceReaction",
    "drug": "medicinalProduct",
    "testResult": "testName",
    "literatureReference": "referenceText",
    "pastDrugHistory": "drugName",
    "linkedReports": "[].linkedReportNumber",
    "primarySources": "[].reporterFamilyName",
    "studyRegistrationNumbers": "[].registrationNumber",
    "medicalHistoryEpisodes": "[].comments",
    "caseSummaryInformation": "[].summaryText",
    "documentsHeldBySender": "[].documentDescription",
    "otherCaseIdentifiers": "[].caseIdentifier",
    "sourceDocuments": "[].sourceDocumentName",
    "fdaCrossReportedIndNumbers": "[].indNumber",
    "reportedCauses": "[].causeText",
    "autopsyCauses": "[].causeText",
    "parentMedicalHistory": "[].comments",
    "parentPastDrugs": "[].drugName",
}
LIFECYCLE_EXTERNAL_CHANGE_GENERATORS = {"senderDiagnoses"}
HARD_DELETE_AUDIT_OWNERS = {"sourceDocuments"}

# Direct-editor names whose audit columns intentionally differ from their DTO names.
# Keep these aliases local to the Case Editor fuzzer: the shared audit fuzzer does
# not own these direct-page contracts.
CASE_EDITOR_AUDIT_FIELD_ALIASES = {
    "documentDescription": "title",
    "causeText": "comments",
}
AUDIT_FIELD_ALIASES.update(CASE_EDITOR_AUDIT_FIELD_ALIASES)

LIFECYCLE_REQUIRED_COLLECTIONS = {
    "reaction", "drug", "pastDrugHistory", "testResult", "literatureReference",
    "linkedReports", "primarySources", "studyRegistrationNumbers",
    "medicalHistoryEpisodes", "caseSummaryInformation", "documentsHeldBySender",
    "otherCaseIdentifiers", "sourceDocuments", "fdaCrossReportedIndNumbers",
    "reportedCauses", "autopsyCauses", "parentMedicalHistory", "parentPastDrugs",
    "senderDiagnoses", "activeSubstances", "indications", "drugReactionAssessments",
    "fdaDevices", "dosageInformation", "relatednessAssessments",
    "fdaDeviceFollowUpTypes", "fdaDeviceProblemCodes", "fdaDeviceRemedialActions",
}


def lifecycle_preflight_gaps() -> dict[str, list[str]]:
    """Return structural lifecycle gaps before any production request is sent."""
    gaps: dict[str, list[str]] = {}
    direct = set(ROW_PAGES.values()) | set(DIRECT_LIFECYCLE_OWNERS.values())
    direct |= set().union(*DIRECT_LIFECYCLE_OWNER_OVERRIDES.values())
    configured = direct | set(DG_NESTED_LIFECYCLE_ENDPOINTS)
    for owner in sorted(LIFECYCLE_REQUIRED_COLLECTIONS):
        missing: list[str] = []
        if owner not in configured:
            missing.append("selector")
        if (owner not in LIFECYCLE_TEXT_PATHS
                and owner not in DG_NESTED_LIFECYCLE_ENDPOINTS
                and owner not in LIFECYCLE_EXTERNAL_CHANGE_GENERATORS):
            missing.append("business_change")
        if owner in DG_NESTED_LIFECYCLE_ENDPOINTS:
            if not DG_NESTED_CREATE_FIELDS.get(owner):
                missing.append("create_readback")
            audit_roots = {
                "activeSubstances": "activeSubstances[]",
                "indications": "indications[]",
                "drugReactionAssessments": "drugReactionAssessments[]",
                "fdaDevices": "fdaDevices[]",
                "dosageInformation": "dosageInformation[]",
            }
            if audit_roots.get(owner, owner) not in NESTED_AUDIT_TABLES and owner not in {
                "relatednessAssessments", "fdaDeviceFollowUpTypes",
                "fdaDeviceProblemCodes", "fdaDeviceRemedialActions",
            }:
                missing.append("audit")
        elif owner not in AUDIT_TABLES:
            missing.append("audit")
        if missing:
            gaps[owner] = missing
    return gaps
NULL_FLAVOR_TOKENS = (
    "NI", "INV", "DER", "OTH", "NINF", "PINF", "UNC", "MSK",
    "NA", "UNK", "ASKU", "NAV", "QS", "TRC", "NP",
)
AUDIT_TABLES = {
    "case": "cases",
    "safetyReportIdentification": "safety_report_identification",
    "patientInformation": "patient_information",
    "patientIdentifiers": "patient_identifiers",
    "medicalHistoryEpisodes": "medical_history_episodes",
    "reportedCauses": "reported_causes_of_death",
    "autopsyCauses": "autopsy_causes_of_death",
    "deathInfo": "patient_death_information",
    "studyInformation": "study_information",
    "studyRegistrationNumbers": "study_registration_numbers",
    "fdaCrossReportedIndNumbers": "study_fda_cross_reported_inds",
    "parentInfo": "parent_information",
    "parentMedicalHistory": "parent_medical_history",
    "parentPastDrugs": "parent_past_drug_history",
    "dosageInformation": "dosage_information",
    "drugInformation": "drug_information",
    "reactions": "reactions",
    "reaction": "reactions",
    "drug": "drug_information",
    "pastDrugHistory": "past_drug_history",
    "testResult": "test_results",
    "literatureReference": "literature_references",
    "primarySources": "primary_sources",
    "senderInformation": "sender_information",
    "receiverInformation": "receiver_information",
    "messageHeaders": "message_headers",
    "sourceDocuments": "source_documents",
    "otherCaseIdentifiers": "other_case_identifiers",
    "linkedReportNumbers": "linked_report_numbers",
    "documentsHeldBySender": "documents_held_by_sender",
    "linkedReports": "linked_report_numbers",
    "caseSummaryInformation": "case_summary_information",
    "narrative": "narrative_information",
    "senderDiagnoses": "sender_diagnoses",
}
NESTED_AUDIT_TABLES = {
    "fdaDevices[].followUpTypes": ("fda_device_codes", "id"),
    "fdaDevices[].deviceProblemCodes": ("fda_device_codes", "id"),
    "fdaDevices[].remedialActions": ("fda_device_codes", "id"),
    "fdaCrossReportedIndNumbers[]": ("study_fda_cross_reported_inds", "id"),
    "activeSubstances[]": ("drug_active_substances", "id"),
    "dosageInformation[]": ("dosage_information", "id"),
    "indications[]": ("drug_indications", "id"),
    "drugReactionAssessments[].sourceOfAssessment": ("relatedness_assessments", "id"),
    "drugReactionAssessments[].methodOfAssessment": ("relatedness_assessments", "id"),
    "drugReactionAssessments[].resultOfAssessment": ("relatedness_assessments", "id"),
    "drugReactionAssessments[].source_of_assessment": ("relatedness_assessments", "id"),
    "drugReactionAssessments[].method_of_assessment": ("relatedness_assessments", "id"),
    "drugReactionAssessments[].result_of_assessment": ("relatedness_assessments", "id"),
    "drugReactionAssessments[]": ("drug_reaction_assessments", "drugReactionAssessmentId"),
    "fdaDevices[]": ("fda_device_information", "id"),
    "fdaDevices[].deviceBrandName": ("fda_device_information", "id"),
    "fdaDevices[].manufacturerName": ("fda_device_information", "id"),
    "dosageInformation[].dosageText": ("dosage_information", "id"),
    "drugReactionAssessments[].sourceOfAssessment": ("relatedness_assessments", "id"),
}
BASE_CANDIDATES = 8
STRING_CANDIDATES = 14
LENGTH_CANDIDATES = 17
IDENTIFIER_CANDIDATES = 18
NULLFLAVOR_CANDIDATES = 14
MAX_LENGTH_RE = re.compile(
    r'max_length\(\s*&mut issues,\s*"([^"]+)",\s*input\.value,\s*(\d+)',
    re.DOTALL,
)
IDENTIFIER_RE = re.compile(
    r'identifier\(\s*&mut issues,\s*"([^"]+)",\s*input\.value',
    re.DOTALL,
)
BOOLEAN_RE = re.compile(
    r'boolean\(\s*&mut issues,\s*"([^"]+)",\s*input\.value',
    re.DOTALL,
)
MAX_LENGTH_ONLY_RE = re.compile(
    r'pub fn \w+\(input: crate::FieldInput<\'_>\) -> Vec<crate::InputIssue> \{'
    r'\s*let mut issues = Vec::new\(\);'
    r'\s*crate::helpers::max_length\('
    r'\s*&mut issues,\s*"([^"]+)",\s*input\.value,\s*\d+,\s*\);'
    r'\s*issues\s*\}',
    re.DOTALL,
)
CANDIDATE_KINDS = (
    "verified_invalid", "null", "primitive_or_blank", "boundary", "blank",
    "unicode_basic", "control_chars", "type_mismatch", "unicode_nfd",
    "unicode_rtl", "unicode_zero_width", "unicode_emoji_sequence",
    "unicode_lone_surrogate", "encoding_edges", "length_max",
    "length_max_plus_one", "length_oversize", "identifier_controls",
)
NULLFLAVOR_CANDIDATE_KINDS = (
    "nullflavor_verified_invalid", "nullflavor_null", "nullflavor_allowed",
    "nullflavor_alternate", "nullflavor_blank", "nullflavor_unknown",
    "nullflavor_number", "nullflavor_object", "nullflavor_disallowed_token",
    "nullflavor_lowercase", "nullflavor_overlong", "nullflavor_array",
    "nullflavor_boolean", "nullflavor_with_value",
)
DEVICE_NULL_FLAVOR_PATHS = {
    "fdaDevices[].deviceBrandNameNullFlavor",
    "fdaDevices[].commonDeviceNameNullFlavor",
}
CI_NON_NULLABLE_TEXT_CODES = {"C.1.10.r", "C.1.9.1.r.1", "C.1.9.1.r.2"}
OWNER_METADATA_KEYS = {"updatedAt", "updatedBy", "updated_at", "updated_by"}
OWNER_TARGET_SENTINEL = "__CASE_EDITOR_FUZZ_TARGET__"


@dataclass
class Event:
    kind: str
    field: str | None
    page: str | None
    owner: str | None
    mutation: str | None
    classification: str
    http_status: int | None
    response: dict[str, Any]


def normalized_classification(actual: Any, before: Any, audit_complete: bool) -> str:
    if actual == [None]:
        if before == [None]:
            return "NOOP_ACCEPTED"
        return "SAVE_NORMALIZED" if audit_complete else "AUDIT_MISMATCH"
    if values_equal(actual, before):
        return "NOOP_ACCEPTED" if is_empty_value(before) else "CLEAR_NOT_APPLIED"
    if actual is not None and not is_blank_candidate(actual):
        return "NORMALIZATION_UNVERIFIED"
    return "SAVE_NORMALIZED" if audit_complete else "AUDIT_MISMATCH"


def mismatched_save_classification(
    candidate: Any, before: Any, actual: Any, audit_complete: bool
) -> str:
    if candidate is None:
        return "CLEAR_NOT_APPLIED"
    if is_blank_candidate(candidate):
        return normalized_classification(actual, before, audit_complete)
    return "SAVE_READBACK_MISMATCH"


def ci_non_nullable_null_classification(
    code: str | None, before: Any, actual: Any, audit_complete: bool
) -> str | None:
    if code not in CI_NON_NULLABLE_TEXT_CODES or actual != "":
        return None
    if before == actual:
        return "NOOP_ACCEPTED"
    return "SAVE_NORMALIZED" if audit_complete else "AUDIT_MISMATCH"


def collection_blank_classification(
    code: str | None, candidate: Any, before: Any, actual: Any, audit_complete: bool
) -> str | None:
    if (code != "FDA.D.11.r.1"
            or not (candidate is None or is_blank_candidate(candidate)) or actual != []):
        return None
    if before == []:
        return "NOOP_ACCEPTED"
    return "SAVE_NORMALIZED" if audit_complete else "AUDIT_MISMATCH"


def run_verdict(events: list[Event], planned: int, interrupted: str | None,
                unsupported: list[str], setup_only: bool = False) -> dict[str, Any]:
    mutations = [event for event in events if event.kind == "mutation"]
    accepted = {
        "PASS", "SAVE_ACCEPTED", "NOOP_ACCEPTED", "SAVE_NORMALIZED",
        "CONSTRAINT_REJECTED", "BACKEND_REJECTED", "CONFLICT_REJECTED",
        "NULLFLAVOR_REJECTED", "CLEAR_REJECTED", "COMPLIANCE_GUARD_REJECTED",
        "AUDIT_CHAIN_PREEXISTING", "GATE_PASS",
    }
    unresolved = sorted({event.classification for event in events if event.classification not in accepted})
    complete = bool(planned) and len(mutations) == planned and not interrupted and not unsupported and not unresolved
    baseline_only = setup_only and planned == 0 and not mutations and not interrupted and not unresolved and any(
        event.kind == "create" and event.classification == "PASS" for event in events
    )
    return {
        "verdict": "SELECTED_API_CHECKS_PASSED" if complete else "BASELINE_ONLY" if baseline_only else "INCOMPLETE_OR_FAILED",
        "planned_mutations": planned,
        "executed_mutations": len(mutations),
        "unsupported_null_flavor_pairs": unsupported,
        "unresolved_classifications": unresolved,
        "official_compliance_verified": False,
        "ui_verified": False,
    }


def persisted_field_report(
    inventory: dict[str, Any], pages: list[str], requested_fields: set[str],
    blocked: list[str], events: list[Event], expected_counts: dict[tuple[str, str], int],
) -> list[dict[str, Any]]:
    accepted = {
        "PASS", "SAVE_ACCEPTED", "NOOP_ACCEPTED", "SAVE_NORMALIZED",
        "CONSTRAINT_REJECTED", "BACKEND_REJECTED", "CONFLICT_REJECTED",
        "NULLFLAVOR_REJECTED", "CLEAR_REJECTED", "COMPLIANCE_GUARD_REJECTED",
    }
    blocked_set = set(blocked)
    result: list[dict[str, Any]] = []
    for record in inventory.get("fields", []):
        if (record.get("page") not in pages
                or (requested_fields and record.get("code") not in requested_fields)):
            continue
        identity = record.get("identity")
        matching = [event for event in events
                    if event.page == record.get("page") and event.field == record.get("code")
                    and event.kind in {"mutation", "noop_resave"}]
        mutation_events = [event for event in matching if event.kind == "mutation"]
        expected = expected_counts.get((record.get("page"), record.get("code")), 0)
        failures = [event for event in matching if event.classification not in accepted]
        if identity in blocked_set:
            status = "BLOCKED_SOURCE_PREREQUISITE"
        elif failures:
            status = "FAILED"
        elif len(mutation_events) != expected or expected == 0:
            status = "MISSING"
        else:
            status = "PASSED"
        result.append({
            "identity": identity, "status": status,
            "expected_mutations": expected, "executed_mutations": len(mutation_events),
            "phases": [{"kind": event.kind, "mutation": event.mutation,
                        "classification": event.classification,
                        "http_status": event.http_status} for event in matching],
        })
    return result


def zero_mutation_admission_blocked(
    planned_mutations: int, values_per_field: int, lifecycle_only: bool
) -> bool:
    return not planned_mutations and values_per_field != 0 and not lifecycle_only


def unwrap(value: Any) -> Any:
    if isinstance(value, dict) and set(value) == {"data"}:
        return value["data"]
    return value.get("data") if isinstance(value, dict) and "data" in value else value


def object_id(value: Any) -> str | None:
    if isinstance(value, dict):
        for key in ("id", "caseId", "rowId"):
            candidate = value.get(key)
            if isinstance(candidate, str) and re.fullmatch(r"[0-9a-fA-F-]{36}", candidate):
                return candidate
        for child in value.values():
            found = object_id(child)
            if found:
                return found
    if isinstance(value, list):
        for child in value:
            found = object_id(child)
            if found:
                return found
    return None


def object_identity(value: Any) -> dict[str, str]:
    if isinstance(value, list):
        value = next((item for item in value if isinstance(item, dict)), None)
    if not isinstance(value, dict):
        return {}
    return {
        key: candidate
        for key in ("id", "drugReactionAssessmentId", "reactionId")
        if isinstance(candidate := value.get(key), str)
        and re.fullmatch(r"[0-9a-fA-F-]{36}", candidate)
    }


def row_with_identity(value: Any, identity: dict[str, str]) -> dict[str, Any] | None:
    if not isinstance(value, list) or not identity:
        return None
    matches = [
        row for row in value
        if isinstance(row, dict)
        and all(object_identity(row).get(key) == expected for key, expected in identity.items())
    ]
    return matches[0] if len(matches) == 1 else None


def replacement_identity(
    response: Any,
    owner: str,
    root: str,
    required_keys: set[str],
) -> dict[str, str]:
    owner_value = response.get(owner) if isinstance(response, dict) else None
    rows = get_path(owner_value, root)
    if not isinstance(rows, list) or len(rows) != 1 or not isinstance(rows[0], dict):
        return {}
    identity = object_identity(rows[0])
    return identity if required_keys and required_keys <= identity.keys() else {}


def set_path(target: dict[str, Any], path: str, value: Any) -> None:
    """Set camelCase payload path; [] means one repeat row."""
    parts = [part for part in path.split(".") if part]
    if parts and parts[0] == "[]":
        parts.pop(0)
    current: Any = target
    for index, part in enumerate(parts):
        repeated = part.endswith("[]")
        key = part[:-2] if repeated else part
        last = index == len(parts) - 1
        if repeated:
            if last:
                current[key] = value if isinstance(value, list) else [value]
                return
            values = current.setdefault(key, [{}])
            if not isinstance(values, list):
                values = [{}]
                current[key] = values
            if not values:
                values.append({})
            current = values[0]
        elif last:
            current[key] = value
        else:
            child = current.get(key)
            if not isinstance(child, dict):
                child = {}
                current[key] = child
            current = child


def get_path(value: Any, path: str) -> Any:
    parts = [part for part in path.split(".") if part and part != "[]"]
    def walk(current: Any, remaining: list[str]) -> Any:
        if isinstance(current, list):
            return [walk(item, remaining) for item in current]
        if not remaining:
            return current
        if not isinstance(current, dict):
            return None
        part, *rest = remaining
        if part in current:
            return walk(current[part], rest)
        normalized = snake(part)
        return walk(next((candidate for key, candidate in current.items() if snake(str(key)) == normalized), None), rest)

    return walk(value, parts)


def leaf_path(payload_path: str) -> str:
    return payload_path.removeprefix("[]").lstrip(".")


def nested_root(payload_path: str) -> str | None:
    parts: list[str] = []
    for part in leaf_path(payload_path).split("."):
        parts.append(part)
        if part.endswith("[]"):
            root = ".".join(parts)
            return root if any(path.startswith(root) for path in NESTED_AUDIT_TABLES) else None
    return None


def projection_leaf(field: dict[str, Any], owner: str) -> str:
    """Use backend projection names for readback/audit, payload names for PATCH."""
    path = str(field.get("projectionPath") or field.get("payloadPath") or "")
    for prefix in (f"{owner}[].", f"{owner}."):
        if path.startswith(prefix):
            return path[len(prefix) :]
    return leaf_path(path)


def candidate_rng(seed: int, field: dict[str, Any], ordinal: int, sample: int) -> random.Random:
    identity = "|".join(
        str(field.get(key, "")) for key in ("authority", "code", "frontendPath", "payloadPath")
    )
    digest = hashlib.sha256(f"{seed}|{identity}|{ordinal}|{sample}".encode()).digest()
    return random.Random(int.from_bytes(digest[:8], "big"))


def candidate_fingerprint(
    field: dict[str, Any],
    ordinal: int,
    sample: int,
    candidate: Any,
) -> str:
    identity = [field.get(key) for key in ("authority", "code", "frontendPath", "payloadPath")]
    raw = json.dumps([identity, ordinal, sample, candidate], sort_keys=True, ensure_ascii=True)
    return hashlib.sha256(raw.encode()).hexdigest()[:16]


def candidate_sample_count(field: dict[str, Any], ordinal: int, requested: int) -> int:
    if field.get("_auditAll"):
        return 1
    if requested <= 1:
        return 1
    if is_nullflavor_field(field):
        return requested if ordinal in {5, 7, 8, 10} else 1
    if field.get("_booleanRule"):
        return requested if ordinal in {2, 7} else 1
    if isinstance(field.get("roundTripValue"), bool):
        return requested if ordinal in {2, 7} else 1
    return 1 if ordinal in {0, 1, 4, 12} else requested


def field_value(field: dict[str, Any], rng: random.Random, ordinal: int, sample: int = 0) -> Any:
    """Grammar-guided candidates, not arbitrary bytes."""
    baseline = field.get("roundTripValue")
    constraint = field.get("constraint", {})
    invalid = constraint.get("invalidValue")
    path = field.get("payloadPath", "")
    code = field.get("code", "")
    if field.get("_auditAll"):
        if is_nullflavor_field(field):
            allowed = tuple(field.get("_allowedNullFlavors", ()))
            if ordinal == 0:
                return copy.deepcopy(baseline)
            if ordinal == 1:
                alternatives = [value for value in allowed if value != baseline]
                if not alternatives:
                    return copy.deepcopy(baseline)
                return alternatives[0]
            if ordinal == 2:
                return None
            raise ValueError(f"unsupported audit NullFlavor ordinal: {ordinal}")
        if ordinal == 0:
            return copy.deepcopy(baseline)
        if ordinal == 2:
            return "" if "empty_string" in str(field.get("_auditClearPolicy", "")) else None
        if ordinal != 1:
            raise ValueError(f"unsupported audit-field candidate ordinal: {ordinal}")
        if "_auditAlternate" in field:
            return copy.deepcopy(field["_auditAlternate"])
        if field.get("_auditRequiresLookup"):
            raise ValueError(f"audit field requires a second source-valid value: {code}")
        if isinstance(baseline, bool):
            return not baseline
        if isinstance(baseline, int) and not isinstance(baseline, bool):
            return baseline + 1
        if isinstance(baseline, float):
            return baseline + 1.0
        if isinstance(baseline, list):
            return [*copy.deepcopy(baseline), "AUDIT-B"]
        if isinstance(baseline, str):
            if re.fullmatch(r"\d{8}", baseline) and "date" in f"{code} {path}".lower():
                import datetime
                try:
                    value = datetime.datetime.strptime(baseline, "%Y%m%d").date()
                    return (value + datetime.timedelta(days=1)).strftime("%Y%m%d")
                except ValueError:
                    pass
            suffix = "-AUDIT-B"
            limit = field.get("_maxLength")
            return (
                f"{baseline[:max(0, limit - len(suffix))]}{suffix}"
                if isinstance(limit, int) else f"{baseline}{suffix}"
            )
        raise ValueError(f"no source value generator for audit field {field.get('code')}")
    if is_nullflavor_field(field):
        allowed = tuple(field.get("_allowedNullFlavors", ()))
        if ordinal == 0:
            return invalid if invalid is not None else f"NF-{rng.randrange(1_000_000):06d}"
        if ordinal == 1:
            return None
        if ordinal == 2:
            return baseline
        if ordinal == 3:
            alternatives = [token for token in allowed if token != baseline]
            candidates = alternatives or [
                token for token in NULL_FLAVOR_TOKENS if token not in allowed
            ]
            if not candidates:
                raise ValueError(f"no alternate NullFlavor candidate for {field.get('code')}")
            return rng.choice(candidates)
        if ordinal == 4:
            return ""
        if ordinal == 5:
            return f"nullflavor-not-valid-{rng.randrange(1_000_000)}"
        if ordinal == 6:
            return 1
        if ordinal == 7:
            return {f"unexpected{rng.randrange(1_000)}": "nullFlavor"}
        if ordinal == 8:
            disallowed = [token for token in NULL_FLAVOR_TOKENS if token not in allowed]
            return (
                disallowed[sample % len(disallowed)]
                if disallowed
                else f"NF-{rng.randrange(1_000_000):06d}"
            )
        if ordinal == 9:
            return str(baseline).lower()
        if ordinal == 10:
            return "NULLFLAVOR-" + "".join(
                rng.choice("XYZ") for _ in range(rng.randrange(64, 257))
            )
        if ordinal == 11:
            return [baseline]
        if ordinal == 12:
            return True
        if ordinal == 13:
            return baseline
        raise ValueError(f"unsupported NullFlavor candidate ordinal: {ordinal}")
    if field.get("_booleanRule"):
        if ordinal == 2:
            return f"not-a-boolean-{rng.randrange(1_000_000)}"
        if ordinal == 7:
            return {f"unexpected{rng.randrange(1_000)}": "object"}
        values = (
            invalid if invalid is not None else f"not-a-boolean-{rng.randrange(1_000_000)}",
            None,
            None,
            False,
            "",
            True,
            False,
        )
        if 0 <= ordinal < len(values):
            return values[ordinal]
        raise ValueError(f"unsupported boolean candidate ordinal: {ordinal}")
    if ordinal == 0:
        if "invalidValue" not in constraint:
            raise ValueError(f"verified field has no invalidValue: {field.get('code')}")
        if code == "C.3.4.8" and isinstance(field.get("_maxLength"), int):
            length = field["_maxLength"] + 1
            return "a" * (length - len("@a.co")) + "@a.co"
        return invalid
    if ordinal == 1:
        return None
    if ordinal == 2:
        if isinstance(baseline, bool):
            return "not-a-boolean"
        if isinstance(baseline, (int, float)) and not isinstance(baseline, bool):
            return f"not-a-number-{rng.randrange(1_000_000)}"
        return rng.choice((" ", "\t")) * (sample + 1 + 3 * rng.randrange(4))
    if ordinal == 3:
        if isinstance(baseline, bool):
            return rng.choice([True, False])
        if "date" in code.lower() or "date" in path.lower():
            return ("00000000", "99999999", f"not-a-date-{rng.randrange(1_000_000)}")[sample % 3]
        if isinstance(baseline, list):
            return [f"fuzz-{rng.randrange(1_000_000)}"]
        if isinstance(baseline, (int, float)) and not isinstance(baseline, bool):
            return (-1, 0, 2**31 - 1, 2**31)[sample % 4]
        return f"<p>fuzz-{rng.randrange(1_000_000)} <strong>rich</strong></p>"
    if ordinal == 4:
        return ""
    if ordinal == 5:
        suffix = rng.randrange(1_000_000)
        if isinstance(baseline, list):
            return [f"한글🙂-{suffix}"]
        if isinstance(baseline, bool):
            return 1
        return f"한글🙂-{suffix}"
    if ordinal == 6:
        value = f"A{sample}" + "".join(
            rng.sample(["\x00", "\t", "\n", "\r", "\x1f", "\x7f"], 3)
        ) + "B"
        if isinstance(baseline, list):
            return [value]
        if isinstance(baseline, bool):
            return 0
        return value + "\x00"
    if ordinal == 7:
        if isinstance(baseline, list):
            return {f"unexpected{rng.randrange(1_000)}": "object"}
        if isinstance(baseline, bool):
            return 1
        if isinstance(baseline, (int, float)) and not isinstance(baseline, bool):
            return {f"unexpected{rng.randrange(1_000)}": rng.randrange(1_000)}
        return ["unexpected", rng.randrange(1_000_000)]
    if ordinal == 8:
        value = unicodedata.normalize(
            "NFD", "".join(chr(rng.randint(0xAC00, 0xD7A3)) for _ in range(3))
        )
        return [value] if isinstance(baseline, list) else value
    if ordinal == 9:
        value = rng.choice([
            "\u202bעברית العربية\u202c",
            "\u202eabc123\u202c",
            "\u2067فارسی\u2069",
        ]) + str(rng.randrange(1_000_000))
        return [value] if isinstance(baseline, list) else value
    if ordinal == 10:
        value = (
            "A"
            + "".join(rng.sample([
                chr(0x200B), chr(0x200C), chr(0x200D), chr(0xFEFF), chr(0x2060),
            ], 3))
            + f"{rng.randrange(1_000_000)}B"
        )
        return [value] if isinstance(baseline, list) else value
    if ordinal == 11:
        value = "".join(
            rng.choice(["👨‍👩‍👧‍👦", "🏳️‍🌈", "👩🏽‍💻", "🧑‍🚀"])
            for _ in range(rng.randrange(32, 97))
        )
        return [value] if isinstance(baseline, list) else value
    if ordinal == 12:
        value = chr(rng.randint(0xD800, 0xDFFF))
        return [value] if isinstance(baseline, list) else value
    if ordinal == 13:
        value = str(sample) + "".join(
            rng.choice([
                "\x7f", "\u0080", "\u0085", "\u009f",
                "\ufffd", "\ufffe", "\uffff", "\U0010ffff",
            ])
            for _ in range(4)
        )
        return [value] if isinstance(baseline, list) else value
    if 14 <= ordinal < 17 and isinstance(baseline, str) and isinstance(field.get("_maxLength"), int):
        limit = field["_maxLength"]
        length = (limit, limit + 1, limit + min(max(limit, 64), 4096))[ordinal - 14]
        if ordinal == 14 and len(baseline) == limit:
            return baseline
        if code == "C.3.4.8":
            return chr(ord("a") + sample % 26) * (length - len("@a.co")) + "@a.co"
        return "".join(chr(rng.randint(0xAC00, 0xD7A3)) for _ in range(length))
    if ordinal == 17 and field.get("_identifierRule"):
        return f"A{sample}{rng.choice([chr(9), chr(10), chr(13)])}B{rng.choice([chr(9), chr(10)])}C"
    raise ValueError(f"unsupported candidate ordinal {ordinal} for {field.get('code')}")


def candidate_count(field: dict[str, Any], requested: int) -> int:
    if field.get("_auditAll"):
        if is_nullflavor_field(field):
            distinct = [value for value in field.get("_allowedNullFlavors", ())
                        if value != field.get("roundTripValue")]
            return 3
        if field.get("_auditRequiresLookup"):
            return 1
        if "_auditAlternate" not in field:
            raise ValueError(f"audit field has no source-reviewed alternate: {field.get('code')}")
        if field.get("_auditAlternate") == field.get("roundTripValue"):
            return 1 if field.get("_auditIndependentMutation") is False else 3
        clear_policy = str(field.get("_auditClearPolicy", ""))
        return 2 if "row deletion" in clear_policy or "derived" in clear_policy else 3
    baseline = field.get("roundTripValue")
    available = BASE_CANDIDATES
    if is_nullflavor_field(field):
        available = NULLFLAVOR_CANDIDATES
    elif field.get("_identifierRule"):
        available = IDENTIFIER_CANDIDATES
    elif isinstance(baseline, str) and isinstance(field.get("_maxLength"), int):
        available = LENGTH_CANDIDATES
    elif isinstance(baseline, (str, list)):
        available = STRING_CANDIDATES
    return min(requested, available)


def candidate_kind(field: dict[str, Any], ordinal: int) -> str:
    if field.get("_auditAll"):
        if (is_nullflavor_field(field) and ordinal == 1
                and not any(value != field.get("roundTripValue")
                            for value in field.get("_allowedNullFlavors", ()))):
            return "audit_invariant"
        if (ordinal == 1 and field.get("_auditAlternate") == field.get("roundTripValue")):
            return "audit_invariant"
        return ("audit_normal", "audit_changed", "audit_clear")[ordinal]
    return (NULLFLAVOR_CANDIDATE_KINDS if is_nullflavor_field(field) else CANDIDATE_KINDS)[ordinal]


def add_nullflavor_partner(field: dict[str, Any], mutation: dict[str, Any], ordinal: int) -> bool:
    path = field.get("_nullFlavorPartnerPath")
    if ordinal != 13 or not isinstance(path, str):
        return False
    set_path(mutation, path, copy.deepcopy(field.get("_nullFlavorPartnerValue")))
    return True


def candidate_expectation(
    field: dict[str, Any], ordinal: int, candidate: Any = ...
) -> tuple[str, str | None] | None:
    if field.get("_auditAll"):
        if field.get("_auditComplianceGuard") and ordinal == 1:
            return None
        return ("accept", None) if ordinal in {0, 1} else None
    if is_nullflavor_field(field):
        if ordinal == 4:
            return "accept", None
        return None
    constraint = field.get("constraint", {})
    if ordinal == 0 and "invalidValue" in constraint:
        return "reject", constraint.get("ruleCode")
    if field.get("_booleanRule"):
        if ordinal == 5:
            return "accept_or_forbidden", None
        return ("accept", None) if ordinal in {1, 3, 6} else ("reject", field["_booleanRule"])
    if ordinal == 6 and isinstance(field.get("roundTripValue"), str):
        return "reject", "INPUT.CONTROL_CHAR.REJECTED"
    if ordinal == 17 and field.get("_identifierRule"):
        return "reject", field["_identifierRule"]
    if isinstance(field.get("_maxLength"), int):
        if ordinal == 14:
            return "length_boundary", field.get("_maxLengthRule")
        if ordinal in {15, 16}:
            return "reject", field.get("_maxLengthRule")
        if field.get("_maxLengthOnly") and candidate is not ...:
            if ordinal == 12:
                return "reject", None
            if ordinal == 13 and contains_rejected_control(candidate):
                return "reject", "INPUT.CONTROL_CHAR.REJECTED"
            if candidate is None:
                return "accept", None
            if isinstance(candidate, (int, float)) and not isinstance(candidate, bool):
                return None
            if not isinstance(candidate, str):
                return "reject", field.get("_maxLengthRule")
            return (
                ("reject", field.get("_maxLengthRule"))
                if len(candidate.strip()) > field["_maxLength"]
                else ("accept", None)
            )
    return None


def blank_nullflavor_requires_identity_refresh(
    field: dict[str, Any], ordinal: int
) -> bool:
    return (
        is_nullflavor_field(field)
        and ordinal == 4
        and field.get("payloadPath") in DEVICE_NULL_FLAVOR_PATHS
    )


def load_max_lengths(root: Path) -> tuple[dict[str, int], set[str]]:
    limits: dict[str, int] = {}
    max_only: set[str] = set()
    for path in sorted((root / "crates/libs/input-contracts/src/generated").glob("*.rs")):
        source = path.read_text(encoding="utf-8")
        for rule_code, limit in MAX_LENGTH_RE.findall(source):
            limits[rule_code] = int(limit)
        max_only.update(MAX_LENGTH_ONLY_RE.findall(source))
    return limits, max_only


def load_generated_rules(root: Path, pattern: re.Pattern[str]) -> set[str]:
    rules: set[str] = set()
    for path in sorted((root / "crates/libs/input-contracts/src/generated").glob("*.rs")):
        rules.update(pattern.findall(path.read_text(encoding="utf-8")))
    return rules


def apply_meddra_baseline(
    contract: list[dict[str, Any]], version: str | None, code: str | None
) -> int:
    """Replace stale contract MedDRA samples for a valid-fixture run."""
    if version is None and code is None:
        return 0
    changed = 0
    for page in contract:
        for field in page.get("fields", []):
            paths = " ".join(
                str(field.get(key, ""))
                for key in ("code", "frontendPath", "payloadPath", "projectionPath")
            ).lower()
            replacement = (
                version
                if version is not None and "meddraversion" in paths
                else code
                if code is not None and "meddracode" in paths
                else None
            )
            if replacement is not None and field.get("roundTripValue") != replacement:
                field["roundTripValue"] = replacement
                changed += 1
    return changed


def field_rule_prefix(field: dict[str, Any]) -> str:
    authority = str(field.get("authority", "ICH")).upper()
    code = str(field.get("code", ""))
    return code if code.upper().startswith(f"{authority}.") else f"{authority}.{code}"


def apply_max_lengths(
    contract: list[dict[str, Any]], metadata: tuple[dict[str, int], set[str]]
) -> int:
    limits, max_only = metadata
    matched = 0
    for page in contract:
        for field in page.get("fields", []):
            rule_code = f"{field_rule_prefix(field)}.LENGTH.MAX"
            if isinstance(field.get("roundTripValue"), str) and rule_code in limits:
                field["_maxLength"] = limits[rule_code]
                field["_maxLengthRule"] = rule_code
                field["_maxLengthOnly"] = rule_code in max_only
                matched += 1
    return matched


def apply_generated_rules(
    contract: list[dict[str, Any]], identifier_rules: set[str], boolean_rules: set[str]
) -> tuple[int, int]:
    identifiers = booleans = 0
    for page in contract:
        for field in page.get("fields", []):
            rule_code = f"{field_rule_prefix(field)}.ALLOWED.VALUE"
            if rule_code in identifier_rules:
                field["_identifierRule"] = rule_code
                identifiers += 1
            if rule_code in boolean_rules:
                field["_booleanRule"] = rule_code
                booleans += 1
    return identifiers, booleans


def summarize_error_detail(detail: Any) -> dict[str, str]:
    if not isinstance(detail, dict):
        return {}
    summary: dict[str, str] = {}
    for output_key, input_keys in {
        "rule_code": ("ruleCode", "rule_code"),
        "path": ("path",),
        "message": ("message",),
    }.items():
        value = next((detail.get(key) for key in input_keys if isinstance(detail.get(key), str)), None)
        if value is not None:
            summary[f"error_{output_key}"] = (
                value if output_key != "message" else f"<fp:{hashlib.sha256(value.encode()).hexdigest()[:12]}>"
            )
    return summary


def expectation_error(
    expectation: tuple[str, str | None] | None,
    status: int | None,
    actual_rule: str | None,
) -> str | None:
    if expectation is None:
        return None
    outcome, expected_rule = expectation
    if outcome == "accept":
        return None if status == 200 else f"expected HTTP 200, got {status}"
    if outcome == "accept_or_forbidden":
        return None if status in {200, 403} else f"expected HTTP 200 or 403, got {status}"
    if outcome == "length_boundary":
        if status == 200:
            return None
        return f"exact maximum was not saved: {actual_rule or status}"
    if status not in {400, 409, 422}:
        return f"expected input rejection, got {status}"
    if expected_rule and actual_rule != expected_rule:
        return f"expected {expected_rule}, got {actual_rule or 'no rule code'}"
    return None


def is_nullflavor_field(field: dict[str, Any]) -> bool:
    path = str(field.get("payloadPath", ""))
    code = str(field.get("code", ""))
    return (
        field.get("constraint", {}).get("status") == "verified"
        and ("nullflavor" in path.lower() or "nullflavor" in code.lower())
    )


def nullflavor_invalid_candidate(field: dict[str, Any], candidate: Any) -> bool:
    if not is_nullflavor_field(field):
        return False
    if candidate is None or candidate == "":
        return False
    allowed = field.get("_allowedNullFlavors")
    if allowed:
        return candidate not in allowed
    invalid = field.get("constraint", {}).get("invalidValue")
    return candidate == (invalid if invalid is not None else "ZZZ")


def snake(value: str) -> str:
    value = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", value)
    return re.sub(r"[^a-zA-Z0-9]+", "_", value).strip("_").lower()


def snake_keys(value: Any) -> Any:
    if isinstance(value, dict):
        return {snake(key): snake_keys(child) for key, child in value.items()}
    if isinstance(value, list):
        return [snake_keys(child) for child in value]
    return copy.deepcopy(value)


def without_nested_owner(value: Any, owner: str, parent_id: str | None = None) -> Any:
    result = owner_business(value)
    if owner == "relatednessAssessments" and parent_id and isinstance(result, dict):
        rows = result.get("drugReactionAssessments")
        if isinstance(rows, list):
            relatedness_keys = {
                "id", "sequenceNumber", "sequence_number", "sourceOfAssessment",
                "source_of_assessment", "methodOfAssessment", "method_of_assessment",
                "methodOfAssessmentKr1", "method_of_assessment_kr1", "resultOfAssessment",
                "result_of_assessment", "resultOfAssessmentKr1", "result_of_assessment_kr1",
                "resultOfAssessmentKr1NullFlavor", "result_of_assessment_kr1_null_flavor",
                "resultOfAssessmentKr2", "result_of_assessment_kr2",
            }
            matching = [row for row in rows if isinstance(row, dict)
                        and row.get("drugReactionAssessmentId") == parent_id]
            if matching:
                parent_business = {key: value for key, value in matching[0].items()
                                   if key not in relatedness_keys}
                parent_business["relatednessTarget"] = OWNER_TARGET_SENTINEL
                result["drugReactionAssessments"] = [
                    *[row for row in rows if not isinstance(row, dict)
                      or row.get("drugReactionAssessmentId") != parent_id],
                    parent_business,
                ]
        return result
    if owner.startswith("fdaDevice") and parent_id and isinstance(result, dict):
        family = {
            "fdaDeviceFollowUpTypes": "followUpTypes",
            "fdaDeviceProblemCodes": "deviceProblemCodes",
            "fdaDeviceRemedialActions": "remedialActions",
        }[owner]
        devices = result.get("fdaDevices")
        if isinstance(devices, list):
            for device in devices:
                if isinstance(device, dict) and device.get("id") == parent_id:
                    device[family] = OWNER_TARGET_SENTINEL
        return result
    key = owner
    if isinstance(result, dict) and key in result:
        result[key] = OWNER_TARGET_SENTINEL
    return result


def sent_fields_match(row: dict[str, Any], payload: dict[str, Any],
                      omitted: set[str] | None = None) -> bool:
    omitted = omitted or set()
    return all(
        key in row and values_equal(value, row[key])
        for key, value in payload.items() if key not in omitted
    )


def audit_transition_matches(before_logs: list[dict[str, Any]], after_logs: list[dict[str, Any]],
                             before: dict[str, Any], after: dict[str, Any],
                             payload: dict[str, Any]) -> bool:
    changed: dict[str, tuple[Any, Any]] = {}
    for key in payload:
        if key in {"id", "sequenceNumber", "sequence_number"}:
            continue
        audit_key = snake(AUDIT_FIELD_ALIASES.get(key, key))
        before_key = key if key in before else audit_key
        after_key = key if key in after else audit_key
        old, new = before.get(before_key), after.get(after_key)
        if old != new:
            changed[audit_key] = (old, new)
    if not changed:
        return False
    def identity(log: dict[str, Any]) -> str:
        for key in ("id", "auditLogId", "audit_log_id"):
            if log.get(key) is not None:
                return f"{key}:{log[key]}"
        return json.dumps(log, sort_keys=True, default=str)
    before_ids = {identity(log) for log in before_logs}
    for log in (item for item in after_logs if identity(item) not in before_ids):
        old_values = log.get("old_values", log.get("oldValues"))
        new_values = log.get("new_values", log.get("newValues"))
        if not isinstance(old_values, dict) or not isinstance(new_values, dict):
            continue
        if all(key in old_values and key in new_values
               and values_equal(old, old_values[key]) and values_equal(new, new_values[key])
               for key, (old, new) in changed.items()):
            return True
    return False


def expected_audit_changes(before: dict[str, Any], after: dict[str, Any],
                           payload: dict[str, Any]) -> dict[str, tuple[Any, Any]]:
    result: dict[str, tuple[Any, Any]] = {}
    for key in payload:
        if key in {"id", "sequenceNumber", "sequence_number"}:
            continue
        # Nested collection scaffolding is persisted and audited by its child
        # table, not by the parent record's CREATE audit.
        if key in {"drugReactionAssessments"}:
            continue
        audit_key = snake(AUDIT_FIELD_ALIASES.get(key, key))
        before_key = key if key in before else audit_key
        after_key = key if key in after else audit_key
        old = before.get(before_key)
        new = after.get(after_key)
        if type(old) is not type(new) or old != new:
            result[audit_key] = (old, new)
    return result


def audit_expected_value(field: str, value: Any, owner: str | None = None) -> Any:
    """Convert only source-declared compact CI date projections to DB audit dates."""
    decimal_fields = {
        "age_at_time_of_onset", "parent_age", "weight_kg", "height_cm",
        "gestation_period", "strength_value", "dose_value", "number_of_units",
        "duration_value", "cumulative_dose_first_reaction_value",
        "gestation_period_exposure_value", "administration_start_interval_value",
        "last_dose_interval_value",
    }
    if field in decimal_fields and isinstance(value, (int, float, str)) \
            and not isinstance(value, bool):
        try:
            return float(value)
        except ValueError:
            return value
    date_fields = {
        "birth_date", "parent_birth_date", "date_of_death",
        "last_menstrual_period_date",
        "first_administration_date", "last_administration_date",
    }
    if owner in {"parentMedicalHistory", "parentPastDrugs", "pastDrugHistory"}:
        date_fields.update({"start_date", "end_date"})
    if field not in date_fields \
            or not isinstance(value, str) or not re.fullmatch(r"\d{8}", value):
        return value
    year, month, day = int(value[:4]), int(value[4:6]), int(value[6:])
    try:
        import datetime
        return datetime.date(year, month, day).isoformat()
    except ValueError:
        return value


def dosage_raw_date_value(value: Any) -> Any:
    if value is None:
        return None
    if isinstance(value, str) and re.fullmatch(r"\d{4}|\d{6}", value):
        return value
    return "" if isinstance(value, str) else value


def deleted_audit_changes(before: dict[str, Any]) -> dict[str, tuple[Any, Any]]:
    return {
        snake(AUDIT_FIELD_ALIASES.get(str(key), str(key))): (value, None)
        for key, value in owner_business(before).items()
        if key not in {
            "createdAt", "created_at", "createdBy", "created_by",
            "updatedAt", "updated_at", "updatedBy", "updated_by",
        }
    }


def audit_delta_exact(
    before_logs: list[dict[str, Any]], after_logs: list[dict[str, Any]], *,
    table: str, record_id: str, action: str, user_id: str, organization_id: str,
    expected_changes: dict[str, tuple[Any, Any]], exact_changed_keys: bool = True,
) -> tuple[bool, dict[str, Any]]:
    """Verify the one audit record caused by one scoped record operation."""
    def audit_id(log: dict[str, Any]) -> int | None:
        value = log.get("id", log.get("auditLogId", log.get("audit_log_id")))
        return value if isinstance(value, int) and not isinstance(value, bool) and value > 0 else None

    def audit_value_equal(expected: Any, actual: Any) -> bool:
        if isinstance(expected, bool) or isinstance(actual, bool):
            return type(expected) is bool and type(actual) is bool and expected is actual
        if isinstance(expected, list) or isinstance(actual, list):
            return (isinstance(expected, list) and isinstance(actual, list)
                    and len(expected) == len(actual)
                    and all(audit_value_equal(left, right)
                            for left, right in zip(expected, actual)))
        if isinstance(expected, dict) or isinstance(actual, dict):
            return (isinstance(expected, dict) and isinstance(actual, dict)
                    and set(expected) == set(actual)
                    and all(audit_value_equal(expected[key], actual[key]) for key in expected))
        if type(expected) is type(actual) and expected == actual:
            return True
        if isinstance(expected, (int, float, Decimal)) and not isinstance(expected, bool) \
                and isinstance(actual, str):
            try:
                return Decimal(str(expected)) == Decimal(actual)
            except InvalidOperation:
                return False
        return False

    before_pairs = [(audit_id(log), log) for log in before_logs if isinstance(log, dict)]
    after_pairs = [(audit_id(log), log) for log in after_logs if isinstance(log, dict)]
    ids_valid = (
        len(before_pairs) == len(before_logs) and len(after_pairs) == len(after_logs)
        and all(value is not None for value, _ in [*before_pairs, *after_pairs])
        and len({value for value, _ in before_pairs}) == len(before_pairs)
        and len({value for value, _ in after_pairs}) == len(after_pairs)
    )
    before_by_id = {value: log for value, log in before_pairs}
    after_by_id = {value: log for value, log in after_pairs}
    prior_unchanged = ids_valid and all(after_by_id.get(value) == log for value, log in before_pairs)
    added = [log for value, log in after_pairs if value not in before_by_id]
    expected_action = action.upper()
    expected = {snake(AUDIT_FIELD_ALIASES.get(key, key)): pair
                for key, pair in expected_changes.items()}
    detail: dict[str, Any] = {
        "newAuditRecords": len(added), "expectedAction": expected_action,
        "expectedTable": table, "expectedRecordId": record_id,
        "expectedChangedKeys": sorted(expected),
    }
    detail.update({"auditIdsValid": ids_valid, "priorAuditRecordsUnchanged": prior_unchanged})
    if not ids_valid or not prior_unchanged or len(added) != 1:
        detail["actualNewRecords"] = added
        return False, detail
    log = added[0]
    old_key = "old_values" if "old_values" in log else "oldValues"
    new_key = "new_values" if "new_values" in log else "newValues"
    changed_key = "changed_fields" if "changed_fields" in log else "changedFields"
    changed_present = changed_key in log
    old_present = old_key in log
    new_present = new_key in log
    changed = log.get(changed_key)
    old_values = log.get(old_key)
    new_values = log.get(new_key)
    changed_valid = changed_present and isinstance(changed, dict)
    changed = changed if changed_valid else {}
    old_valid = old_values is None or isinstance(old_values, dict)
    new_valid = new_values is None or isinstance(new_values, dict)
    actual_keys = set(changed)
    envelope_ok = (
        str(log.get("table_name", log.get("tableName", ""))) == table
        and str(log.get("record_id", log.get("recordId", ""))) == record_id
        and str(log.get("action", "")).upper() == expected_action
        and str(log.get("user_id", log.get("userId", ""))) == user_id
        and str(log.get("organization_id", log.get("organizationId", ""))) == organization_id
    )
    action_shape_ok = (
        changed_valid and old_present and new_present and old_valid and new_valid
        and ((expected_action == "CREATE" and old_values is None and isinstance(new_values, dict))
        or (expected_action == "UPDATE" and isinstance(old_values, dict) and isinstance(new_values, dict))
        or (expected_action == "DELETE" and isinstance(old_values, dict) and new_values is None))
    )
    old_snapshot = old_values if isinstance(old_values, dict) else {}
    new_snapshot = new_values if isinstance(new_values, dict) else {}
    fields_ok = all(
        key in changed and isinstance(changed[key], dict)
        and "old" in changed[key] and "new" in changed[key]
        and audit_value_equal(old, changed[key]["old"])
        and audit_value_equal(new, changed[key]["new"])
        and (old_values is None or (key in old_snapshot and audit_value_equal(old, old_snapshot[key])))
        and (new_values is None or (key in new_snapshot and audit_value_equal(new, new_snapshot[key])))
        for key, (old, new) in expected.items()
    )
    snapshot_delta_exact = True
    if expected_action == "UPDATE" and isinstance(old_values, dict) and isinstance(new_values, dict):
        metadata = {"updated_at", "updated_by"}
        def snapshot_changes(old: Any, new: Any, prefix: str = "") -> dict[str, tuple[Any, Any]]:
            if isinstance(old, dict) and isinstance(new, dict):
                result: dict[str, tuple[Any, Any]] = {}
                for child in set(old) | set(new):
                    if not prefix and child in metadata:
                        continue
                    path = f"{prefix}.{child}" if prefix else child
                    result.update(snapshot_changes(old.get(child), new.get(child), path))
                return result
            return {} if audit_value_equal(old, new) else {prefix or "$": (old, new)}

        snapshot_delta = snapshot_changes(old_values, new_values)
        snapshot_delta_exact = set(snapshot_delta) == actual_keys and all(
            isinstance(changed.get(key), dict)
            and "old" in changed[key] and "new" in changed[key]
            and audit_value_equal(old, changed[key]["old"])
            and audit_value_equal(new, changed[key]["new"])
            for key, (old, new) in snapshot_delta.items()
        )
    keys_ok = actual_keys == set(expected) if exact_changed_keys else set(expected) <= actual_keys
    detail.update({
        "envelopeExact": envelope_ok, "actionSnapshotsExact": action_shape_ok,
        "changedValuesExact": fields_ok, "changedKeysExact": keys_ok,
        "snapshotDeltaExact": snapshot_delta_exact,
        "actualAction": log.get("action"), "actualTable": log.get("table_name", log.get("tableName")),
        "actualRecordId": log.get("record_id", log.get("recordId")),
        "actualUserId": log.get("user_id", log.get("userId")),
        "actualOrganizationId": log.get("organization_id", log.get("organizationId")),
        "actualChangedKeys": sorted(actual_keys),
    })
    complete = audit_log_complete(log)
    detail["auditEnvelopeComplete"] = complete
    return (ids_valid and prior_unchanged and envelope_ok and action_shape_ok
            and fields_ok and keys_ok and snapshot_delta_exact and complete), detail


def audit_no_delta_exact(
    before_logs: list[dict[str, Any]], after_logs: list[dict[str, Any]],
) -> tuple[bool, dict[str, Any]]:
    """Verify that a scoped record operation emitted no audit record or mutation."""
    def keyed(logs: list[dict[str, Any]]) -> dict[int, dict[str, Any]] | None:
        result: dict[int, dict[str, Any]] = {}
        for log in logs:
            if not isinstance(log, dict):
                return None
            value = log.get("id", log.get("auditLogId", log.get("audit_log_id")))
            if not isinstance(value, int) or isinstance(value, bool) or value <= 0 or value in result:
                return None
            result[value] = log
        return result

    before = keyed(before_logs)
    after = keyed(after_logs)
    valid = before is not None and after is not None
    unchanged = valid and before == after
    return bool(unchanged), {
        "auditIdsValid": valid,
        "newAuditRecords": 0 if unchanged else (
            len(set(after or {}) - set(before or {})) if valid else None
        ),
        "priorAuditRecordsUnchanged": unchanged,
    }


UNFILTERED_AUDIT_FIELDS = {"drug_additional_info_codes_json"}


def redacted(value: Any) -> dict[str, Any]:
    raw = json.dumps(value, sort_keys=True, default=str)
    return {"type": type(value).__name__, "length": len(raw), "fingerprint": hashlib.sha256(raw.encode()).hexdigest()[:12]}


def values_equal(candidate: Any, actual: Any) -> bool:
    """Treat API decimal strings (e.g. 64.50) as the numeric input they represent."""
    if candidate == actual:
        return True
    if isinstance(actual, list):
        return len(actual) == 1 and values_equal(candidate, actual[0])
    if isinstance(candidate, (int, float)) and not isinstance(candidate, bool) and isinstance(actual, str):
        try:
            return Decimal(str(candidate)) == Decimal(actual)
        except InvalidOperation:
            return False
    return False


def is_blank_candidate(value: Any) -> bool:
    return isinstance(value, str) and not value.strip()


def is_empty_value(value: Any) -> bool:
    if value is None or is_blank_candidate(value):
        return True
    return isinstance(value, list) and bool(value) and all(is_empty_value(item) for item in value)


def contains_rejected_control(value: Any) -> bool:
    if isinstance(value, str):
        return any(
            unicodedata.category(character) == "Cc"
            and character not in {"\t", "\n", "\r"}
            for character in value
        )
    if isinstance(value, list):
        return any(contains_rejected_control(item) for item in value)
    if isinstance(value, dict):
        return any(contains_rejected_control(item) for item in value.values())
    return False


def contract_rows(
    contract: list[dict[str, Any]], page: str, audit_all_fields: bool = False,
) -> list[dict[str, Any]]:
    return [
        field
        for field in next((item["fields"] for item in contract if item["pageId"] == page), [])
        if (audit_all_fields or field.get("constraint", {}).get("status") == "verified")
        and (field.get("roundTripValue") is not None
             or (audit_all_fields and field.get("_auditRequiresLookup")))
        and field.get("payloadPath")
    ]


def apply_audit_field_inventory(
    contract: list[dict[str, Any]], inventory: dict[str, Any], *, add_supplements: bool,
) -> int:
    """Apply source-reviewed A/B/clear descriptors to the complete persisted inventory."""
    records = inventory.get("fields")
    if not isinstance(records, list):
        raise ValueError("persisted field inventory has no fields array")
    by_identity = {record.get("identity"): record for record in records if isinstance(record, dict)}
    added = 0
    page_map = {page.get("pageId"): page for page in contract}
    for record in records:
        if not isinstance(record, dict) or record.get("page") == "RE":
            continue
        identity = record.get("identity")
        page = page_map.get(record.get("page"))
        if not isinstance(identity, str) or not isinstance(page, dict):
            continue
        candidates = [field for field in page.get("fields", []) if
                      all(field.get(key) == record.get(key) for key in
                          ("authority", "code", "frontendPath"))]
        if not candidates and add_supplements:
            descriptor = record.get("verificationDescriptor")
            if not isinstance(descriptor, dict):
                continue
            owner = record.get("owner")
            field = {
                "authority": record.get("authority"), "code": record.get("code"),
                "frontendPath": record.get("frontendPath"),
                "payloadPath": record.get("payloadPath"),
                "projectionPath": record.get("projectionPath"),
                "roundTripValue": copy.deepcopy(descriptor.get("alternateA")),
                "constraint": {"status": "source_confirmed"},
                "patch": {"kind": "row" if owner in ROW_PAGES.values() else "rows",
                          "owner": owner},
                "_apiAuthority": "ICH" if record.get("authority") == "APP" else record.get("authority"),
            }
            page.setdefault("fields", []).append(field)
            candidates = [field]
            added += 1
        for field in candidates:
            descriptor = record.get("verificationDescriptor")
            if not isinstance(descriptor, dict):
                continue
            field["roundTripValue"] = copy.deepcopy(descriptor.get("alternateA"))
            field["_auditAlternate"] = copy.deepcopy(descriptor.get("alternateB"))
            field["_auditClearPolicy"] = descriptor.get("clearPolicy")
            field["_auditWriteMechanism"] = descriptor.get("writeMechanism")
            field["_auditIndependentMutation"] = descriptor.get("independentMutation", True)
            field["_auditInventoryIdentity"] = identity
    return added


def validate_persisted_inventory(inventory: dict[str, Any]) -> tuple[int, list[str]]:
    denominator = inventory.get("counts", {}).get("persistedFieldDenominator")
    fields = inventory.get("fields")
    identities = [field.get("identity") for field in fields] if isinstance(fields, list) else []
    if (not isinstance(denominator, int) or denominator <= 0
            or len(identities) != denominator
            or not all(isinstance(identity, str) and identity for identity in identities)
            or len(set(identities)) != len(identities)):
        raise ValueError("persisted field inventory identities/denominator are inconsistent")
    return denominator, identities


def apply_audit_alternate_map(
    contract: list[dict[str, Any]], alternate_map: dict[str, Any],
) -> tuple[int, list[str]]:
    """Attach source-reviewed distinct values; never synthesize controlled values."""
    records = alternate_map.get("fields")
    if not isinstance(records, list):
        raise ValueError("audit alternate map has no fields array")
    by_identity = {record.get("identity"): record for record in records
                   if isinstance(record, dict) and isinstance(record.get("identity"), str)}
    if len(by_identity) != len(records):
        raise ValueError("audit alternate map identities are missing or duplicated")
    applied = 0
    unresolved: list[str] = []
    for page in contract:
        for field in page.get("fields", []):
            identity = "|".join(str(field.get(key, "")) for key in
                                ("page", "authority", "code", "frontendPath"))
            # Generated contract fields do not carry page; use the enclosing page.
            identity = "|".join((str(page.get("pageId", "")), str(field.get("authority", "")),
                                 str(field.get("code", "")), str(field.get("frontendPath", ""))))
            record = by_identity.get(identity)
            if record is None:
                continue
            field["roundTripValue"] = copy.deepcopy(record.get("alternateA"))
            if record.get("alternateB") is not None:
                field["_auditAlternate"] = copy.deepcopy(record["alternateB"])
            declared_nf_alternate = (is_nullflavor_field(field)
                                     and any(value != field.get("roundTripValue")
                                             for value in field.get("_allowedNullFlavors", ())))
            if record.get("requiresExistingLookup") and not declared_nf_alternate:
                field["_auditRequiresLookup"] = True
                unresolved.append(identity)
            if record.get("strategy") == "compliance_gated_expected_rejection":
                field["_auditComplianceGuard"] = True
            if isinstance(record.get("_mutationFixedPayload"), dict):
                field["_mutationFixedPayload"] = copy.deepcopy(record["_mutationFixedPayload"])
            field["_auditAlternateStrategy"] = record.get("strategy")
            applied += 1
    return applied, sorted(set(unresolved))


def load_null_flavor_pairs(path: Path) -> dict[str, list[dict[str, str]]]:
    source = path.read_text(encoding="utf-8")
    match = re.search(r"const PAIRS:[^=]+=\s*(\{.*?\n\});", source, re.DOTALL)
    if not match:
        raise ValueError(f"could not parse NullFlavor pairs from {path}")
    return json.loads(match.group(1))


def load_dictionary_null_flavors(root: Path) -> dict[str, list[str]]:
    result: dict[str, list[str]] = {}
    for name in ("ich-e2br3.json", "fda-regional.json", "mfds-regional.json"):
        value = json.loads((root / "registry/dictionary" / name).read_text(encoding="utf-8"))
        for entry in value.get("entries", value):
            flavors = entry.get("null_flavors")
            if flavors:
                result[entry["code"]] = flavors
    return result


def replace_path_leaf(path: str, replacement_path: str) -> str:
    replacement = replacement_path.rsplit(".", 1)[-1]
    return f"{path.rsplit('.', 1)[0]}.{replacement}" if "." in path else replacement


def null_flavor_projection(base: dict[str, Any], pair: dict[str, str]) -> str:
    path = str(base.get("projectionPath") or base["payloadPath"])
    prefix, separator, leaf = path.rpartition(".")
    leaf = leaf.removesuffix("[]")
    null_leaf = "race_code_null_flavor" if leaf == "race_codes" else f"{leaf}_null_flavor"
    return f"{prefix}{separator}{null_leaf}"


def expand_null_flavor_contracts(
    contract: list[dict[str, Any]],
    pairs_by_page: dict[str, list[dict[str, str]]],
    allowed_by_code: dict[str, list[str]],
    unsupported: list[str] | None = None,
) -> int:
    identifier_codes = {
        "gpMedicalRecordNumber": ("D.1.1.1", "1"),
        "specialistRecordNumber": ("D.1.1.2", "2"),
        "hospitalRecordNumber": ("D.1.1.3", "3"),
        "investigationNumber": ("D.1.1.4", "4"),
    }
    device_codes = {
        "fdaDevices[].deviceBrandName": "FDA.G.k.12.r.4",
        "fdaDevices[].commonDeviceName": "FDA.G.k.12.r.5",
    }
    derived = 0
    unresolved: list[str] = []
    for page in contract:
        page_id = page["pageId"]
        fields = page["fields"]
        for pair in pairs_by_page.get(page_id, []):
            existing = next(
                (
                    field
                    for field in fields
                    if str(field.get("payloadPath", "")).endswith(pair["nullFlavor"])
                ),
                None,
            )
            candidates = [
                field
                for field in fields
                if str(field.get("frontendPath", "")).endswith(pair["value"])
                or str(field.get("payloadPath", "")).removesuffix("[]") == pair["value"]
            ]
            base = min(
                candidates,
                key=lambda field: (
                    str(field.get("payloadPath", "")).removesuffix("[]") != pair["value"],
                    len(str(field.get("frontendPath", ""))),
                ),
                default=None,
            )
            fixed_payload: dict[str, Any] | None = None
            if base is None and page_id == "DM" and pair["value"] in identifier_codes:
                code, identifier_type = identifier_codes[pair["value"]]
                base = {
                    "code": code,
                    "authority": "ICH",
                    "frontendPath": f"patientInformation.{pair['value']}",
                    "projectionPath": "patientIdentifiers[].identifier_value",
                    "patch": {"kind": "rows", "owner": "patientIdentifiers"},
                    "payloadPath": "[].identifierValue",
                }
                fixed_payload = {"identifierTypeCode": identifier_type}
            if base is None and page_id == "DG" and pair["value"] == "drugReactionAssessments[].resultOfAssessmentKr1":
                base = {
                    "code": "G.k.9.i.2.r.3.KR.1",
                    "authority": "MFDS",
                    "frontendPath": f"drugs[].{pair['value']}",
                    "projectionPath": "drug.drugReactionAssessments[].result_of_assessment_kr1",
                    "patch": {"kind": "row", "owner": "drug"},
                    "payloadPath": pair["value"],
                }
            if base is None and page_id == "DG" and pair["value"] in device_codes:
                code = device_codes[pair["value"]]
                leaf = pair["value"].rsplit(".", 1)[-1]
                sibling = next(path for path in device_codes if path != pair["value"])
                base = {
                    "code": code,
                    "authority": "FDA",
                    "frontendPath": f"drugs[].{pair['value']}",
                    "projectionPath": f"drug.fdaDevices[].{snake(leaf)}",
                    "patch": {"kind": "row", "owner": "drug"},
                    "payloadPath": pair["value"],
                    "roundTripValue": f"Fuzz {leaf}",
                    "_mutationFixedPayload": {sibling: f"Fuzz {sibling.rsplit('.', 1)[-1]}"},
                }
            if base is None:
                unresolved.append(f"{page_id}:{pair['value']}")
                continue
            allowed = allowed_by_code.get(base["code"], [])
            if not allowed:
                unresolved.append(f"{page_id}:{base['code']}:dictionary")
                continue
            if existing is not None:
                existing["_allowedNullFlavors"] = allowed
                existing["_nullFlavorPartnerPath"] = base["payloadPath"]
                existing["_nullFlavorPartnerValue"] = base.get("roundTripValue", "FUZZ-VALUE")
                existing["_nullFlavorPartnerProjection"] = projection_leaf(
                    base, base["patch"]["owner"]
                )
                continue
            official_code = base["code"]
            authority = str(base.get("authority", "ICH")).upper()
            rule_prefix = official_code if official_code.startswith(f"{authority}.") else f"{authority}.{official_code}"
            payload_path = (
                "[].identifierValueNullFlavor"
                if fixed_payload
                else replace_path_leaf(base["payloadPath"], pair["nullFlavor"])
            )
            projection_path = (
                "patientIdentifiers[].identifier_value_null_flavor"
                if fixed_payload
                else null_flavor_projection(base, pair)
            )
            fields.append(
                {
                    **base,
                    "code": f"{official_code}.nullFlavor",
                    "frontendPath": replace_path_leaf(base["frontendPath"], pair["nullFlavor"]),
                    "projectionPath": projection_path,
                    "roundTripValue": allowed[0],
                    "constraint": {
                        "status": "verified",
                        "ruleCode": f"{rule_prefix}.NULLFLAVOR.ALLOWED",
                        "invalidValue": "ZZZ",
                    },
                    "payloadPath": payload_path,
                    "nullFlavorPartnerCode": official_code,
                    "_allowedNullFlavors": allowed,
                    "_nullFlavorPartnerPath": base["payloadPath"],
                    "_nullFlavorPartnerValue": base.get("roundTripValue", "FUZZ-VALUE"),
                    "_nullFlavorPartnerProjection": projection_leaf(
                        base, base["patch"]["owner"]
                    ),
                    "_derivedNullFlavor": True,
                    **({"_fixedPayload": fixed_payload} if fixed_payload else {}),
                }
            )
            derived += 1
    if unresolved:
        if unsupported is None:
            raise ValueError(f"unresolved NullFlavor pairs: {', '.join(unresolved)}")
        unsupported.extend(unresolved)
    return derived


def baseline_for(
    fields: list[dict[str, Any]], minimal: bool = False, safe: bool = False
) -> dict[str, Any]:
    result: dict[str, Any] = {}
    concrete = [field for field in fields if not is_nullflavor_field(field)]
    if minimal:
        fields = (concrete or fields)[:2]
    elif concrete:
        fields = concrete
    for field in fields:
        for path, value in field.get("_fixedPayload", {}).items():
            set_path(result, path, copy.deepcopy(value))
        value = field["roundTripValue"]
        set_path(result, field["payloadPath"], False if safe and value is True else copy.deepcopy(value))
    return result


def owner_patch(owner: str, row: dict[str, Any], array_owner: bool) -> dict[str, Any]:
    return {owner: [row] if array_owner else row}


def authorities_for(fields: list[dict[str, Any]]) -> list[str]:
    values = {str(field.get("_apiAuthority", field.get("authority", "ICH"))).lower()
              for field in fields}
    return sorted(values or {"ich"})


def extract_row_id(projection: Any, owner: str) -> str | None:
    rows = projection.get("rows", projection) if isinstance(projection, dict) else projection
    if isinstance(rows, dict):
        value = rows.get(owner)
        if isinstance(value, list) and value:
            return value[0].get("id") if isinstance(value[0], dict) else None
        if isinstance(value, dict):
            return value.get("id")
    return None


def projection_rows(projection: Any, owner: str) -> list[dict[str, Any]]:
    rows = projection.get("rows", projection) if isinstance(projection, dict) else projection
    if isinstance(rows, dict):
        rows = rows.get(owner, rows.get("rows"))
    return [row for row in rows if isinstance(row, dict)] if isinstance(rows, list) else []


def direct_lifecycle_rows(projection: Any, owner: str) -> list[dict[str, Any]]:
    if owner != "fdaCrossReportedIndNumbers":
        rows = projection_rows(projection, owner)
        if owner in {
            "documentsHeldBySender", "otherCaseIdentifiers", "sourceDocuments",
            "senderDiagnoses",
        }:
            if not all(isinstance(get_path(row, "sequenceNumber"), int) for row in rows):
                return rows
            return sorted(rows, key=lambda row: (
                get_path(row, "sequenceNumber"), str(row.get("id", "")),
            ))
        return rows
    rows = projection.get("rows", projection) if isinstance(projection, dict) else {}
    study = rows.get("studyInformation") if isinstance(rows, dict) else None
    nested = study.get(owner) if isinstance(study, dict) else None
    return [row for row in nested if isinstance(row, dict)] if isinstance(nested, list) else []


def direct_page_business_equal(left: Any, right: Any, owner: str) -> bool:
    """Compare a direct page without treating an unspecified SQL row order as business."""
    left_source = direct_lifecycle_rows(left, owner)
    right_source = direct_lifecycle_rows(right, owner)
    if (not all(isinstance(row.get("id"), str) and row["id"] for row in [*left_source, *right_source])
            or len({row["id"] for row in left_source}) != len(left_source)
            or len({row["id"] for row in right_source}) != len(right_source)):
        return False
    left_rows = {row["id"]: owner_business(row) for row in left_source}
    right_rows = {row["id"]: owner_business(row) for row in right_source}
    return (left_rows == right_rows
            and page_without_owner(left, owner) == page_without_owner(right, owner))


def direct_lifecycle_patch(owner: str, rows: list[dict[str, Any]], parent_id: str | None) -> dict[str, Any]:
    if owner == "fdaCrossReportedIndNumbers":
        if not parent_id:
            raise ValueError("studyInformation id is required for FDA IND lifecycle")
        return {"studyInformation": {"id": parent_id, owner: rows}}
    return {owner: rows}


def lifecycle_identity_check(
    initial_ids: list[str], survivor_ids: list[str], final_ids: list[str],
    deleted_id: str, fresh_id: str | None,
    before_details: dict[str, Any], survivor_details: dict[str, Any],
) -> bool:
    return (
        len(initial_ids) == 3 and len(set(initial_ids)) == 3
        and deleted_id == initial_ids[1]
        and survivor_ids == [initial_ids[0], initial_ids[2]]
        and deleted_id not in survivor_ids
        and survivor_details == {key: before_details[key] for key in survivor_ids if key in before_details}
        and len(survivor_details) == 2
        and bool(fresh_id) and fresh_id not in initial_ids
        and final_ids == [initial_ids[0], initial_ids[2], fresh_id]
    )


def page_without_owner(value: Any, owner: str) -> Any:
    result = owner_business(value)
    rows = result.get("rows") if isinstance(result, dict) else None
    if owner == "fdaCrossReportedIndNumbers" and isinstance(rows, dict):
        study = rows.get("studyInformation")
        if isinstance(study, dict) and owner in study:
            study[owner] = OWNER_TARGET_SENTINEL
        return result
    if isinstance(rows, dict) and owner in rows:
        target = rows[owner]
        aliases = {
            "parentMedicalHistory": ("medicalHistory",),
            "parentPastDrugs": ("pastDrugHistory", "pastDrugs"),
        }.get(owner, ())
        if aliases and isinstance(target, list) and target:
            parent_ids = {row.get("parent_id") for row in target if isinstance(row, dict)}
            parents = rows.get("parents")
            if len(parent_ids) == 1 and None not in parent_ids and isinstance(parents, list):
                matching = [parent for parent in parents
                            if isinstance(parent, dict) and parent.get("id") in parent_ids]
                if len(matching) == 1 and all(matching[0].get(alias) == target for alias in aliases):
                    for alias in aliases:
                        matching[0][alias] = OWNER_TARGET_SENTINEL
        rows[owner] = OWNER_TARGET_SENTINEL
    return result


def lifecycle_payload(base: dict[str, Any], fields: list[dict[str, Any]],
                      owner: str, suffix: str) -> dict[str, Any]:
    result = copy.deepcopy(base)
    safe_path = LIFECYCLE_TEXT_PATHS.get(owner)
    field = next((item for item in fields if item.get("payloadPath") == safe_path), None)
    if field:
        value = f"{field['roundTripValue']}-{suffix}"
        limit = field.get("_maxLength")
        if isinstance(limit, int):
            suffix_value = f"-{suffix}"
            value = f"{str(field['roundTripValue'])[:max(0, limit - len(suffix_value))]}{suffix_value}"
        set_path(result, field["payloadPath"], value)
    return result


def payload_fields_match(row: dict[str, Any], payload: dict[str, Any],
                         fields: list[dict[str, Any]], owner: str) -> bool:
    checked = 0
    for field in fields:
        payload_value = get_path(payload, field["payloadPath"])
        if payload_value is None or (
            isinstance(payload_value, list) and payload_value
            and all(item is None for item in payload_value)
        ):
            continue
        projected = get_path(row, projection_leaf(field, owner))
        if not values_equal(payload_value, projected):
            return False
        checked += 1
    return checked > 0


def created_row_id(value: Any) -> str | None:
    if isinstance(value, dict):
        candidate = value.get("rowId")
        if isinstance(candidate, str) and re.fullmatch(r"[0-9a-fA-F-]{36}", candidate):
            return candidate
        candidate = value.get("id")
        if isinstance(candidate, str) and re.fullmatch(r"[0-9a-fA-F-]{36}", candidate):
            return candidate
        for child in value.values():
            found = created_row_id(child)
            if found:
                return found
    if isinstance(value, list):
        for child in value:
            found = created_row_id(child)
            if found:
                return found
    return None


def owner_business(value: Any) -> Any:
    """Return the complete DTO minus the finite server-owned update metadata."""
    if isinstance(value, dict):
        return {key: owner_business(child) for key, child in value.items()
                if key not in OWNER_METADATA_KEYS}
    if isinstance(value, list):
        return [owner_business(child) for child in value]
    return copy.deepcopy(value)


def owner_path_value(value: Any, path: str,
                     nested_identity: dict[str, str] | None = None) -> tuple[bool, Any]:
    current = value
    relative = leaf_path(path)
    if nested_identity:
        root = nested_root(path)
        if not root:
            return False, None
        current = row_with_identity(get_path(value, root), nested_identity)
        if current is None:
            return False, None
        relative = path.removeprefix(root).lstrip(".")
    for part in (part.removesuffix("[]") for part in relative.split(".") if part):
        if not isinstance(current, dict):
            return False, None
        normalized = snake(part)
        matches = [key for key in current if snake(str(key)) == normalized]
        if len(matches) != 1:
            return False, None
        current = current[matches[0]]
    return True, current


def nullflavor_partner_projection(field: dict[str, Any], owner: str,
                                  owner_fields: list[dict[str, Any]]) -> str | None:
    explicit = field.get("_nullFlavorPartnerProjection")
    if isinstance(explicit, str):
        return explicit
    declared = field.get("_nullFlavorPartnerPath")
    if not isinstance(declared, str):
        return None
    matches = [candidate for candidate in owner_fields
               if candidate.get("payloadPath") == declared]
    return projection_leaf(matches[0], owner) if len(matches) == 1 else None


def owner_without_target(value: Any, path: str,
                         nested_identity: dict[str, str] | None = None,
                         companion_path: str | None = None) -> Any:
    result = owner_business(value)

    def mask_existing(target_path: str) -> None:
        root = nested_root(target_path) if nested_identity else None
        target = (
            row_with_identity(get_path(result, root), nested_identity)
            if root and nested_identity else result
        )
        relative = target_path.removeprefix(root).lstrip(".") if root else leaf_path(target_path)
        parts = [part.removesuffix("[]") for part in relative.split(".") if part]
        current: Any = target
        for part in parts[:-1]:
            if not isinstance(current, dict):
                return
            matches = [key for key in current if snake(str(key)) == snake(part)]
            if len(matches) != 1:
                return
            current = current[matches[0]]
        if isinstance(current, dict) and parts:
            matches = [key for key in current if snake(str(key)) == snake(parts[-1])]
            if len(matches) == 1:
                current[matches[0]] = OWNER_TARGET_SENTINEL

    mask_existing(path)
    if companion_path:
        mask_existing(companion_path)
    return result


def run_gate(command: list[str], cwd: Path, timeout: float) -> tuple[str, dict[str, Any]]:
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return "GATE_BLOCKED", {"command": command, "error": type(error).__name__}
    return (
        "GATE_PASS" if result.returncode == 0 else "GATE_FAIL",
        {
            "command": command,
            "exit_code": result.returncode,
            "stdout_fingerprint": hashlib.sha256(result.stdout.encode()).hexdigest()[:12],
            "stderr_fingerprint": hashlib.sha256(result.stderr.encode()).hexdigest()[:12],
        },
    )


def main(args: argparse.Namespace) -> int:
    runner_sha256 = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    print(json.dumps({"kind": "startup", "runner_sha256": runner_sha256}), flush=True)
    guard_target(args.base_url, args.allow_remote)
    if args.samples_per_category < 1:
        raise SystemExit("--samples-per-category must be at least 1")
    if args.values_per_field < 0:
        raise SystemExit("--values-per-field must be nonnegative")
    if args.audit_all_fields and (args.lifecycle_only or args.null_flavor_only):
        raise SystemExit("--audit-all-fields cannot be combined with lifecycle-only/null-flavor-only")
    if not args.dry_run and not args.product_key:
        raise SystemExit("set --product-key or E2BR3_PRODUCT_KEY to an accessible product ID")
    if not args.dry_run and (not args.meddra_version or not args.meddra_code):
        raise SystemExit("set both --meddra-version and --meddra-code for live intake creation")
    if not args.password and not args.dry_run:
        raise SystemExit("set E2BR3_ADMIN_PASSWORD")
    contract_path = Path(args.contract).resolve()
    contract = json.loads(contract_path.read_text())
    audit_inventory: dict[str, Any] | None = None
    persisted_field_denominator: int | None = None
    persisted_field_inventory_sha256: str | None = None
    supplemental_audit_fields = 0
    audit_alternate_map_sha256: str | None = None
    audit_alternate_unresolved: list[str] = []
    audit_alternate_map: dict[str, Any] | None = None
    if args.audit_all_fields:
        if not args.persisted_field_inventory:
            raise SystemExit("--audit-all-fields requires --persisted-field-inventory")
        inventory_bytes = Path(args.persisted_field_inventory).read_bytes()
        persisted_field_inventory_sha256 = hashlib.sha256(inventory_bytes).hexdigest()
        audit_inventory = json.loads(inventory_bytes)
        persisted_field_denominator, _ = validate_persisted_inventory(audit_inventory)
        if not any(page.get("pageId") == "RE" for page in contract):
            contract.append({"pageId": "RE", "fields": []})
        supplemental_audit_fields = apply_audit_field_inventory(
            contract, audit_inventory, add_supplements=True,
        )
        if not args.audit_alternate_map:
            raise SystemExit("--audit-all-fields requires --audit-alternate-map")
        alternate_path = Path(args.audit_alternate_map).resolve()
        alternate_bytes = alternate_path.read_bytes()
        audit_alternate_map_sha256 = hashlib.sha256(alternate_bytes).hexdigest()
        audit_alternate_map = json.loads(alternate_bytes)
    apply_meddra_baseline(contract, args.meddra_version, args.meddra_code)
    backend_root = Path(__file__).resolve().parents[1]
    pages = [page.strip().upper() for page in args.pages.split(",") if page.strip()]
    default_pages = "CI,RP,SD,LR,SI,DM,DH,NR,AE,LB,DG"
    if args.audit_all_fields and args.pages == default_pages and "RE" not in pages:
        pages.append("RE")
    unknown_pages = set(pages) - {page["pageId"] for page in contract}
    if unknown_pages:
        raise SystemExit(f"unsupported pages: {', '.join(sorted(unknown_pages))}")
    if args.lifecycle_owner:
        if not args.lifecycle_only:
            raise SystemExit("--lifecycle-owner requires --lifecycle-only")
        if len(pages) != 1:
            raise SystemExit("--lifecycle-owner requires exactly one --pages value")
        allowed = DIRECT_LIFECYCLE_OWNER_OVERRIDES.get(pages[0], set())
        if args.lifecycle_owner not in allowed:
            raise SystemExit(
                f"unsupported lifecycle owner for {pages[0]}: {args.lifecycle_owner}"
            )
    if args.nested_lifecycle_owner:
        if not args.lifecycle_only or pages != ["DG"]:
            raise SystemExit("--nested-lifecycle-owner requires --lifecycle-only --pages DG")
        if args.lifecycle_owner or args.nested_lifecycle_owner not in DG_NESTED_LIFECYCLE_ENDPOINTS:
            raise SystemExit(f"unsupported nested lifecycle owner: {args.nested_lifecycle_owner}")
    max_length_fields = apply_max_lengths(contract, load_max_lengths(backend_root))
    identifier_fields, boolean_fields = apply_generated_rules(
        contract,
        load_generated_rules(backend_root, IDENTIFIER_RE),
        load_generated_rules(backend_root, BOOLEAN_RE),
    )
    unsupported: list[str] = []
    derived_null_flavors = expand_null_flavor_contracts(
        [page for page in contract if page["pageId"] != "RE"],
        load_null_flavor_pairs(Path(args.null_flavor_pairs).resolve()),
        load_dictionary_null_flavors(Path(__file__).resolve().parents[1]),
        unsupported,
    )
    if audit_inventory is not None:
        apply_audit_field_inventory(contract, audit_inventory, add_supplements=False)
    if audit_alternate_map is not None:
        applied_alternates, audit_alternate_unresolved = apply_audit_alternate_map(
            contract, audit_alternate_map,
        )
        if applied_alternates != audit_alternate_map.get("counts", {}).get("fields"):
            raise ValueError("audit alternate map does not match expanded contract")
    requested_fields = set(args.field or ())
    selected_blocked_identities = [
        record.get("identity") for record in (audit_inventory or {}).get("fields", [])
        if record.get("identity") in set(audit_alternate_unresolved)
        and record.get("page") in pages
        and (not requested_fields or record.get("code") in requested_fields)
    ]
    available_fields = {
        field.get("code") for page in pages for field in contract_rows(contract, page, args.audit_all_fields)
    }
    if args.audit_all_fields:
        available_fields.update(
            field.get("code") for field in (audit_inventory or {}).get("fields", [])
            if field.get("page") == "RE"
        )
    unknown_fields = requested_fields - available_fields
    if unknown_fields:
        raise SystemExit(f"unknown field codes: {', '.join(sorted(unknown_fields))}")
    started = time.monotonic()
    events: list[Event] = []
    interrupted: str | None = None
    client = ApiClient(args.base_url, args.timeout)
    case_id: str | None = None
    request_count = 0
    reaction_id: str | None = None

    def page_fields(page: str) -> list[dict[str, Any]]:
        fields = contract_rows(contract, page, args.audit_all_fields)
        if args.audit_all_fields:
            for field in fields:
                field["_auditAll"] = True
        if requested_fields:
            fields = [field for field in fields if field.get("code") in requested_fields]
        if args.audit_all_fields:
            fields = [field for field in fields if not field.get("_auditRequiresLookup")]
        return [field for field in fields if is_nullflavor_field(field)] if args.null_flavor_only else fields

    def add(event: Event) -> None:
        events.append(event)

    planned_mutations = 0 if args.lifecycle_only else sum(
        candidate_sample_count(field, ordinal, args.samples_per_category)
        for page in pages for field in page_fields(page)
        for ordinal in range(candidate_count(field, args.values_per_field))
    )
    selected_re_codes = {
        record.get("code") for record in (audit_inventory or {}).get("fields", [])
        if record.get("page") == "RE"
        and (not requested_fields or record.get("code") in requested_fields)
    } if args.audit_all_fields and "RE" in pages else set()
    planned_mutations += 4 * len(selected_re_codes)
    excluded_fields = [
        {"page": page["pageId"], "code": field["code"]}
        for page in contract if page["pageId"] in pages
        for field in page["fields"] if field not in contract_rows(contract, page["pageId"], args.audit_all_fields)
    ]

    def request(method: str, path: str, payload: dict[str, Any] | None = None) -> tuple[int | None, Any, dict[str, Any]]:
        nonlocal interrupted, request_count
        if request_count >= args.max_actions:
            interrupted = interrupted or "max_actions"
            return None, None, {}
        if time.monotonic() - started >= args.deadline_seconds:
            interrupted = interrupted or "deadline"
            return None, None, {}
        request_count += 1
        status, body, transport = client.request(method, path, payload)
        summary = response_summary(status, body)
        try:
            error_value = json.loads(body)
            error = error_value.get("error", {}) if isinstance(error_value, dict) else {}
            if isinstance(error.get("code"), str):
                summary["error_code"] = error["code"]
            detail = error.get("data", {}).get("detail") if isinstance(error, dict) else None
            summary.update(summarize_error_detail(detail))
        except (UnicodeDecodeError, json.JSONDecodeError, AttributeError):
            pass
        if transport:
            summary["transport_error"] = transport
            interrupted = interrupted or "transport_error"
            add(Event("request_failure", None, None, None, None, "INCONCLUSIVE", status,
                      {**summary, "method": method, "path": path}))
        if status == 429:
            interrupted = interrupted or "rate_limited"
        if status is not None and status >= 500:
            interrupted = interrupted or "server_error"
        try:
            value = json.loads(body)
        except (UnicodeDecodeError, json.JSONDecodeError):
            value = None
        return status, unwrap(value), summary

    if args.dry_run:
        total = 0
        for page in pages:
            fields = page_fields(page)
            groups: dict[str, list[dict[str, Any]]] = {}
            for field in fields:
                groups.setdefault(field["patch"]["owner"], []).append(field)
            mutations = sum(
                candidate_sample_count(field, ordinal, args.samples_per_category)
                for field in fields
                for ordinal in range(candidate_count(field, args.values_per_field))
            )
            displayed_fields = len(fields)
            if page == "RE":
                displayed_fields = len(selected_re_codes)
                mutations = 4 * displayed_fields
            total += mutations
            print(f"{page}: fields={displayed_fields} owners={len(groups)} mutations={mutations}")
        print(f"seed={args.seed} total_mutations={planned_mutations} derived_null_flavors={derived_null_flavors} max_length_fields={max_length_fields} identifier_fields={identifier_fields} boolean_fields={boolean_fields} contract={contract_path}")
        print(json.dumps({"verdict": "NOT_RUN", "planned_mutations": planned_mutations,
                          "audit_all_fields": args.audit_all_fields,
                          "supplemental_audit_fields": supplemental_audit_fields,
                          "persisted_field_denominator": persisted_field_denominator,
                          "audit_alternate_map_sha256": audit_alternate_map_sha256,
                          "audit_alternate_unresolved": selected_blocked_identities,
                          "selected_persisted_fields": (
                              sum(len(page_fields(page)) for page in pages)
                              + len(selected_re_codes) + len(selected_blocked_identities)
                          ),
                          "executable_persisted_fields": (
                              sum(len(page_fields(page)) for page in pages)
                              + len(selected_re_codes)
                          ),
                          "blocked_persisted_fields": selected_blocked_identities,
                          "excluded_fields": excluded_fields,
                          "unsupported_null_flavor_pairs": unsupported}, sort_keys=True))
        return 1 if unsupported or not total else 0

    if zero_mutation_admission_blocked(
        planned_mutations, args.values_per_field, args.lifecycle_only
    ):
        raise SystemExit("no input mutations selected; no API requests were sent")

    status, _, summary = request(
        "POST",
        "/auth/v1/login",
        {"email": args.email, "pwd": args.password},
    )
    add(Event("login", None, None, None, None, "PASS" if status == 200 else "FAIL", status, summary))
    if status != 200:
        interrupted = interrupted or "login_failed"

    audit_user_id: str | None = None
    audit_organization_id: str | None = None
    if not interrupted:
        profile_status, profile, profile_summary = request("GET", "/api/users/me/profile")
        profile_data = profile.get("data", profile) if isinstance(profile, dict) else {}
        profile_user = profile_data.get("user") if isinstance(profile_data, dict) else None
        active_org = profile_data.get("activeOrganization") if isinstance(profile_data, dict) else None
        if isinstance(profile_user, dict) and isinstance(active_org, dict):
            audit_user_id = profile_user.get("id")
            audit_organization_id = active_org.get("id")
        context_ok = (
            profile_status == 200 and isinstance(audit_user_id, str)
            and isinstance(audit_organization_id, str)
        )
        add(Event("audit_context", None, None, None, None,
                  "PASS" if context_ok else "INCONCLUSIVE", profile_status,
                  {**profile_summary, "userId": audit_user_id,
                   "organizationId": audit_organization_id}))
        if not context_ok:
            interrupted = interrupted or "audit_context_unavailable"

    if not interrupted:
        status, products, summary = request("GET", "/api/presaves/products")
        product_found = (
            status == 200
            and isinstance(products, list)
            and all(isinstance(product, dict) for product in products)
            and any(product.get("product_id") == args.product_key for product in products)
        )
        add(Event("product", None, None, None, None, "PASS" if product_found else "FAIL", status, summary))
        if not product_found:
            interrupted = interrupted or "product_key_not_accessible"

    if not interrupted:
        intake_id = f"FUZZ-{uuid.uuid4().hex[:12]}"
        intake_date = time.strftime("%Y%m%d", time.gmtime())
        status, created, summary = request(
            "POST",
            "/api/cases/from-intake",
            {"data": {
                "authority": "ich",
                "safety_report_id": intake_id,
                "date_first_received_from_source": intake_date,
                "date_of_most_recent_information": intake_date,
                "report_type": "1",
                "status": "draft",
                "patient_initials": intake_id[-6:],
                "age_d2_2a": "41",
                "sex_d5": "2",
                "dg_prd_key": args.product_key,
                "reaction_meddra_version": args.meddra_version,
                "reaction_meddra_code": args.meddra_code,
                "ae_start_date": intake_date,
            }},
        )
        case_id = created.get("case_id") if isinstance(created, dict) else None
        if not isinstance(case_id, str) or not re.fullmatch(r"[0-9a-fA-F-]{36}", case_id):
            case_id = None
        add(Event("create", None, None, None, None, "PASS" if status == 201 and case_id else "FAIL", status, summary))
        if status != 201 or not case_id:
            interrupted = interrupted or "case_create_failed"

    if case_id and not interrupted:
        status, _, summary = request("GET", f"/api/cases/{case_id}/editor/shell")
        add(Event("shell", None, None, "editor_shell", None, "PASS" if status == 200 else "FAIL", status, summary))
        if status != 200:
            interrupted = interrupted or "editor_shell_missing"

    chain_before: int | None = None
    if case_id and not interrupted and args.verify_global_audit_chain:
        status, value, summary = request("GET", "/api/audit-logs/verify-integrity")
        if status == 200 and isinstance(value, dict):
            chain_before = value.get("broken_rows", value.get("brokenRows"))
        else:
            add(Event(
                "audit_chain_before", None, None, None, None,
                "INCONCLUSIVE" if status is None else "AUDIT_MISMATCH", status, summary,
            ))

    def audit_logs(owner: str | None = None, row_id: str | None = None,
                   field_path: str = "", nested_identity: dict[str, str] | None = None) -> list[dict[str, Any]]:
        nested_target = next(
            (target for prefix, target in NESTED_AUDIT_TABLES.items() if field_path.startswith(prefix)),
            None,
        )
        if nested_target:
            nested_table, identity_key = nested_target
            nested_id = (nested_identity or {}).get(identity_key)
            if not nested_id:
                add(Event("nested_audit", None, None, owner, field_path,
                          "INCONCLUSIVE", None,
                          {"reason": "exact nested audit identity unavailable",
                           "table": nested_table, "identity_key": identity_key}))
                return []
            status, value, _ = request(
                "GET", f"/api/audit-logs/by-record/{nested_table}/{nested_id}"
            )
            if status != 200 or not isinstance(value, list):
                add(Event("nested_audit", None, None, owner, field_path,
                          "INCONCLUSIVE", status,
                          {"reason": "nested audit response unavailable or malformed",
                           "table": nested_table, "record_id": nested_id}))
                return []
            return value
        table = AUDIT_TABLES.get(owner or "")
        target = f"{table}/{row_id}" if table and row_id else f"cases/{case_id}"
        audit_field = audit_field_key(field_path) if field_path else ""
        # Exact-delta checks need the complete record stream so unexpected
        # same-record audit entries cannot be hidden by a field filter.
        field_query = ""
        status, value, _ = request("GET", f"/api/audit-logs/by-record/{target}{field_query}")
        if status != 200:
            add(Event("record_audit", None, None, owner, field_path,
                      "INCONCLUSIVE", status,
                      {"reason": "record audit response unavailable", "target": target}))
            return []
        if isinstance(value, list):
            return value
        logs = value.get("items", value.get("data", value)) if isinstance(value, dict) else []
        if not isinstance(logs, list):
            add(Event("record_audit", None, None, owner, field_path,
                      "INCONCLUSIVE", status,
                      {"reason": "record audit response malformed", "target": target}))
            return []
        return logs

    def exact_audit_delta(
        before_logs: list[dict[str, Any]], after_logs: list[dict[str, Any]], *,
        owner: str, row_id: str, action: str,
        expected_changes: dict[str, tuple[Any, Any]], field_path: str = "",
        nested_identity: dict[str, str] | None = None,
        exact_changed_keys: bool = True,
    ) -> tuple[bool, dict[str, Any]]:
        nested_target = next(
            (target for prefix, target in NESTED_AUDIT_TABLES.items()
             if field_path.startswith(prefix)), None,
        )
        if nested_target:
            table, identity_key = nested_target
            target_id = (nested_identity or {}).get(identity_key)
        else:
            table = AUDIT_TABLES.get(owner)
            target_id = case_id if owner == "case" else row_id
        if not table or not target_id or not audit_user_id or not audit_organization_id:
            return False, {"reason": "exact audit target/context unavailable", "owner": owner,
                           "table": table, "recordId": target_id}
        return audit_delta_exact(
            before_logs, after_logs, table=table, record_id=target_id,
            action=action, user_id=audit_user_id, organization_id=audit_organization_id,
            expected_changes=expected_changes, exact_changed_keys=exact_changed_keys,
        )

    def exact_create_audit(
        logs: list[dict[str, Any]], *, owner: str, row_id: str,
        stored: dict[str, Any], payload: dict[str, Any], field_path: str = "",
        nested_identity: dict[str, str] | None = None,
    ) -> tuple[bool, dict[str, Any]]:
        if owner not in {"medicalHistoryEpisodes", "parentMedicalHistory"}:
            return exact_audit_delta(
                [], logs, owner=owner, row_id=row_id, action="CREATE",
                expected_changes=expected_audit_changes({}, stored, payload),
                field_path=field_path, nested_identity=nested_identity,
                exact_changed_keys=False,
            )
        creates = [log for log in logs if str(log.get("action", "")).upper() == "CREATE"]
        updates = [log for log in logs if str(log.get("action", "")).upper() == "UPDATE"]
        if len(logs) != 2 or len(creates) != 1 or len(updates) != 1:
            return False, {"reason": "expected source-defined CREATE then UPDATE audit sequence",
                           "actions": [log.get("action") for log in logs]}
        create_id = creates[0].get("id")
        update_id = updates[0].get("id")
        create_new = creates[0].get("new_values", creates[0].get("newValues"))
        update_old = updates[0].get("old_values", updates[0].get("oldValues"))
        if (not isinstance(create_id, int) or isinstance(create_id, bool)
                or not isinstance(update_id, int) or isinstance(update_id, bool)
                or create_id >= update_id or create_new != update_old):
            return False, {
                "reason": "CREATE/UPDATE audit sequence is not chronological or continuous",
                "createId": create_id, "updateId": update_id,
                "intermediateSnapshotExact": create_new == update_old,
            }
        create_payload = {
            key: value for key, value in payload.items()
            if snake(AUDIT_FIELD_ALIASES.get(key, key)) in {
                "meddra_code", "start_date_null_flavor", "continuing_null_flavor",
                "end_date_null_flavor",
            }
        }
        update_payload = {
            key: value for key, value in payload.items()
            if snake(AUDIT_FIELD_ALIASES.get(key, key)) in {"comments", "meddra_version"}
        }
        create_ok, create_detail = exact_audit_delta(
            [], creates, owner=owner, row_id=row_id, action="CREATE",
            expected_changes=expected_audit_changes({}, stored, create_payload),
            field_path=field_path, nested_identity=nested_identity,
            exact_changed_keys=False,
        )
        update_ok, update_detail = exact_audit_delta(
            creates, [*creates, *updates], owner=owner, row_id=row_id, action="UPDATE",
            expected_changes=expected_audit_changes({}, stored, update_payload),
            field_path=field_path, nested_identity=nested_identity,
        )
        return create_ok and update_ok, {
            "sourceDefinedCreateThenUpdate": True,
            "create": create_detail, "update": update_detail,
        }

    def read_current(page: str, owner: str, row_route: bool, row_id: str | None) -> tuple[int | None, Any]:
        route = f"/api/cases/{case_id}/editor/pages/{page}"
        if row_route:
            status, value, _ = request("GET", f"{route}/rows/{row_id or ''}")
            current = value.get("data", value) if isinstance(value, dict) else value
            current = current.get(owner, current) if isinstance(current, dict) else current
        else:
            status, value, _ = request("GET", route)
            rows = value.get("rows", {}) if isinstance(value, dict) else {}
            current = rows.get(owner) if isinstance(rows, dict) else None
            if isinstance(current, list):
                current = next(
                    (
                        row for row in current
                        if row_id and isinstance(row, dict) and row.get("id") == row_id
                    ),
                    None,
                )
        return status, current

    def readback(
        page: str,
        owner: str,
        payload_path: str,
        row_route: bool,
        row_id: str | None,
        nested_identity: dict[str, str] | None = None,
    ) -> tuple[int | None, Any, bool]:
        status, current = read_current(page, owner, row_route, row_id)
        root = nested_root(payload_path)
        if status == 200 and root and nested_identity:
            nested = row_with_identity(get_path(current, root), nested_identity)
            if nested is None:
                return None, None, False
            remainder = payload_path.removeprefix(root).lstrip(".")
            return status, get_path(nested, remainder), remainder in nested
        leaf = leaf_path(payload_path)
        return status, get_path(current, leaf), isinstance(current, dict) and leaf in current

    if args.audit_all_fields and case_id and not interrupted and selected_re_codes:
        re_descriptors = {
            record["code"]: record["verificationDescriptor"]
            for record in (audit_inventory or {}).get("fields", [])
            if record.get("page") == "RE" and isinstance(record.get("verificationDescriptor"), dict)
        }
        re_fields = {
            "RE.receiver": "receiver", "RE.reportDueDate": "reportDueDate",
            "RE.reportedDate": "reportedDate",
        }
        re_descriptors = {key: value for key, value in re_descriptors.items()
                          if key in selected_re_codes}
        re_fields = {key: value for key, value in re_fields.items()
                     if key in selected_re_codes}
        if set(re_descriptors) != selected_re_codes or set(re_fields) != selected_re_codes:
            interrupted = interrupted or "review_receiver_inventory_incomplete"
            add(Event("audit_field_inventory", None, "RE", "reviewReceivers", None,
                      "INCONCLUSIVE", None, {"expected": sorted(re_fields),
                                             "actual": sorted(re_descriptors)}))
        else:
            row = {"id": f"audit-{uuid.uuid4().hex[:12]}"}
            for code, key in re_fields.items():
                row[key] = copy.deepcopy(re_descriptors[code]["alternateA"])

            def case_snapshot() -> tuple[int | None, dict[str, Any]]:
                status, value, _ = request("GET", f"/api/cases/{case_id}")
                return status, value if isinstance(value, dict) else {}

            def review_raw(rows: list[dict[str, Any]]) -> str:
                return json.dumps(rows, separators=(",", ":"))

            before_setup_status, before_setup_case = case_snapshot()
            before_setup_raw = before_setup_case.get("review_receivers_json")
            before_setup_logs = audit_logs("case", case_id)
            expected_setup_raw = review_raw([row])
            setup_status, _, _ = request(
                "PUT", f"/api/cases/{case_id}",
                {"data": {"review_receivers_json": expected_setup_raw},
                 "reason_for_change": "Initialize audit-all review receiver"},
            )
            current_status, current_case = case_snapshot()
            current_raw = current_case.get("review_receivers_json")
            current_logs = audit_logs("case", case_id)
            setup_audit_ok, setup_audit_detail = audit_delta_exact(
                before_setup_logs, current_logs, table="cases", record_id=case_id,
                action="UPDATE", user_id=audit_user_id or "",
                organization_id=audit_organization_id or "",
                expected_changes={"review_receivers_json":
                                  (before_setup_raw, expected_setup_raw)},
            )
            before_setup_without = copy.deepcopy(before_setup_case)
            before_setup_without.pop("review_receivers_json", None)
            current_without = copy.deepcopy(current_case)
            current_without.pop("review_receivers_json", None)
            setup_preserved = (owner_business(before_setup_without)
                               == owner_business(current_without))
            setup_ok = (before_setup_status == setup_status == current_status == 200
                        and current_raw == expected_setup_raw and setup_audit_ok
                        and setup_preserved)
            add(Event("audit_field_setup", None, "RE", "reviewReceivers", "create",
                      "PASS" if setup_ok else "AUDIT_MISMATCH", setup_status,
                      {"readback_exact": current_raw == expected_setup_raw,
                       "audit_exact_delta": setup_audit_ok,
                       "audit_detail": setup_audit_detail,
                       "whole_case_unrelated_preserved": setup_preserved}))
            if not setup_ok:
                interrupted = interrupted or "review_receiver_setup_failed"
            else:
                for code, key in re_fields.items():
                    descriptor = re_descriptors[code]
                    for phase, value in (
                        ("changed", descriptor["alternateB"]),
                        ("clear", ""), ("restore", descriptor["alternateA"]),
                    ):
                        before_case = copy.deepcopy(current_case)
                        before_raw = current_raw
                        before_value = copy.deepcopy(row[key])
                        row[key] = copy.deepcopy(value)
                        expected_raw = review_raw([row])
                        status, _, summary = request(
                            "PUT", f"/api/cases/{case_id}",
                            {"data": {"review_receivers_json": expected_raw},
                             "reason_for_change": f"Audit-all {code} {phase}"},
                        )
                        after_status, after_case = case_snapshot()
                        after_raw = after_case.get("review_receivers_json")
                        after_logs = audit_logs("case", case_id)
                        if status == 200:
                            audit_ok, audit_detail = audit_delta_exact(
                                current_logs, after_logs, table="cases", record_id=case_id,
                                action="UPDATE", user_id=audit_user_id or "",
                                organization_id=audit_organization_id or "",
                                expected_changes={"review_receivers_json":
                                                  (before_raw, expected_raw)},
                            )
                        else:
                            audit_ok, audit_detail = audit_no_delta_exact(current_logs, after_logs)
                        before_case_without = copy.deepcopy(before_case)
                        before_case_without.pop("review_receivers_json", None)
                        after_case_without = copy.deepcopy(after_case)
                        after_case_without.pop("review_receivers_json", None)
                        preserved = (owner_business(before_case_without)
                                     == owner_business(after_case_without))
                        accepted = status == after_status == 200
                        rejected_clear = (phase == "clear" and status in {400, 409, 422}
                                          and after_status == 200 and after_raw == before_raw
                                          and owner_business(after_case)
                                          == owner_business(before_case))
                        readback_exact = (after_raw == expected_raw if accepted
                                          else after_raw == before_raw)
                        ok = (accepted or rejected_clear) and readback_exact and audit_ok and preserved
                        add(Event("mutation", code, "RE", "reviewReceivers", phase,
                                  ("CLEAR_REJECTED" if ok and rejected_clear else
                                   "SAVE_ACCEPTED" if ok else "AUDIT_MISMATCH"), status,
                                  {**summary, "audit_exact_delta": audit_ok,
                                   "audit_detail": audit_detail,
                                   "readback_exact": readback_exact,
                                   "whole_case_unrelated_preserved": preserved}))
                        if rejected_clear:
                            row[key] = before_value
                        current_case, current_raw, current_logs = after_case, after_raw, after_logs
                    noop_status, _, noop_summary = request(
                        "PUT", f"/api/cases/{case_id}",
                        {"data": {"review_receivers_json": review_raw([row])}},
                    )
                    noop_case_status, noop_case = case_snapshot()
                    noop_logs = audit_logs("case", case_id)
                    noop_ok = (noop_status == noop_case_status == 200
                               and owner_business(noop_case) == owner_business(current_case)
                               and audit_no_delta_exact(current_logs, noop_logs)[0])
                    add(Event("mutation", code, "RE", "reviewReceivers", "noop",
                              "NOOP_ACCEPTED" if noop_ok else "AUDIT_MISMATCH", noop_status,
                              {**noop_summary, "audit_no_delta_exact":
                               audit_no_delta_exact(current_logs, noop_logs)[0]}))

    for page in pages:
        if interrupted or not case_id:
            break
        if page == "RE":
            continue
        fields = page_fields(page)
        groups: dict[str, list[dict[str, Any]]] = {}
        for field in fields:
            groups.setdefault(field["patch"]["owner"], []).append(field)
        setup_groups: dict[str, list[dict[str, Any]]] = {}
        for field in contract_rows(contract, page, args.audit_all_fields):
            setup_groups.setdefault(field["patch"]["owner"], []).append(field)
        all_setup_groups: dict[str, list[dict[str, Any]]] = {}
        page_contract = next(item for item in contract if item["pageId"] == page)
        for field in page_contract["fields"]:
            patch = field.get("patch")
            if isinstance(patch, dict) and isinstance(patch.get("owner"), str):
                all_setup_groups.setdefault(patch["owner"], []).append(field)
        if args.lifecycle_only and args.lifecycle_owner == "sourceDocuments":
            setup_groups["sourceDocuments"] = list(all_setup_groups.get("sourceDocuments", []))
        if not fields:
            add(Event("page", None, page, None, None, "SKIPPED", None, {"reason": "no_verified_contract_fields"}))
            continue
        route = f"/api/cases/{case_id}/editor/pages/{page}"
        if page == "DG" and reaction_id is None:
            reaction_status, reaction_projection, reaction_summary = request(
                "GET", f"/api/cases/{case_id}/editor/pages/AE"
            )
            reaction_id = extract_row_id(reaction_projection, "rows")
            ready = reaction_status == 200 and bool(reaction_id)
            add(Event("dependency_create", None, page, "reaction", None, "PASS" if ready else "BASELINE_REJECTED", reaction_status, reaction_summary))
            if not ready:
                interrupted = interrupted or "intake_reaction_missing"
                continue
        row_route = page in ROW_PAGES
        row_ids: dict[str, str | None] = {}
        owner_ready: dict[str, bool] = {}
        owner_items = list(setup_groups.items())
        if page == "SI":
            owner_items.sort(key=lambda item: 0 if item[0] == "studyInformation" else 1)
        if page == "DM":
            owner_items.sort(key=lambda item: {"patientInformation": 0, "parentInfo": 1}.get(item[0], 2))
        for owner, owner_fields in owner_items:
            if page == "CI" and owner == "safetyReportIdentification":
                status, projection, summary = request("GET", route)
                row_ids[owner] = extract_row_id(projection, owner) if status == 200 else None
                owner_ready[owner] = status == 200 and bool(row_ids[owner])
                if owner_ready[owner] and args.complete_baseline:
                    baseline = baseline_for(
                        [field for field in setup_groups[owner] if field.get("code") != "C.1.1"],
                        minimal=False,
                        safe=True,
                    )
                    baseline["id"] = row_ids[owner]
                    status, _, summary = request(
                        "PATCH",
                        route,
                        {"authorities": authorities_for(owner_fields), "rows": {owner: baseline}},
                    )
                    owner_ready[owner] = status == 200
                add(Event("baseline", None, page, owner, None, "PASS" if owner_ready[owner] else "BASELINE_REJECTED", status, {**summary, "reason": "reuse case-created safety report row"}))
                continue
            baseline_fields = [field for field in setup_groups.get(owner, owner_fields) if field.get("code") != "C.1.1"]
            baseline = baseline_for(
                baseline_fields,
                minimal=not args.complete_baseline,
                safe=args.complete_baseline,
            )
            if owner == "sourceDocuments":
                baseline.update({
                    "sourceDocumentName": "case-editor-fuzzer.txt",
                    "sourceDocumentBase64": "Y2FzZS1lZGl0b3ItZnV6emVyCg==",
                    "sourceDocumentMediaType": "text/plain",
                })
            if page == "AE" and owner == "reaction":
                # AE create contract requires an explicit positive sequence.
                baseline.setdefault("sequenceNumber", 1)
                baseline.setdefault("reactionMeddraVersionLLT", args.meddra_version)
                baseline.setdefault("reactionMeddraCodeLLT", args.meddra_code)
            if page == "DG" and owner == "drug":
                baseline.setdefault("drugCharacterization", "1")
                baseline.setdefault("medicinalProduct", "Fuzz product")
                if reaction_id:
                    set_path(baseline, "drugReactionAssessments[].reactionId", reaction_id)
            if row_route:
                if page == "AE" and owner == "reaction":
                    read_status, projection, _ = request("GET", route)
                    existing_id = extract_row_id(projection, "rows") if read_status == 200 else None
                    if not existing_id:
                        owner_ready[owner] = False
                        add(Event("row_create", None, page, owner, None, "BASELINE_REJECTED", read_status, {"reason": "intake reaction missing"}))
                        interrupted = interrupted or "intake_reaction_missing"
                        break
                    status, _, summary = request("PATCH", f"{route}/rows/{existing_id}", {"authorities": authorities_for(owner_fields), "rows": {owner: baseline}})
                    row_ids[owner] = existing_id
                    owner_ready[owner] = status == 200
                    reaction_id = existing_id
                else:
                    status, value, summary = request("POST", f"{route}/rows", {"authorities": authorities_for(owner_fields), "rows": {owner: baseline}})
                    row_ids[owner] = created_row_id(value)
                    owner_ready[owner] = status == 201 and bool(row_ids[owner])
                add(Event("row_create", None, page, owner, None, "PASS" if owner_ready[owner] else "BASELINE_REJECTED", status, summary))
            else:
                array_owner = any(str(field["payloadPath"]).startswith("[]") for field in owner_fields)
                status, _, summary = request("PATCH", route, {"authorities": authorities_for(owner_fields), "rows": owner_patch(owner, baseline, array_owner)})
                owner_ready[owner] = status == 200
                add(Event("baseline", None, page, owner, None, "PASS" if status == 200 else "BASELINE_REJECTED", status, summary))
                status, projection, _ = request("GET", route)
                if status == 200:
                    row_ids[owner] = extract_row_id(projection, owner)
            if interrupted:
                break

        if args.lifecycle_only:
            gaps = lifecycle_preflight_gaps()
            if gaps:
                add(Event("lifecycle", None, page, None, "preflight", "FAIL", None,
                          {"structural_gaps": gaps}))
                interrupted = interrupted or "lifecycle_preflight_failed"
                continue
            if page == "DG" and args.nested_lifecycle_owner:
                owner = "drug"
                drug_id = row_ids.get(owner)
                nested_owner = args.nested_lifecycle_owner
                device_code_specs = {
                    "fdaDeviceFollowUpTypes": ("fdaDevices[].followUpTypes", "follow_up_type", "1"),
                    "fdaDeviceProblemCodes": ("fdaDevices[].deviceProblemCodes", "device_problem", "1234567"),
                    "fdaDeviceRemedialActions": ("fdaDevices[].remedialActions", "remedial_action", "2"),
                }
                root = ("drugReactionAssessments[].sourceOfAssessment"
                        if nested_owner == "relatednessAssessments"
                        else device_code_specs.get(nested_owner, (f"{nested_owner}[]", "", ""))[0])
                edit_audit_path = {
                    "fdaDevices": "fdaDevices[].manufacturerName",
                    "dosageInformation": "dosageInformation[].dosageText",
                    "relatednessAssessments": "drugReactionAssessments[].sourceOfAssessment",
                }.get(nested_owner, root)
                owner_fields = setup_groups.get(owner, [])
                root_fields = [
                    field for field in owner_fields
                    if (leaf_path(field["payloadPath"]).startswith("drugReactionAssessments[]")
                        if nested_owner == "relatednessAssessments"
                        else leaf_path(field["payloadPath"]).startswith(root))
                ]
                endpoint_name = DG_NESTED_LIFECYCLE_ENDPOINTS[nested_owner]
                endpoint = f"/api/cases/{case_id}/drugs/{drug_id}/{endpoint_name}"
                if (not owner_ready.get(owner) or not drug_id
                        or (not root_fields and nested_owner not in device_code_specs)):
                    add(Event("lifecycle", None, page, nested_owner, "admission", "FAIL", None,
                              {"reason": "owned drug or nested source contract unavailable"}))
                    interrupted = interrupted or "nested_lifecycle_admission_failed"
                    continue

                relatedness_parent_id: str | None = None
                if nested_owner == "relatednessAssessments":
                    status, created, summary = request(
                        "POST", f"/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments",
                        {"data": {"drug_id": drug_id, "reaction_id": reaction_id}},
                    )
                    relatedness_parent_id = created.get("id") if isinstance(created, dict) else None
                    if status != 201 or not relatedness_parent_id:
                        add(Event("lifecycle", None, page, nested_owner, "parent_prerequisite",
                                  "FAIL", status, summary))
                        interrupted = interrupted or "relatedness_parent_prerequisite_failed"
                        continue
                    endpoint = (
                        f"/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/"
                        f"{relatedness_parent_id}/relatedness"
                    )
                device_parent_id: str | None = None
                if nested_owner in device_code_specs:
                    status, created, summary = request(
                        "POST", f"/api/cases/{case_id}/drugs/{drug_id}/devices",
                        {"data": {"drug_id": drug_id, "sequence_number": 100,
                                  "device_brand_name": f"Fuzz {nested_owner}"}},
                    )
                    device_parent_id = created.get("id") if isinstance(created, dict) else None
                    if status != 201 or not device_parent_id:
                        add(Event("lifecycle", None, page, nested_owner, "parent_prerequisite",
                                  "FAIL", status, summary))
                        interrupted = interrupted or "device_code_parent_prerequisite_failed"
                        continue
                    endpoint = f"/api/cases/{case_id}/drugs/{drug_id}/devices/{device_parent_id}/codes"
                target_parent_id = relatedness_parent_id or device_parent_id

                reaction_ids = [reaction_id] if reaction_id else []
                if nested_owner == "drugReactionAssessments":
                    ae_fields = [field for field in contract_rows(contract, "AE", args.audit_all_fields)
                                 if field.get("patch", {}).get("owner") == "reaction"]
                    for sequence in (2, 3, 4):
                        reaction_payload = baseline_for(ae_fields, minimal=True, safe=True)
                        reaction_payload.update({
                            "sequenceNumber": sequence,
                            "reactionMeddraVersionLLT": args.meddra_version,
                            "reactionMeddraCodeLLT": args.meddra_code,
                        })
                        status, created, create_summary = request(
                            "POST", f"/api/cases/{case_id}/editor/pages/AE/rows",
                            {"authorities": ["ich"], "rows": {"reaction": reaction_payload}},
                        )
                        created_id = created_row_id(created)
                        if status != 201 or not created_id:
                            add(Event("lifecycle", None, page, nested_owner, "reaction_prerequisite",
                                      "FAIL", status, create_summary))
                            interrupted = interrupted or "nested_reaction_prerequisite_failed"
                            break
                        reaction_ids.append(created_id)
                    if interrupted:
                        continue
                    if len(reaction_ids) != 4 or len(set(reaction_ids)) != 4:
                        add(Event("lifecycle", None, page, nested_owner, "reaction_prerequisite",
                                  "FAIL", None, {"reaction_ids": reaction_ids}))
                        interrupted = interrupted or "nested_reaction_identity_failed"
                        continue

                before_detail_status, before_detail = read_current(page, owner, True, drug_id)
                list_status, before_rows, _ = request("GET", endpoint)
                if before_detail_status != 200 or list_status != 200 or not isinstance(before_rows, list):
                    add(Event("lifecycle", None, page, nested_owner, "admission", "FAIL", list_status,
                              {"reason": "nested list/detail admission unavailable",
                               "detail_status": before_detail_status,
                               "list_type": type(before_rows).__name__}))
                    interrupted = interrupted or "nested_lifecycle_list_admission_failed"
                    continue
                before_ids = [row.get("id") for row in before_rows if isinstance(row, dict)]
                if len(before_ids) != len(before_rows) or len(set(before_ids)) != len(before_ids):
                    add(Event("lifecycle", None, page, nested_owner, "admission", "FAIL", list_status,
                              {"reason": "nested list identities missing or duplicated",
                               "before_ids": before_ids, "row_count": len(before_rows)}))
                    interrupted = interrupted or "nested_lifecycle_identity_admission_failed"
                    continue
                before_business = {row["id"]: owner_business(row) for row in before_rows}
                sequences = [row.get("sequence_number") for row in before_rows if isinstance(row, dict)]
                max_sequence = max((value for value in sequences if isinstance(value, int)), default=0)
                nested_template = baseline_for(root_fields, minimal=True, safe=True)
                template_rows = nested_template.get(
                    "drugReactionAssessments" if nested_owner == "relatednessAssessments" else nested_owner
                )
                template = snake_keys(template_rows[0]) if isinstance(template_rows, list) and template_rows else {}
                required_edit_path = {
                    "dosageInformation": "dosage_text",
                    "fdaDevices": "manufacturer_name",
                    "relatednessAssessments": "source_of_assessment",
                }.get(nested_owner)
                if required_edit_path:
                    full_template = baseline_for(root_fields, minimal=False, safe=True)
                    full_rows = full_template.get(
                        "drugReactionAssessments" if nested_owner == "relatednessAssessments" else nested_owner
                    )
                    full_row = snake_keys(full_rows[0]) if isinstance(full_rows, list) and full_rows else {}
                    if required_edit_path in full_row:
                        template[required_edit_path] = full_row[required_edit_path]
                required_seed = DG_NESTED_REQUIRED_EDIT_SEEDS.get(nested_owner)
                if required_seed:
                    seed_key, seed_value = required_seed
                    template[seed_key] = seed_value
                if nested_owner in device_code_specs:
                    _, element, value_code = device_code_specs[nested_owner]
                    template = {"element": element, "value_code": value_code}
                for key in ("id", "deleted", "created_at", "updated_at", "created_by", "updated_by"):
                    template.pop(key, None)
                template = {key: value for key, value in template.items()
                            if key in DG_NESTED_CREATE_FIELDS[nested_owner]}
                if (required_edit_path
                        and (not isinstance(template.get(required_edit_path), str)
                             or not template[required_edit_path].strip())):
                    add(Event("lifecycle", None, page, nested_owner, "edit_preflight", "FAIL", None,
                              {"reason": "required non-empty edit seed unavailable",
                               "required_field": required_edit_path}))
                    interrupted = interrupted or "nested_lifecycle_edit_seed_unavailable"
                    continue

                created_rows: list[dict[str, Any]] = []
                create_payloads: dict[str, dict[str, Any]] = {}
                for index in range(3):
                    payload = copy.deepcopy(template)
                    payload["drug_id"] = drug_id
                    if nested_owner == "relatednessAssessments":
                        payload.pop("drug_id", None)
                        payload["drug_reaction_assessment_id"] = relatedness_parent_id
                        payload["sequence_number"] = max_sequence + index + 1
                    elif nested_owner in device_code_specs:
                        payload.pop("drug_id", None)
                        payload["device_id"] = device_parent_id
                        payload["sequence_number"] = max_sequence + index + 1
                    elif nested_owner != "drugReactionAssessments":
                        payload["sequence_number"] = max_sequence + index + 1
                    else:
                        payload["reaction_id"] = reaction_ids[index]
                    for text_key in ("substance_name", "indication_text", "device_brand_name", "source_of_assessment"):
                        if isinstance(payload.get(text_key), str):
                            payload[text_key] = f"{payload[text_key]}-{index + 1}"
                    status, created, summary = request("POST", endpoint, {"data": payload})
                    if status != 201 or not isinstance(created, dict) or not created.get("id"):
                        add(Event("lifecycle", None, page, nested_owner, "create_three", "FAIL", status,
                                  {**summary, "created_count": len(created_rows), "payload": payload}))
                        interrupted = interrupted or "nested_lifecycle_create_failed"
                        break
                    created_rows.append(created)
                    create_payloads[created["id"]] = payload
                if interrupted:
                    continue
                created_ids = [row["id"] for row in created_rows]
                after_create_status, after_create_rows, _ = request("GET", endpoint)
                if after_create_status != 200 or not isinstance(after_create_rows, list):
                    add(Event("lifecycle", None, page, nested_owner, "create_three", "FAIL", after_create_status,
                              {"reason": "nested list unavailable after create"}))
                    interrupted = interrupted or "nested_lifecycle_create_read_failed"
                    continue
                after_create_ids = [row.get("id") for row in after_create_rows]
                after_create_business = {row["id"]: owner_business(row) for row in after_create_rows
                                         if isinstance(row.get("id"), str)}
                created_payloads_match = all(
                    row_id in after_create_business
                    and sent_fields_match(after_create_business[row_id], payload, {"drug_id"})
                    for row_id, payload in create_payloads.items()
                )
                created_audit_results = []
                for created_id, payload in create_payloads.items():
                    identity = ({"drugReactionAssessmentId": created_id}
                                if nested_owner == "drugReactionAssessments"
                                else {"id": created_id})
                    logs = audit_logs(owner, created_id, root, identity)
                    audit_ok, audit_detail = exact_create_audit(
                        logs, owner=owner, row_id=created_id,
                        stored=after_create_business.get(created_id, {}), payload=payload,
                        field_path=root, nested_identity=identity,
                    )
                    created_audit_results.append((audit_ok, audit_detail))
                expected_created_ids = sorted([*before_ids, *created_ids])
                ordered_endpoint = nested_owner == "drugReactionAssessments"
                order_ok = (
                    after_create_ids == expected_created_ids if ordered_endpoint
                    else set(after_create_ids) == set(expected_created_ids)
                )
                if (before_detail_status != 200 or list_status != 200 or after_create_status != 200
                        or not order_ok or len(set(after_create_ids)) != len(after_create_ids)
                        or not created_payloads_match
                        or not all(ok for ok, _ in created_audit_results)
                        or any(after_create_business.get(row_id) != before_business[row_id]
                               for row_id in before_ids)):
                    add(Event("lifecycle", None, page, nested_owner, "create_three", "FAIL", after_create_status,
                              {"before_ids": before_ids, "created_ids": created_ids,
                               "actual_ids": after_create_ids,
                               "expected_ids": expected_created_ids,
                               "ordering_contract": "id_ascending" if ordered_endpoint else "identity_set",
                               "preexisting_business_unchanged": all(
                                   after_create_business.get(row_id) == before_business[row_id]
                                   for row_id in before_ids),
                               "create_audits": [detail for _, detail in created_audit_results]}))
                    interrupted = interrupted or "nested_lifecycle_create_mismatch"
                    continue

                middle_id = created_ids[1]
                edit_payload = {key: value for key, value in create_payloads[middle_id].items()
                                if key not in {"drug_id", "drug_reaction_assessment_id",
                                               "fda_device_information_id", "device_id",
                                               "sequence_number", "reaction_id"}}
                edited = False
                for text_key in ("substance_name", "indication_text", "device_brand_name",
                                 "manufacturer_name", "dosage_text", "source_of_assessment", "value_code"):
                    if isinstance(edit_payload.get(text_key), str):
                        edit_payload[text_key] = f"{edit_payload[text_key]}-EDIT"
                        edited = True
                        break
                if nested_owner == "drugReactionAssessments" and not edited:
                    edit_payload["expectedness"] = "1"
                    edited = True
                if nested_owner in device_code_specs:
                    edit_payload["value_code"] = {
                        "fdaDeviceFollowUpTypes": "2",
                        "fdaDeviceProblemCodes": "1234",
                        "fdaDeviceRemedialActions": "1",
                    }[nested_owner]
                    edited = True
                edit_audit_before = audit_logs(
                    owner, middle_id, edit_audit_path,
                    {"drugReactionAssessmentId": middle_id}
                    if nested_owner == "drugReactionAssessments" else {"id": middle_id},
                )
                edit_status, _, edit_summary = request("PUT", f"{endpoint}/{middle_id}", {"data": edit_payload})
                edit_read_status, edit_rows, _ = request("GET", endpoint)
                edit_rows = edit_rows if isinstance(edit_rows, list) else []
                edit_business = {row["id"]: owner_business(row) for row in edit_rows
                                 if isinstance(row.get("id"), str)}
                edit_audit_after = audit_logs(
                    owner, middle_id, edit_audit_path,
                    {"drugReactionAssessmentId": middle_id}
                    if nested_owner == "drugReactionAssessments" else {"id": middle_id},
                )
                edit_audit_exact, edit_audit_detail = exact_audit_delta(
                    edit_audit_before, edit_audit_after, owner=owner, row_id=middle_id,
                    action="UPDATE", expected_changes=expected_audit_changes(
                        after_create_business.get(middle_id, {}),
                        edit_business.get(middle_id, {}), edit_payload,
                    ), field_path=edit_audit_path,
                    nested_identity=({"drugReactionAssessmentId": middle_id}
                                     if nested_owner == "drugReactionAssessments"
                                     else {"id": middle_id}),
                )
                edit_business_changed = (
                    after_create_business.get(middle_id) != edit_business.get(middle_id)
                )
                edit_ok = (
                    edited and edit_business_changed and edit_status == 200 and edit_read_status == 200
                    and set(edit_business) == set(after_create_business)
                    and sent_fields_match(edit_business.get(middle_id, {}), edit_payload)
                    and all(edit_business[row_id] == after_create_business[row_id]
                            for row_id in after_create_business if row_id != middle_id)
                    and edit_audit_exact
                )
                if not edit_ok:
                    add(Event("lifecycle", None, page, nested_owner, "middle_edit", "FAIL", edit_status,
                              {**edit_summary, "ids_unchanged": set(edit_business) == set(after_create_business),
                               "payload_matches": sent_fields_match(edit_business.get(middle_id, {}), edit_payload),
                               "business_changed": edit_business_changed,
                               "audit_exact_delta": edit_audit_exact,
                               "audit_detail": edit_audit_detail}))
                    interrupted = interrupted or "nested_lifecycle_edit_failed"
                    continue
                after_create_business[middle_id] = edit_business[middle_id]
                middle_identity = ({"drugReactionAssessmentId": middle_id}
                                   if nested_owner == "drugReactionAssessments" else {"id": middle_id})
                middle_audit_before = audit_logs(owner, middle_id, root, middle_identity)
                delete_status, _, delete_summary = request("DELETE", f"{endpoint}/{middle_id}")
                deleted_status, deleted_rows, _ = request("GET", endpoint)
                if deleted_status != 200 or not isinstance(deleted_rows, list):
                    add(Event("lifecycle", None, page, nested_owner, "middle_delete", "FAIL", deleted_status,
                              {**delete_summary, "reason": "nested list unavailable after delete"}))
                    interrupted = interrupted or "nested_lifecycle_delete_read_failed"
                    continue
                deleted_ids = [row.get("id") for row in deleted_rows]
                expected_deleted_ids = sorted(row_id for row_id in after_create_ids if row_id != middle_id)
                deleted_business = {row["id"]: owner_business(row) for row in deleted_rows
                                    if isinstance(row.get("id"), str)}
                deleted_id_ok = (
                    deleted_ids == expected_deleted_ids if ordered_endpoint
                    else set(deleted_ids) == set(expected_deleted_ids)
                )
                target_survivors_unchanged = all(
                    deleted_business.get(row_id) == after_create_business[row_id]
                    for row_id in expected_deleted_ids
                )
                middle_audit_after = audit_logs(owner, middle_id, root, middle_identity)
                hard_delete = nested_owner == "drugReactionAssessments"
                delete_audit_exact, delete_audit_detail = exact_audit_delta(
                    middle_audit_before, middle_audit_after, owner=owner, row_id=middle_id,
                    action="DELETE" if hard_delete else "UPDATE",
                    expected_changes=(deleted_audit_changes(after_create_business[middle_id])
                                      if hard_delete else {"deleted": (False, True)}),
                    field_path=root, nested_identity=middle_identity,
                    exact_changed_keys=not hard_delete,
                )
                after_delete_detail_status, after_delete_detail = read_current(page, owner, True, drug_id)
                delete_ok = (
                    delete_status in {200, 204} and deleted_status == 200
                    and deleted_id_ok and target_survivors_unchanged
                    and delete_audit_exact
                    and after_delete_detail_status == 200
                    and without_nested_owner(before_detail, nested_owner, target_parent_id)
                    == without_nested_owner(after_delete_detail, nested_owner, target_parent_id)
                )
                if not delete_ok:
                    add(Event("lifecycle", None, page, nested_owner, "middle_delete", "FAIL", delete_status,
                              {**delete_summary, "expected_ids": expected_deleted_ids,
                               "actual_ids": deleted_ids,
                               "target_survivors_unchanged": target_survivors_unchanged,
                               "sibling_business_unchanged": without_nested_owner(before_detail, nested_owner, target_parent_id)
                               == without_nested_owner(after_delete_detail, nested_owner, target_parent_id),
                               "audit_exact_delta": delete_audit_exact,
                               "audit_detail": delete_audit_detail}))
                    interrupted = interrupted or "nested_lifecycle_delete_failed"
                    continue

                fresh_payload = copy.deepcopy(template)
                fresh_payload["drug_id"] = drug_id
                if nested_owner == "relatednessAssessments":
                    fresh_payload.pop("drug_id", None)
                    fresh_payload["drug_reaction_assessment_id"] = relatedness_parent_id
                    fresh_payload["sequence_number"] = max_sequence + 4
                elif nested_owner in device_code_specs:
                    fresh_payload.pop("drug_id", None)
                    fresh_payload["device_id"] = device_parent_id
                    fresh_payload["sequence_number"] = max_sequence + 4
                elif nested_owner != "drugReactionAssessments":
                    fresh_payload["sequence_number"] = max_sequence + 4
                else:
                    fresh_payload["reaction_id"] = reaction_ids[3]
                for text_key in ("substance_name", "indication_text", "device_brand_name", "source_of_assessment"):
                    if isinstance(fresh_payload.get(text_key), str):
                        fresh_payload[text_key] = f"{fresh_payload[text_key]}-4"
                readd_status, fresh_row, readd_summary = request("POST", endpoint, {"data": fresh_payload})
                final_status, final_rows, _ = request("GET", endpoint)
                if final_status != 200 or not isinstance(final_rows, list):
                    add(Event("lifecycle", None, page, nested_owner, "fresh_readd", "FAIL", final_status,
                              {**readd_summary, "reason": "nested list unavailable after readd"}))
                    interrupted = interrupted or "nested_lifecycle_readd_read_failed"
                    continue
                fresh_id = fresh_row.get("id") if isinstance(fresh_row, dict) else None
                final_ids = [row.get("id") for row in final_rows]
                expected_final_ids = sorted([*expected_deleted_ids, fresh_id]) if fresh_id else []
                final_id_ok = (
                    final_ids == expected_final_ids if ordered_endpoint
                    else set(final_ids) == set(expected_final_ids)
                )
                final_detail_status, final_detail = read_current(page, owner, True, drug_id)
                fresh_business = owner_business(fresh_row) if isinstance(fresh_row, dict) else None
                stored_fresh = next((owner_business(row) for row in final_rows if row.get("id") == fresh_id), None)
                final_business = {row["id"]: owner_business(row) for row in final_rows
                                  if isinstance(row.get("id"), str)}
                target_survivors_unchanged = all(
                    final_business.get(row_id) == deleted_business[row_id]
                    for row_id in expected_deleted_ids
                )
                fresh_payload_matches = isinstance(stored_fresh, dict) and sent_fields_match(
                    stored_fresh, fresh_payload, {"drug_id"}
                )
                fresh_identity = ({"drugReactionAssessmentId": fresh_id}
                                  if nested_owner == "drugReactionAssessments" else {"id": fresh_id})
                fresh_audit_logs = audit_logs(owner, fresh_id, root, fresh_identity)
                fresh_audit_exact, fresh_audit_detail = exact_create_audit(
                    fresh_audit_logs, owner=owner, row_id=fresh_id,
                    stored=stored_fresh or {}, payload=fresh_payload,
                    field_path=root, nested_identity=fresh_identity,
                )
                readd_ok = (
                    readd_status == 201 and final_status == 200 and final_detail_status == 200
                    and fresh_id not in after_create_ids and final_id_ok
                    and stored_fresh == fresh_business and target_survivors_unchanged
                    and fresh_payload_matches
                    and fresh_audit_exact
                    and without_nested_owner(before_detail, nested_owner, target_parent_id)
                    == without_nested_owner(final_detail, nested_owner, target_parent_id)
                )
                if not readd_ok:
                    add(Event("lifecycle", None, page, nested_owner, "fresh_readd", "FAIL", readd_status,
                              {**readd_summary, "expected_ids": expected_final_ids, "actual_ids": final_ids,
                               "fresh_id": fresh_id, "fresh_row": fresh_row, "stored_fresh": stored_fresh,
                               "ordering_contract": "id_ascending" if ordered_endpoint else "identity_set",
                               "created_payloads_match": created_payloads_match,
                               "target_survivors_unchanged": target_survivors_unchanged,
                               "fresh_payload_matches": fresh_payload_matches,
                               "fresh_audit_exact": fresh_audit_exact,
                               "fresh_audit_detail": fresh_audit_detail,
                               "sibling_business_unchanged": without_nested_owner(before_detail, nested_owner, target_parent_id)
                               == without_nested_owner(final_detail, nested_owner, target_parent_id)}))
                    interrupted = interrupted or "nested_lifecycle_readd_failed"
                    continue

                update_payload = {key: value for key, value in fresh_payload.items()
                                  if key not in {"drug_id", "drug_reaction_assessment_id",
                                                 "fda_device_information_id", "device_id",
                                                 "sequence_number", "reaction_id"}}
                fresh_identity = ({"drugReactionAssessmentId": fresh_id}
                                  if nested_owner == "drugReactionAssessments" else {"id": fresh_id})
                noop_audit_before = audit_logs(owner, fresh_id, root, fresh_identity)
                noop_status, _, noop_summary = request("PUT", f"{endpoint}/{fresh_id}", {"data": update_payload})
                noop_row_status, noop_row, _ = request("GET", f"{endpoint}/{fresh_id}")
                noop_detail_status, noop_detail = read_current(page, owner, True, drug_id)
                noop_audit_after = audit_logs(owner, fresh_id, root, fresh_identity)
                noop_ok = (
                    noop_status == 200 and noop_row_status == 200 and noop_detail_status == 200
                    and owner_business(noop_row) == fresh_business
                    and owner_business(noop_detail) == owner_business(final_detail)
                    and audit_no_delta_exact(noop_audit_before, noop_audit_after)[0]
                )
                add(Event("lifecycle", None, page, nested_owner,
                          "three_middle_delete_fresh_readd_noop", "PASS" if noop_ok else "FAIL", noop_status,
                          {**noop_summary, "created_ids": created_ids, "deleted_id": middle_id,
                           "fresh_id": fresh_id, "final_ids": final_ids,
                           "created_payloads_match": created_payloads_match,
                           "fresh_payload_matches": fresh_payload_matches,
                           "ordering_contract": "id_ascending" if ordered_endpoint else "identity_set",
                           "noop_business_unchanged": owner_business(noop_row) == fresh_business,
                           "noop_page_unchanged": owner_business(noop_detail) == owner_business(final_detail),
                           "noop_audit_unchanged": audit_no_delta_exact(noop_audit_before, noop_audit_after)[0],
                           "raw_metadata_changed": noop_row != fresh_row or noop_detail != final_detail}))
                if not noop_ok:
                    interrupted = interrupted or "nested_lifecycle_noop_failed"
                continue
            if not row_route:
                lifecycle_owner = args.lifecycle_owner or DIRECT_LIFECYCLE_OWNERS.get(page)
                owner_fields = setup_groups.get(lifecycle_owner or "", [])
                lifecycle_parent_id: str | None = None
                if lifecycle_owner == "fdaCrossReportedIndNumbers":
                    owner_fields = []
                    for source_field in all_setup_groups.get("studyInformation", []):
                        prefix = "fdaCrossReportedIndNumbers"
                        if not str(source_field.get("payloadPath", "")).startswith(f"{prefix}[]"):
                            continue
                        field = copy.deepcopy(source_field)
                        field["payloadPath"] = str(field["payloadPath"]).removeprefix(prefix)
                        if isinstance(field.get("projectionPath"), str):
                            projection_suffix = field["projectionPath"].split(f"{prefix}[]", 1)
                            if len(projection_suffix) != 2:
                                raise ValueError("FDA IND projection path does not contain its collection root")
                            field["projectionPath"] = f"[]{projection_suffix[1]}"
                        owner_fields.append(field)
                    lifecycle_parent_id = row_ids.get("studyInformation")
                    seed = baseline_for(owner_fields, minimal=False, safe=True)
                    seed_rows = [seed] if seed else []
                    if lifecycle_parent_id and isinstance(seed_rows, list) and seed_rows:
                        seed_status, _, _ = request(
                            "PATCH", route,
                            {"authorities": authorities_for(owner_fields),
                             "rows": direct_lifecycle_patch(
                                 lifecycle_owner, [seed_rows[0]], lifecycle_parent_id
                             )},
                        )
                        if seed_status == 200:
                            _, seeded_projection, _ = request("GET", route)
                            seeded_rows = direct_lifecycle_rows(seeded_projection, lifecycle_owner)
                            row_ids[lifecycle_owner] = (
                                seeded_rows[0].get("id") if seeded_rows else None
                            )
                initial_id = row_ids.get(lifecycle_owner or "")
                if not lifecycle_owner or not owner_fields or not initial_id:
                    add(Event("lifecycle", None, page, lifecycle_owner, None, "SKIPPED", None,
                              {"reason": "no source-confirmed direct-array lifecycle owner"}))
                    continue
                before_status, before_projection, _ = request("GET", route)
                before_rows = direct_lifecycle_rows(before_projection, lifecycle_owner)
                before_ids = [row.get("id") for row in before_rows]
                sequence_values = [get_path(row, "sequenceNumber") for row in before_rows]
                if before_rows and not all(isinstance(value, int) for value in sequence_values):
                    add(Event("lifecycle", None, page, lifecycle_owner, "sequence_admission",
                              "FAIL", before_status,
                              {"reason": "existing row lacks exact sequenceNumber",
                               "sequence_values": sequence_values}))
                    interrupted = interrupted or "lifecycle_sequence_missing"
                    continue
                max_sequence = max(
                    (int(get_path(row, "sequenceNumber")) for row in before_rows
                     if isinstance(get_path(row, "sequenceNumber"), int)),
                    default=0,
                )
                baseline = baseline_for(owner_fields, minimal=True, safe=True)
                if lifecycle_owner == "sourceDocuments":
                    baseline.update({
                        "sourceDocumentName": "case-editor-fuzzer.txt",
                        "sourceDocumentBase64": "Y2FzZS1lZGl0b3ItZnV6emVyCg==",
                        "sourceDocumentMediaType": "text/plain",
                    })
                sender_diagnosis_edit: tuple[str, str] | None = None
                if lifecycle_owner == "senderDiagnoses":
                    term_status, term_body, term_summary = request(
                        "GET", "/api/terminology/meddra?q=headache&version=28.1&language=en&level=llt&limit=2"
                    )
                    term_rows = (
                        term_body.get("data", term_body.get("items"))
                        if isinstance(term_body, dict) else term_body
                    )
                    valid_terms = [row for row in term_rows if isinstance(row, dict)
                                   and isinstance(row.get("code"), str)] if isinstance(term_rows, list) else []
                    if term_status != 200 or len(valid_terms) < 2:
                        add(Event("lifecycle", None, page, lifecycle_owner, "terminology_preflight",
                                  "FAIL", term_status, term_summary))
                        interrupted = interrupted or "sender_diagnosis_terminology_unavailable"
                        continue
                    baseline.update({"diagnosisMeddraVersion": "28.1",
                                     "diagnosisMeddraCode": valid_terms[0]["code"]})
                    sender_diagnosis_edit = ("28.1", valid_terms[1]["code"])
                create_b = lifecycle_payload(baseline, owner_fields, lifecycle_owner, "B")
                create_c = lifecycle_payload(baseline, owner_fields, lifecycle_owner, "C")
                create_b["sequenceNumber"] = max_sequence + 1
                create_c["sequenceNumber"] = max_sequence + 2
                create_status, _, create_summary = request(
                    "PATCH", route,
                    {"authorities": authorities_for(owner_fields),
                     "rows": direct_lifecycle_patch(
                         lifecycle_owner, [create_b, create_c], lifecycle_parent_id
                     )},
                )
                created_status, created_projection, _ = request("GET", route)
                created_rows = direct_lifecycle_rows(created_projection, lifecycle_owner)
                created_ids = [row.get("id") for row in created_rows]
                new_ids = [row_id for row_id in created_ids if row_id not in before_ids]
                if (before_status != 200 or create_status != 200 or created_status != 200
                        or len(new_ids) != 2 or len(set(created_ids)) != len(created_ids)
                        or created_ids != [*before_ids, *new_ids]):
                    add(Event("lifecycle", None, page, lifecycle_owner, "create_three", "FAIL", create_status,
                              {**create_summary, "before_ids": before_ids, "created_ids": created_ids,
                               "new_ids": new_ids}))
                    interrupted = interrupted or "lifecycle_create_failed"
                    continue
                lifecycle_ids = [initial_id, *new_ids]
                all_before_details = {row["id"]: owner_business(row) for row in created_rows
                                      if row.get("id")}
                create_payload_by_id = dict(zip(new_ids, (create_b, create_c)))
                create_audit_results = []
                for created_id, payload in create_payload_by_id.items():
                    logs = audit_logs(lifecycle_owner, created_id)
                    audit_ok, audit_detail = exact_create_audit(
                        logs, owner=lifecycle_owner, row_id=created_id,
                        stored=all_before_details.get(created_id, {}), payload=payload,
                    )
                    create_audit_results.append((audit_ok, audit_detail))
                if not all(ok for ok, _ in create_audit_results):
                    add(Event("lifecycle", None, page, lifecycle_owner, "create_three_audit",
                              "FAIL", create_status,
                              {"create_audits": [detail for _, detail in create_audit_results]}))
                    interrupted = interrupted or "lifecycle_create_audit_failed"
                    continue
                before_details = {key: all_before_details[key] for key in lifecycle_ids}
                middle_id = lifecycle_ids[1]
                edit_payload = lifecycle_payload(baseline, owner_fields, lifecycle_owner, "EDIT")
                if sender_diagnosis_edit:
                    edit_payload.update({"diagnosisMeddraVersion": sender_diagnosis_edit[0],
                                         "diagnosisMeddraCode": sender_diagnosis_edit[1]})
                edit_payload.update({"id": middle_id, "sequenceNumber": max_sequence + 1})
                edit_audit_before = audit_logs(lifecycle_owner, middle_id)
                edit_status, _, edit_summary = request(
                    "PATCH", route,
                    {"authorities": authorities_for(owner_fields),
                     "rows": direct_lifecycle_patch(
                         lifecycle_owner, [edit_payload], lifecycle_parent_id
                     )},
                )
                edit_read_status, edit_projection, _ = request("GET", route)
                edit_rows = direct_lifecycle_rows(edit_projection, lifecycle_owner)
                edit_ids = [row.get("id") for row in edit_rows]
                edited_row = next((row for row in edit_rows if row.get("id") == middle_id), None)
                edit_audit_after = audit_logs(lifecycle_owner, middle_id)
                edit_audit_exact, edit_audit_detail = exact_audit_delta(
                    edit_audit_before, edit_audit_after, owner=lifecycle_owner,
                    row_id=middle_id, action="UPDATE",
                    expected_changes=expected_audit_changes(
                        all_before_details[middle_id], owner_business(edited_row or {}), edit_payload,
                    ),
                )
                edit_business_changed = (
                    all_before_details[middle_id] != owner_business(edited_row or {})
                )
                edit_ok = (
                    edit_business_changed and edit_status == 200 and edit_read_status == 200
                    and edit_ids == created_ids
                    and edited_row is not None
                    and payload_fields_match(edited_row, edit_payload, owner_fields, lifecycle_owner)
                    and all(owner_business(row) == all_before_details[row["id"]]
                            for row in edit_rows if row.get("id") != middle_id)
                    and edit_audit_exact
                )
                if not edit_ok:
                    add(Event("lifecycle", None, page, lifecycle_owner, "middle_edit", "FAIL", edit_status,
                              {**edit_summary, "ids_unchanged": edit_ids == created_ids,
                               "business_changed": edit_business_changed,
                               "payload_matches": edited_row is not None and payload_fields_match(
                                   edited_row, edit_payload, owner_fields, lifecycle_owner),
                               "audit_exact_delta": edit_audit_exact,
                               "audit_detail": edit_audit_detail}))
                    interrupted = interrupted or "lifecycle_edit_failed"
                    continue
                all_before_details[middle_id] = owner_business(edited_row)
                before_details[middle_id] = owner_business(edited_row)
                middle_audit_before = audit_logs(lifecycle_owner, middle_id)
                delete_status, _, delete_summary = request(
                    "PATCH", route,
                    {"authorities": authorities_for(owner_fields),
                     "rows": direct_lifecycle_patch(
                         lifecycle_owner, [{"id": middle_id, "deleted": True}], lifecycle_parent_id
                     )},
                )
                deleted_status, deleted_projection, _ = request("GET", route)
                deleted_rows = direct_lifecycle_rows(deleted_projection, lifecycle_owner)
                deleted_ids_all = [row.get("id") for row in deleted_rows]
                survivor_ids = [row.get("id") for row in deleted_rows if row.get("id") in lifecycle_ids]
                all_survivor_details = {row["id"]: owner_business(row) for row in deleted_rows
                                        if row.get("id")}
                survivor_details = {key: all_survivor_details[key] for key in survivor_ids}
                middle_audit_after = audit_logs(lifecycle_owner, middle_id)
                hard_delete = lifecycle_owner in HARD_DELETE_AUDIT_OWNERS
                delete_audit_exact, delete_audit_detail = exact_audit_delta(
                    middle_audit_before, middle_audit_after, owner=lifecycle_owner,
                    row_id=middle_id, action="DELETE" if hard_delete else "UPDATE",
                    expected_changes=(deleted_audit_changes(all_before_details[middle_id])
                                      if hard_delete else {"deleted": (False, True)}),
                    exact_changed_keys=not hard_delete,
                )
                if (delete_status != 200 or deleted_status != 200
                        or survivor_ids != [lifecycle_ids[0], lifecycle_ids[2]]
                        or deleted_ids_all != [row_id for row_id in created_ids if row_id != middle_id]
                        or all_survivor_details != {key: all_before_details[key] for key in deleted_ids_all}
                        or not delete_audit_exact):
                    add(Event("lifecycle", None, page, lifecycle_owner, "middle_delete", "FAIL", delete_status,
                              {**delete_summary, "initial_ids": lifecycle_ids,
                               "actual_survivor_ids": survivor_ids,
                               "audit_exact_delta": delete_audit_exact,
                               "audit_detail": delete_audit_detail}))
                    interrupted = interrupted or "lifecycle_delete_failed"
                    continue
                fresh_payload = lifecycle_payload(baseline, owner_fields, lifecycle_owner, "D")
                fresh_payload["sequenceNumber"] = max_sequence + 3
                readd_status, _, readd_summary = request(
                    "PATCH", route,
                    {"authorities": authorities_for(owner_fields),
                     "rows": direct_lifecycle_patch(
                         lifecycle_owner, [fresh_payload], lifecycle_parent_id
                     )},
                )
                final_status, final_projection, _ = request("GET", route)
                final_rows = direct_lifecycle_rows(final_projection, lifecycle_owner)
                final_ids_all = [row.get("id") for row in final_rows]
                fresh_ids = [row_id for row_id in final_ids_all if row_id not in created_ids]
                fresh_id = fresh_ids[0] if len(fresh_ids) == 1 else None
                final_ids = [lifecycle_ids[0], lifecycle_ids[2], fresh_id] if fresh_id else []
                final_target_details = {row["id"]: owner_business(row) for row in final_rows
                                        if row.get("id") in final_ids}
                expected_final_all = [*deleted_ids_all, fresh_id] if fresh_id else []
                identity_ok = lifecycle_identity_check(
                    lifecycle_ids, survivor_ids, final_ids, middle_id, fresh_id,
                    before_details, survivor_details,
                )
                cross_owner_unchanged = (
                    page_without_owner(before_projection, lifecycle_owner)
                    == page_without_owner(final_projection, lifecycle_owner)
                )
                fresh_row = next((row for row in final_rows if row.get("id") == fresh_id), None)
                fresh_audit_logs = audit_logs(lifecycle_owner, fresh_id)
                fresh_audit_exact, fresh_audit_detail = exact_create_audit(
                    fresh_audit_logs, owner=lifecycle_owner, row_id=fresh_id,
                    stored=owner_business(fresh_row or {}), payload=fresh_payload,
                )
                if (readd_status != 200 or final_status != 200 or not identity_ok
                        or final_ids_all != expected_final_all or not cross_owner_unchanged
                        or fresh_row is None
                        or not fresh_audit_exact
                        or not payload_fields_match(fresh_row, fresh_payload, owner_fields, lifecycle_owner)
                        or final_target_details.get(lifecycle_ids[0]) != before_details[lifecycle_ids[0]]
                        or final_target_details.get(lifecycle_ids[2]) != before_details[lifecycle_ids[2]]):
                    add(Event("lifecycle", None, page, lifecycle_owner, "fresh_readd", "FAIL", readd_status,
                              {**readd_summary, "old_ids": lifecycle_ids, "fresh_ids": fresh_ids,
                               "actual_final_ids": final_ids_all,
                               "expected_final_ids": expected_final_all,
                               "readd_status_ok": readd_status == 200,
                               "final_status_ok": final_status == 200,
                               "identity_ok": identity_ok,
                               "final_ids_exact": final_ids_all == expected_final_all,
                               "cross_owner_unchanged": cross_owner_unchanged,
                               "fresh_row_present": fresh_row is not None,
                               "fresh_audit_exact": fresh_audit_exact,
                               "fresh_audit_detail": fresh_audit_detail,
                               "fresh_payload_matches": fresh_row is not None and payload_fields_match(fresh_row, fresh_payload, owner_fields, lifecycle_owner),
                               "survivor_first_unchanged": final_target_details.get(lifecycle_ids[0]) == before_details[lifecycle_ids[0]],
                               "survivor_last_unchanged": final_target_details.get(lifecycle_ids[2]) == before_details[lifecycle_ids[2]],
                               "fresh_payload": fresh_payload, "fresh_row": fresh_row,
                               "before_target_business": before_details,
                               "final_target_business": final_target_details,
                               "page_before": before_projection, "page_after": final_projection}))
                    interrupted = interrupted or "lifecycle_readd_failed"
                    continue
                noop_audit_before = audit_logs(lifecycle_owner, fresh_id)
                noop_status, _, noop_summary = request(
                    "PATCH", route,
                    {"authorities": authorities_for(owner_fields),
                     "rows": direct_lifecycle_patch(
                         lifecycle_owner, [{**fresh_payload, "id": fresh_id}], lifecycle_parent_id
                     )},
                )
                noop_read_status, noop_projection, _ = request("GET", route)
                noop_rows = direct_lifecycle_rows(noop_projection, lifecycle_owner)
                noop_row = next((row for row in noop_rows
                                 if row.get("id") == fresh_id), None)
                noop_audit_after = audit_logs(lifecycle_owner, fresh_id)
                noop_ok = (noop_status == 200 and noop_read_status == 200 and noop_row is not None
                           and owner_business(noop_row) == owner_business(fresh_row)
                           and direct_page_business_equal(
                               noop_projection, final_projection, lifecycle_owner,
                           )
                           and audit_no_delta_exact(noop_audit_before, noop_audit_after)[0])
                add(Event("lifecycle", None, page, lifecycle_owner,
                          "three_middle_delete_fresh_readd_noop",
                          "PASS" if noop_ok else "FAIL", noop_status,
                          {**noop_summary, "initial_ids": lifecycle_ids, "deleted_id": middle_id,
                           "fresh_id": fresh_id, "final_ids": final_ids_all,
                           "noop_business_unchanged": noop_row is not None and owner_business(noop_row) == owner_business(fresh_row),
                           "noop_audit_unchanged": audit_no_delta_exact(noop_audit_before, noop_audit_after)[0],
                           "raw_metadata_changed": noop_row != fresh_row or noop_projection != final_projection,
                           **({} if noop_ok else {
                               "noop_status_ok": noop_status == 200,
                               "noop_read_status_ok": noop_read_status == 200,
                               "noop_row_present": noop_row is not None,
                               "noop_business_row_exact": owner_business(noop_row) == owner_business(fresh_row),
                               "noop_business_page_exact": direct_page_business_equal(
                                   noop_projection, final_projection, lifecycle_owner,
                               ),
                               "row_before": fresh_row, "row_after": noop_row,
                               "page_before": final_projection, "page_after": noop_projection,
                               "audit_before": noop_audit_before, "audit_after": noop_audit_after,
                           })}))
                if not noop_ok:
                    interrupted = interrupted or "lifecycle_noop_failed"
                continue
            for owner, owner_fields in owner_items:
                first_id = row_ids.get(owner)
                if not owner_ready.get(owner) or not first_id:
                    add(Event("lifecycle", None, page, owner, None, "FAIL", None,
                              {"reason": "initial owned row unavailable"}))
                    interrupted = interrupted or "lifecycle_setup_failed"
                    break
                lifecycle_row = baseline_for(owner_fields, minimal=True, safe=True)
                if page == "AE":
                    lifecycle_row.update({
                        "sequenceNumber": 2,
                        "reactionMeddraVersionLLT": args.meddra_version,
                        "reactionMeddraCodeLLT": args.meddra_code,
                    })
                if page == "DG":
                    lifecycle_row.setdefault("drugCharacterization", "1")
                    lifecycle_row.setdefault("medicinalProduct", "Fuzz lifecycle product")
                    if reaction_id:
                        set_path(lifecycle_row, "drugReactionAssessments[].reactionId", reaction_id)
                initial_page_status, initial_projection, _ = request("GET", route)
                initial_rows = projection_rows(initial_projection, owner)
                initial_live_ids = [row.get("id") for row in initial_rows]
                sequence_values = [get_path(row, "sequenceNumber") for row in initial_rows]
                if initial_rows and not all(isinstance(value, int) for value in sequence_values):
                    add(Event("lifecycle", None, page, owner, "sequence_admission",
                              "FAIL", initial_page_status,
                              {"reason": "existing row lacks exact sequenceNumber",
                               "sequence_values": sequence_values}))
                    interrupted = interrupted or "lifecycle_sequence_missing"
                    break
                max_sequence = max(
                    (int(get_path(row, "sequenceNumber")) for row in initial_rows
                     if isinstance(get_path(row, "sequenceNumber"), int)),
                    default=0,
                )
                initial_row_business = {row["id"]: owner_business(row) for row in initial_rows if row.get("id")}
                if initial_page_status != 200 or first_id not in initial_live_ids:
                    add(Event("lifecycle", None, page, owner, "initial_snapshot", "FAIL", initial_page_status,
                              {"first_id": first_id, "initial_live_ids": initial_live_ids}))
                    interrupted = interrupted or "lifecycle_initial_snapshot_failed"
                    break
                create_results: list[tuple[int | None, str | None]] = []
                for sequence in (max_sequence + 1, max_sequence + 2):
                    payload = lifecycle_payload(lifecycle_row, owner_fields, owner, str(sequence))
                    payload["sequenceNumber"] = sequence
                    create_status, created, _ = request(
                        "POST", f"{route}/rows",
                        {"authorities": authorities_for(owner_fields), "rows": {owner: payload}},
                    )
                    create_results.append((create_status, created_row_id(created)))
                if any(status != 201 or not created_id for status, created_id in create_results):
                    add(Event("lifecycle", None, page, owner, "create_three", "FAIL", None,
                              {"create_results": create_results}))
                    interrupted = interrupted or "lifecycle_create_failed"
                    break
                expected_ids = [first_id, create_results[0][1], create_results[1][1]]
                list_status, projection, _ = request("GET", route)
                rows = projection_rows(projection, owner)
                live_ids = [row.get("id") for row in rows]
                all_before_details = {}
                for row_id in live_ids:
                    detail_status, detail = read_current(page, owner, True, row_id)
                    if detail_status == 200:
                        all_before_details[row_id] = owner_business(detail)
                before_details = {key: all_before_details[key] for key in expected_ids
                                  if key in all_before_details}
                created_payloads = [
                    lifecycle_payload(lifecycle_row, owner_fields, owner, str(max_sequence + offset))
                    for offset in (1, 2)
                ]
                for offset, payload in zip((1, 2), created_payloads):
                    payload["sequenceNumber"] = max_sequence + offset
                create_audit_results = []
                for (_, created_id), payload in zip(create_results, created_payloads):
                    logs = audit_logs(owner, created_id)
                    audit_ok, audit_detail = exact_create_audit(
                        logs, owner=owner, row_id=created_id,
                        stored=all_before_details.get(created_id, {}), payload=payload,
                    )
                    create_audit_results.append((audit_ok, audit_detail))
                expected_all_ids = [*initial_live_ids, create_results[0][1], create_results[1][1]]
                if (list_status != 200 or live_ids != expected_all_ids
                        or len(all_before_details) != len(live_ids) or len(before_details) != 3
                        or not all(ok for ok, _ in create_audit_results)):
                    add(Event("lifecycle", None, page, owner, "create_three", "FAIL", list_status,
                              {"expected_ids": expected_all_ids, "actual_ids": live_ids,
                               "details_found": sorted(all_before_details),
                               "create_audits": [detail for _, detail in create_audit_results]}))
                    interrupted = interrupted or "lifecycle_create_mismatch"
                    break
                middle_id = expected_ids[1]
                edit_payload = lifecycle_payload(lifecycle_row, owner_fields, owner, "EDIT")
                edit_payload.update({"id": middle_id, "sequenceNumber": max_sequence + 1})
                edit_audit_before = audit_logs(owner, middle_id)
                edit_status, _, edit_summary = request(
                    "PATCH", f"{route}/rows/{middle_id}",
                    {"authorities": authorities_for(owner_fields), "rows": {owner: edit_payload}},
                )
                edit_read_status, edited_detail = read_current(page, owner, True, middle_id)
                edit_page_status, edit_projection, _ = request("GET", route)
                edit_ids = [row.get("id") for row in projection_rows(edit_projection, owner)]
                edit_audit_after = audit_logs(owner, middle_id)
                edit_audit_exact, edit_audit_detail = exact_audit_delta(
                    edit_audit_before, edit_audit_after, owner=owner, row_id=middle_id,
                    action="UPDATE", expected_changes=expected_audit_changes(
                        before_details[middle_id], owner_business(edited_detail), edit_payload,
                    ),
                )
                edit_ok = (
                    edit_status == 200 and edit_read_status == 200 and edit_page_status == 200
                    and edit_ids == expected_all_ids
                    and payload_fields_match(edited_detail, edit_payload, owner_fields, owner)
                    and edit_audit_exact
                )
                if not edit_ok:
                    add(Event("lifecycle", None, page, owner, "middle_edit", "FAIL", edit_status,
                              {**edit_summary, "ids_unchanged": edit_ids == expected_all_ids,
                               "payload_matches": edit_read_status == 200 and payload_fields_match(
                                   edited_detail, edit_payload, owner_fields, owner),
                               "audit_exact_delta": edit_audit_exact,
                               "audit_detail": edit_audit_detail}))
                    interrupted = interrupted or "lifecycle_edit_failed"
                    break
                all_before_details[middle_id] = owner_business(edited_detail)
                before_details[middle_id] = owner_business(edited_detail)
                middle_audit_before = audit_logs(owner, middle_id)
                delete_status, _, delete_summary = request("DELETE", f"{route}/rows/{middle_id}")
                after_delete_status, after_delete_projection, _ = request("GET", route)
                survivor_ids = [row.get("id") for row in projection_rows(after_delete_projection, owner)]
                expected_survivor_all = [row_id for row_id in expected_all_ids if row_id != middle_id]
                all_survivor_details = {}
                for row_id in survivor_ids:
                    detail_status, detail = read_current(page, owner, True, row_id)
                    if detail_status == 200:
                        all_survivor_details[row_id] = owner_business(detail)
                survivor_details = {key: all_survivor_details[key]
                                    for key in (expected_ids[0], expected_ids[2])
                                    if key in all_survivor_details}
                middle_audit_after = audit_logs(owner, middle_id)
                delete_audit_exact, delete_audit_detail = exact_audit_delta(
                    middle_audit_before, middle_audit_after, owner=owner, row_id=middle_id,
                    action="UPDATE", expected_changes={"deleted": (False, True)},
                )
                delete_ok = (
                    delete_status in {200, 204} and after_delete_status == 200
                    and survivor_ids == expected_survivor_all
                    and all_survivor_details == {key: all_before_details[key] for key in survivor_ids}
                    and delete_audit_exact
                )
                if not delete_ok:
                    add(Event("lifecycle", None, page, owner, "middle_delete", "FAIL", delete_status,
                              {**delete_summary, "expected_survivor_ids": expected_survivor_all,
                               "actual_survivor_ids": survivor_ids,
                               "survivor_business_unchanged": survivor_details == {key: before_details[key] for key in survivor_ids if key in before_details},
                               "audit_exact_delta": delete_audit_exact,
                               "audit_detail": delete_audit_detail}))
                    interrupted = interrupted or "lifecycle_delete_failed"
                    break
                readd_payload = lifecycle_payload(lifecycle_row, owner_fields, owner, "4")
                readd_payload["sequenceNumber"] = max_sequence + 3
                readd_status, readded, readd_summary = request(
                    "POST", f"{route}/rows",
                    {"authorities": authorities_for(owner_fields), "rows": {owner: readd_payload}},
                )
                fresh_id = created_row_id(readded)
                final_status, final_projection, _ = request("GET", route)
                final_rows = projection_rows(final_projection, owner)
                final_ids = [row.get("id") for row in final_rows]
                final_existing_business = {row["id"]: owner_business(row) for row in final_rows
                                           if row.get("id") in initial_live_ids}
                expected_final_all = [*expected_survivor_all, fresh_id] if fresh_id else []
                final_details = {}
                for row_id in (expected_ids[0], expected_ids[2], fresh_id):
                    if not row_id:
                        continue
                    detail_status, detail = read_current(page, owner, True, row_id)
                    if detail_status == 200:
                        final_details[row_id] = owner_business(detail)
                target_final_ids = [expected_ids[0], expected_ids[2], fresh_id] if fresh_id else []
                identity_ok = lifecycle_identity_check(
                    expected_ids, [expected_ids[0], expected_ids[2]], target_final_ids, middle_id, fresh_id,
                    before_details, survivor_details,
                )
                fresh_audit_logs = audit_logs(owner, fresh_id) if fresh_id else []
                fresh_audit_exact, fresh_audit_detail = exact_create_audit(
                    fresh_audit_logs, owner=owner, row_id=fresh_id or "",
                    stored=final_details.get(fresh_id, {}) if fresh_id else {},
                    payload=readd_payload,
                )
                if (readd_status != 201 or not fresh_id or fresh_id in expected_ids
                        or final_status != 200
                        or final_ids != expected_final_all or not identity_ok
                        or final_existing_business != initial_row_business
                        or final_details.get(expected_ids[0]) != before_details[expected_ids[0]]
                        or final_details.get(expected_ids[2]) != before_details[expected_ids[2]]
                        or fresh_id not in final_details
                        or not fresh_audit_exact
                        or not payload_fields_match(final_details[fresh_id], readd_payload, owner_fields, owner)):
                    add(Event("lifecycle", None, page, owner, "fresh_readd", "FAIL", readd_status,
                              {**readd_summary, "old_ids": expected_ids, "fresh_id": fresh_id,
                               "actual_final_ids": final_ids, "expected_final_ids": expected_final_all,
                               "readd_status_ok": readd_status == 201,
                               "fresh_id_present": bool(fresh_id),
                               "fresh_id_is_new": bool(fresh_id) and fresh_id not in expected_ids,
                               "final_status_ok": final_status == 200,
                               "final_ids_exact": final_ids == expected_final_all,
                               "identity_ok": identity_ok,
                               "fresh_audit_exact": fresh_audit_exact,
                               "fresh_audit_detail": fresh_audit_detail,
                               "preexisting_business_unchanged": final_existing_business == initial_row_business,
                               "survivor_first_unchanged": final_details.get(expected_ids[0]) == before_details[expected_ids[0]],
                               "survivor_last_unchanged": final_details.get(expected_ids[2]) == before_details[expected_ids[2]],
                               "fresh_detail_present": fresh_id in final_details,
                               "fresh_payload_matches": fresh_id in final_details and payload_fields_match(final_details[fresh_id], readd_payload, owner_fields, owner),
                               "fresh_payload": readd_payload, "fresh_detail": final_details.get(fresh_id),
                               "initial_row_business": initial_row_business,
                               "final_existing_business": final_existing_business,
                               "before_target_business": before_details,
                               "final_target_business": final_details,
                               "page_before": projection, "page_after": final_projection}))
                    interrupted = interrupted or "lifecycle_readd_failed"
                    break
                noop_payload = {**readd_payload, "id": fresh_id}
                noop_before_status, noop_before = read_current(page, owner, True, fresh_id)
                noop_audit_before = audit_logs(owner, fresh_id)
                noop_status, _, noop_summary = request(
                    "PATCH", f"{route}/rows/{fresh_id}",
                    {"authorities": authorities_for(owner_fields), "rows": {owner: noop_payload}},
                )
                noop_after_status, noop_after = read_current(page, owner, True, fresh_id)
                noop_page_status, noop_projection, _ = request("GET", route)
                noop_audit_after = audit_logs(owner, fresh_id)
                noop_ok = (
                    noop_status == 200 and noop_before_status == noop_after_status == 200
                    and owner_business(noop_before) == owner_business(noop_after)
                    and noop_page_status == 200
                    and owner_business(noop_projection) == owner_business(final_projection)
                    and audit_no_delta_exact(noop_audit_before, noop_audit_after)[0]
                )
                add(Event("lifecycle", None, page, owner, "three_middle_delete_fresh_readd_noop",
                          "PASS" if noop_ok else "FAIL", noop_status,
                          {**noop_summary, "initial_ids": expected_ids, "deleted_id": middle_id,
                           "survivor_ids": survivor_ids, "fresh_id": fresh_id,
                           "final_ids": final_ids, "noop_business_unchanged": owner_business(noop_before) == owner_business(noop_after),
                           "noop_audit_unchanged": audit_no_delta_exact(noop_audit_before, noop_audit_after)[0],
                           "raw_metadata_changed": noop_before != noop_after or noop_projection != final_projection,
                           **({} if noop_ok else {
                               "noop_status_ok": noop_status == 200,
                               "noop_before_status_ok": noop_before_status == 200,
                               "noop_after_status_ok": noop_after_status == 200,
                               "noop_business_row_exact": owner_business(noop_before) == owner_business(noop_after),
                               "noop_page_status_ok": noop_page_status == 200,
                               "noop_business_page_exact": owner_business(noop_projection) == owner_business(final_projection),
                               "row_before": noop_before, "row_after": noop_after,
                               "page_before": final_projection, "page_after": noop_projection,
                               "audit_before": noop_audit_before, "audit_after": noop_audit_after,
                           })}))
                if not noop_ok:
                    interrupted = interrupted or "lifecycle_noop_failed"
            continue

        nested_row_identities: dict[tuple[str, str], dict[str, str]] = {}
        for field in fields:
            if interrupted:
                break
            owner = field["patch"]["owner"]
            payload_path = field["payloadPath"]
            array_owner = str(payload_path).startswith("[]")
            if not owner_ready.get(owner, False):
                add(Event("mutation", field["code"], page, owner, None, "SKIPPED_BASELINE", None, {"reason": "owner baseline did not save"}))
                continue
            active_row_id = row_ids.get(owner)
            fixed_payload = field.get("_fixedPayload", {})
            if fixed_payload and not row_route:
                fixed_status, fixed_projection, _ = request("GET", route)
                fixed_rows = projection_rows(fixed_projection, owner)
                matches = [row for row in fixed_rows if all(
                    values_equal(value, get_path(row, path))
                    for path, value in fixed_payload.items()
                )]
                if not matches:
                    prepare_status, _, prepare_summary = request(
                        "PATCH", route,
                        {"authorities": authorities_for([field]),
                         "rows": owner_patch(owner, baseline_for([field]), True)},
                    )
                    fixed_status, fixed_projection, _ = request("GET", route)
                    fixed_rows = projection_rows(fixed_projection, owner)
                    matches = [row for row in fixed_rows if all(
                        values_equal(value, get_path(row, path))
                        for path, value in fixed_payload.items()
                    )]
                    if prepare_status != 200:
                        add(Event("baseline", field["code"], page, owner, "fixed_slot",
                                  "BASELINE_REJECTED", prepare_status, prepare_summary))
                if fixed_status != 200 or len(matches) != 1 or not matches[0].get("id"):
                    add(Event("mutation", field["code"], page, owner, None,
                              "SKIPPED_BASELINE", fixed_status,
                              {"reason": "exact fixed-slot row unavailable", "matches": len(matches)}))
                    continue
                active_row_id = matches[0]["id"]
            root = nested_root(payload_path)
            if root and (owner, root) not in nested_row_identities:
                root_fields = [
                    candidate
                    for candidate in setup_groups.get(owner, fields)
                    if nested_root(candidate["payloadPath"]) == root
                ]
                nested_baseline = baseline_for(
                    root_fields,
                    minimal=not args.complete_baseline,
                    safe=args.complete_baseline,
                )
                if page == "DG" and root.startswith("drugReactionAssessments[]") and reaction_id:
                    set_path(nested_baseline, "drugReactionAssessments[].reactionId", reaction_id)
                if active_row_id:
                    nested_baseline["id"] = active_row_id
                if row_route:
                    status, _, summary = request("PATCH", f"{route}/rows/{active_row_id or ''}", {"authorities": authorities_for(root_fields), "rows": {owner: nested_baseline}})
                else:
                    status, _, summary = request("PATCH", route, {"authorities": authorities_for(root_fields), "rows": owner_patch(owner, nested_baseline, array_owner)})
                read_status, current = read_current(page, owner, row_route, active_row_id)
                identity = object_identity(get_path(current, root)) if read_status == 200 else {}
                required_identity = (
                    {"id", "drugReactionAssessmentId", "reactionId"}
                    if page == "DG" and root.startswith("drugReactionAssessments[]")
                    else {"id"}
                )
                missing_identity = sorted(required_identity - identity.keys())
                ready = status == 200 and not missing_identity
                nested_row_identities[(owner, root)] = identity if ready else {}
                add(Event("nested_baseline", field["code"], page, owner, None, "PASS" if ready else "BASELINE_REJECTED", status, {**summary, **({"reason": f"nested projection missing {', '.join(missing_identity)}"} if missing_identity else {})}))
                if not ready:
                    continue
            if root and not nested_row_identities.get((owner, root)):
                add(Event("mutation", field["code"], page, owner, None, "SKIPPED_BASELINE", None, {"reason": "nested row baseline identity unavailable"}))
                continue
            if field.get("_mutationFixedPayload"):
                companion_payload: dict[str, Any] = {}
                for path, value in field["_mutationFixedPayload"].items():
                    set_path(companion_payload, path, copy.deepcopy(value))
                if root:
                    for key, value in nested_row_identities[(owner, root)].items():
                        set_path(companion_payload, f"{root}.{key}", value)
                if active_row_id:
                    companion_payload["id"] = active_row_id
                companion_status, companion_value, companion_summary = request(
                    "PATCH", f"{route}/rows/{active_row_id or ''}" if row_route else route,
                    {"authorities": authorities_for([field]),
                     "rows": ({owner: companion_payload} if row_route
                              else owner_patch(owner, companion_payload, array_owner))},
                )
                if companion_status != 200:
                    add(Event("baseline", field["code"], page, owner, "fixed_companion",
                              "BASELINE_REJECTED", companion_status, companion_summary))
                    continue
                if root:
                    refreshed = replacement_identity(
                        companion_value, owner, root,
                        set(nested_row_identities[(owner, root)]),
                    )
                    if refreshed:
                        nested_row_identities[(owner, root)] = refreshed
            candidates = (
                (ordinal, sample)
                for ordinal in range(candidate_count(field, args.values_per_field))
                for sample in range(candidate_sample_count(field, ordinal, args.samples_per_category))
            )
            before_status, before_actual, _ = readback(
                page,
                owner,
                projection_leaf(field, owner),
                row_route,
                active_row_id,
                nested_row_identities.get((owner, root)) if root else None,
            )
            before_owner_status, before_owner = read_current(
                page, owner, row_route, active_row_id
            )
            logs_before = audit_logs(
                owner, active_row_id, projection_leaf(field, owner),
                nested_row_identities.get((owner, root)) if root else None,
            )
            for ordinal, sample in candidates:
                if interrupted:
                    break
                before_nested_identity = copy.deepcopy(
                    nested_row_identities.get((owner, root)) if root else None
                )
                candidate = field_value(
                    field,
                    candidate_rng(args.seed, field, ordinal, sample),
                    ordinal,
                    sample,
                )
                mutation = copy.deepcopy(baseline_for([field]))
                for path, value in field.get("_mutationFixedPayload", {}).items():
                    set_path(mutation, path, copy.deepcopy(value))
                set_path(mutation, leaf_path(payload_path), candidate)
                nullflavor_with_value = add_nullflavor_partner(field, mutation, ordinal)
                if page == "DG" and "drugReactionAssessments[]" in payload_path and reaction_id:
                    set_path(mutation, "drugReactionAssessments[].reactionId", reaction_id)
                if root:
                    for key, value in nested_row_identities[(owner, root)].items():
                        set_path(mutation, f"{root}.{key}", value)
                if active_row_id:
                    mutation["id"] = active_row_id
                audit_path = projection_leaf(field, owner)
                if row_route:
                    status, value, summary = request(
                        "PATCH", f"{route}/rows/{active_row_id or ''}", {"authorities": authorities_for([field]), "rows": {owner: mutation}}
                    )
                else:
                    status, value, summary = request(
                        "PATCH", route, {"authorities": authorities_for([field]), "rows": owner_patch(owner, mutation, array_owner)}
                    )
                refreshed_identity: dict[str, str] = {}
                if status == 200 and root:
                    current_identity = nested_row_identities[(owner, root)]
                    refreshed_identity = replacement_identity(
                        value, owner, root, set(current_identity)
                    )
                    if refreshed_identity:
                        nested_row_identities[(owner, root)] = refreshed_identity
                identity_refreshed = bool(refreshed_identity)
                identity_replaced = bool(
                    root and before_nested_identity and refreshed_identity
                    and before_nested_identity != refreshed_identity
                )
                if status == 200:
                    classification = "SAVE_ACCEPTED"
                elif status == 422:
                    classification = "CONSTRAINT_REJECTED"
                elif status == 400:
                    classification = "BACKEND_REJECTED"
                elif status == 409:
                    classification = "CONFLICT_REJECTED"
                elif status in {401, 403, 404}:
                    classification = "UNEXPECTED_STATUS"
                elif status is None:
                    classification = "INCONCLUSIVE"
                else:
                    classification = "SERVER_ERROR" if status >= 500 else "UNEXPECTED_STATUS"
                invalid_nullflavor = nullflavor_invalid_candidate(field, candidate) or nullflavor_with_value
                expectation = candidate_expectation(field, ordinal, candidate)
                if is_nullflavor_field(field):
                    if invalid_nullflavor:
                        expectation = ("reject", field.get("constraint", {}).get("ruleCode"))
                    elif ordinal in {1, 2, 3}:
                        expectation = ("accept", None)
                detail: dict[str, Any] = {
                    "candidate": redacted(candidate),
                    "candidate_kind": candidate_kind(field, ordinal),
                    "candidate_ordinal": ordinal,
                    "sample_ordinal": sample,
                    "generation_fingerprint": candidate_fingerprint(field, ordinal, sample, candidate),
                    "rule_code": field.get("constraint", {}).get("ruleCode"),
                }
                if expectation:
                    detail.update({"expected_outcome": expectation[0], "expected_rule_code": expectation[1]})
                    if isinstance(field.get("_maxLength"), int):
                        detail["max_length"] = field["_maxLength"]
                if is_nullflavor_field(field):
                    detail["nullflavor_expected_reject"] = invalid_nullflavor
                    detail["nullflavor_value_conflict"] = nullflavor_with_value
                read_status = None
                actual = None
                logs_after = None
                if status == 200:
                    read_status, actual, readback_key_present = readback(
                        page,
                        owner,
                        projection_leaf(field, owner),
                        row_route,
                        active_row_id,
                        nested_row_identities.get((owner, root)) if root else None,
                    )
                    detail.update({"readback_status": read_status, "readback": redacted(actual), "row_id_present": bool(active_row_id)})
                    logs_after = audit_logs(
                        owner, active_row_id, audit_path,
                        nested_row_identities.get((owner, root)) if root else None,
                    )
                    after_owner_status, after_owner = read_current(
                        page, owner, row_route, active_row_id
                    )
                    declared_partner = (
                        field.get("_nullFlavorPartnerPath")
                        if is_nullflavor_field(field)
                        and candidate in field.get("_allowedNullFlavors", [])
                        else None
                    )
                    partner_projection = (
                        nullflavor_partner_projection(
                            field, owner, all_setup_groups.get(owner, setup_groups[owner])
                        )
                        if declared_partner else None
                    )
                    partner_present, partner_value = (
                        owner_path_value(
                            after_owner, partner_projection,
                            nested_row_identities.get((owner, root)) if root else None,
                        )
                        if partner_projection else (False, None)
                    )
                    partner_cleared = (
                        not declared_partner or partner_present and (
                            partner_value is None
                            or field.get("code") == "FDA.D.11.r.1.nullFlavor" and partner_value == []
                        )
                    )
                    alias_projection = (
                        "drugAuthorizationCountry" if field.get("code") == "G.k.3.2" else None
                    )
                    alias_present, alias_value = (
                        owner_path_value(after_owner, alias_projection)
                        if alias_projection else (False, None)
                    )
                    alias_matches = not alias_projection or (
                        alias_present and values_equal(actual, alias_value)
                    )
                    comparison_companion = partner_projection or alias_projection
                    owner_preserved = (
                        before_owner_status == after_owner_status == 200
                        and partner_cleared and alias_matches
                        and owner_without_target(
                            before_owner, audit_path, before_nested_identity, comparison_companion
                        )
                        == owner_without_target(
                            after_owner, audit_path,
                            nested_row_identities.get((owner, root)) if root else None,
                            comparison_companion,
                        )
                    )
                    detail["whole_owner_unrelated_preserved"] = owner_preserved
                    if declared_partner:
                        detail.update({
                            "nullflavor_partner_path": declared_partner,
                            "nullflavor_partner_projection_path": partner_projection,
                            "nullflavor_partner_present": partner_present,
                            "nullflavor_partner_cleared": partner_cleared,
                        })
                    if alias_projection:
                        detail.update({"alias_projection_path": alias_projection,
                                       "alias_matches": alias_matches})
                    if not owner_preserved:
                        classification = "SAVE_READBACK_MISMATCH"
                        detail.update({
                            "whole_owner_before": copy.deepcopy(owner_business(before_owner)),
                            "whole_owner_after": copy.deepcopy(owner_business(after_owner)),
                        })
                    changed = [log for log in logs_after if log not in logs_before and isinstance(log, dict)]
                    main_audit_key = audit_field_key(audit_path)
                    generic_audit_changes = {}
                    if type(before_actual) is not type(actual) or before_actual != actual:
                        generic_audit_changes[main_audit_key] = (
                            audit_expected_value(main_audit_key, before_actual, owner),
                            audit_expected_value(main_audit_key, actual, owner),
                        )
                        if main_audit_key in {
                            "first_administration_date", "last_administration_date",
                        }:
                            before_raw_date = dosage_raw_date_value(before_actual)
                            after_raw_date = dosage_raw_date_value(actual)
                            if before_raw_date != after_raw_date:
                                generic_audit_changes[f"{main_audit_key}_raw"] = (
                                    before_raw_date, after_raw_date,
                                )
                    if partner_projection:
                        before_partner_present, before_partner_value = owner_path_value(
                            before_owner, partner_projection, before_nested_identity,
                        )
                        if before_partner_present and partner_present \
                                and (type(before_partner_value) is not type(partner_value)
                                     or before_partner_value != partner_value):
                            partner_key = audit_field_key(partner_projection)
                            generic_audit_changes[partner_key] = (
                                audit_expected_value(partner_key, before_partner_value, owner),
                                audit_expected_value(partner_key, partner_value, owner),
                            )
                    if alias_projection and field.get("code") != "G.k.3.2":
                        before_alias_present, before_alias_value = owner_path_value(
                            before_owner, alias_projection, before_nested_identity,
                        )
                        if before_alias_present and alias_present \
                                and (type(before_alias_value) is not type(alias_value)
                                     or before_alias_value != alias_value):
                            generic_audit_changes[audit_field_key(alias_projection)] = (
                                before_alias_value, alias_value,
                            )
                    audit_exact = False
                    audit_detail: dict[str, Any] = {}
                    if identity_replaced and root and before_nested_identity:
                        old_logs_after = audit_logs(
                            owner, active_row_id, audit_path, before_nested_identity,
                        )
                        old_exact, old_detail = exact_audit_delta(
                            logs_before, old_logs_after, owner=owner,
                            row_id=active_row_id or "", action="UPDATE",
                            expected_changes={"deleted": (False, True)},
                            field_path=audit_path, nested_identity=before_nested_identity,
                        )
                        new_exact, new_detail = exact_audit_delta(
                            [], logs_after, owner=owner, row_id=active_row_id or "",
                            action="CREATE", expected_changes={
                                audit_field_key(audit_path): (None, actual),
                            }, field_path=audit_path,
                            nested_identity=nested_row_identities.get((owner, root)),
                            exact_changed_keys=False,
                        )
                        audit_exact = old_exact and new_exact
                        audit_detail = {"replacementDelete": old_detail,
                                        "replacementCreate": new_detail}
                    elif generic_audit_changes:
                        audit_exact, audit_detail = exact_audit_delta(
                            logs_before, logs_after, owner=owner,
                            row_id=active_row_id or "", action="UPDATE",
                            expected_changes=generic_audit_changes,
                            field_path=audit_path,
                            nested_identity=nested_row_identities.get((owner, root)) if root else None,
                        )
                    if read_status != 200:
                        classification = "INCONCLUSIVE"
                    elif values_equal(candidate, actual):
                        changed_fields = [
                            log.get("changedFields", log.get("changed_fields", {}))
                            for log in changed
                            if isinstance(log.get("changedFields", log.get("changed_fields", {})), dict)
                        ]
                        field_match = any(audit_key_matches(fields_map, audit_path) for fields_map in changed_fields)
                        matched_logs = [
                            log for log in changed
                            if audit_key_matches(log.get("changedFields", log.get("changed_fields", {})), audit_path)
                        ]
                        audit_complete = any(audit_log_complete(log) for log in matched_logs)
                        if (
                            not changed
                            and before_status == 200
                            and values_equal(candidate, before_actual)
                        ):
                            classification = "NOOP_ACCEPTED" if logs_after == logs_before else "AUDIT_MISMATCH"
                        elif not audit_exact:
                            classification = "AUDIT_MISMATCH"
                        detail.update({"audit_new_logs": len(changed), "audit_field_match": field_match, "audit_complete": audit_complete, "audit_exact_delta": audit_exact, "audit_detail": audit_detail, "audit_path": audit_path, "audit_changed_keys": sorted({str(key) for fields_map in changed_fields for key in fields_map})})
                    else:
                        audit_complete = False
                        if is_blank_candidate(candidate) or (
                            candidate is None and field.get("code") in CI_NON_NULLABLE_TEXT_CODES
                        ):
                            matched_logs = [log for log in changed if audit_key_matches(log.get("changedFields", log.get("changed_fields", {})), audit_path)]
                            audit_complete = audit_exact
                            detail.update({"audit_new_logs": len(changed), "audit_complete": audit_complete})
                        if field.get("code") == "FDA.D.11.r.1" and candidate is None:
                            audit_complete = audit_exact
                        classification = mismatched_save_classification(
                            candidate, before_actual, actual, audit_complete
                        )
                        collection_classification = collection_blank_classification(
                            field.get("code"), candidate, before_actual, actual, audit_complete
                        )
                        if collection_classification:
                            classification = collection_classification
                        if candidate is None:
                            ci_null_classification = ci_non_nullable_null_classification(
                                field.get("code"), before_actual, actual, audit_complete
                            )
                            if ci_null_classification:
                                classification = ci_null_classification
                    business_changed = identity_replaced or bool(generic_audit_changes)
                    no_audit_delta, no_audit_detail = audit_no_delta_exact(
                        logs_before, logs_after,
                    )
                    # This invariant is terminal for every accepted HTTP 200 mutation:
                    # changed business requires the exact expected delta; unchanged
                    # business requires byte-identical scoped audit history.
                    audit_invariant = audit_exact if business_changed else no_audit_delta
                    detail.update({
                        "audit_business_changed": business_changed,
                        "audit_invariant_exact": audit_invariant,
                        "audit_exact_delta": audit_exact,
                        "audit_detail": audit_detail,
                        "audit_no_delta_detail": no_audit_detail,
                    })
                    if not audit_invariant and classification not in {
                        "SAVE_READBACK_MISMATCH", "INCONCLUSIVE",
                    }:
                        classification = "AUDIT_MISMATCH"
                    if classification in {"SAVE_ACCEPTED", "NOOP_ACCEPTED", "SAVE_NORMALIZED"} and owner_preserved:
                        noop_payload = {"authorities": authorities_for([field]), "rows": (
                            {owner: mutation} if row_route else owner_patch(owner, mutation, array_owner)
                        )}
                        noop_path = f"{route}/rows/{active_row_id or ''}" if row_route else route
                        noop_status, _, noop_summary = request("PATCH", noop_path, noop_payload)
                        noop_read_status, noop_owner = read_current(
                            page, owner, row_route, active_row_id
                        )
                        noop_logs = audit_logs(
                            owner, active_row_id, audit_path,
                            nested_row_identities.get((owner, root)) if root else None,
                        )
                        noop_business_same = owner_business(noop_owner) == owner_business(after_owner)
                        noop_ok = (
                            noop_status == 200 and noop_read_status == 200
                            and noop_business_same and audit_no_delta_exact(logs_after, noop_logs)[0]
                        )
                        add(Event(
                            "noop_resave", field["code"], page, owner,
                            f"value_{ordinal}_sample_{sample}",
                            "PASS" if noop_ok else "AUDIT_MISMATCH", noop_status,
                            {**noop_summary, "business_owner_unchanged": noop_business_same,
                             "audit_unchanged": audit_no_delta_exact(logs_after, noop_logs)[0],
                             **({"whole_owner_before": copy.deepcopy(owner_business(after_owner)),
                                 "whole_owner_after": copy.deepcopy(owner_business(noop_owner))}
                                if not noop_business_same else {})},
                        ))
                        if not noop_ok:
                            classification = "AUDIT_MISMATCH"
                    if classification == "CLEAR_NOT_APPLIED" and candidate == "":
                        detail.update({
                            "surface": "raw_api",
                            "normalization_explanation": "UI save canonicalizes blank strings to null",
                        })
                    if blank_nullflavor_requires_identity_refresh(field, ordinal):
                        normalized = (
                            read_status == 200
                            and readback_key_present
                            and actual is None
                            and identity_refreshed
                            and classification == "SAVE_NORMALIZED"
                        )
                        classification = "SAVE_NORMALIZED" if normalized else "FAIL"
                        detail.update({
                            "readback_key_present": readback_key_present,
                            "nested_identity_refreshed": identity_refreshed,
                            "normalization_explanation": (
                                "Device NullFlavor blank is trimmed to null by the API"
                            ),
                        })
                elif status in {400, 409, 422}:
                    read_status, actual, _ = readback(
                        page,
                        owner,
                        projection_leaf(field, owner),
                        row_route,
                        active_row_id,
                        nested_row_identities.get((owner, root)) if root else None,
                    )
                    logs_after = audit_logs(
                        owner, active_row_id, audit_path,
                        nested_row_identities.get((owner, root)) if root else None,
                    )
                    rejected_owner_status, rejected_owner = read_current(
                        page, owner, row_route, active_row_id
                    )
                    changed = [log for log in logs_after if log not in logs_before and isinstance(log, dict)]
                    rejected_audit_exact, rejected_audit_detail = audit_no_delta_exact(
                        logs_before, logs_after,
                    )
                    structured_error = summary.get("error_code") != "SERVICE_ERROR" and bool(
                        summary.get("error_code") or summary.get("error_rule_code") or summary.get("error_path") or summary.get("detail_type") == "dict"
                    )
                    if ordinal == 12 and status == 400:
                        # A lone surrogate is invalid JSON Unicode and is rejected
                        # before the input-contract error mapper can attach a rule.
                        structured_error = True
                    detail.update({
                        "before_readback_status": before_status,
                        "before_readback": redacted(before_actual),
                        "readback_status": read_status,
                        "readback": redacted(actual),
                        "audit_new_logs": len(changed),
                        "audit_no_delta_exact": rejected_audit_exact,
                        "audit_no_delta_detail": rejected_audit_detail,
                        "structured_error": structured_error,
                        "whole_owner_unchanged": (
                            before_owner_status == rejected_owner_status == 200
                            and owner_business(before_owner) == owner_business(rejected_owner)
                        ),
                    })
                    if owner_business(before_owner) != owner_business(rejected_owner):
                        detail.update({
                            "whole_owner_before": copy.deepcopy(owner_business(before_owner)),
                            "whole_owner_after": copy.deepcopy(owner_business(rejected_owner)),
                        })
                    if not rejected_audit_exact:
                        classification = "AUDIT_MISMATCH"
                    elif before_status != 200 or read_status != 200:
                        classification = "INCONCLUSIVE"
                    elif before_actual != actual:
                        classification = "SAVE_READBACK_MISMATCH"
                    elif owner_business(before_owner) != owner_business(rejected_owner):
                        classification = "SAVE_READBACK_MISMATCH"
                    elif not structured_error:
                        classification = "UNEXPECTED_STATUS"
                    if (field.get("_auditAll") and ordinal == 2
                            and rejected_audit_exact
                            and structured_error
                            and before_status == read_status == 200
                            and before_owner_status == rejected_owner_status == 200
                            and before_actual == actual
                            and owner_business(before_owner) == owner_business(rejected_owner)):
                        classification = "CLEAR_REJECTED"
                    if (field.get("_auditComplianceGuard") and ordinal == 1
                            and status == 400 and rejected_audit_exact
                            and before_status == read_status == 200
                            and before_actual == actual
                            and owner_business(before_owner) == owner_business(rejected_owner)):
                        classification = "COMPLIANCE_GUARD_REJECTED"
                if invalid_nullflavor:
                    if status in {400, 422} and classification in {"BACKEND_REJECTED", "CONSTRAINT_REJECTED"}:
                        classification = "NULLFLAVOR_REJECTED"
                    elif status == 200:
                        classification = "FAIL"
                    elif status is None:
                        classification = "INCONCLUSIVE"
                    else:
                        classification = "UNEXPECTED_STATUS"
                detail["persistence_classification"] = classification
                mismatch = expectation_error(expectation, status, summary.get("error_rule_code"))
                if expectation and expectation[0] == "accept_or_forbidden" and status == 403:
                    classification = "AUTHORIZATION_BLOCKED"
                if mismatch:
                    detail["expectation_error"] = mismatch
                    if classification not in {
                        "AUDIT_MISMATCH", "INCONCLUSIVE", "SERVER_ERROR", "SAVE_READBACK_MISMATCH"
                    }:
                        classification = (
                            "BOUNDARY_BLOCKED"
                            if expectation and expectation[0] == "length_boundary"
                            and status in {400, 409, 422}
                            and summary.get("error_rule_code") != expectation[1]
                            else "FAIL"
                        )
                if expectation is None and not field.get("_auditAll") and classification not in {
                    "AUDIT_MISMATCH", "FAIL", "INCONCLUSIVE", "SERVER_ERROR",
                    "SAVE_READBACK_MISMATCH", "UNEXPECTED_STATUS",
                    "CLEAR_NOT_APPLIED", "NORMALIZATION_UNVERIFIED", "BOUNDARY_BLOCKED",
                }:
                    classification = "NO_EXPECTATION"
                if detail.get("whole_owner_unrelated_preserved") is False:
                    classification = "SAVE_READBACK_MISMATCH"
                add(Event("mutation", field["code"], page, owner, f"value_{ordinal}_sample_{sample}", classification, status, {**summary, **detail}))
                if status == 200 and read_status == 200:
                    before_status, before_actual = read_status, actual
                if logs_after is not None:
                    logs_before = logs_after
                if status == 200:
                    before_owner_status, before_owner = read_current(
                        page, owner, row_route, active_row_id
                    )

    if case_id and not interrupted and args.verify_global_audit_chain:
        status, value, summary = request("GET", "/api/audit-logs/verify-integrity")
        broken = value.get("broken_rows", value.get("brokenRows")) if isinstance(value, dict) else None
        if status == 200 and broken == 0:
            chain_classification = "PASS"
        elif status == 200 and chain_before is not None and broken == chain_before:
            chain_classification = "AUDIT_CHAIN_PREEXISTING"
        else:
            chain_classification = "AUDIT_MISMATCH"
        add(Event("audit_chain", None, None, None, None, chain_classification, status, {**summary, "broken_rows": broken, "broken_rows_before": chain_before}))

    if args.run_gates:
        frontend_root = contract_path.parents[3]
        classification, detail = run_gate(
            ["cargo", "test", "-p", "validator", "--lib"],
            backend_root,
            args.gate_timeout,
        )
        add(Event("validator_gate", None, None, None, None, classification, None, detail))
        classification, detail = run_gate(
            [
                "env",
                f"E2BR3_BACKEND_ROOT={backend_root}",
                "npm",
                "run",
                "test:editor-contracts",
                "--",
                "--runInBand",
            ],
            frontend_root,
            args.gate_timeout,
        )
        add(Event("frontend_gate", None, None, None, None, classification, None, detail))

    out_dir = Path(args.artifact_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    artifact = out_dir / f"case-editor-{args.seed}.jsonl"
    verdict = run_verdict(events, planned_mutations, interrupted, unsupported, args.values_per_field == 0)
    audit_field_report: list[dict[str, Any]] = []
    if args.audit_all_fields:
        expected_counts = {
            (page, field["code"]): candidate_count(field, args.values_per_field)
            for page in pages for field in page_fields(page)
        }
        expected_counts.update({("RE", code): 4 for code in selected_re_codes})
        audit_field_report = persisted_field_report(
            audit_inventory or {}, pages, requested_fields, selected_blocked_identities,
            events, expected_counts,
        )
        field_counts = {
            status: sum(item["status"] == status for item in audit_field_report)
            for status in {item["status"] for item in audit_field_report}
        }
        if field_counts.get("FAILED") or field_counts.get("MISSING") \
                or field_counts.get("BLOCKED_SOURCE_PREREQUISITE"):
            verdict["verdict"] = "INCOMPLETE_OR_FAILED"
        verdict["persisted_field_results"] = field_counts
    if args.lifecycle_only:
        lifecycle_events = [event for event in events if event.kind == "lifecycle"]
        lifecycle_passed = [event for event in lifecycle_events if event.classification == "PASS"]
        lifecycle_blocked = [event for event in lifecycle_events if event.classification == "SKIPPED"]
        lifecycle_failed = [event for event in lifecycle_events if event.classification not in {"PASS", "SKIPPED"}]
        verdict.update({
            "verdict": (
                "LIFECYCLE_CHECKS_PASSED"
                if (lifecycle_passed and not lifecycle_failed and not lifecycle_blocked
                    and not interrupted and not verdict["unresolved_classifications"])
                else "INCOMPLETE_OR_FAILED"
            ),
            "lifecycle_passed": len(lifecycle_passed),
            "lifecycle_blocked": len(lifecycle_blocked),
            "lifecycle_failed": len(lifecycle_failed),
        })
    with artifact.open("w", encoding="utf-8") as handle:
        for event in events:
            handle.write(json.dumps({"seed": args.seed, "commit": commit_sha(), **asdict(event)}, sort_keys=True) + "\n")
        handle.write(json.dumps({"kind": "run", **verdict, "excluded_fields": excluded_fields, "contract_sha256": hashlib.sha256(contract_path.read_bytes()).hexdigest(), "runner_sha256": runner_sha256, "seed": args.seed, "case_id": case_id, "cases": len(events), "requests": request_count, "elapsed_seconds": round(time.monotonic() - started, 3), "interrupted": interrupted, "artifact": str(artifact), "contract": str(contract_path), "candidate_schema_version": 6, "samples_per_category": args.samples_per_category, "field_filter": sorted(requested_fields), "derived_null_flavors": derived_null_flavors, "max_length_fields": max_length_fields, "identifier_fields": identifier_fields, "boolean_fields": boolean_fields, "null_flavor_only": args.null_flavor_only, "audit_all_fields": args.audit_all_fields, "persisted_field_denominator": persisted_field_denominator, "persisted_field_inventory_sha256": persisted_field_inventory_sha256, "audit_alternate_map_sha256": audit_alternate_map_sha256, "audit_alternate_unresolved": selected_blocked_identities, "audit_field_report": audit_field_report, "selected_persisted_fields": sum(len(page_fields(page)) for page in pages) + len(selected_re_codes) + len(selected_blocked_identities), "executable_persisted_fields": sum(len(page_fields(page)) for page in pages) + len(selected_re_codes), "surface": "api", "global_audit_chain": "run" if args.verify_global_audit_chain else "NOT_RUN", "validator_excluded": not args.run_gates, "frontend_gate": "run" if args.run_gates else "not_run"}, sort_keys=True) + "\n")
    counts: dict[str, int] = {}
    for event in events:
        counts[event.classification] = counts.get(event.classification, 0) + 1
    print(f"events={len(events)} counts={json.dumps(counts, sort_keys=True)} artifact={artifact}")
    print(json.dumps(verdict, sort_keys=True))
    return 2 if interrupted else 0 if verdict["verdict"] in {
        "SELECTED_API_CHECKS_PASSED", "BASELINE_ONLY", "LIFECYCLE_CHECKS_PASSED"
    } else 1


def parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-url", default=os.getenv("E2BR3_BASE_URL", "http://127.0.0.1:8080"))
    parser.add_argument("--email", default=os.getenv("E2BR3_ADMIN_EMAIL", "demo.cro.admin@example.com"))
    parser.add_argument("--password", default=os.getenv("E2BR3_ADMIN_PASSWORD", "welcome"))
    parser.add_argument("--seed", type=int, default=int(time.time()))
    # DH needs the DM patient row; create it immediately after DM before later
    # page mutations can make the case setup harder to authorize.
    parser.add_argument("--pages", default="CI,RP,SD,LR,SI,DM,DH,NR,AE,LB,DG")
    parser.add_argument("--field", action="append", help="run only this field code (repeatable)")
    parser.add_argument("--values-per-field", type=int, default=IDENTIFIER_CANDIDATES, help="upper bound; only candidates applicable to each field are used")
    parser.add_argument(
        "--samples-per-category",
        type=int,
        default=3,
        help="seeded variants for randomized grammar categories",
    )
    parser.add_argument("--max-actions", type=int, default=30000)
    parser.add_argument("--deadline-seconds", type=float, default=900)
    parser.add_argument("--timeout", type=float, default=20)
    parser.add_argument("--contract", default=str(DEFAULT_CONTRACT))
    parser.add_argument("--null-flavor-pairs", default=str(DEFAULT_NULL_FLAVOR_PAIRS))
    parser.add_argument("--meddra-version", default=os.getenv("E2BR3_MEDDRA_VERSION"), help="live MedDRA version (or E2BR3_MEDDRA_VERSION)")
    parser.add_argument("--meddra-code", default=os.getenv("E2BR3_MEDDRA_CODE"), help="live MedDRA code (or E2BR3_MEDDRA_CODE)")
    parser.add_argument("--product-key", default=os.getenv("E2BR3_PRODUCT_KEY"), help="accessible product_id from GET /api/presaves/products")
    parser.add_argument("--null-flavor-only", action="store_true")
    parser.add_argument(
        "--audit-all-fields", action="store_true",
        help="audit every persisted Case Edit field independently of portable validation status",
    )
    parser.add_argument("--persisted-field-inventory")
    parser.add_argument("--audit-alternate-map")
    parser.add_argument("--complete-baseline", action="store_true", help="populate every concrete contract field during owner setup")
    parser.add_argument("--lifecycle-only", action="store_true", help="run exact owned-row lifecycle checks without candidate mutations")
    parser.add_argument("--lifecycle-owner", help="source-confirmed direct-array owner override; requires --lifecycle-only and one page")
    parser.add_argument("--nested-lifecycle-owner", help="source-confirmed DG nested owner lifecycle override")
    parser.add_argument("--artifact-dir", default="tmp/rbac-rls-fuzz/case-editor-contract")
    parser.add_argument("--allow-remote", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--run-gates", action=argparse.BooleanOptionalAction, default=True)
    parser.add_argument("--verify-global-audit-chain", action=argparse.BooleanOptionalAction, default=True)
    parser.add_argument("--gate-timeout", type=float, default=180)
    return parser


if __name__ == "__main__":
    try:
        sys.exit(main(parser().parse_args()))
    except KeyboardInterrupt:
        sys.exit(2)
