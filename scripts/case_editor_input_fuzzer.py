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
NULL_FLAVOR_TOKENS = (
    "NI", "INV", "DER", "OTH", "NINF", "PINF", "UNC", "MSK",
    "NA", "UNK", "ASKU", "NAV", "QS", "TRC", "NP",
)
AUDIT_TABLES = {
    "patientInformation": "patient_information",
    "patientIdentifiers": "patient_identifiers",
    "medicalHistoryEpisodes": "medical_history_episodes",
    "reportedCauses": "reported_causes_of_death",
    "autopsyCauses": "autopsy_causes_of_death",
    "deathInfo": "patient_death_information",
    "studyInformation": "study_information",
    "studyRegistrationNumbers": "study_registration_numbers",
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
    "fdaCrossReportedIndNumbers[]": "study_fda_cross_reported_inds",
    "activeSubstances[]": "drug_active_substances",
    "dosageInformation[]": "dosage_information",
    "indications[]": "drug_indications",
    "drugReactionAssessments[].sourceOfAssessment": "relatedness_assessments",
    "drugReactionAssessments[].methodOfAssessment": "relatedness_assessments",
    "drugReactionAssessments[].resultOfAssessment": "relatedness_assessments",
    "drugReactionAssessments[].source_of_assessment": "relatedness_assessments",
    "drugReactionAssessments[].method_of_assessment": "relatedness_assessments",
    "drugReactionAssessments[].result_of_assessment": "relatedness_assessments",
    "drugReactionAssessments[]": "drug_reaction_assessments",
    "fdaDevices[]": "fda_device_information",
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
        return "NOOP_ACCEPTED" if values_equal(actual, before) else "CLEAR_NOT_APPLIED"
    if is_blank_candidate(candidate):
        return normalized_classification(actual, before, audit_complete)
    return "SAVE_READBACK_MISMATCH"


def run_verdict(events: list[Event], planned: int, interrupted: str | None,
                unsupported: list[str], setup_only: bool = False) -> dict[str, Any]:
    mutations = [event for event in events if event.kind == "mutation"]
    accepted = {
        "PASS", "SAVE_ACCEPTED", "NOOP_ACCEPTED", "SAVE_NORMALIZED",
        "CONSTRAINT_REJECTED", "BACKEND_REJECTED", "CONFLICT_REJECTED",
        "NULLFLAVOR_REJECTED", "AUDIT_CHAIN_PREEXISTING", "GATE_PASS",
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
    if is_nullflavor_field(field):
        if field.get("payloadPath") in DEVICE_NULL_FLAVOR_PATHS and ordinal == 4:
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


def contract_rows(contract: list[dict[str, Any]], page: str) -> list[dict[str, Any]]:
    return [
        field
        for field in next((item["fields"] for item in contract if item["pageId"] == page), [])
        if field.get("constraint", {}).get("status") == "verified"
        and field.get("roundTripValue") is not None
        and field.get("payloadPath")
    ]


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
    values = {str(field.get("authority", "ICH")).lower() for field in fields}
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
    guard_target(args.base_url, args.allow_remote)
    if args.samples_per_category < 1:
        raise SystemExit("--samples-per-category must be at least 1")
    if args.values_per_field < 0:
        raise SystemExit("--values-per-field must be nonnegative")
    if not args.dry_run and not args.product_key:
        raise SystemExit("set --product-key or E2BR3_PRODUCT_KEY to an accessible product ID")
    if not args.dry_run and (not args.meddra_version or not args.meddra_code):
        raise SystemExit("set both --meddra-version and --meddra-code for live intake creation")
    if not args.password and not args.dry_run:
        raise SystemExit("set E2BR3_ADMIN_PASSWORD")
    contract_path = Path(args.contract).resolve()
    contract = json.loads(contract_path.read_text())
    apply_meddra_baseline(contract, args.meddra_version, args.meddra_code)
    backend_root = Path(__file__).resolve().parents[1]
    pages = [page.strip().upper() for page in args.pages.split(",") if page.strip()]
    unknown_pages = set(pages) - {page["pageId"] for page in contract}
    if unknown_pages:
        raise SystemExit(f"unsupported pages: {', '.join(sorted(unknown_pages))}")
    max_length_fields = apply_max_lengths(contract, load_max_lengths(backend_root))
    identifier_fields, boolean_fields = apply_generated_rules(
        contract,
        load_generated_rules(backend_root, IDENTIFIER_RE),
        load_generated_rules(backend_root, BOOLEAN_RE),
    )
    unsupported: list[str] = []
    derived_null_flavors = expand_null_flavor_contracts(
        [page for page in contract if page["pageId"] in pages],
        load_null_flavor_pairs(Path(args.null_flavor_pairs).resolve()),
        load_dictionary_null_flavors(Path(__file__).resolve().parents[1]),
        unsupported,
    )
    requested_fields = set(args.field or ())
    available_fields = {
        field.get("code") for page in pages for field in contract_rows(contract, page)
    }
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
        fields = contract_rows(contract, page)
        if requested_fields:
            fields = [field for field in fields if field.get("code") in requested_fields]
        return [field for field in fields if is_nullflavor_field(field)] if args.null_flavor_only else fields

    def add(event: Event) -> None:
        events.append(event)

    planned_mutations = sum(
        candidate_sample_count(field, ordinal, args.samples_per_category)
        for page in pages for field in page_fields(page)
        for ordinal in range(candidate_count(field, args.values_per_field))
    )
    excluded_fields = [
        {"page": page["pageId"], "code": field["code"]}
        for page in contract if page["pageId"] in pages
        for field in page["fields"] if field not in contract_rows(contract, page["pageId"])
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
            total += mutations
            print(f"{page}: fields={len(fields)} owners={len(groups)} mutations={mutations}")
        print(f"seed={args.seed} total_mutations={total} derived_null_flavors={derived_null_flavors} max_length_fields={max_length_fields} identifier_fields={identifier_fields} boolean_fields={boolean_fields} contract={contract_path}")
        print(json.dumps({"verdict": "NOT_RUN", "planned_mutations": total,
                          "excluded_fields": excluded_fields,
                          "unsupported_null_flavor_pairs": unsupported}, sort_keys=True))
        return 1 if unsupported or not total else 0

    if not planned_mutations and args.values_per_field != 0:
        raise SystemExit("no input mutations selected; no API requests were sent")

    status, _, summary = request(
        "POST",
        "/auth/v1/login",
        {"email": args.email, "pwd": args.password},
    )
    add(Event("login", None, None, None, None, "PASS" if status == 200 else "FAIL", status, summary))
    if status != 200:
        interrupted = interrupted or "login_failed"

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
    if case_id and not interrupted:
        status, value, _ = request("GET", "/api/audit-logs/verify-integrity")
        if status == 200 and isinstance(value, dict):
            chain_before = value.get("broken_rows", value.get("brokenRows"))

    def audit_logs(owner: str | None = None, row_id: str | None = None, field_path: str = "") -> list[dict[str, Any]]:
        nested_table = next(
            (table for prefix, table in NESTED_AUDIT_TABLES.items() if field_path.startswith(prefix)),
            None,
        )
        if nested_table:
            status, value, _ = request("GET", f"/api/audit-logs/by-record/cases/{case_id}")
            if status != 200:
                return []
            return [log for log in value if isinstance(log, dict) and log.get("table_name") == nested_table] if isinstance(value, list) else []
        table = AUDIT_TABLES.get(owner or "")
        target = f"{table}/{row_id}" if table and row_id else f"cases/{case_id}"
        audit_field = audit_field_key(field_path) if field_path else ""
        field_query = (
            f"?field={urllib.parse.quote(audit_field)}"
            if table and row_id and audit_field and audit_field not in UNFILTERED_AUDIT_FIELDS
            else ""
        )
        status, value, _ = request("GET", f"/api/audit-logs/by-record/{target}{field_query}")
        if status != 200:
            return []
        if isinstance(value, list):
            return value
        logs = value.get("items", value.get("data", value)) if isinstance(value, dict) else []
        return logs if isinstance(logs, list) else []

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

    for page in pages:
        if interrupted or not case_id:
            break
        fields = page_fields(page)
        groups: dict[str, list[dict[str, Any]]] = {}
        for field in fields:
            groups.setdefault(field["patch"]["owner"], []).append(field)
        setup_groups: dict[str, list[dict[str, Any]]] = {}
        for field in contract_rows(contract, page):
            setup_groups.setdefault(field["patch"]["owner"], []).append(field)
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
                if row_ids.get(owner):
                    nested_baseline["id"] = row_ids[owner]
                if row_route:
                    status, _, summary = request("PATCH", f"{route}/rows/{row_ids.get(owner, '')}", {"authorities": authorities_for(root_fields), "rows": {owner: nested_baseline}})
                else:
                    status, _, summary = request("PATCH", route, {"authorities": authorities_for(root_fields), "rows": owner_patch(owner, nested_baseline, array_owner)})
                read_status, current = read_current(page, owner, row_route, row_ids.get(owner))
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
                row_ids.get(owner),
                nested_row_identities.get((owner, root)) if root else None,
            )
            logs_before = audit_logs(owner, row_ids.get(owner), projection_leaf(field, owner))
            for ordinal, sample in candidates:
                if interrupted:
                    break
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
                if row_ids.get(owner):
                    mutation["id"] = row_ids[owner]
                audit_path = projection_leaf(field, owner)
                if row_route:
                    status, value, summary = request(
                        "PATCH", f"{route}/rows/{row_ids.get(owner, '')}", {"authorities": authorities_for([field]), "rows": {owner: mutation}}
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
                        row_ids.get(owner),
                        nested_row_identities.get((owner, root)) if root else None,
                    )
                    detail.update({"readback_status": read_status, "readback": redacted(actual), "row_id_present": bool(row_ids.get(owner))})
                    logs_after = audit_logs(owner, row_ids.get(owner), audit_path)
                    changed = [log for log in logs_after if log not in logs_before and isinstance(log, dict)]
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
                            classification = "NOOP_ACCEPTED"
                        elif not changed or not field_match or not audit_complete:
                            classification = "AUDIT_MISMATCH"
                        detail.update({"audit_new_logs": len(changed), "audit_field_match": field_match, "audit_complete": audit_complete, "audit_path": audit_path, "audit_changed_keys": sorted({str(key) for fields_map in changed_fields for key in fields_map})})
                    else:
                        audit_complete = False
                        if is_blank_candidate(candidate):
                            matched_logs = [log for log in changed if audit_key_matches(log.get("changedFields", log.get("changed_fields", {})), audit_path)]
                            audit_complete = any(audit_log_complete(log) for log in matched_logs)
                            detail.update({"audit_new_logs": len(changed), "audit_complete": audit_complete})
                        classification = mismatched_save_classification(
                            candidate, before_actual, actual, audit_complete
                        )
                    if classification == "CLEAR_NOT_APPLIED" and candidate == "":
                        detail.update({
                            "surface": "raw_api",
                            "normalization_explanation": "UI save canonicalizes blank strings to null",
                        })
                    if (
                        payload_path in DEVICE_NULL_FLAVOR_PATHS
                        and ordinal == 4
                    ):
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
                        row_ids.get(owner),
                        nested_row_identities.get((owner, root)) if root else None,
                    )
                    logs_after = audit_logs(owner, row_ids.get(owner), audit_path)
                    changed = [log for log in logs_after if log not in logs_before and isinstance(log, dict)]
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
                        "structured_error": structured_error,
                    })
                    if changed:
                        classification = "AUDIT_MISMATCH"
                    elif before_status != 200 or read_status != 200:
                        classification = "INCONCLUSIVE"
                    elif before_actual != actual:
                        classification = "SAVE_READBACK_MISMATCH"
                    elif not structured_error:
                        classification = "UNEXPECTED_STATUS"
                if invalid_nullflavor:
                    if status in {400, 422} and classification in {"BACKEND_REJECTED", "CONSTRAINT_REJECTED"}:
                        classification = "NULLFLAVOR_REJECTED"
                    elif status == 200:
                        classification = "FAIL"
                    elif status is None:
                        classification = "INCONCLUSIVE"
                    else:
                        classification = "UNEXPECTED_STATUS"
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
                if expectation is None and classification not in {
                    "AUDIT_MISMATCH", "FAIL", "INCONCLUSIVE", "SERVER_ERROR",
                    "SAVE_READBACK_MISMATCH", "UNEXPECTED_STATUS",
                    "CLEAR_NOT_APPLIED", "NORMALIZATION_UNVERIFIED", "BOUNDARY_BLOCKED",
                }:
                    classification = "NO_EXPECTATION"
                add(Event("mutation", field["code"], page, owner, f"value_{ordinal}_sample_{sample}", classification, status, {**summary, **detail}))
                if status == 200 and read_status == 200:
                    before_status, before_actual = read_status, actual
                if logs_after is not None:
                    logs_before = logs_after

    if case_id and not interrupted:
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
    with artifact.open("w", encoding="utf-8") as handle:
        for event in events:
            handle.write(json.dumps({"seed": args.seed, "commit": commit_sha(), **asdict(event)}, sort_keys=True) + "\n")
        handle.write(json.dumps({"kind": "run", **verdict, "excluded_fields": excluded_fields, "contract_sha256": hashlib.sha256(contract_path.read_bytes()).hexdigest(), "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), "seed": args.seed, "case_id": case_id, "cases": len(events), "requests": request_count, "elapsed_seconds": round(time.monotonic() - started, 3), "interrupted": interrupted, "artifact": str(artifact), "contract": str(contract_path), "candidate_schema_version": 6, "samples_per_category": args.samples_per_category, "field_filter": sorted(requested_fields), "derived_null_flavors": derived_null_flavors, "max_length_fields": max_length_fields, "identifier_fields": identifier_fields, "boolean_fields": boolean_fields, "null_flavor_only": args.null_flavor_only, "surface": "api", "validator_excluded": not args.run_gates, "frontend_gate": "run" if args.run_gates else "not_run"}, sort_keys=True) + "\n")
    counts: dict[str, int] = {}
    for event in events:
        counts[event.classification] = counts.get(event.classification, 0) + 1
    print(f"events={len(events)} counts={json.dumps(counts, sort_keys=True)} artifact={artifact}")
    print(json.dumps(verdict, sort_keys=True))
    return 2 if interrupted else 0 if verdict["verdict"] in {"SELECTED_API_CHECKS_PASSED", "BASELINE_ONLY"} else 1


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
    parser.add_argument("--complete-baseline", action="store_true", help="populate every concrete contract field during owner setup")
    parser.add_argument("--artifact-dir", default="tmp/rbac-rls-fuzz/case-editor-contract")
    parser.add_argument("--allow-remote", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--run-gates", action=argparse.BooleanOptionalAction, default=True)
    parser.add_argument("--gate-timeout", type=float, default=180)
    return parser


if __name__ == "__main__":
    try:
        sys.exit(main(parser().parse_args()))
    except KeyboardInterrupt:
        sys.exit(2)
