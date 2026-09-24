#!/usr/bin/env python3
"""Seeded black-box fuzzer for persisted Case business-validator rules.

Each scenario proves both edges of one rule: a valid persisted baseline does
not emit the rule, one field mutation emits it, and restoring that field
removes it. Save readback and audit evidence are checked on both mutations.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import random
import re
import sys
import time
import uuid
from dataclasses import asdict, dataclass, replace
from pathlib import Path
from typing import Any

from case_editor_input_fuzzer import (
    AUDIT_TABLES,
    NESTED_AUDIT_TABLES,
    commit_sha,
    created_row_id,
    get_path,
    object_id,
    object_identity,
    redacted,
    response_summary,
    set_path,
    snake,
    unwrap,
    values_equal,
)
from audit_trail_consistency_fuzzer import audit_key_matches, audit_log_complete
from rbac_rls_blackbox import ApiClient, guard_target


ROOT = Path(__file__).resolve().parents[1]
RULE_RE = re.compile(r'"((?:ICH|FDA|MFDS)\.[A-Za-z0-9_.-]+)"')
INPUT_ONLY_SUFFIXES = (".LENGTH.MAX", ".ALLOWED.VALUE", ".NULLFLAVOR.ALLOWED")
RETAINED_ALLOWED_VALUE_RULES = {
    "ICH.C.1.6.1.r.2.ALLOWED.VALUE",
    "ICH.C.4.r.2.ALLOWED.VALUE",
    "ICH.D.6.NULLFLAVOR.ALLOWED",
    "ICH.D.7.1.r.1a.ALLOWED.VALUE",
    "ICH.D.10.7.1.r.1a.ALLOWED.VALUE",
    "ICH.E.i.2.1a.ALLOWED.VALUE",
    "ICH.F.r.2.2a.ALLOWED.VALUE",
    "ICH.G.k.7.r.2a.ALLOWED.VALUE",
    "ICH.H.3.r.1a.ALLOWED.VALUE",
}
GENERATED_BUSINESS_RULE_CODES = {
    *(f"ICH.E.i.3.2{suffix}.REQUIRED" for suffix in "abcdef"),
    *(
        f"ICH.{section}.r.{suffix}{part}.{kind}"
        for section in ("D.8", "D.10.8")
        for suffix in ("6", "7")
        for part in ("a", "b")
        for kind in ("REQUIRED", "VOCABULARY")
    ),
    *(
        f"ICH.D.9.{group}.r.{field}.{kind}"
        for group in ("2", "4")
        for field, kinds in (("1a", ("REQUIRED", "VOCABULARY")), ("1b", ("REQUIRED", "VOCABULARY")), ("2", ("REQUIRED",)))
        for kind in kinds
    ),
}

MEDDRA_CODE_SCENARIO_IDS = {
    "d-history-code-vocabulary",
    "d-parent-history-code-vocabulary",
    "d-past-indication-code-vocabulary",
    "d-past-reaction-code-vocabulary",
    "d-parent-past-indication-code-vocabulary",
    "d-parent-past-reaction-code-vocabulary",
    "d-reported-cause-code-vocabulary",
    "d-autopsy-cause-code-vocabulary",
    "e-reaction-code-vocabulary",
    "f-test-code-vocabulary",
    "g-indication-code-vocabulary",
    "h-diagnosis-code-vocabulary",
}

MEDDRA_CONTEXT_SCENARIO_IDS = {
    "d-family-history-allowed-value",
    "reactions-collection-required",
    "f-test-meddra-code-required",
    "reaction-seriousness-null-flavor-ni-only",
    "mfds-test-date-null-flavor-vocabulary",
}

MEDDRA_VERSION_SCENARIO_PATHS = {
    "d-history-version-vocabulary": "patientInformation.medicalHistory.0.meddraVersion",
    "d-parent-history-version-vocabulary": "patientInformation.parents.0.medicalHistory.0.meddraVersion",
    "d-past-indication-version-vocabulary": "patientInformation.pastDrugHistory.0.indicationMeddraVersion",
    "d-past-reaction-version-vocabulary": "patientInformation.pastDrugHistory.0.reactionMeddraVersion",
    "d-parent-past-indication-version-vocabulary": "patientInformation.parents.0.pastDrugs.0.indicationMeddraVersion",
    "d-parent-past-reaction-version-vocabulary": "patientInformation.parents.0.pastDrugs.0.reactionMeddraVersion",
    "d-reported-cause-version-vocabulary": "patientInformation.death.reportedCauses.0.meddraVersion",
    "d-autopsy-cause-version-vocabulary": "patientInformation.death.autopsyCauses.0.meddraVersion",
    "e-reaction-version-vocabulary": "reactions.0.reactionMeddraVersion",
    "f-test-version-vocabulary": "testResults.0.testMeddraVersion",
    "g-indication-version-vocabulary": "drugs.0.indications.0.indicationMeddraVersion",
    "h-diagnosis-version-vocabulary": "narrative.senderDiagnoses.0.diagnosisMeddraVersion",
}

EXPECTED_SCENARIO_ISSUE_PATHS = {
    "d-concomitant-therapy-allowed-value": "patientInformation.concomitantTherapy",
    "d-family-history-allowed-value": "patientInformation.medicalHistory.0.familyHistory",
    "d-parent-age-unit-allowed-value": "patientInformation.parents.0.parentAgeUnit",
    "g-investigational-product-allowed-value": "drugs.0.investigationalProductBlinded",
    "g-dosage-frequency-unit-vocabulary": "drugs.0.dosageInformation.0.frequencyUnit",
    "c1-other-identifier-format": "otherCaseIdentifiers.0.caseIdentifier",
    "d-patient-height-integer": "patientInformation.heightCm",
    "d-parent-height-integer": "patientInformation.parents.0.heightCm",
    "d-lmp-null-flavor-allowed": "patientInformation.lastMenstrualPeriodDateNullFlavor",
    "g-drug-characterization-required": "drugs.0.drugCharacterization",
    "g-medicinal-product-required": "drugs.0.medicinalProduct",
    "fda-postmarket-cross-report-forbidden": (
        "studyInformation.0.fdaCrossReportedIndNumbers.0.indNumber"
    ),
    "primary-sources-collection-required": "primarySources",
    "drugs-collection-required": "drugs",
    "f-test-name-group-required": "testResults.0.testName",
    "f-test-name-text-required": "testResults.0.testName",
    "c1-safety-report-id-required": "safetyReportIdentification.safetyReportId",
    "c1-transmission-date-required": "safetyReportIdentification.transmissionDate",
    "reactions-collection-required": "reactions",
    "f-test-meddra-code-required": "testResults.0.testMeddraCode",
    "reaction-seriousness-null-flavor-ni-only": "reactions.0.seriousnessCriteria",
    "mfds-test-date-null-flavor-vocabulary": "testResults.0.testDateNullFlavor",
    "fda-device-collection-required": "drugs.0.fdaDevices",
    "fda-assessment-collection-required": "drugs.0.drugReactionAssessments",
}

EXPECTED_SCENARIO_ISSUE_MESSAGES = {
    "d-concomitant-therapy-allowed-value": "Dictionary allowed values constraint.",
    "d-family-history-allowed-value": "Dictionary allowed values constraint.",
    "d-parent-age-unit-allowed-value": (
        "[D.10.2.2b] Parent age unit must be years (a) or decades (10.a)."
    ),
    "g-investigational-product-allowed-value": "Dictionary allowed values constraint.",
    "g-dosage-frequency-unit-vocabulary": "Dictionary allowed values constraint.",
    "c1-other-identifier-format": (
        "[C.1.9.1.r.2] Case identifier must use country code-company or regulator name-report number format."
    ),
}

UCUM_FREQUENCY_SCENARIO_ID = "g-dosage-frequency-unit-vocabulary"

SUPPLEMENTAL_RETAINED_SCENARIO_IDS = {
    "d-concomitant-therapy-allowed-value",
    "d-family-history-allowed-value",
    "d-parent-age-unit-allowed-value",
    "g-investigational-product-allowed-value",
}

XML_BOUNDARY_SCENARIO_IDS = {
    "n-batch-number-required",
    "n-batch-sender-required",
    "n-batch-receiver-required",
    "n-batch-transmission-required",
    "n-batch-transmission-future",
    "n-message-sender-required",
    "n-message-receiver-required",
    "fda-postmarket-batch-route",
    "fda-premarket-batch-route",
    "fda-postmarket-message-route",
    "fda-premarket-message-route",
    "fda-sender-route-pair",
    "fda-vaers-route-pair",
    "mfds-batch-receiver-route",
    "mfds-message-receiver-route",
    "mfds-receiver-route-pair",
}

MEDDRA_FIELDS_BY_OWNER = {
    "medicalHistoryEpisodes": (("meddraVersion", "meddraCode"),),
    "parentMedicalHistory": (("meddraVersion", "meddraCode"),),
    "pastDrugHistory": (
        ("indicationMeddraVersion", "indicationMeddraCode"),
        ("reactionMeddraVersion", "reactionMeddraCode"),
    ),
    "parentPastDrugs": (
        ("indicationMeddraVersion", "indicationMeddraCode"),
        ("reactionMeddraVersion", "reactionMeddraCode"),
    ),
    "reportedCauses": (("meddraVersion", "meddraCode"),),
    "autopsyCauses": (("meddraVersion", "meddraCode"),),
    "reaction": (("reactionMeddraVersionLLT", "reactionMeddraCodeLLT"),),
    "testResult": (("testMeddraVersion", "testMeddraCode"),),
    "drug": (("indications[].indicationMeddraVersion", "indications[].indicationMeddraCode"),),
    "senderDiagnoses": (("diagnosisMeddraVersion", "diagnosisMeddraCode"),),
}

WHODRUG_VERSION_SCENARIOS = {
    "mfds-d-past-product-version-required": "MFDS.D.8.r.1.KR.1b.VOCABULARY",
    "mfds-d-parent-past-product-version-required": "MFDS.D.10.8.r.1.KR.1b.VOCABULARY",
    "mfds-g-product-version-required": "MFDS.G.k.2.1.KR.1b.VOCABULARY",
}

WHODRUG_CODE_FIELDS = {
    "pastDrugHistory": "mfdsMedicinalProductId",
    "parentPastDrugs": "mfdsMedicinalProductId",
    "drug": "mfdsMpid",
}

DISPOSITION_GROUPS = {
    "EXTERNAL_VOCABULARY_FIXTURE": (
        "The rule is conditional on an active MedDRA, WHO Drug, EDQM, or MFDS reference-data release; a clean isolated database intentionally has no authoritative release to invent.",
        {
            "ICH.D.10.7.1.r.1a.VOCABULARY", "ICH.D.10.7.1.r.1b.VOCABULARY",
            "ICH.D.10.8.r.6a.VOCABULARY", "ICH.D.10.8.r.6b.VOCABULARY",
            "ICH.D.10.8.r.7a.VOCABULARY", "ICH.D.10.8.r.7b.VOCABULARY",
            "ICH.D.7.1.r.1a.VOCABULARY", "ICH.D.7.1.r.1b.VOCABULARY",
            "ICH.D.8.r.6a.VOCABULARY", "ICH.D.8.r.6b.VOCABULARY",
            "ICH.D.8.r.7a.VOCABULARY", "ICH.D.8.r.7b.VOCABULARY",
            "ICH.D.9.2.r.1a.VOCABULARY", "ICH.D.9.2.r.1b.VOCABULARY",
            "ICH.D.9.4.r.1a.VOCABULARY", "ICH.D.9.4.r.1b.VOCABULARY",
            "ICH.E.i.2.1a.VOCABULARY", "ICH.E.i.2.1b.VOCABULARY",
            "ICH.F.r.2.2a.VOCABULARY", "ICH.F.r.2.2b.VOCABULARY",
            "ICH.G.k.2.3.r.3b.VOCABULARY", "ICH.G.k.4.r.3.VOCABULARY",
            "ICH.G.k.7.r.2a.VOCABULARY", "ICH.G.k.7.r.2b.VOCABULARY",
            "ICH.H.3.r.1a.VOCABULARY", "ICH.H.3.r.1b.VOCABULARY",
            "MFDS.D.10.8.r.1.KR.1b.VOCABULARY", "MFDS.D.8.r.1.KR.1b.VOCABULARY",
            "MFDS.G.k.2.1.KR.1b.VOCABULARY", "MFDS.G.k.2.3.r.1.KR.1b.VOCABULARY",
        },
    ),
    "REFERENCE_DATA_DEPENDENT_EDGE": (
        "The valid edge requires an active receiver-specific MFDS product or substance record, so it must run with a separately versioned regulatory dictionary fixture.",
        {
            "MFDS.D.10.8.r.1.KR.1a.REQUIRED", "MFDS.D.10.8.r.1.KR.1b.REQUIRED",
            "MFDS.D.8.r.1.KR.1a.REQUIRED", "MFDS.D.8.r.1.KR.1b.REQUIRED",
            "MFDS.G.k.2.1.KR.1a.REQUIRED", "MFDS.G.k.2.1.KR.1b.REQUIRED",
            "MFDS.G.k.2.3.r.1.KR.1a.REQUIRED", "MFDS.G.k.2.3.r.1.KR.1b.REQUIRED",
        },
    ),
    "INPUT_CONTRACT_OR_PERSISTENCE_GUARD": (
        "The invalid state is rejected or normalized before persistence, so it cannot be a persisted business-validator edge with readback and audit evidence.",
        set(),
    ),
    "SERVER_MANAGED_OR_DEFERRED_SINGLETON": (
        "The API creates, defaults, or deliberately defers this singleton field, so the missing state cannot be retained by a one-field editor save.",
        {"ICH.D.1.REQUIRED", "ICH.H.1.REQUIRED"},
    ),
    "COLLECTION_TOPOLOGY_NOT_FIELD_MUTATION": (
        "The violation is absence or presence of an entire repeating row/section rather than a mutation of one existing field, which is outside this fuzzer's one-field contract.",
        {
            "FDA.W0001", "FDA.W0002", "FDA.W0010",
            "ICH.D.1.1.4.REQUIRED", "MFDS.C.5.1.r.1.NULLFLAVOR.FORBIDDEN",
            "MFDS.C.5.1.r.1.RECEIVER.REQUIRED", "MFDS.D.1.1.4.REQUIRED",
        },
    ),
    "SPECIALIZED_DEVICE_SUBRESOURCE": (
        "The predicate depends on FDA device rows/codes loaded through the dedicated device child model, not a Case editor field owned by this fuzzer.",
        {
            "FDA.D.1.R0027", "FDA.G.K.12.R.3.REQUIRED", "FDA.G.K.12.REQUIRED",
            "FDA.G.k.12.r.4-6.AT_LEAST_ONE", "FDA.R0072", "FDA.W0007",
        },
    ),
    "COEMITTED_MIRROR_WARNING": (
        "This warning is emitted by the same predicate as the covered blocking route rule FDA.R0069; a second identical mutation would add no independent edge.",
        {"FDA.W0005"},
    ),
}

TEST_BACKED_RULES = {
}


def rule_dispositions(root: Path = ROOT) -> dict[str, dict[str, str]]:
    dispositions: dict[str, dict[str, str]] = {}
    integrated = {
        scenario.expected_code
        for scenario in (
            reference_vocabulary_scenarios(0) + reference_required_scenarios(0)
            + device_integration_scenarios(0) + mirror_warning_scenarios(0)
            + topology_integration_scenarios(0)
            + singleton_integration_scenarios(0)
        )
    }
    for category, (reason, codes) in DISPOSITION_GROUPS.items():
        for code in codes:
            if code in integrated or code in TEST_BACKED_RULES:
                continue
            section = code.split(".", 2)[1].lower()
            if section not in "cdefghn":
                section = "g" if code.startswith("FDA.G") else "d"
            evidence = root / f"crates/libs/validator/src/case/sections/{section}.rs"
            for name in "cdefghn":
                candidate = root / f"crates/libs/validator/src/case/sections/{name}.rs"
                if code in candidate.read_text().split("\n#[cfg(test)]", 1)[0]:
                    evidence = candidate
                    break
            dispositions[code] = {
                "category": category,
                "reason": reason,
                "evidence": str(evidence),
            }
    return dispositions


@dataclass(frozen=True)
class Scenario:
    ordinal: int
    scenario_id: str
    authority: str
    page: str
    owner: str
    field: str
    projection_field: str
    expected_code: str
    invalid_value: Any
    valid_value: Any
    fixture_values: tuple[tuple[str, Any], ...] = ()
    ci_values: tuple[tuple[str, Any], ...] = ()
    header_values: tuple[tuple[str, Any], ...] = ()
    study_values: tuple[tuple[str, Any], ...] = ()
    readback_values: tuple[Any, Any] | None = None
    reaction_values: tuple[tuple[str, Any], ...] = ()
    reference_fixture: bool = False
    surface: str = "editor"
    generator_family: str | None = None
    sample_ordinal: int | None = None
    generation_token: str | None = None
    generation_fingerprint: str | None = None


@dataclass
class Event:
    kind: str
    scenario_id: str | None
    scenario_ordinal: int | None
    sample_ordinal: int | None
    generator_family: str | None
    generation_fingerprint: str | None
    classification: str
    http_status: int | None
    response: dict[str, Any]


def discover_business_rule_codes(root: Path = ROOT) -> set[str]:
    codes: set[str] = set()
    section_root = root / "crates/libs/validator/src/case/sections"
    for name in "cdefgh":
        path = section_root / f"{name}.rs"
        production_source = path.read_text().split("\n#[cfg(test)]", 1)[0]
        codes.update(RULE_RE.findall(production_source))
    codes.update(GENERATED_BUSINESS_RULE_CODES)
    return {
        code
        for code in codes
        if not code.endswith((".", ".r"))
        and code != "ICH.E.i.3.2"
        and (
            code in RETAINED_ALLOWED_VALUE_RULES
            or not code.endswith(INPUT_ONLY_SUFFIXES)
        )
    }


def scenario_catalog(seed: int) -> list[Scenario]:
    rng = random.Random(seed)
    year = rng.randint(2020, 2024)
    suffix = f"{rng.getrandbits(32):08x}"
    scenarios = [
        Scenario(
            0,
            "c1-received-date-order",
            "ich",
            "CI",
            "safetyReportIdentification",
            "dateOfMostRecentInformation",
            "dateOfMostRecentInformation",
            "ICH.C.1.4.AFTER_C.1.5.FORBIDDEN",
            f"{year}0302",
            f"{year}0304",
        ),
        Scenario(
            1,
            "g-mpid-phpid-exclusive",
            "ich",
            "DG",
            "drug",
            "phpid",
            "phpid",
            "ICH.G.k.2.1.MPID_PHPID.EXCLUSIVE",
            f"PHPID-{suffix}",
            None,
        ),
        Scenario(
            2,
            "g-cumulative-dose-unit-required",
            "ich",
            "DG",
            "drug",
            "cumulativeDoseUnit",
            "cumulative_dose_first_reaction_unit",
            "ICH.G.k.5b.REQUIRED",
            None,
            "mg",
        ),
        Scenario(
            3,
            "g-cumulative-dose-value-required",
            "ich",
            "DG",
            "drug",
            "cumulativeDoseValue",
            "cumulative_dose_first_reaction_value",
            "ICH.G.k.5a.REQUIRED",
            None,
            12.5,
        ),
        Scenario(
            4,
            "g-suspect-drug-aggregate-required",
            "ich",
            "DG",
            "drug",
            "drugCharacterization",
            "drugCharacterization",
            "ICH.G.k.1.AGGREGATE.REQUIRED",
            "2",
            "1",
        ),
        Scenario(
            5,
            "g-authorization-country-required",
            "ich",
            "DG",
            "drug",
            "drugAuthorizationCountry",
            "manufacturer_country",
            "ICH.G.k.3.2.REQUIRED",
            None,
            "KR",
        ),
        Scenario(
            6,
            "c1-report-type-required",
            "ich",
            "CI",
            "safetyReportIdentification",
            "reportType",
            "reportType",
            "ICH.C.1.3.REQUIRED",
            None,
            "1",
        ),
        Scenario(
            7,
            "c1-first-received-required",
            "ich",
            "CI",
            "safetyReportIdentification",
            "dateFirstReceivedFromSource",
            "dateFirstReceivedFromSource",
            "ICH.C.1.4.REQUIRED",
            None,
            f"{year}0303",
        ),
        Scenario(
            8,
            "c1-most-recent-required",
            "ich",
            "CI",
            "safetyReportIdentification",
            "dateOfMostRecentInformation",
            "dateOfMostRecentInformation",
            "ICH.C.1.5.REQUIRED",
            None,
            f"{year}0304",
        ),
        Scenario(
            9,
            "c1-transmission-date-future",
            "ich",
            "CI",
            "safetyReportIdentification",
            "transmissionDate",
            "transmissionDate",
            "ICH.C.1.2.FUTURE_DATE.FORBIDDEN",
            "29990305120000+0900",
            f"{year}0305120000+0900",
        ),
        Scenario(
            10,
            "c1-first-received-future",
            "ich",
            "CI",
            "safetyReportIdentification",
            "dateFirstReceivedFromSource",
            "dateFirstReceivedFromSource",
            "ICH.C.1.4.FUTURE_DATE.FORBIDDEN",
            "29990303",
            f"{year}0303",
        ),
        Scenario(
            11,
            "c1-most-recent-future",
            "ich",
            "CI",
            "safetyReportIdentification",
            "dateOfMostRecentInformation",
            "dateOfMostRecentInformation",
            "ICH.C.1.5.FUTURE_DATE.FORBIDDEN",
            "29990304",
            f"{year}0304",
        ),
        Scenario(
            12,
            "g-active-ingredient-required",
            "ich",
            "DG",
            "drug",
            "mpid",
            "mpid",
            "ICH.G.k.2.3.r.REQUIRED",
            None,
            f"MPID-{suffix}",
        ),
        Scenario(
            13,
            "g-gestation-value-required",
            "ich",
            "DG",
            "drug",
            "gestationPeriodExposureUnit",
            "gestation_period_exposure_unit",
            "ICH.G.k.6a.REQUIRED",
            "week",
            None,
        ),
        Scenario(
            14,
            "g-gestation-unit-required",
            "ich",
            "DG",
            "drug",
            "gestationPeriodExposureValue",
            "gestation_period_exposure_value",
            "ICH.G.k.6b.REQUIRED",
            2,
            None,
        ),
        Scenario(
            15,
            "g-investigational-product-study-only",
            "ich",
            "DG",
            "drug",
            "investigationalProductBlinded",
            "investigational_product_blinded",
            "ICH.G.k.2.5.STUDY.ONLY",
            False,
            None,
        ),
        Scenario(
            16,
            "g-dosage-dose-unit-required",
            "ich",
            "DG",
            "drug",
            "dosageInformation[].doseUnit",
            "dosageInformation.dose_unit",
            "ICH.G.k.4.r.1b.REQUIRED",
            None,
            "mg",
            (("dosageInformation[].doseValue", 2.5),),
        ),
        Scenario(
            17,
            "g-dosage-frequency-unit-required",
            "ich",
            "DG",
            "drug",
            "dosageInformation[].frequencyUnit",
            "dosageInformation.frequency_unit",
            "ICH.G.k.4.r.3.REQUIRED",
            None,
            "d",
            (("dosageInformation[].numberOfUnits", 1),),
        ),
        Scenario(
            18,
            "g-dosage-duration-value-required",
            "ich",
            "DG",
            "drug",
            "dosageInformation[].durationValue",
            "dosageInformation.duration_value",
            "ICH.G.k.4.r.6a.REQUIRED",
            None,
            2,
            (("dosageInformation[].durationUnit", "d"),),
        ),
        Scenario(
            19,
            "g-dosage-duration-unit-required",
            "ich",
            "DG",
            "drug",
            "dosageInformation[].durationUnit",
            "dosageInformation.duration_unit",
            "ICH.G.k.4.r.6b.REQUIRED",
            None,
            "d",
            (("dosageInformation[].durationValue", 2),),
        ),
        Scenario(
            20,
            "g-dose-form-term-version-required",
            "ich",
            "DG",
            "drug",
            "dosageInformation[].doseFormTermIdVersion",
            "dosageInformation.dose_form_termid_version",
            "ICH.G.k.4.r.9.2a.REQUIRED",
            None,
            "1",
            (("dosageInformation[].doseFormTermId", "DF-1"),),
        ),
        Scenario(
            22,
            "g-parent-route-term-version-required",
            "ich",
            "DG",
            "drug",
            "dosageInformation[].parentRouteTermIdVersion",
            "dosageInformation.parent_route_termid_version",
            "ICH.G.k.4.r.11.2a.REQUIRED",
            None,
            "1",
            (("dosageInformation[].parentRouteTermId", "PROUTE-1"),),
        ),
        Scenario(
            23,
            "g-active-substance-name-required",
            "ich",
            "DG",
            "drug",
            "activeSubstances[].substanceName",
            "activeSubstances.substance_name",
            "ICH.G.k.2.3.r.1.REQUIRED",
            None,
            "Fuzz substance",
            (
                ("mpid", None),
                ("activeSubstances[].substanceStrengthValue", 10),
                ("activeSubstances[].substanceStrengthUnit", "mg"),
            ),
        ),
        Scenario(
            24,
            "g-active-substance-term-version-required",
            "ich",
            "DG",
            "drug",
            "activeSubstances[].substanceTermIdVersion",
            "activeSubstances.substance_termid_version",
            "ICH.G.k.2.3.r.2a.REQUIRED",
            None,
            "1",
            (("activeSubstances[].substanceTermId", "SUBSTANCE-1"),),
        ),
        Scenario(
            25,
            "g-active-substance-strength-unit-required",
            "ich",
            "DG",
            "drug",
            "activeSubstances[].substanceStrengthUnit",
            "activeSubstances.strength_unit",
            "ICH.G.k.2.3.r.3b.REQUIRED",
            None,
            "mg",
            (("activeSubstances[].substanceStrengthValue", 10),),
        ),
        Scenario(
            26,
            "g-indication-meddra-version-required",
            "ich",
            "DG",
            "drug",
            "indications[].indicationMeddraVersion",
            "indications.indication_meddra_version",
            "ICH.G.k.7.r.2a.REQUIRED",
            None,
            "26.0",
            (("indications[].indicationMeddraCode", "10000001"),),
        ),
        Scenario(
            27,
            "g-indication-meddra-code-required",
            "ich",
            "DG",
            "drug",
            "indications[].indicationMeddraCode",
            "indications.indication_meddra_code",
            "ICH.G.k.7.r.2b.REQUIRED",
            None,
            "10000001",
            (("indications[].indicationMeddraVersion", "26.0"),),
        ),
        Scenario(
            28,
            "g-dosage-future-date-forbidden",
            "ich",
            "DG",
            "drug",
            "dosageInformation[].firstAdministrationDate",
            "dosageInformation.first_administration_date",
            "ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN",
            "29990303",
            f"{year}0303",
        ),
        Scenario(29, "e-reaction-language-required", "ich", "AE", "reaction", "reactionLanguage", "reaction_language", "ICH.E.i.1.1b.REQUIRED", None, "eng"),
        Scenario(30, "e-reaction-meddra-version-required", "ich", "AE", "reaction", "reactionMeddraVersionLLT", "reaction_meddra_version", "ICH.E.i.2.1a.REQUIRED", None, "26.0"),
        Scenario(31, "e-reaction-meddra-code-required", "ich", "AE", "reaction", "reactionMeddraCodeLLT", "reaction_meddra_code", "ICH.E.i.2.1b.REQUIRED", None, "10000001"),
        Scenario(32, "e-reaction-future-date-forbidden", "ich", "AE", "reaction", "reactionStartDate", "start_date", "ICH.E.i.4-5.FUTURE_DATE.FORBIDDEN", "29990303", f"{year}0303"),
        Scenario(33, "e-reaction-duration-value-required", "ich", "AE", "reaction", "reactionDuration.value", "duration_value", "ICH.E.i.6a.REQUIRED", None, "1", (("reactionDuration.unit", "d"),)),
        Scenario(34, "e-reaction-duration-unit-required", "ich", "AE", "reaction", "reactionDuration.unit", "duration_unit", "ICH.E.i.6b.REQUIRED", None, "d", (("reactionDuration.value", "1"),)),
        Scenario(35, "e-reaction-outcome-required", "ich", "AE", "reaction", "reactionOutcome", "outcome", "ICH.E.i.7.REQUIRED", None, "1"),
        Scenario(
            36,
            "e-seriousness-criteria-required",
            "ich",
            "AE",
            "reaction",
            "seriousness.serious",
            "serious",
            "ICH.E.i.3.2.CRITERIA.REQUIRED",
            True,
            False,
            tuple((f"seriousness.{field}", None) for field in (
                "criteriaResultsInDeath",
                "criteriaLifeThreatening",
                "criteriaHospitalization",
                "criteriaDisabling",
                "criteriaCongenitalAnomaly",
                "criteriaOtherMedicallyImportant",
            )),
        ),
        Scenario(37, "e-criteria-death-required", "ich", "AE", "reaction", "seriousness.criteriaResultsInDeath", "criteria_death", "ICH.E.i.3.2a.REQUIRED", None, True),
        Scenario(38, "e-criteria-life-threatening-required", "ich", "AE", "reaction", "seriousness.criteriaLifeThreatening", "criteria_life_threatening", "ICH.E.i.3.2b.REQUIRED", None, True),
        Scenario(39, "e-criteria-hospitalization-required", "ich", "AE", "reaction", "seriousness.criteriaHospitalization", "criteria_hospitalization", "ICH.E.i.3.2c.REQUIRED", None, True),
        Scenario(40, "e-criteria-disabling-required", "ich", "AE", "reaction", "seriousness.criteriaDisabling", "criteria_disabling", "ICH.E.i.3.2d.REQUIRED", None, True),
        Scenario(41, "e-criteria-congenital-required", "ich", "AE", "reaction", "seriousness.criteriaCongenitalAnomaly", "criteria_congenital_anomaly", "ICH.E.i.3.2e.REQUIRED", None, True),
        Scenario(42, "e-criteria-medically-important-required", "ich", "AE", "reaction", "seriousness.criteriaOtherMedicallyImportant", "criteria_other_medically_important", "ICH.E.i.3.2f.REQUIRED", None, True),
        Scenario(43, "f-test-date-required", "ich", "LB", "testResult", "testDate", "test_date", "ICH.F.r.1.REQUIRED", None, f"{year}0303"),
        Scenario(44, "f-test-future-date-forbidden", "ich", "LB", "testResult", "testDate", "test_date", "ICH.F.r.1.FUTURE_DATE.FORBIDDEN", "29990303", f"{year}0303"),
        Scenario(45, "f-test-meddra-version-required", "ich", "LB", "testResult", "testMeddraVersion", "test_meddra_version", "ICH.F.r.2.2a.REQUIRED", None, "26.0"),
        Scenario(46, "f-test-result-unit-required", "ich", "LB", "testResult", "testUnit", "test_result_unit", "ICH.F.r.3.3.REQUIRED", None, "mg/dL"),
        Scenario(47, "f-coded-result-required", "ich", "LB", "testResult", "testResultCode", "test_result_code", "ICH.F.r.3.1.REQUIRED", None, "1", (("testResult", None), ("testResultUnstructured", None))),
        Scenario(48, "f-value-result-required", "ich", "LB", "testResult", "testResult", "test_result_value", "ICH.F.r.3.2.REQUIRED", None, "12.5", (("testResultCode", None), ("testResultUnstructured", None))),
        Scenario(49, "f-unstructured-result-required", "ich", "LB", "testResult", "testResultUnstructured", "result_unstructured", "ICH.F.r.3.4.REQUIRED", None, "Normal", (("testResultCode", None), ("testResult", None))),
        Scenario(50, "d-patient-age-exclusive", "ich", "DM", "patientInformation", "patientAgeGroup", "age_group", "ICH.D.2.EXCLUSIVE", "5", None, (("patientBirthDate", f"{year}0303"),)),
        Scenario(51, "d-patient-birth-date-future", "ich", "DM", "patientInformation", "patientBirthDate", "birth_date", "ICH.D.2.1.FUTURE_DATE.FORBIDDEN", "29990303", f"{year}0303"),
        Scenario(52, "d-patient-age-value-required", "ich", "DM", "patientInformation", "patientAge.value", "age_at_time_of_onset", "ICH.D.2.2a.REQUIRED", None, 36.5, (("patientAge.unit", "a"),)),
        Scenario(53, "d-patient-age-unit-required", "ich", "DM", "patientInformation", "patientAge.unit", "age_unit", "ICH.D.2.2b.REQUIRED", None, "a", (("patientAge.value", 36.5),)),
        Scenario(54, "d-patient-gestation-value-required", "ich", "DM", "patientInformation", "gestationPeriod.value", "gestation_period", "ICH.D.2.2.1a.REQUIRED", None, 22, (("gestationPeriod.unit", "wk"),)),
        Scenario(55, "d-patient-gestation-unit-required", "ich", "DM", "patientInformation", "gestationPeriod.unit", "gestation_period_unit", "ICH.D.2.2.1b.REQUIRED", None, "wk", (("gestationPeriod.value", 22),)),
        Scenario(56, "d-patient-lmp-future", "ich", "DM", "patientInformation", "lastMenstrualPeriodDate", "last_menstrual_period_date", "ICH.D.6.FUTURE_DATE.FORBIDDEN", "29990303", f"{year}0303"),
        Scenario(57, "h-diagnosis-meddra-version-required", "ich", "NR", "senderDiagnoses", "diagnosisMeddraVersion", "diagnosis_meddra_version", "ICH.H.3.r.1a.REQUIRED", None, "26.0"),
        Scenario(58, "h-diagnosis-meddra-code-required", "ich", "NR", "senderDiagnoses", "diagnosisMeddraCode", "diagnosis_meddra_code", "ICH.H.3.r.1b.REQUIRED", None, "10000001"),
        Scenario(59, "h-case-summary-language-required", "ich", "NR", "caseSummaryInformation", "languageCode", "language_code", "ICH.H.5.r.1b.REQUIRED", None, "eng"),
        Scenario(60, "d-medical-history-text-required", "ich", "DM", "patientInformation", "medicalHistoryText", "medical_history_text", "ICH.D.7.2.REQUIRED", None, "No relevant history"),
        Scenario(61, "d-history-meddra-version-required", "ich", "DM", "medicalHistoryEpisodes", "meddraVersion", "meddra_version", "ICH.D.7.1.r.1a.REQUIRED", None, "26.0"),
        Scenario(62, "d-history-meddra-code-required", "ich", "DM", "medicalHistoryEpisodes", "meddraCode", "meddra_code", "ICH.D.7.1.r.1b.REQUIRED", None, "10000001"),
        Scenario(63, "d-history-future-date-forbidden", "ich", "DM", "medicalHistoryEpisodes", "startDate", "start_date", "ICH.D.7.1.r.FUTURE_DATE.FORBIDDEN", "29990303", f"{year}0303"),
        Scenario(64, "d-past-drug-name-required", "ich", "DH", "pastDrugHistory", "drugName", "drug_name", "ICH.D.8.r.1.REQUIRED", None, "Prior drug"),
        Scenario(65, "d-past-drug-identifier-exclusive", "ich", "DH", "pastDrugHistory", "phpid", "phpid", "ICH.D.8.MPID_PHPID.EXCLUSIVE", f"PHPID-{suffix}", None),
        Scenario(66, "d-past-drug-future-date-forbidden", "ich", "DH", "pastDrugHistory", "startDate", "start_date", "ICH.D.8.r.FUTURE_DATE.FORBIDDEN", "29990303", f"{year}0303"),
        Scenario(67, "d-past-drug-indication-version-required", "ich", "DH", "pastDrugHistory", "indicationMeddraVersion", "indication_meddra_version", "ICH.D.8.r.6a.REQUIRED", None, "26.0"),
        Scenario(68, "d-past-drug-indication-code-required", "ich", "DH", "pastDrugHistory", "indicationMeddraCode", "indication_meddra_code", "ICH.D.8.r.6b.REQUIRED", None, "10000001"),
        Scenario(69, "d-past-drug-reaction-version-required", "ich", "DH", "pastDrugHistory", "reactionMeddraVersion", "reaction_meddra_version", "ICH.D.8.r.7a.REQUIRED", None, "26.0"),
        Scenario(70, "d-past-drug-reaction-code-required", "ich", "DH", "pastDrugHistory", "reactionMeddraCode", "reaction_meddra_code", "ICH.D.8.r.7b.REQUIRED", None, "10000001"),
        Scenario(71, "d-death-date-future", "ich", "DM", "deathInfo", "dateOfDeath", "date_of_death", "ICH.D.9.1.FUTURE_DATE.FORBIDDEN", "29990303", f"{year}0303"),
        Scenario(72, "d-autopsy-performed-required", "ich", "DM", "deathInfo", "autopsyPerformed", "autopsy_performed", "ICH.D.9.3.REQUIRED", None, True, (("dateOfDeath", f"{year}0303"),)),
        Scenario(73, "d-reported-cause-version-required", "ich", "DM", "reportedCauses", "meddraVersion", "meddra_version", "ICH.D.9.2.r.1a.REQUIRED", None, "26.0"),
        Scenario(74, "d-reported-cause-code-required", "ich", "DM", "reportedCauses", "meddraCode", "meddra_code", "ICH.D.9.2.r.1b.REQUIRED", None, "10000001"),
        Scenario(75, "d-reported-cause-text-required", "ich", "DM", "reportedCauses", "causeText", "comments", "ICH.D.9.2.r.2.REQUIRED", None, "Reported cause"),
        Scenario(76, "d-autopsy-cause-version-required", "ich", "DM", "autopsyCauses", "meddraVersion", "meddra_version", "ICH.D.9.4.r.1a.REQUIRED", None, "26.0"),
        Scenario(77, "d-autopsy-cause-code-required", "ich", "DM", "autopsyCauses", "meddraCode", "meddra_code", "ICH.D.9.4.r.1b.REQUIRED", None, "10000001"),
        Scenario(78, "d-autopsy-cause-text-required", "ich", "DM", "autopsyCauses", "causeText", "comments", "ICH.D.9.4.r.2.REQUIRED", None, "Autopsy cause"),
        Scenario(79, "d-parent-birth-date-future", "ich", "DM", "parentInfo", "parentBirthDate", "parent_birth_date", "ICH.D.10.2.1.FUTURE_DATE.FORBIDDEN", "29990303", f"{year}0303"),
        Scenario(80, "d-parent-age-value-required", "ich", "DM", "parentInfo", "parentAge.value", "parent_age", "ICH.D.10.2.2a.REQUIRED", None, 54, (("parentAge.unit", "a"),)),
        Scenario(81, "d-parent-age-unit-required", "ich", "DM", "parentInfo", "parentAge.unit", "parent_age_unit", "ICH.D.10.2.2b.REQUIRED", None, "a", (("parentAge.value", 54),)),
        Scenario(82, "d-parent-birth-age-exclusive", "ich", "DM", "parentInfo", "parentBirthDate", "parent_birth_date", "ICH.D.10.2.EXCLUSIVE", f"{year}0303", None, (("parentAge.value", 54), ("parentAge.unit", "a"))),
        Scenario(83, "d-parent-lmp-future", "ich", "DM", "parentInfo", "parentLastMenstrualPeriodDate", "last_menstrual_period_date", "ICH.D.10.3.FUTURE_DATE.FORBIDDEN", "29990303", f"{year}0303"),
        Scenario(84, "d-parent-sex-required", "ich", "DM", "parentInfo", "parentSex", "sex", "ICH.D.10.6.REQUIRED", None, "2", (("parentIdentification", "FUZZ-PARENT"),)),
        Scenario(85, "d-parent-history-version-required", "ich", "DM", "parentMedicalHistory", "meddraVersion", "meddra_version", "ICH.D.10.7.1.r.1a.REQUIRED", None, "26.0"),
        Scenario(86, "d-parent-history-code-required", "ich", "DM", "parentMedicalHistory", "meddraCode", "meddra_code", "ICH.D.10.7.1.r.1b.REQUIRED", None, "10000001"),
        Scenario(87, "d-parent-history-future-date", "ich", "DM", "parentMedicalHistory", "startDate", "start_date", "ICH.D.10.7.1.r.FUTURE_DATE.FORBIDDEN", "29990303", f"{year}0303"),
        Scenario(88, "d-parent-past-drug-mpid-version-required", "ich", "DM", "parentPastDrugs", "mpidVersion", "mpid_version", "ICH.D.10.8.r.2a.REQUIRED", None, "1"),
        Scenario(89, "d-parent-past-drug-phpid-version-required", "ich", "DM", "parentPastDrugs", "phpidVersion", "phpid_version", "ICH.D.10.8.r.3a.REQUIRED", None, "1", (("mpid", None), ("mpidVersion", None), ("phpid", f"PHPID-{suffix}"))),
        Scenario(90, "d-parent-past-drug-future-date", "ich", "DM", "parentPastDrugs", "startDate", "start_date", "ICH.D.10.8.r.FUTURE_DATE.FORBIDDEN", "29990303", f"{year}0303"),
        Scenario(91, "d-parent-past-drug-identifier-exclusive", "ich", "DM", "parentPastDrugs", "phpid", "phpid", "ICH.D.10.8.MPID_PHPID.EXCLUSIVE", f"PHPID-{suffix}", None, (("phpidVersion", "1"),)),
        Scenario(92, "d-parent-past-drug-indication-version-required", "ich", "DM", "parentPastDrugs", "indicationMeddraVersion", "indication_meddra_version", "ICH.D.10.8.r.6a.REQUIRED", None, "26.0"),
        Scenario(93, "d-parent-past-drug-indication-code-required", "ich", "DM", "parentPastDrugs", "indicationMeddraCode", "indication_meddra_code", "ICH.D.10.8.r.6b.REQUIRED", None, "10000001"),
        Scenario(94, "d-parent-past-drug-reaction-version-required", "ich", "DM", "parentPastDrugs", "reactionMeddraVersion", "reaction_meddra_version", "ICH.D.10.8.r.7a.REQUIRED", None, "26.0"),
        Scenario(95, "d-parent-past-drug-reaction-code-required", "ich", "DM", "parentPastDrugs", "reactionMeddraCode", "reaction_meddra_code", "ICH.D.10.8.r.7b.REQUIRED", None, "10000001"),
        Scenario(96, "d-history-parent-duplicate", "ich", "DM", "medicalHistoryEpisodes", "familyHistory", "family_history", "ICH.D.7.1.r.6.PARENT_DUPLICATE", True, False),
        Scenario(97, "c1-expedited-criteria-required", "ich", "CI", "safetyReportIdentification", "fulfilExpeditedCriteria", "fulfilExpeditedCriteria", "ICH.C.1.7.REQUIRED", None, True),
        Scenario(98, "c1-other-identifiers-flag-required", "ich", "CI", "safetyReportIdentification", "otherCaseIdentifiersExist", "otherCaseIdentifiersExist", "ICH.C.1.9.1.REQUIRED", None, True),
        Scenario(99, "c1-nullification-reason-required", "ich", "CI", "safetyReportIdentification", "nullificationReason", "nullificationReason", "ICH.C.1.11.2.REQUIRED", None, "Amendment reason", (("nullificationAmendmentCode", "2"),)),
        Scenario(100, "g-assessment-administration-value-required", "ich", "DG", "drug", "drugReactionAssessments[].administrationStartIntervalValue", "drugReactionAssessments[].administrationStartIntervalValue", "ICH.G.k.9.i.3.1a.REQUIRED", None, 1.5, (("drugReactionAssessments[].administrationStartIntervalUnit", "d"),)),
        Scenario(101, "mfds-c1-additional-documents-required", "mfds", "CI", "safetyReportIdentification", "additionalDocumentsAvailable", "additionalDocumentsAvailable", "ICH.C.1.6.1.REQUIRED", None, False),
        Scenario(102, "mfds-c1-worldwide-id-required", "mfds", "CI", "safetyReportIdentification", "worldwideUniqueId", "worldwideUniqueId", "ICH.C.1.8.1.REQUIRED", None, f"KR-BUSINESS-{suffix}"),
        Scenario(103, "mfds-c1-first-sender-required", "mfds", "CI", "safetyReportIdentification", "firstSenderType", "firstSenderType", "ICH.C.1.8.2.REQUIRED", None, "1"),
        Scenario(104, "c2-reporter-country-required", "ich", "RP", "primarySources", "reporterCountry", "reporterCountry", "ICH.C.2.r.3.REQUIRED", None, "KR"),
        Scenario(105, "c2-reporter-qualification-required", "ich", "RP", "primarySources", "qualification", "qualification", "ICH.C.2.r.4.REQUIRED", None, "1"),
        Scenario(106, "c2-primary-source-required", "ich", "RP", "primarySources", "primarySourceForRegulatoryPurposes", "primarySourceForRegulatoryPurposes", "ICH.C.2.r.5.REQUIRED", None, "1"),
        Scenario(107, "c2-primary-source-exactly-once", "ich", "RP", "primarySources", "primarySourceForRegulatoryPurposes", "primarySourceForRegulatoryPurposes", "ICH.C.2.r.5.EXACTLY_ONCE", "1", None),
        Scenario(108, "c3-sender-type-required", "ich", "SD", "senderInformation", "senderType", "senderType", "ICH.C.3.1.REQUIRED", None, "1"),
        Scenario(109, "c3-sender-organization-required", "ich", "SD", "senderInformation", "organizationName", "organizationName", "ICH.C.3.2.REQUIRED", None, "Business Sender"),
        Scenario(111, "c5-study-type-reaction-required", "ich", "SI", "studyInformation", "studyTypeReaction", "study_type_reaction", "ICH.C.5.4.REQUIRED", None, "1"),
        Scenario(112, "mfds-d-past-drug-mpid-version-required", "mfds", "DH", "pastDrugHistory", "mpidVersion", "mpid_version", "MFDS.D.8.r.2a.REQUIRED", None, "1"),
        Scenario(113, "mfds-d-past-drug-mpid-required", "mfds", "DH", "pastDrugHistory", "mpid", "mpid", "MFDS.D.8.r.2b.REQUIRED", None, f"MPID-{suffix}", (("mpidVersion", "1"),)),
        Scenario(114, "mfds-d-past-drug-phpid-version-required", "mfds", "DH", "pastDrugHistory", "phpidVersion", "phpid_version", "MFDS.D.8.r.3a.REQUIRED", None, "1", (("mpid", None), ("phpid", f"PHPID-{suffix}"))),
        Scenario(115, "mfds-d-past-drug-phpid-required", "mfds", "DH", "pastDrugHistory", "phpid", "phpid", "MFDS.D.8.r.3b.REQUIRED", None, f"PHPID-{suffix}", (("mpid", None), ("mpidVersion", None), ("phpidVersion", "1"))),
        Scenario(116, "mfds-d-parent-past-drug-mpid-required", "mfds", "DM", "parentPastDrugs", "mpid", "mpid", "MFDS.D.10.8.r.2b.REQUIRED", None, f"MPID-{suffix}", (("mpidVersion", "1"),)),
        Scenario(117, "mfds-d-parent-past-drug-phpid-required", "mfds", "DM", "parentPastDrugs", "phpid", "phpid", "MFDS.D.10.8.r.3b.REQUIRED", None, f"PHPID-{suffix}", (("mpid", None), ("mpidVersion", None), ("phpidVersion", "1"))),
        Scenario(118, "g-assessment-administration-unit-required", "ich", "DG", "drug", "drugReactionAssessments[].administrationStartIntervalUnit", "drugReactionAssessments[].administrationStartIntervalUnit", "ICH.G.k.9.i.3.1b.REQUIRED", None, "d", (("drugReactionAssessments[].administrationStartIntervalValue", 1.5),)),
        Scenario(119, "g-assessment-last-dose-value-required", "ich", "DG", "drug", "drugReactionAssessments[].lastDoseIntervalValue", "drugReactionAssessments[].lastDoseIntervalValue", "ICH.G.k.9.i.3.2a.REQUIRED", None, 2.5, (("drugReactionAssessments[].lastDoseIntervalUnit", "d"),)),
        Scenario(120, "g-assessment-last-dose-unit-required", "ich", "DG", "drug", "drugReactionAssessments[].lastDoseIntervalUnit", "drugReactionAssessments[].lastDoseIntervalUnit", "ICH.G.k.9.i.3.2b.REQUIRED", None, "d", (("drugReactionAssessments[].lastDoseIntervalValue", 2.5),)),
        Scenario(121, "mfds-reaction-eu-country-forbidden", "mfds", "AE", "reaction", "reactionCountry", "country_code", "MFDS.E.i.9.EU.FORBIDDEN", "EU", "KR"),
        Scenario(122, "fda-required-intervention-required", "fda", "AE", "reaction", "requiredIntervention", "required_intervention", "FDA.E.i.3.2h.REQUIRED", None, True),
        Scenario(123, "reaction-hcp-medical-confirmation-omit", "ich", "AE", "reaction", "medicalConfirmation", "medical_confirmation", "ICH.E.i.8.HCP.OMIT", True, None),
        Scenario(124, "mfds-more-test-info-documents-required", "mfds", "LB", "testResult", "moreInformationAvailable", "more_info_available", "MFDS.F.r.7.C.1.6.1.REQUIRED", True, False),
        Scenario(125, "mfds-drug-mpid-version-required", "mfds", "DG", "drug", "mpidVersion", "mpid_version", "MFDS.G.k.2.1.1a.REQUIRED", None, "1"),
        Scenario(126, "mfds-drug-mpid-required", "mfds", "DG", "drug", "mpid", "mpid", "MFDS.G.k.2.1.1b.REQUIRED", None, f"MPID-{suffix}", (("mpidVersion", "1"),)),
        Scenario(127, "mfds-drug-phpid-version-required", "mfds", "DG", "drug", "phpidVersion", "phpid_version", "MFDS.G.k.2.1.2a.REQUIRED", None, "1", (("phpid", f"PHPID-{suffix}"),)),
        Scenario(128, "mfds-drug-phpid-required", "mfds", "DG", "drug", "phpid", "phpid", "MFDS.G.k.2.1.2b.REQUIRED", None, f"PHPID-{suffix}", (("phpidVersion", "1"),)),
        Scenario(129, "mfds-substance-term-id-required", "mfds", "DG", "drug", "activeSubstances[].substanceTermId", "activeSubstances[].substance_termid", "MFDS.G.k.2.3.r.2b.REQUIRED", None, f"SUB-{suffix}", (("activeSubstances[].substanceTermIdVersion", "1"),)),
        Scenario(130, "mfds-domestic-product-code-required", "mfds", "DG", "drug", "mfdsMpid", "mfds_mpid", "MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED", None, "KR12345678", (("obtainDrugCountry", "KR"),)),
        Scenario(131, "mfds-foreign-whompid-required", "mfds", "DG", "drug", "mfdsMpid", "mfds_mpid", "MFDS.KR.FOREIGN.WHOMPID.REQUIRED", None, "WH12345678", (("obtainDrugCountry", "US"),)),
        Scenario(132, "mfds-domestic-ingredient-code-required", "mfds", "DG", "drug", "activeSubstances[].mfdsId", "activeSubstances[].mfds_id", "MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED", None, "KS12345678", (("obtainDrugCountry", "KR"), ("activeSubstances[].substanceName", "Business ingredient"))),
        Scenario(133, "c1-document-description-required", "ich", "CI", "documentsHeldBySender", "documentDescription", "documentDescription", "ICH.C.1.6.1.r.1.REQUIRED", None, "Clinical evidence", (("includedDocument", "SGVsbG8="),)),
        Scenario(134, "c1-document-base64-required", "ich", "CI", "documentsHeldBySender", "includedDocument", "includedDocument", "ICH.C.1.6.1.r.2.ALLOWED.VALUE", "not-base64", "SGVsbG8=", (("documentDescription", "Clinical evidence"),)),
        Scenario(135, "fda-document-file-name-required", "fda", "CI", "documentsHeldBySender", "fileName", "file_name", "FDA.C.1.6.1.r.2.FILE_NAME.REQUIRED", None, "evidence.pdf", (("documentDescription", "Clinical evidence"), ("includedDocument", "SGVsbG8="), ("mediaType", "application/pdf"))),
        Scenario(136, "fda-document-media-type-match", "fda", "CI", "documentsHeldBySender", "mediaType", "media_type", "FDA.C.1.6.1.r.2.MEDIA_TYPE.MATCH", "text/plain", "application/pdf", (("documentDescription", "Clinical evidence"), ("includedDocument", "SGVsbG8="), ("fileName", "evidence.pdf"))),
        Scenario(137, "c2-study-reporter-organization-required", "ich", "RP", "primarySources", "reporterOrganization", "reporterOrganization", "ICH.C.2.r.2.1.REQUIRED", None, "Business Reporter"),
        Scenario(138, "fda-combination-indicator-required", "fda", "CI", "safetyReportIdentification", "combinationProductReportIndicator", "combinationProductReportIndicator", "FDA.C.1.12.REQUIRED", None, "false"),
        Scenario(139, "fda-combination-indicator-recommended", "fda", "CI", "safetyReportIdentification", "combinationProductReportIndicator", "combinationProductReportIndicator", "FDA.C.1.12.RECOMMENDED", None, "false"),
        Scenario(140, "fda-local-criteria-required", "fda", "CI", "safetyReportIdentification", "localCriteriaReportType", "localCriteriaReportType", "FDA.C.1.7.1.REQUIRED", None, "1"),
        Scenario(141, "fda-documents-flag-row-required", "fda", "CI", "safetyReportIdentification", "additionalDocumentsAvailable", "additionalDocumentsAvailable", "FDA.R0009", True, False),
        Scenario(142, "fda-identifiers-flag-row-required", "fda", "CI", "safetyReportIdentification", "otherCaseIdentifiersExist", "otherCaseIdentifiersExist", "FDA.R0017", True, False),
        Scenario(143, "fda-primary-qualification-required", "fda", "RP", "primarySources", "qualification", "qualification", "FDA.R0020", None, "1"),
        Scenario(144, "fda-sender-contact-required", "fda", "SD", "senderInformation", "email", "email", "FDA.C.3.SENDER.REQUIRED", None, "sender@example.com"),
        Scenario(145, "mfds-primary-qualification-required", "mfds", "RP", "primarySources", "qualification", "qualification", "MFDS.C.2.r.4.REQUIRED", None, "1"),
        Scenario(146, "d-history-meddra-version-format", "ich", "DM", "medicalHistoryEpisodes", "meddraVersion", "meddra_version", "ICH.D.7.1.r.1a.ALLOWED.VALUE", "bad", "26.0"),
        Scenario(147, "d-parent-history-meddra-version-format", "ich", "DM", "parentMedicalHistory", "meddraVersion", "meddra_version", "ICH.D.10.7.1.r.1a.ALLOWED.VALUE", "bad", "26.0"),
        Scenario(148, "e-reaction-meddra-version-format", "ich", "AE", "reaction", "reactionMeddraVersionLLT", "reaction_meddra_version", "ICH.E.i.2.1a.ALLOWED.VALUE", "bad", "26.0"),
        Scenario(149, "f-test-meddra-version-format", "ich", "LB", "testResult", "testMeddraVersion", "test_meddra_version", "ICH.F.r.2.2a.ALLOWED.VALUE", "bad", "26.0"),
        Scenario(150, "g-indication-meddra-version-format", "ich", "DG", "drug", "indications[].indicationMeddraVersion", "indications[].indication_meddra_version", "ICH.G.k.7.r.2a.ALLOWED.VALUE", "bad", "26.0", (("indications[].indicationMeddraCode", "10000001"),)),
        Scenario(151, "h-diagnosis-meddra-version-format", "ich", "NR", "senderDiagnoses", "diagnosisMeddraVersion", "diagnosis_meddra_version", "ICH.H.3.r.1a.ALLOWED.VALUE", "bad", "26.0"),
        Scenario(152, "c4-literature-base64-format", "ich", "LR", "literatureReference", "documentBase64", "document_base64", "ICH.C.4.r.2.ALLOWED.VALUE", "not-base64", "SGVsbG8="),
        Scenario(153, "fda-literature-file-name-required", "fda", "LR", "literatureReference", "fileName", "file_name", "FDA.C.4.r.2.FILE_NAME.REQUIRED", None, "article.pdf", (("documentBase64", "SGVsbG8="), ("mediaType", "application/pdf"))),
        Scenario(154, "fda-literature-media-type-match", "fda", "LR", "literatureReference", "mediaType", "media_type", "FDA.C.4.r.2.MEDIA_TYPE.MATCH", "text/plain", "application/pdf", (("documentBase64", "SGVsbG8="), ("fileName", "article.pdf"))),
        Scenario(155, "fda-race-required", "fda", "DM", "patientInformation", "raceCodeNullFlavor", "raceCodeNullFlavor", "FDA.D.11.r.1.REQUIRED", None, "NA"),
        Scenario(156, "fda-ethnicity-required", "fda", "DM", "patientInformation", "ethnicityCodeNullFlavor", "ethnicityCodeNullFlavor", "FDA.D.12.REQUIRED", None, "NA"),
        Scenario(157, "fda-aggregate-race-na-recommended", "fda", "DM", "patientInformation", "raceCodeNullFlavor", "raceCodeNullFlavor", "FDA.W0003", None, "NA", (("patientInitials", "AGGREGATE"),)),
        Scenario(158, "fda-aggregate-ethnicity-na-recommended", "fda", "DM", "patientInformation", "ethnicityCodeNullFlavor", "ethnicityCodeNullFlavor", "FDA.W0004", None, "NA", (("patientInitials", "AGGREGATE"),)),
        Scenario(160, "mfds-relatedness-method-required", "mfds", "DG", "drug", "drugReactionAssessments[].methodOfAssessmentKr1", "drugReactionAssessments[].methodOfAssessmentKr1", "MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED", None, "1", (("drugReactionAssessments[].sourceOfAssessment", "Sponsor"),)),
        Scenario(161, "mfds-relatedness-krct-result-required", "mfds", "DG", "drug", "drugReactionAssessments[].resultOfAssessmentKr2", "drugReactionAssessments[].resultOfAssessmentKr2", "MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED", None, "1", (("drugReactionAssessments[].sourceOfAssessment", "Sponsor"), ("drugReactionAssessments[].methodOfAssessmentKr1", "2"))),
        Scenario(162, "n-batch-number-required", "ich", "N", "messageHeaders", "batchNumber", "batch_number", "ICH.N.1.2.REQUIRED", "", "BATCH-VALID"),
        Scenario(163, "n-batch-sender-required", "ich", "N", "messageHeaders", "batchSenderIdentifier", "batch_sender_identifier", "ICH.N.1.3.REQUIRED", None, "SENDER"),
        Scenario(164, "n-batch-receiver-required", "ich", "N", "messageHeaders", "batchReceiverIdentifier", "batch_receiver_identifier", "ICH.N.1.4.REQUIRED", None, "RECEIVER"),
        Scenario(165, "n-batch-transmission-required", "ich", "N", "messageHeaders", "batchTransmissionDate", "batch_transmission_date", "ICH.N.1.5.REQUIRED", None, [year, 65, 0, 0, 0, 0, 0, 0, 0]),
        Scenario(166, "n-batch-transmission-future", "ich", "N", "messageHeaders", "batchTransmissionDate", "batch_transmission_date", "ICH.N.1.5.FUTURE_DATE.FORBIDDEN", [2999, 1, 0, 0, 0, 0, 0, 0, 0], [year, 65, 0, 0, 0, 0, 0, 0, 0]),
        Scenario(167, "n-message-sender-required", "ich", "N", "messageHeaders", "messageSenderIdentifier", "message_sender_identifier", "ICH.N.2.r.2.REQUIRED", "", "SENDER"),
        Scenario(168, "n-message-receiver-required", "ich", "N", "messageHeaders", "messageReceiverIdentifier", "message_receiver_identifier", "ICH.N.2.r.3.REQUIRED", "", "RECEIVER"),
        Scenario(169, "fda-postmarket-batch-route", "fda", "N", "messageHeaders", "batchReceiverIdentifier", "batch_receiver_identifier", "FDA.R0004", "WRONG", "ZZFDA", (("message_receiver_identifier", "CDER"),)),
        Scenario(170, "fda-premarket-batch-route", "fda", "N", "messageHeaders", "batchReceiverIdentifier", "batch_receiver_identifier", "FDA.R0005", "WRONG", "ZZFDA_PREMKT", (("message_receiver_identifier", "CDER_IND"),)),
        Scenario(171, "fda-postmarket-message-route", "fda", "N", "messageHeaders", "messageReceiverIdentifier", "message_receiver_identifier", "FDA.R0006", "CBER", "CDER", (("batch_receiver_identifier", "ZZFDA"),)),
        Scenario(172, "fda-premarket-message-route", "fda", "N", "messageHeaders", "messageReceiverIdentifier", "message_receiver_identifier", "FDA.R0007", "CDER", "CDER_IND", (("batch_receiver_identifier", "ZZFDA_PREMKT"),)),
        Scenario(173, "fda-sender-route-pair", "fda", "N", "messageHeaders", "messageSenderIdentifier", "message_sender_identifier", "FDA.R0100", "OTHER", "SENDER", (("batch_sender_identifier", "SENDER"),)),
        Scenario(174, "fda-vaers-route-pair", "fda", "N", "messageHeaders", "batchReceiverIdentifier", "batch_receiver_identifier", "FDA.VAERS.N.ROUTE.PAIR", "CBER VAERS", "CBER_VAERS", (("message_receiver_identifier", "CBER_VAERS"),)),
        Scenario(175, "mfds-batch-receiver-route", "mfds", "N", "messageHeaders", "batchReceiverIdentifier", "batch_receiver_identifier", "MFDS.N.1.4.ROUTE", "WRONG", "MFDS-O-KR", (("message_receiver_identifier", "MFDS-O-KR"),)),
        Scenario(176, "mfds-message-receiver-route", "mfds", "N", "messageHeaders", "messageReceiverIdentifier", "message_receiver_identifier", "MFDS.N.2.r.3.ROUTE", "WRONG", "MFDS-O-KR", (("batch_receiver_identifier", "MFDS-O-KR"),)),
        Scenario(177, "mfds-receiver-route-pair", "mfds", "N", "messageHeaders", "messageReceiverIdentifier", "message_receiver_identifier", "MFDS.N.ROUTE.PAIR", "MFDS-O-FR", "MFDS-O-KR", (("batch_receiver_identifier", "MFDS-O-KR"),)),
        Scenario(178, "c2-reporter-country-vocabulary", "ich", "RP", "primarySources", "reporterCountry", "reporterCountry", "ICH.C.2.r.3.VOCABULARY", "ZZ", "KR"),
        Scenario(179, "c3-sender-country-vocabulary", "ich", "SD", "senderInformation", "countryCode", "countryCode", "ICH.C.3.4.5.VOCABULARY", "ZZ", "KR"),
        Scenario(180, "e-reaction-language-vocabulary", "ich", "AE", "reaction", "reactionLanguage", "reaction_language", "ICH.E.i.1.1b.VOCABULARY", "zzz", "eng"),
        Scenario(181, "h-summary-language-vocabulary", "ich", "NR", "caseSummaryInformation", "languageCode", "language_code", "ICH.H.5.r.1b.VOCABULARY", "zzz", "eng"),
        Scenario(182, "fda-initial-expedited-ni-forbidden", "fda", "CI", "safetyReportIdentification", "fulfilExpeditedCriteriaNullFlavor", "fulfilExpeditedCriteriaNullFlavor", "FDA.R0011", "NI", None, header_values=(("batch_receiver_identifier", "ZZFDA"), ("message_receiver_identifier", "CDER"))),
        Scenario(183, "fda-initial-nullification-forbidden", "fda", "CI", "safetyReportIdentification", "nullificationAmendmentCode", "nullificationAmendmentCode", "FDA.R0101", "2", None, (("nullificationReason", "Business fuzz amendment"),), header_values=(("batch_receiver_identifier", "ZZFDA"), ("message_receiver_identifier", "CDER"))),
        Scenario(184, "fda-postmarket-combination-expedited-route", "fda", "CI", "safetyReportIdentification", "localCriteriaReportType", "localCriteriaReportType", "FDA.R0012", "2", "1", (("combinationProductReportIndicator", "true"), ("fulfilExpeditedCriteria", True)), header_values=(("batch_receiver_identifier", "ZZFDA"), ("message_receiver_identifier", "CDER"))),
        Scenario(185, "fda-postmarket-combination-nonexpedited-route", "fda", "CI", "safetyReportIdentification", "localCriteriaReportType", "localCriteriaReportType", "FDA.R0013", "1", "2", (("combinationProductReportIndicator", "true"), ("fulfilExpeditedCriteria", False)), header_values=(("batch_receiver_identifier", "ZZFDA"), ("message_receiver_identifier", "CDER"))),
        Scenario(186, "fda-postmarket-expedited-route", "fda", "CI", "safetyReportIdentification", "localCriteriaReportType", "localCriteriaReportType", "FDA.R0014", "2", "1", (("combinationProductReportIndicator", "false"), ("fulfilExpeditedCriteria", True)), header_values=(("batch_receiver_identifier", "ZZFDA"), ("message_receiver_identifier", "CDER"))),
        Scenario(187, "fda-postmarket-nonexpedited-route", "fda", "CI", "safetyReportIdentification", "localCriteriaReportType", "localCriteriaReportType", "FDA.R0015", "1", "2", (("combinationProductReportIndicator", "false"), ("fulfilExpeditedCriteria", False)), header_values=(("batch_receiver_identifier", "ZZFDA"), ("message_receiver_identifier", "CDER"))),
        Scenario(188, "fda-premarket-expedited-route", "fda", "CI", "safetyReportIdentification", "localCriteriaReportType", "localCriteriaReportType", "FDA.R0016", "2", "1", (("reportType", "1"), ("combinationProductReportIndicator", "false"), ("fulfilExpeditedCriteria", True)), header_values=(("batch_receiver_identifier", "ZZFDA_PREMKT"), ("message_receiver_identifier", "CDER_IND"))),
        Scenario(189, "fda-premarket-study-type-required", "fda", "SI", "studyInformation", "studyTypeReaction", "studyTypeReaction", "FDA.R0102", None, "1", ci_values=(("reportType", "2"),), header_values=(("batch_receiver_identifier", "ZZFDA_PREMKT"), ("message_receiver_identifier", "CDER_IND"))),
        Scenario(190, "fda-postmarket-study-type-required", "fda", "SI", "studyInformation", "studyTypeReaction", "studyTypeReaction", "FDA.R0104", None, "1", ci_values=(("reportType", "2"),), header_values=(("batch_receiver_identifier", "ZZFDA"), ("message_receiver_identifier", "CDER"))),
        Scenario(191, "fda-postmarket-spontaneous-study-type-forbidden", "fda", "SI", "studyInformation", "studyTypeReaction", "studyTypeReaction", "FDA.R0103", "1", None, ci_values=(("reportType", "1"),), header_values=(("batch_receiver_identifier", "ZZFDA"), ("message_receiver_identifier", "CDER"))),
        Scenario(192, "fda-premarket-spontaneous-study-type-forbidden", "fda", "SI", "studyInformation", "studyTypeReaction", "studyTypeReaction", "FDA.R0113", "1", None, ci_values=(("reportType", "1"),), header_values=(("batch_receiver_identifier", "ZZFDA_PREMKT"), ("message_receiver_identifier", "CDER_IND"))),
        Scenario(193, "fda-ind-number-required", "fda", "SI", "studyInformation", "fdaIndNumberOccurred", "fdaIndNumberOccurred", "FDA.R0024", None, "123456", ci_values=(("reportType", "1"),), header_values=(("batch_receiver_identifier", "ZZFDA_PREMKT"), ("message_receiver_identifier", "CDER_IND"))),
        Scenario(194, "fda-ind-number-format", "fda", "SI", "studyInformation", "fdaIndNumberOccurred", "fdaIndNumberOccurred", "FDA.R0024.FORMAT", "ABC", "123456", ci_values=(("reportType", "1"),), header_values=(("batch_receiver_identifier", "ZZFDA_PREMKT"), ("message_receiver_identifier", "CDER_IND"))),
        Scenario(195, "fda-postmarket-ind-number-forbidden", "fda", "SI", "studyInformation", "fdaIndNumberOccurred", "fdaIndNumberOccurred", "FDA.R0107", "123456", None, ci_values=(("reportType", "1"),), header_values=(("batch_receiver_identifier", "ZZFDA"), ("message_receiver_identifier", "CDER"))),
        Scenario(196, "fda-preanda-number-required", "fda", "SI", "studyInformation", "fdaPreAndaNumberOccurred", "fdaPreAndaNumberOccurred", "FDA.R0025", None, "123456", (("studyTypeReaction", "1"),), (("reportType", "2"),), (("batch_receiver_identifier", "ZZFDA_PREMKT"), ("message_receiver_identifier", "CDER_IND_EXEMPT_BA_BE"))),
        Scenario(197, "fda-preanda-number-format", "fda", "SI", "studyInformation", "fdaPreAndaNumberOccurred", "fdaPreAndaNumberOccurred", "FDA.R0025.FORMAT", "ABC", "123456", (("studyTypeReaction", "1"),), (("reportType", "2"),), (("batch_receiver_identifier", "ZZFDA_PREMKT"), ("message_receiver_identifier", "CDER_IND_EXEMPT_BA_BE"))),
        Scenario(198, "fda-postmarket-preanda-number-forbidden", "fda", "SI", "studyInformation", "fdaPreAndaNumberOccurred", "fdaPreAndaNumberOccurred", "FDA.R0108", "123456", None, ci_values=(("reportType", "1"),), header_values=(("batch_receiver_identifier", "ZZFDA"), ("message_receiver_identifier", "CDER"))),
        Scenario(199, "fda-cross-reported-ind-required", "fda", "SI", "studyInformation", "fdaCrossReportedIndNumbers[].indNumber", "fdaCrossReportedIndNumbers[].indNumber", "FDA.R0026", "", "123456", (("fdaIndNumberOccurred", "123456"),), (("reportType", "1"),), (("batch_receiver_identifier", "ZZFDA_PREMKT"), ("message_receiver_identifier", "CDER_IND")), readback_values=(None, "123456")),
        Scenario(200, "fda-ind-report-type-route", "fda", "CI", "safetyReportIdentification", "reportType", "reportType", "FDA.R0110", "2", "1", header_values=(("batch_receiver_identifier", "ZZFDA_PREMKT"), ("message_receiver_identifier", "CDER_IND")), study_values=(("fdaIndNumberOccurred", "123456"),)),
        Scenario(201, "fda-ind-study-report-type-route", "fda", "CI", "safetyReportIdentification", "reportType", "reportType", "FDA.R0008", "1", "2", header_values=(("batch_receiver_identifier", "ZZFDA_PREMKT"), ("message_receiver_identifier", "CDER_IND")), study_values=(("fdaIndNumberOccurred", "123456"), ("studyTypeReaction", "1"))),
        Scenario(202, "fda-preanda-report-type-route", "fda", "CI", "safetyReportIdentification", "reportType", "reportType", "FDA.R0111", "1", "2", header_values=(("batch_receiver_identifier", "ZZFDA_PREMKT"), ("message_receiver_identifier", "CDER_IND_EXEMPT_BA_BE")), study_values=(("fdaPreAndaNumberOccurred", "123456"), ("studyTypeReaction", "1"))),
        Scenario(203, "fda-ind-report-type-forbidden", "fda", "CI", "safetyReportIdentification", "reportType", "reportType", "FDA.R0112", "3", "1", header_values=(("batch_receiver_identifier", "ZZFDA_PREMKT"), ("message_receiver_identifier", "CDER_IND"))),
        Scenario(204, "fda-postmarket-cross-report-forbidden", "fda", "N", "messageHeaders", "batchReceiverIdentifier", "batch_receiver_identifier", "FDA.R0109", "ZZFDA", "ZZFDA_PREMKT", (("message_receiver_identifier", "CDER"),), study_values=(("fdaCrossReportedIndNumbers[].indNumber", "123456"),)),
        Scenario(205, "fda-cder-drug-role-route", "fda", "DG", "drug", "drugCharacterization", "drugCharacterization", "FDA.R0069", "2", "1", header_values=(("batch_receiver_identifier", "ZZFDA"), ("message_receiver_identifier", "CDER"))),
        Scenario(206, "fda-ind-drug-role-route", "fda", "DG", "drug", "drugCharacterization", "drugCharacterization", "FDA.R0070", "4", "1", ci_values=(("reportType", "2"),), header_values=(("batch_receiver_identifier", "ZZFDA_PREMKT"), ("message_receiver_identifier", "CDER_IND"))),
        Scenario(207, "fda-preanda-drug-role-route", "fda", "DG", "drug", "drugCharacterization", "drugCharacterization", "FDA.R0071", "2", "1", ci_values=(("reportType", "2"),), header_values=(("batch_receiver_identifier", "ZZFDA_PREMKT"), ("message_receiver_identifier", "CDER_IND_EXEMPT_BA_BE"))),
        Scenario(208, "fda-postmarket-drug-role-route", "fda", "DG", "drug", "drugCharacterization", "drugCharacterization", "FDA.G.k.1.ROUTE", "4", "1", header_values=(("batch_receiver_identifier", "ZZFDA"), ("message_receiver_identifier", "CBER"))),
        Scenario(209, "mfds-ct-report-type-route", "mfds", "CI", "safetyReportIdentification", "reportType", "reportType", "MFDS.C.1.3.RECEIVER", "1", "2", header_values=(("batch_receiver_identifier", "MFDS-O-CT"), ("message_receiver_identifier", "MFDS-O-CT"))),
        Scenario(210, "mfds-ct-expedited-route", "mfds", "CI", "safetyReportIdentification", "fulfilExpeditedCriteria", "fulfilExpeditedCriteria", "MFDS.C.1.7.RECEIVER", False, True, header_values=(("batch_receiver_identifier", "MFDS-O-CT"), ("message_receiver_identifier", "MFDS-O-CT"))),
        Scenario(211, "mfds-r2-retransmission-provenance", "mfds", "CI", "safetyReportIdentification", "fulfilExpeditedCriteriaNullFlavor", "fulfilExpeditedCriteriaNullFlavor", "MFDS.C.1.7.NULLFLAVOR.R2.RETRANSMISSION.REQUIRED", "NI", None, header_values=(("batch_receiver_identifier", "MFDS-O-KR"), ("message_receiver_identifier", "MFDS-O-KR"))),
        Scenario(212, "mfds-ct-primary-identity-required", "mfds", "RP", "primarySources", "reporterGivenName", "reporterGivenName", "MFDS.C.2.RECEIVER.REQUIRED", None, "Business", header_values=(("batch_receiver_identifier", "MFDS-O-CT"), ("message_receiver_identifier", "MFDS-O-CT"))),
        Scenario(213, "mfds-ct-primary-address-required", "mfds", "RP", "primarySources", "reporterStreet", "reporterStreet", "MFDS.C.2.r.2.3-5.RECEIVER.REQUIRED", None, "1 Test Street", header_values=(("batch_receiver_identifier", "MFDS-O-CT"), ("message_receiver_identifier", "MFDS-O-CT"))),
        Scenario(214, "mfds-ct-sender-name-required", "mfds", "SD", "senderInformation", "personGivenName", "personGivenName", "MFDS.C.3.3.3.RECEIVER.REQUIRED", None, "Business", header_values=(("batch_receiver_identifier", "MFDS-O-CT"), ("message_receiver_identifier", "MFDS-O-CT"))),
        Scenario(215, "mfds-ct-study-name-required", "mfds", "SI", "studyInformation", "studyName", "study_name", "MFDS.C.5.RECEIVER.REQUIRED", None, "Business Study", ci_values=(("reportType", "2"),), header_values=(("batch_receiver_identifier", "MFDS-O-CT"), ("message_receiver_identifier", "MFDS-O-CT"))),
        Scenario(216, "mfds-ct-study-type-route", "mfds", "SI", "studyInformation", "studyTypeReaction", "study_type_reaction", "MFDS.C.5.4.RECEIVER", "2", "1", ci_values=(("reportType", "2"),), header_values=(("batch_receiver_identifier", "MFDS-O-CT"), ("message_receiver_identifier", "MFDS-O-CT"))),
        Scenario(217, "mfds-reporter-qualification-kind-required", "mfds", "RP", "primarySources", "qualificationKr1", "qualificationKr1", "MFDS.C.2.r.4.KR.1.REQUIRED", None, "1", (("qualification", "3"),), header_values=(("batch_receiver_identifier", "MFDS-O-KR"), ("message_receiver_identifier", "MFDS-O-KR"))),
        Scenario(218, "mfds-sender-health-professional-kind-required", "mfds", "SD", "senderInformation", "healthProfessionalTypeKr1", "healthProfessionalTypeKr1", "MFDS.C.3.1.KR.1.REQUIRED", None, "1", (("senderType", "3"),), header_values=(("batch_receiver_identifier", "MFDS-O-KR"), ("message_receiver_identifier", "MFDS-O-KR"))),
        Scenario(219, "mfds-study-type-kind-required", "mfds", "SI", "studyInformation", "studyTypeReactionKr1", "study_type_reaction_kr1", "MFDS.C.5.4.KR.1.REQUIRED", None, "1", (("studyTypeReaction", "3"),), header_values=(("batch_receiver_identifier", "MFDS-O-KR"), ("message_receiver_identifier", "MFDS-O-KR"))),
        Scenario(220, "mfds-ct-patient-age-required", "mfds", "DM", "patientInformation", "patientAge.value", "age_at_time_of_onset", "MFDS.D.2.2a.REQUIRED", None, 36.5, header_values=(("batch_receiver_identifier", "MFDS-O-CT"), ("message_receiver_identifier", "MFDS-O-CT"))),
        Scenario(221, "mfds-ct-patient-age-unit-required", "mfds", "DM", "patientInformation", "patientAge.unit", "age_unit", "MFDS.D.2.2b.REQUIRED", None, "a", header_values=(("batch_receiver_identifier", "MFDS-O-CT"), ("message_receiver_identifier", "MFDS-O-CT"))),
        Scenario(222, "mfds-ct-patient-sex-required", "mfds", "DM", "patientInformation", "patientSex", "sex", "MFDS.D.5.REQUIRED", None, "2", header_values=(("batch_receiver_identifier", "MFDS-O-CT"), ("message_receiver_identifier", "MFDS-O-CT"))),
        Scenario(223, "mfds-ct-reaction-start-required", "mfds", "AE", "reaction", "reactionStartDate", "start_date", "MFDS.E.i.4.REQUIRED", None, f"{year}0303", header_values=(("batch_receiver_identifier", "MFDS-O-CT"), ("message_receiver_identifier", "MFDS-O-CT"))),
        Scenario(224, "mfds-ct-reaction-end-required", "mfds", "AE", "reaction", "reactionEndDate", "end_date", "MFDS.E.i.5.REQUIRED", None, f"{year}0304", header_values=(("batch_receiver_identifier", "MFDS-O-CT"), ("message_receiver_identifier", "MFDS-O-CT"))),
        Scenario(225, "mfds-korean-sender-comments", "mfds", "NR", "narrative", "senderComments", "sender_comments", "MFDS.H.4.KOREAN.REQUIRED", "English only", "한국어 의견", header_values=(("batch_receiver_identifier", "MFDS-O-KR"), ("message_receiver_identifier", "MFDS-O-KR"))),
        Scenario(226, "e-reaction-country-vocabulary", "ich", "AE", "reaction", "reactionCountry", "country_code", "ICH.E.i.9.VOCABULARY", "ZZ", "KR"),
        Scenario(227, "g-obtain-country-vocabulary", "ich", "DG", "drug", "obtainDrugCountry", "obtain_drug_country", "ICH.G.k.2.4.VOCABULARY", "ZZ", "KR"),
        Scenario(228, "g-authorization-country-vocabulary", "ich", "DG", "drug", "drugAuthorizationCountry", "manufacturer_country", "ICH.G.k.3.2.VOCABULARY", "ZZ", "KR"),
        Scenario(229, "f-test-unit-vocabulary", "ich", "LB", "testResult", "testUnit", "test_result_unit", "ICH.F.r.3.3.VOCABULARY", "not-a-unit", "mg/dL"),
        Scenario(230, "c5-registration-country-vocabulary", "ich", "SI", "studyRegistrationNumbers", "countryCode", "country_code", "ICH.C.5.1.r.2.VOCABULARY", "ZZ", "KR"),
        Scenario(231, "c1-other-identifier-source-required", "ich", "CI", "otherCaseIdentifiers", "source", "source", "ICH.C.1.9.1.r.1.REQUIRED", "", "CI source", (("caseIdentifier", "KR-ORG-001"),)),
        Scenario(232, "c1-other-identifier-required", "ich", "CI", "otherCaseIdentifiers", "caseIdentifier", "caseIdentifier", "ICH.C.1.9.1.r.2.REQUIRED", "", "KR-ORG-001", (("source", "CI source"),)),
        Scenario(233, "c1-other-identifier-format", "ich", "CI", "otherCaseIdentifiers", "caseIdentifier", "caseIdentifier", "ICH.C.1.9.1.r.2.FORMAT", "bad", "KR-ORG-001", (("source", "CI source"),)),
        Scenario(234, "fda-vaers-primary-contact-required", "fda", "RP", "primarySources", "reporterGivenName", "reporterGivenName", "FDA.C.2.PRIMARY.REQUIRED", None, "Business", (("reporterFamilyName", "Reporter"), ("reporterStreet", "1 Test Street"), ("reporterCity", "Seoul"), ("reporterState", "Seoul"), ("reporterPostcode", "04524"), ("reporterTelephone", "+82-2-1234-5678")), header_values=(("batch_receiver_identifier", "CBER_VAERS"), ("message_receiver_identifier", "CBER_VAERS"))),
        Scenario(235, "fda-vaers-primary-msk-forbidden", "fda", "RP", "primarySources", "reporterGivenNameNullFlavor", "reporterGivenNameNullFlavor", "FDA.C.2.PRIMARY.MSK.FORBIDDEN", "MSK", None, (("reporterCountry", "US"),), header_values=(("batch_receiver_identifier", "CBER_VAERS"), ("message_receiver_identifier", "CBER_VAERS"))),
        Scenario(236, "fda-vaers-primary-email-required", "fda", "RP", "primarySources", "reporterEmail", "reporterEmail", "FDA.C.2.r.2.8.REQUIRED", None, "reporter@example.com", header_values=(("batch_receiver_identifier", "CBER_VAERS"), ("message_receiver_identifier", "CBER_VAERS"))),
        Scenario(237, "fda-vaers-primary-email-msk-forbidden", "fda", "RP", "primarySources", "reporterEmailNullFlavor", "reporterEmailNullFlavor", "FDA.C.2.r.2.8.MSK.FORBIDDEN", "MSK", None, (("reporterCountry", "US"),), header_values=(("batch_receiver_identifier", "CBER_VAERS"), ("message_receiver_identifier", "CBER_VAERS"))),
        Scenario(238, "fda-vaers-race-required", "fda", "DM", "patientInformation", "raceCodeNullFlavor", "raceCodeNullFlavor", "FDA.D.11.REQUIRED", None, "UNK", header_values=(("batch_receiver_identifier", "CBER_VAERS"), ("message_receiver_identifier", "CBER_VAERS"))),
        Scenario(239, "fda-vaers-race-nullflavor-route", "fda", "DM", "patientInformation", "raceCodeNullFlavor", "raceCodeNullFlavor", "FDA.D.11.NULLFLAVOR.ROUTE", "NA", "UNK", header_values=(("batch_receiver_identifier", "CBER_VAERS"), ("message_receiver_identifier", "CBER_VAERS"))),
        Scenario(240, "fda-vaers-ethnicity-nullflavor-route", "fda", "DM", "patientInformation", "ethnicityCodeNullFlavor", "ethnicityCodeNullFlavor", "FDA.D.12.NULLFLAVOR.ROUTE", "NA", "UNK", header_values=(("batch_receiver_identifier", "CBER_VAERS"), ("message_receiver_identifier", "CBER_VAERS"))),
        Scenario(241, "fda-vaers-patient-age-required", "fda", "DM", "patientInformation", "patientAge.value", "age_at_time_of_onset", "FDA.D.2.REQUIRED", None, 36.5, header_values=(("batch_receiver_identifier", "CBER_VAERS"), ("message_receiver_identifier", "CBER_VAERS"))),
        Scenario(242, "fda-premarket-required-intervention-ni", "fda", "AE", "reaction", "requiredInterventionNullFlavor", "required_intervention_null_flavor", "FDA.E.i.3.2h.PREMARKET.NI.REQUIRED", None, "NI", header_values=(("batch_receiver_identifier", "ZZFDA_PREMKT"), ("message_receiver_identifier", "CDER_IND"))),
        Scenario(243, "fda-vaers-reaction-timing-required", "fda", "AE", "reaction", "reactionStartDate", "start_date", "FDA.E.i.4-6.REQUIRED", None, f"{year}0303", (("reactionEndDate", None), ("reactionDuration.value", None)), header_values=(("batch_receiver_identifier", "CBER_VAERS"), ("message_receiver_identifier", "CBER_VAERS"))),
        Scenario(244, "fda-ind-relatedness-source-required", "fda", "DG", "drug", "drugReactionAssessments[].sourceOfAssessment", "drugReactionAssessments[].sourceOfAssessment", "FDA.G.k.9.i.2.r.1.REQUIRED", None, "Sponsor", (("drugReactionAssessments[].methodOfAssessment", "FDA"), ("drugReactionAssessments[].resultOfAssessment", "Suspected")), (("reportType", "2"),), (("batch_receiver_identifier", "ZZFDA_PREMKT"), ("message_receiver_identifier", "CDER_IND")), (("fdaIndNumberOccurred", "123456"),)),
        Scenario(245, "fda-ind-relatedness-method-required", "fda", "DG", "drug", "drugReactionAssessments[].methodOfAssessment", "drugReactionAssessments[].methodOfAssessment", "FDA.G.k.9.i.2.r.2.REQUIRED", None, "FDA", (("drugReactionAssessments[].sourceOfAssessment", "Sponsor"), ("drugReactionAssessments[].resultOfAssessment", "Suspected")), (("reportType", "2"),), (("batch_receiver_identifier", "ZZFDA_PREMKT"), ("message_receiver_identifier", "CDER_IND")), (("fdaIndNumberOccurred", "123456"),)),
        Scenario(246, "fda-ind-relatedness-result-required", "fda", "DG", "drug", "drugReactionAssessments[].resultOfAssessment", "drugReactionAssessments[].resultOfAssessment", "FDA.G.k.9.i.2.r.3.REQUIRED", None, "Suspected", (("drugReactionAssessments[].sourceOfAssessment", "Sponsor"), ("drugReactionAssessments[].methodOfAssessment", "FDA")), (("reportType", "2"),), (("batch_receiver_identifier", "ZZFDA_PREMKT"), ("message_receiver_identifier", "CDER_IND")), (("fdaIndNumberOccurred", "123456"),)),
        Scenario(247, "mfds-ct-qualification-nullflavor-forbidden", "mfds", "RP", "primarySources", "qualificationNullFlavor", "qualificationNullFlavor", "MFDS.C.2.r.4.NULLFLAVOR.FORBIDDEN.CT_CU", "UNK", None, (("qualification", None),), header_values=(("batch_receiver_identifier", "MFDS-O-CT"), ("message_receiver_identifier", "MFDS-O-CT"))),
        Scenario(248, "mfds-ct-action-taken-required", "mfds", "DG", "drug", "drugActionTaken", "action_taken", "MFDS.G.k.8.REQUIRED", None, "1", header_values=(("batch_receiver_identifier", "MFDS-O-CT"), ("message_receiver_identifier", "MFDS-O-CT"))),
        Scenario(249, "mfds-who-umc-result-required", "mfds", "DG", "drug", "drugReactionAssessments[].resultOfAssessmentKr1", "drugReactionAssessments[].resultOfAssessmentKr1", "MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED", None, "1", (("drugReactionAssessments[].sourceOfAssessment", "Sponsor"), ("drugReactionAssessments[].methodOfAssessmentKr1", "1")), header_values=(("batch_receiver_identifier", "MFDS-O-KR"), ("message_receiver_identifier", "MFDS-O-KR"))),
        Scenario(250, "fda-premarket-death-date-required", "fda", "DM", "deathInfo", "dateOfDeath", "date_of_death", "FDA.D.9.1.REQUIRED", None, f"{year}0303", ci_values=(("reportType", "2"),), header_values=(("batch_receiver_identifier", "ZZFDA_PREMKT"), ("message_receiver_identifier", "CDER_IND")), reaction_values=(("seriousness.criteriaResultsInDeath", True),)),
        Scenario(251, "fda-preanda-additional-info-recommended", "fda", "DG", "drug", "fdaAdditionalInfoCoded", "fda_additional_info_coded", "FDA.W0006", None, "1", ci_values=(("reportType", "2"),), header_values=(("batch_receiver_identifier", "ZZFDA_PREMKT"), ("message_receiver_identifier", "CDER_IND_EXEMPT_BA_BE")), study_values=(("fdaPreAndaNumberOccurred", "123456"), ("studyTypeReaction", "1"))),
    ]
    scenarios = [
        replace(scenario, ordinal=ordinal)
        for ordinal, scenario in enumerate(scenarios)
    ]
    scenarios.extend([
        Scenario(
            len(scenarios), "d-patient-height-integer", "ich", "DM",
            "patientInformation", "patientHeight.value", "height_cm",
            "ICH.D.4.INTEGER", 1.5, 175,
        ),
        Scenario(
            len(scenarios) + 1, "d-parent-height-integer", "ich", "DM",
            "parentInfo", "parentHeight.value", "height_cm",
            "ICH.D.10.5.INTEGER", 1.5, 175,
        ),
        Scenario(
            len(scenarios) + 2, "d-lmp-null-flavor-allowed", "ich", "DM",
            "patientInformation", "lastMenstrualPeriodDateNullFlavor",
            "last_menstrual_period_date_null_flavor",
            "ICH.D.6.NULLFLAVOR.ALLOWED", "NASK", "MSK",
        ),
        Scenario(
            len(scenarios) + 3, "g-drug-characterization-required", "ich", "DG",
            "drug", "drugCharacterization", "drug_characterization",
            "ICH.G.k.1.REQUIRED", "", "1",
        ),
        Scenario(
            len(scenarios) + 4, "g-medicinal-product-required", "ich", "DG",
            "drug", "medicinalProduct", "medicinal_product",
            "ICH.G.k.2.2.REQUIRED", "", "Business fuzz product",
        ),
        Scenario(
            len(scenarios) + 5, "f-test-name-group-required", "ich", "LB",
            "testResult", "testName", "test_name",
            "ICH.F.r.2.REQUIRED", "", "ALT",
            (("testMeddraVersion", None), ("testMeddraCode", None)),
        ),
        Scenario(
            len(scenarios) + 6, "f-test-name-text-required", "ich", "LB",
            "testResult", "testName", "test_name",
            "ICH.F.r.2.1.REQUIRED", "", "ALT",
            (("testMeddraVersion", None), ("testMeddraCode", None)),
        ),
        Scenario(
            len(scenarios) + 7, "f-test-meddra-code-required", "ich", "LB",
            "testResult", "testMeddraCode", "test_meddra_code",
            "ICH.F.r.2.2b.REQUIRED", None, "BOUND-MEDDRA-CODE",
            (("testName", ""),), reference_fixture=True,
        ),
        Scenario(
            len(scenarios) + 8, "reaction-seriousness-null-flavor-ni-only",
            "ich", "AE", "reaction",
            "seriousness.criteriaResultsInDeathNullFlavor",
            "criteria_death_null_flavor", "ICH.E.i.3.2.NI.ONLY",
            "UNK", "NI", (("seriousness.criteriaResultsInDeath", None),),
            reference_fixture=True,
        ),
        Scenario(
            len(scenarios) + 9, "mfds-test-date-null-flavor-vocabulary",
            "mfds", "LB", "testResult", "testDateNullFlavor",
            "test_date_null_flavor", "MFDS.F.r.1.NULLFLAVOR.VOCABULARY",
            "MSK", "UNK", (("testDate", None),), reference_fixture=True,
        ),
    ])
    scenarios.extend([
        Scenario(
            len(scenarios), "d-concomitant-therapy-allowed-value", "ich", "DM",
            "patientInformation", "concomitantTherapies", "concomitant_therapy",
            "ICH.D.7.3.ALLOWED.VALUE", False, True,
        ),
        Scenario(
            len(scenarios) + 1, "d-family-history-allowed-value", "ich", "DM",
            "medicalHistoryEpisodes", "familyHistory", "family_history",
            "ICH.D.7.1.r.6.ALLOWED.VALUE", False, True,
            reference_fixture=True,
        ),
        Scenario(
            len(scenarios) + 2, "d-parent-age-unit-allowed-value", "ich", "DM",
            "parentInfo", "parentAge.unit", "parent_age_unit",
            "ICH.D.10.2.2b.ALLOWED.VALUE", "mo", "a",
            (("parentAge.value", 54),),
        ),
        Scenario(
            len(scenarios) + 3, "g-investigational-product-allowed-value", "ich", "DG",
            "drug", "investigationalProductBlinded", "investigational_product_blinded",
            "ICH.G.k.2.5.ALLOWED.VALUE", False, True,
            ci_values=(("reportType", "2"),),
            study_values=(("studyTypeReaction", "1"),),
        ),
    ])
    scenarios.extend(reference_vocabulary_scenarios(len(scenarios)))
    scenarios.extend(reference_required_scenarios(len(scenarios)))
    scenarios.extend(device_integration_scenarios(len(scenarios)))
    scenarios.extend(mirror_warning_scenarios(len(scenarios)))
    scenarios.extend(topology_integration_scenarios(len(scenarios)))
    scenarios.extend(singleton_integration_scenarios(len(scenarios)))
    return scenarios


def reference_vocabulary_scenarios(start: int) -> list[Scenario]:
    scenarios: list[Scenario] = []

    def add(
        scenario_id: str,
        authority: str,
        page: str,
        owner: str,
        field: str,
        projection_field: str,
        code: str,
        invalid_value: Any,
        valid_value: Any,
        fixture_values: tuple[tuple[str, Any], ...] = (),
        header_values: tuple[tuple[str, Any], ...] = (),
    ) -> None:
        scenarios.append(Scenario(
            start + len(scenarios), scenario_id, authority, page, owner, field,
            projection_field, code, invalid_value, valid_value, fixture_values,
            header_values=header_values, reference_fixture=True,
        ))

    meddra_pairs = [
        ("d-history", "DM", "medicalHistoryEpisodes", "meddraVersion", "meddraCode", "meddra_version", "meddra_code", "ICH.D.7.1.r.1a.VOCABULARY", "ICH.D.7.1.r.1b.VOCABULARY"),
        ("d-parent-history", "DM", "parentMedicalHistory", "meddraVersion", "meddraCode", "meddra_version", "meddra_code", "ICH.D.10.7.1.r.1a.VOCABULARY", "ICH.D.10.7.1.r.1b.VOCABULARY"),
        ("d-past-indication", "DH", "pastDrugHistory", "indicationMeddraVersion", "indicationMeddraCode", "indication_meddra_version", "indication_meddra_code", "ICH.D.8.r.6a.VOCABULARY", "ICH.D.8.r.6b.VOCABULARY"),
        ("d-past-reaction", "DH", "pastDrugHistory", "reactionMeddraVersion", "reactionMeddraCode", "reaction_meddra_version", "reaction_meddra_code", "ICH.D.8.r.7a.VOCABULARY", "ICH.D.8.r.7b.VOCABULARY"),
        ("d-parent-past-indication", "DM", "parentPastDrugs", "indicationMeddraVersion", "indicationMeddraCode", "indication_meddra_version", "indication_meddra_code", "ICH.D.10.8.r.6a.VOCABULARY", "ICH.D.10.8.r.6b.VOCABULARY"),
        ("d-parent-past-reaction", "DM", "parentPastDrugs", "reactionMeddraVersion", "reactionMeddraCode", "reaction_meddra_version", "reaction_meddra_code", "ICH.D.10.8.r.7a.VOCABULARY", "ICH.D.10.8.r.7b.VOCABULARY"),
        ("d-reported-cause", "DM", "reportedCauses", "meddraVersion", "meddraCode", "meddra_version", "meddra_code", "ICH.D.9.2.r.1a.VOCABULARY", "ICH.D.9.2.r.1b.VOCABULARY"),
        ("d-autopsy-cause", "DM", "autopsyCauses", "meddraVersion", "meddraCode", "meddra_version", "meddra_code", "ICH.D.9.4.r.1a.VOCABULARY", "ICH.D.9.4.r.1b.VOCABULARY"),
        ("e-reaction", "AE", "reaction", "reactionMeddraVersionLLT", "reactionMeddraCodeLLT", "reaction_meddra_version", "reaction_meddra_code", "ICH.E.i.2.1a.VOCABULARY", "ICH.E.i.2.1b.VOCABULARY"),
        ("f-test", "LB", "testResult", "testMeddraVersion", "testMeddraCode", "test_meddra_version", "test_meddra_code", "ICH.F.r.2.2a.VOCABULARY", "ICH.F.r.2.2b.VOCABULARY"),
        ("g-indication", "DG", "drug", "indications[].indicationMeddraVersion", "indications[].indicationMeddraCode", "indications[].indication_meddra_version", "indications[].indication_meddra_code", "ICH.G.k.7.r.2a.VOCABULARY", "ICH.G.k.7.r.2b.VOCABULARY"),
        ("h-diagnosis", "NR", "senderDiagnoses", "diagnosisMeddraVersion", "diagnosisMeddraCode", "diagnosis_meddra_version", "diagnosis_meddra_code", "ICH.H.3.r.1a.VOCABULARY", "ICH.H.3.r.1b.VOCABULARY"),
    ]
    for prefix, page, owner, version_field, code_field, version_projection, code_projection, version_rule, code_rule in meddra_pairs:
        add(f"{prefix}-version-vocabulary", "ich", page, owner, version_field, version_projection, version_rule, "99.9", "26.0", ((code_field, "10000001"),))
        add(f"{prefix}-code-vocabulary", "ich", page, owner, code_field, code_projection, code_rule, "99999999", "10000001", ((version_field, "26.0"),))

    add("g-substance-strength-unit-vocabulary", "ich", "DG", "drug", "activeSubstances[].substanceStrengthUnit", "activeSubstances[].strength_unit", "ICH.G.k.2.3.r.3b.VOCABULARY", "not-a-unit", "mg", (("activeSubstances[].substanceName", "Fuzz substance"), ("activeSubstances[].substanceStrengthValue", 10)))
    scenarios[-1] = replace(scenarios[-1], reference_fixture=False)
    add("g-dosage-frequency-unit-vocabulary", "ich", "DG", "drug", "dosageInformation[].frequencyUnit", "dosageInformation[].frequency_unit", "ICH.G.k.4.r.3.VOCABULARY", "not-a-frequency", "d", (("dosageInformation[].numberOfUnits", 1),))

    kr_header = (("batch_receiver_identifier", "MFDS-O-KR"), ("message_receiver_identifier", "MFDS-O-KR"))
    add("mfds-d-past-product-vocabulary", "mfds", "DH", "pastDrugHistory", "mfdsMedicinalProductId", "mfds_medicinal_product_id", "MFDS.D.8.r.1.KR.1b.VOCABULARY", "9999999999", "1234567890", header_values=kr_header)
    add("mfds-d-parent-past-product-vocabulary", "mfds", "DM", "parentPastDrugs", "mfdsMedicinalProductId", "mfds_medicinal_product_id", "MFDS.D.10.8.r.1.KR.1b.VOCABULARY", "9999999999", "1234567890", header_values=kr_header)
    add("mfds-g-product-vocabulary", "mfds", "DG", "drug", "mfdsMpid", "mfds_mpid", "MFDS.G.k.2.1.KR.1b.VOCABULARY", "9999999999", "1234567890", (("obtainDrugCountry", "KR"),), kr_header)
    add("mfds-g-substance-vocabulary", "mfds", "DG", "drug", "activeSubstances[].mfdsId", "activeSubstances[].mfds_id", "MFDS.G.k.2.3.r.1.KR.1b.VOCABULARY", "ZZZZZZZZZZ", "MFDS-SUB-1", (("activeSubstances[].substanceName", "Fuzz substance"),), kr_header)
    return scenarios


def reference_required_scenarios(start: int) -> list[Scenario]:
    kr_header = (("batch_receiver_identifier", "MFDS-O-KR"), ("message_receiver_identifier", "MFDS-O-KR"))
    fr_header = (("batch_receiver_identifier", "MFDS-O-FR"), ("message_receiver_identifier", "MFDS-O-FR"))
    definitions = [
        ("mfds-d-past-product-required", "DH", "pastDrugHistory", "mfdsMedicinalProductId", "mfds_medicinal_product_id", "MFDS.D.8.r.1.KR.1b.REQUIRED", None, "1234567890", (), kr_header),
        ("mfds-d-past-product-version-required", "DH", "pastDrugHistory", "mfdsMedicinalProductVersion", "mfds_medicinal_product_version", "MFDS.D.8.r.1.KR.1a.REQUIRED", None, "FUZZ1", (("mfdsMedicinalProductId", "WHO0001"),), fr_header),
        ("mfds-d-parent-past-product-required", "DM", "parentPastDrugs", "mfdsMedicinalProductId", "mfds_medicinal_product_id", "MFDS.D.10.8.r.1.KR.1b.REQUIRED", None, "1234567890", (), kr_header),
        ("mfds-d-parent-past-product-version-required", "DM", "parentPastDrugs", "mfdsMedicinalProductVersion", "mfds_medicinal_product_version", "MFDS.D.10.8.r.1.KR.1a.REQUIRED", None, "FUZZ1", (("mfdsMedicinalProductId", "WHO0001"),), fr_header),
        ("mfds-g-product-required", "DG", "drug", "mfdsMpid", "mfds_mpid", "MFDS.G.k.2.1.KR.1b.REQUIRED", None, "1234567890", (("obtainDrugCountry", "KR"),), kr_header),
        ("mfds-g-product-version-required", "DG", "drug", "mfdsMpidVersion", "mfds_mpid_version", "MFDS.G.k.2.1.KR.1a.REQUIRED", None, "FUZZ1", (("mfdsMpid", "WHO0001"),), fr_header),
        ("mfds-g-substance-required", "DG", "drug", "activeSubstances[].mfdsId", "activeSubstances[].mfds_id", "MFDS.G.k.2.3.r.1.KR.1b.REQUIRED", None, "MFDS-SUB-1", (("activeSubstances[].substanceName", "Fuzz substance"),), kr_header),
        ("mfds-g-substance-version-required", "DG", "drug", "activeSubstances[].mfdsVersion", "activeSubstances[].mfds_version", "MFDS.G.k.2.3.r.1.KR.1a.REQUIRED", None, "FUZZ1", (("activeSubstances[].substanceName", "Fuzz substance"), ("activeSubstances[].mfdsId", "CAS123")), fr_header),
    ]
    return [
        Scenario(
            start + index, scenario_id, "mfds", page, owner, field,
            projection, code, invalid, valid, fixtures,
            header_values=headers, reference_fixture=True,
        )
        for index, (
            scenario_id, page, owner, field, projection, code, invalid, valid,
            fixtures, headers,
        ) in enumerate(definitions)
    ]


def device_integration_scenarios(start: int) -> list[Scenario]:
    definitions = [
        ("fda-device-patient-na-required", "patientInitialsNullFlavor", "patientInitialsNullFlavor", "FDA.D.1.R0027", None, "NA"),
        ("fda-device-malfunction-required", "malfunction", "fdaDevices[].malfunction", "FDA.G.K.12.REQUIRED", False, True),
        ("fda-device-identity-required", "deviceBrandName", "fdaDevices[].device_brand_name", "FDA.G.k.12.r.4-6.AT_LEAST_ONE", None, "Business Device"),
        ("fda-device-problem-required", "deviceProblemCodes", "fdaDevices[].deviceProblemCodes", "FDA.G.K.12.R.3.REQUIRED", None, "1234567"),
        ("fda-device-other-characterization-required", "fdaOtherCharacterization", "fda_other_characterization", "FDA.R0072", None, "1"),
        ("fda-device-remedial-action-recommended", "remedialActions", "fdaDevices[].remedialActions", "FDA.W0007", None, "1"),
        ("fda-device-collection-required", "collectionPresent", "collection_present", "FDA.G.k.12.COLLECTION.REQUIRED", False, True),
    ]
    return [
        Scenario(
            start + index, scenario_id, "fda", "DV", "fdaDevices", field,
            projection, code, invalid, valid, surface="device",
        )
        for index, (scenario_id, field, projection, code, invalid, valid)
        in enumerate(definitions)
    ]


def mirror_warning_scenarios(start: int) -> list[Scenario]:
    return [Scenario(
        start,
        "fda-cder-drug-role-warning",
        "fda",
        "DG",
        "drug",
        "drugCharacterization",
        "drugCharacterization",
        "FDA.W0005",
        "2",
        "1",
        header_values=(("batch_receiver_identifier", "ZZFDA"), ("message_receiver_identifier", "CDER")),
    )]


def topology_integration_scenarios(start: int) -> list[Scenario]:
    definitions = [
        ("fda-aggregate-linked-report-topology", "fda", "linkedReports", "FDA.W0001", False, True),
        ("fda-aggregate-study-type-topology", "fda", "studyTypeReaction", "FDA.W0002", "2", "1"),
        ("fda-ind-aggregate-patient-topology", "fda", "patientInitials", "FDA.W0010", "PERSON", "AGGREGATE"),
        ("ich-study-patient-identifier-topology", "ich", "patientIdentifiers", "ICH.D.1.1.4.REQUIRED", False, True),
        ("mfds-study-patient-identifier-topology", "mfds", "patientIdentifiers", "MFDS.D.1.1.4.REQUIRED", False, True),
        ("mfds-study-registration-topology", "mfds", "studyRegistrationNumbers", "MFDS.C.5.1.r.1.RECEIVER.REQUIRED", False, True),
        ("mfds-study-registration-nullflavor-topology", "mfds", "registrationNumberNullFlavor", "MFDS.C.5.1.r.1.NULLFLAVOR.FORBIDDEN", "ASKU", None),
        ("fda-assessment-collection-required", "fda", "drugReactionAssessments", "FDA.G.k.9.REQUIRED", False, True),
    ]
    return [
        Scenario(
            start + index, scenario_id, authority, "TP", "topology", field,
            field, code, invalid, valid, surface="topology",
        )
        for index, (scenario_id, authority, field, code, invalid, valid)
        in enumerate(definitions)
    ]


def singleton_integration_scenarios(start: int) -> list[Scenario]:
    return [
        Scenario(start, "singleton-patient-required", "ich", "SN", "patientInformation", "patientInitials", "patientInitials", "ICH.D.1.REQUIRED", None, "BUSINESS-FUZZ", surface="singleton"),
        Scenario(start + 1, "singleton-narrative-required", "ich", "SN", "narrative", "caseNarrative", "caseNarrative", "ICH.H.1.REQUIRED", None, "Business fuzz narrative", surface="singleton"),
        Scenario(start + 2, "primary-sources-collection-required", "ich", "SN", "primarySources", "collectionPresent", "collection_present", "ICH.C.2.r.REQUIRED", False, True, surface="singleton"),
        Scenario(start + 3, "drugs-collection-required", "ich", "SN", "drugs", "collectionPresent", "collection_present", "ICH.G.k.REQUIRED", False, True, surface="singleton"),
        Scenario(start + 4, "c1-safety-report-id-required", "ich", "SN", "safetyReportIdentification", "safetyReportId", "safetyReportId", "ICH.C.1.1.REQUIRED", None, "BUSINESS-FUZZ", surface="singleton"),
        Scenario(start + 5, "c1-transmission-date-required", "ich", "SN", "safetyReportIdentification", "transmissionDate", "transmissionDate", "ICH.C.1.2.REQUIRED", None, "20260305000000", surface="singleton"),
        Scenario(start + 6, "reactions-collection-required", "ich", "SN", "reaction", "collectionPresent", "collection_present", "ICH.E.i.REQUIRED", False, True, reference_fixture=True, surface="singleton"),
    ]


GENERATOR_FAMILIES = {
    "boolean_condition",
    "collection_topology",
    "device_condition",
    "lexical_condition",
    "numeric_condition",
    "presence_condition",
    "relational_condition",
    "temporal_condition",
    "vocabulary_condition",
}
DATE_TEXT_RE = re.compile(r"^(\d{4})(\d{2})(\d{2})(.*)$")


def generator_family(scenario: Scenario) -> str:
    code = scenario.expected_code
    if scenario.surface == "topology":
        return "collection_topology"
    if scenario.surface == "device":
        return "device_condition"
    if "FUTURE_DATE" in code or ".AFTER_" in code:
        return "temporal_condition"
    if "VOCABULARY" in code:
        return "vocabulary_condition"
    if any(token in code for token in ("EXCLUSIVE", ".PAIR", "AT_LEAST_ONE")):
        return "relational_condition"
    if isinstance(scenario.invalid_value, bool) or isinstance(scenario.valid_value, bool):
        return "boolean_condition"
    if isinstance(scenario.invalid_value, (int, float)) or isinstance(scenario.valid_value, (int, float)):
        return "numeric_condition"
    if scenario.invalid_value is None or scenario.valid_value is None or "REQUIRED" in code or "FORBIDDEN" in code:
        return "presence_condition"
    if isinstance(scenario.invalid_value, str) and isinstance(scenario.valid_value, str):
        return "lexical_condition"
    raise ValueError(f"no generator family for {scenario.scenario_id}: {code}")


def _date_value(template: str, year: int, month: int, day: int) -> str:
    match = DATE_TEXT_RE.fullmatch(template)
    if not match:
        return template
    return f"{year:04d}{month:02d}{day:02d}{match.group(4)}"


def _temporal_values(scenario: Scenario, rng: random.Random) -> tuple[Any, Any]:
    if isinstance(scenario.invalid_value, list) and isinstance(scenario.valid_value, list):
        invalid = list(scenario.invalid_value)
        valid = list(scenario.valid_value)
        invalid[:2] = [rng.randint(2030, 2099), rng.randint(1, 365)]
        valid[:2] = [rng.randint(2010, 2024), rng.randint(1, 365)]
        return invalid, valid
    if ".AFTER_" in scenario.expected_code:
        match = DATE_TEXT_RE.fullmatch(str(scenario.invalid_value))
        if not match:
            raise ValueError(f"temporal generator needs a date: {scenario.scenario_id}")
        year = int(match.group(1))
        return (
            _date_value(str(scenario.invalid_value), year, 3, rng.choice((1, 2))),
            _date_value(str(scenario.valid_value), year, 3, rng.randint(4, 28)),
        )
    return (
        _date_value(str(scenario.invalid_value), rng.randint(2030, 2099), rng.randint(1, 12), rng.randint(1, 28)),
        _date_value(str(scenario.valid_value), rng.randint(2010, 2024), rng.randint(1, 12), rng.randint(1, 28)),
    )


def _fixed_field(path: str) -> bool:
    normalized = snake(path)
    return any(token in normalized for token in (
        "batch_receiver_identifier", "message_receiver_identifier",
        "code", "country", "indicator", "language", "null_flavor",
        "qualification", "route", "sex", "type", "unit", "version",
    ))


def _generated_string(
    scenario: Scenario,
    path: str,
    value: str,
    edge: str,
    family: str,
    rng: random.Random,
    token: str,
) -> str:
    if value == "":
        return value
    normalized = snake(path)
    if family == "lexical_condition":
        if normalized in {"included_document", "document_base64"}:
            return (
                f"%%%{token[:8]}"
                if edge == "invalid"
                else base64.b64encode(token.encode()).decode()
            )
        if "meddra_version" in normalized:
            return (
                "".join(rng.choice("abcxyz") for _ in value)
                if edge == "invalid"
                else f"{rng.randint(20, 29)}.{rng.randint(0, 9)}"
            )
        if normalized in {"fda_ind_number_occurred", "fda_pre_anda_number_occurred", "ind_number"}:
            return f"X{token[:5]}" if edge == "invalid" else f"{rng.randint(0, 999999):06d}"
        if normalized == "case_identifier":
            return f"bad-{token[:6]}" if edge == "invalid" else f"KR-ORG-{rng.randint(1, 999999):06d}"
        if _fixed_field(path) or any(token_name in normalized for token_name in (
            "characterization", "local_criteria", "null_flavor", "report_type",
        )):
            return value
        if edge == "invalid":
            return f"{value}-{token[:6]}"
        return value
    if family == "vocabulary_condition" and edge == "invalid":
        if scenario.scenario_id == "mfds-test-date-null-flavor-vocabulary":
            return value
        if value == "99.9":
            return f"{rng.randint(80, 99)}.{rng.randint(0, 9)}"
        if value.isdigit() and set(value) == {"9"}:
            return str(rng.randint(10 ** (len(value) - 1) * 9, 10 ** len(value) - 1))
        if len(value) == 2 and value.isalpha():
            return rng.choice(("QQ", "QZ", "XZ", "ZZ"))
        if value.startswith("not-a-"):
            return f"{value[:12]}-{token[:6]}"
        return "".join(rng.choice("QXYZ") for _ in range(max(2, len(value))))
    if family == "vocabulary_condition":
        return value
    if value == "AGGREGATE":
        return value
    if normalized in {"included_document", "document_base64"}:
        return value
    if normalized.endswith(("method_of_assessment", "result_of_assessment")):
        return value
    if "date" in normalized and DATE_TEXT_RE.fullmatch(value):
        match = DATE_TEXT_RE.fullmatch(value)
        assert match is not None
        return _date_value(value, int(match.group(1)), rng.randint(1, 12), rng.randint(1, 28))
    if "@" in value:
        domain = value.split("@", 1)[1]
        return f"fuzz-{token[:8]}@{domain}"
    if any("가" <= character <= "힣" for character in value):
        return f"무작위 한글 의견 {token[:6]}"
    if _fixed_field(path):
        return value
    if value.isdigit() and len(value) <= 2:
        return value
    if re.fullmatch(r"-?\d+(?:\.\d+)?", value):
        if "." in value:
            return f"{rng.randint(1, 999)}.{rng.randint(1, 9)}"
        return str(rng.randint(1, max(9, 10 ** min(len(value), 6) - 1)))
    if any(token_name in normalized for token_name in (
        "id", "identifier", "number", "mpid", "phpid", "termid",
    )):
        digits = re.search(r"\d+$", value)
        if digits:
            replacement = str(rng.randint(1, 10 ** len(digits.group()) - 1)).zfill(len(digits.group()))
            return f"{value[:digits.start()]}{replacement}"
        return f"{value[:40]}-{token[:8]}"
    if any(token_name in normalized for token_name in (
        "comments", "description", "name", "narrative", "organization",
        "city", "reason", "reference", "source", "state", "street",
        "telephone", "text", "title", "product",
        "initial", "unstructured",
    )):
        if normalized.endswith("source_of_assessment"):
            return value
        return f"{value[:80]} {token[:8]}"
    raise ValueError(f"no string generator for {scenario.scenario_id}: {path}")


def _generated_value(
    scenario: Scenario,
    path: str,
    value: Any,
    edge: str,
    family: str,
    rng: random.Random,
    token: str,
) -> Any:
    if value is None or isinstance(value, bool):
        return value
    if isinstance(value, int):
        return rng.randint(1, 999)
    if isinstance(value, float):
        upper = 9 if scenario.scenario_id in {
            "d-patient-height-integer", "d-parent-height-integer",
        } else 999
        return (rng.randint(1, upper) * 10 + rng.randint(1, 9)) / 10
    if isinstance(value, list):
        if len(value) == 9 and all(isinstance(item, int) for item in value):
            return [rng.randint(2010, 2024), rng.randint(1, 365), *value[2:]]
        return [_generated_value(scenario, path, item, edge, family, rng, token) for item in value]
    if isinstance(value, str):
        return _generated_string(scenario, path, value, edge, family, rng, token)
    raise TypeError(f"unsupported generated value for {scenario.scenario_id}: {type(value).__name__}")


def scenario_fingerprint(scenario: Scenario) -> str:
    return hashlib.sha256(json.dumps({
        "invalid": scenario.invalid_value,
        "valid": scenario.valid_value,
        "fixture": scenario.fixture_values,
        "ci": scenario.ci_values,
        "header": scenario.header_values,
        "study": scenario.study_values,
        "reaction": scenario.reaction_values,
        "token": scenario.generation_token,
    }, sort_keys=True, default=str).encode()).hexdigest()[:16]


def generated_scenario(scenario: Scenario, seed: int, sample_ordinal: int) -> Scenario:
    family = generator_family(scenario)
    if family not in GENERATOR_FAMILIES:
        raise ValueError(f"unsupported generator family: {family}")
    rng = random.Random(f"{seed}:{scenario.scenario_id}:{sample_ordinal}")
    token = f"{rng.getrandbits(48):012x}"
    if family == "temporal_condition":
        invalid_value, valid_value = _temporal_values(scenario, rng)
    else:
        invalid_value = _generated_value(scenario, scenario.field, scenario.invalid_value, "invalid", family, rng, token)
        valid_value = _generated_value(scenario, scenario.field, scenario.valid_value, "valid", family, rng, token)
    if (
        scenario.scenario_id == "d-parent-age-unit-allowed-value"
        and sample_ordinal % 2 == 1
    ):
        valid_value = "10.a"

    def generated_pairs(items: tuple[tuple[str, Any], ...]) -> tuple[tuple[str, Any], ...]:
        return tuple(
            (path, _generated_value(scenario, path, value, "context", family, rng, token))
            for path, value in items
        )

    readback_values = scenario.readback_values
    if readback_values is not None:
        readback_values = (
            invalid_value if readback_values[0] == scenario.invalid_value else readback_values[0],
            valid_value if readback_values[1] == scenario.valid_value else readback_values[1],
        )
    generated = replace(
        scenario,
        invalid_value=invalid_value,
        valid_value=valid_value,
        fixture_values=generated_pairs(scenario.fixture_values),
        ci_values=generated_pairs(scenario.ci_values),
        header_values=generated_pairs(scenario.header_values),
        study_values=generated_pairs(scenario.study_values),
        reaction_values=generated_pairs(scenario.reaction_values),
        readback_values=readback_values,
        generator_family=family,
        sample_ordinal=sample_ordinal,
        generation_token=token,
    )
    return replace(generated, generation_fingerprint=scenario_fingerprint(generated))


def issue_codes(value: Any) -> set[str]:
    issues = value.get("issues", []) if isinstance(value, dict) else []
    return {
        issue["code"]
        for issue in issues
        if isinstance(issue, dict) and isinstance(issue.get("code"), str)
    }


def issue_complete(value: Any, code: str) -> bool:
    issues = value.get("issues", []) if isinstance(value, dict) else []
    return any(
        isinstance(issue, dict)
        and issue.get("code") == code
        and isinstance(issue.get("message"), str)
        and bool(issue.get("message"))
        and isinstance(issue.get("path"), str)
        and bool(issue.get("path"))
        and isinstance(issue.get("section"), str)
        and bool(issue.get("section"))
        and isinstance(issue.get("subsection"), str)
        and bool(issue.get("subsection"))
        and "field_path" in issue
        and (issue["field_path"] is None or (
            isinstance(issue["field_path"], str) and bool(issue["field_path"])
        ))
        for issue in issues
    )


def complete_issue_count(
    value: Any, code: str, field_path: str, message: str | None = None,
) -> int:
    issues = value.get("issues", []) if isinstance(value, dict) else []
    return sum(
        issue_complete({"issues": [issue]}, code)
        and issue.get("field_path") == field_path
        and (message is None or issue.get("message") == message)
        for issue in issues
        if isinstance(issue, dict)
    )


def rules_with_both_edges_passed(scenarios: list[Scenario], events: list[Event]) -> set[str]:
    passed_edges = {
        (event.scenario_id, event.sample_ordinal, event.kind)
        for event in events
        if event.classification == "PASS" and event.kind in {"invalid_edge", "valid_edge"}
    }
    return {
        code
        for code in {
            scenario.expected_code for scenario in scenarios
            if scenario.scenario_id not in MEDDRA_VERSION_SCENARIO_PATHS
        }
        if all(
            (scenario.scenario_id, scenario.sample_ordinal, edge) in passed_edges
            for scenario in scenarios if scenario.expected_code == code
            for edge in ("invalid_edge", "valid_edge")
        )
    }


def meddra_binding(
    rows: Any,
    absent_rows: Any,
    invalid_candidate: str = "99999999",
) -> tuple[str, str, str]:
    if not isinstance(rows, list) or len(rows) != 1 or not isinstance(rows[0], dict):
        raise ValueError("active MedDRA lookup must return exactly one row")
    row = rows[0]
    version = row.get("version")
    code = row.get("code")
    if not (
        row.get("active") is True
        and version == "28.1"
        and str(row.get("language", "")).lower() == "en"
        and str(row.get("level", "")).upper() == "LLT"
        and isinstance(code, str)
        and code.isdecimal()
        and code
    ):
        raise ValueError("active MedDRA row metadata does not match 28.1/en/LLT")
    if not isinstance(absent_rows, list) or len(absent_rows) >= 100:
        raise ValueError("invalid MedDRA lookup is malformed or may be truncated")
    if any(not isinstance(item, dict) for item in absent_rows):
        raise ValueError("invalid MedDRA lookup contains a malformed row")
    if any(item.get("code") == invalid_candidate for item in absent_rows):
        raise ValueError("invalid MedDRA candidate exists in the active release")
    return version, code, invalid_candidate


def meddra_unavailable_version(
    releases: Any, absent_version_rows: Any, valid_version: str,
    invalid_version: str = "27.1",
) -> str:
    if not isinstance(releases, list) or any(
        not isinstance(item, dict) for item in releases
    ):
        raise ValueError("MedDRA release lookup is malformed")
    active = [
        item for item in releases
        if item.get("dictionary") == "meddra"
        and str(item.get("language", "")).lower() == "en"
        and item.get("status") == "active"
        and isinstance(item.get("loaded_rows", item.get("loadedRows")), int)
        and item.get("loaded_rows", item.get("loadedRows")) > 0
    ]
    if len(active) != 1 or active[0].get("version") != valid_version:
        raise ValueError("MedDRA 28.1 must be the sole active loaded en release")
    if not isinstance(absent_version_rows, list) or absent_version_rows:
        raise ValueError("MedDRA 27.1 has active terminology rows")
    return invalid_version


def bind_meddra_code_scenarios(
    scenarios: list[Scenario],
    version: str,
    valid_code: str,
    invalid_code: str,
) -> list[Scenario]:
    bound: list[Scenario] = []
    for scenario in scenarios:
        if scenario.scenario_id not in (
            MEDDRA_CODE_SCENARIO_IDS | MEDDRA_CONTEXT_SCENARIO_IDS
        ):
            bound.append(scenario)
            continue
        fixtures = dict(scenario.fixture_values)
        for version_field, code_field in MEDDRA_FIELDS_BY_OWNER[scenario.owner]:
            fixtures[version_field] = version
            fixtures[code_field] = valid_code
        rebound = replace(
            scenario,
            invalid_value=(
                scenario.invalid_value
                if scenario.scenario_id in MEDDRA_CONTEXT_SCENARIO_IDS
                else invalid_code
            ),
            valid_value=(
                valid_code
                if scenario.scenario_id == "f-test-meddra-code-required"
                else scenario.valid_value
                if scenario.scenario_id in MEDDRA_CONTEXT_SCENARIO_IDS
                else valid_code
            ),
            fixture_values=tuple(fixtures.items()),
            reference_fixture=False,
        )
        bound.append(replace(
            rebound, generation_fingerprint=scenario_fingerprint(rebound),
        ))
    return bound


def bind_meddra_version_scenarios(
    scenarios: list[Scenario], valid_version: str, invalid_version: str,
    valid_code: str,
) -> list[Scenario]:
    bound: list[Scenario] = []
    for scenario in scenarios:
        if scenario.scenario_id not in MEDDRA_VERSION_SCENARIO_PATHS:
            bound.append(scenario)
            continue
        fixtures = dict(scenario.fixture_values)
        for version_field, code_field in MEDDRA_FIELDS_BY_OWNER[scenario.owner]:
            fixtures[version_field] = valid_version
            fixtures[code_field] = valid_code
        rebound = replace(
            scenario,
            expected_code="ICH.MEDDRA.VERSION.UNAVAILABLE",
            invalid_value=invalid_version,
            valid_value=valid_version,
            fixture_values=tuple(fixtures.items()),
            reference_fixture=False,
        )
        bound.append(replace(
            rebound, generation_fingerprint=scenario_fingerprint(rebound),
        ))
    return bound


def whodrug_binding(releases: Any, products: Any) -> tuple[str, str]:
    if not isinstance(releases, list) or any(
        not isinstance(item, dict) for item in releases
    ):
        raise ValueError("WHODrug release lookup is malformed")
    active = [
        item for item in releases
        if item.get("dictionary") == "whodrug"
        and str(item.get("language", "")).lower() == "en"
        and item.get("status") == "active"
    ]
    if len(active) != 1:
        raise ValueError("WHODrug must have exactly one active en release")
    release = active[0]
    version = release.get("version")
    loaded_rows = release.get("loaded_rows", release.get("loadedRows"))
    if not isinstance(version, str) or not version or not isinstance(loaded_rows, int) or loaded_rows < 1:
        raise ValueError("active WHODrug release metadata is incomplete")
    if not isinstance(products, list) or len(products) != 1 or not isinstance(products[0], dict):
        raise ValueError("active WHODrug product lookup must return exactly one row")
    product = products[0]
    code = product.get("code")
    if not (
        product.get("active") is True
        and product.get("version") == version
        and str(product.get("language", "")).lower() == "en"
        and isinstance(code, str)
        and bool(code)
        and isinstance(product.get("drug_name"), str)
        and bool(product["drug_name"])
    ):
        raise ValueError("WHODrug product metadata does not match the active en release")
    return version, code


def bind_whodrug_version_scenarios(
    scenarios: list[Scenario], version: str, product_code: str,
) -> list[Scenario]:
    bound: list[Scenario] = []
    for scenario in scenarios:
        if scenario.scenario_id not in WHODRUG_VERSION_SCENARIOS:
            bound.append(scenario)
            continue
        fixtures = dict(scenario.fixture_values)
        fixtures[WHODRUG_CODE_FIELDS[scenario.owner]] = product_code
        rebound = replace(
            scenario,
            invalid_value=None,
            valid_value=version,
            fixture_values=tuple(fixtures.items()),
            reference_fixture=False,
        )
        bound.append(replace(
            rebound, generation_fingerprint=scenario_fingerprint(rebound),
        ))
    return bound


def omit_whodrug_unrelated_meddra(payload: dict[str, Any], scenario: Scenario) -> None:
    if scenario.scenario_id not in WHODRUG_VERSION_SCENARIOS:
        return
    for field in (
        "indicationMeddraVersion", "indicationMeddraCode",
        "reactionMeddraVersion", "reactionMeddraCode",
    ):
        payload.pop(field, None)


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser()
    result.add_argument("--base-url", default=os.getenv("E2BR3_BASE_URL", "http://127.0.0.1:8080"))
    result.add_argument("--email", default=os.getenv("E2BR3_ADMIN_EMAIL", "demo.cro.admin@example.com"))
    result.add_argument("--password", default=os.getenv("E2BR3_ADMIN_PASSWORD", "welcome"))
    result.add_argument(
        "--message-sender-identifier",
        default=os.getenv("E2BR3_DEFAULT_MESSAGE_SENDER"),
    )
    result.add_argument(
        "--ich-message-receiver-identifier",
        default=os.getenv("E2BR3_DEFAULT_MESSAGE_RECEIVER_ICH"),
    )
    result.add_argument("--seed", type=int, default=int(time.time()))
    result.add_argument("--artifact-dir", default="tmp/rbac-rls-fuzz/business-validator")
    result.add_argument("--timeout", type=float, default=20)
    result.add_argument("--max-actions", type=int, default=30000)
    result.add_argument("--deadline-seconds", type=float, default=600)
    result.add_argument("--samples-per-scenario", type=int, default=3)
    result.add_argument("--scenario", action="append", help="run only this scenario id (repeatable)")
    result.add_argument("--allow-remote", action="store_true")
    result.add_argument("--dry-run", action="store_true")
    return result


def main(args: argparse.Namespace) -> int:
    guard_target(args.base_url, args.allow_remote)
    if args.samples_per_scenario < 1:
        raise SystemExit("--samples-per-scenario must be at least 1")
    scenario_templates = scenario_catalog(args.seed)
    catalog_covered = {scenario.expected_code for scenario in scenario_templates}
    if args.scenario:
        requested = set(args.scenario)
        unknown = requested - {scenario.scenario_id for scenario in scenario_templates}
        if unknown:
            raise SystemExit(f"unknown scenario ids: {', '.join(sorted(unknown))}")
        scenario_templates = [scenario for scenario in scenario_templates if scenario.scenario_id in requested]
    scenarios = [
        generated_scenario(scenario, args.seed, sample_ordinal)
        for scenario in scenario_templates
        for sample_ordinal in range(args.samples_per_scenario)
    ]
    inventory = discover_business_rule_codes()
    planned_rules = {
        scenario.expected_code for scenario in scenarios
        if scenario.scenario_id not in XML_BOUNDARY_SCENARIO_IDS
        and scenario.scenario_id not in MEDDRA_VERSION_SCENARIO_PATHS
        and scenario.scenario_id not in SUPPLEMENTAL_RETAINED_SCENARIO_IDS
    }
    supplemental_planned_rules = {
        scenario.expected_code for scenario in scenarios
        if scenario.scenario_id in SUPPLEMENTAL_RETAINED_SCENARIO_IDS
    }
    xml_boundary_planned_rules = {
        scenario.expected_code for scenario in scenarios
        if scenario.scenario_id in XML_BOUNDARY_SCENARIO_IDS
    }
    raw_uncovered = inventory - catalog_covered
    dispositions = {
        code: detail
        for code, detail in rule_dispositions().items()
        if code in raw_uncovered
    }
    test_backed = {
        code: TEST_BACKED_RULES[code]
        for code in sorted(raw_uncovered & TEST_BACKED_RULES.keys())
    }
    unexplained = raw_uncovered - dispositions.keys() - test_backed.keys()
    if args.dry_run:
        print(json.dumps({
            "seed": args.seed,
            "scenarios": len(scenario_templates),
            "scenario_templates": len(scenario_templates),
            "generated_scenarios": len(scenarios),
            "samples_per_scenario": args.samples_per_scenario,
            "generator_families": dict(sorted(
                (family, sum(item.generator_family == family for item in scenarios))
                for family in GENERATOR_FAMILIES
            )),
            "catalog_rule_count": len(catalog_covered),
            "catalog_rules": sorted(catalog_covered),
            "planned_rule_count": len(planned_rules),
            "planned_rules": sorted(planned_rules),
            "supplemental_retained_planned_rule_count": len(supplemental_planned_rules),
            "supplemental_retained_planned_rules": sorted(supplemental_planned_rules),
            "supplemental_retained_verified_rules": [],
            "supplemental_retained_unverified_rules": sorted(supplemental_planned_rules),
            "xml_boundary_planned_rule_count": len(xml_boundary_planned_rules),
            "xml_boundary_planned_rules": sorted(xml_boundary_planned_rules),
            "unverified_xml_boundary_rule_count": len(xml_boundary_planned_rules),
            "unverified_xml_boundary_rules": sorted(xml_boundary_planned_rules),
            "inventory_catalog_intersection_rule_count": len(inventory & catalog_covered),
            "inventory_catalog_intersection_rules": sorted(inventory & catalog_covered),
            "verified_both_edges_rules": [],
            "verified_meddra_version_field_scenarios": [],
            "verified_meddra_version_field_scenario_count": 0,
            "unverified_planned_rules": sorted(planned_rules),
            "inventory_rule_count": len(inventory),
            "raw_uncovered_rules": sorted(raw_uncovered),
            "dispositioned_rules": dispositions,
            "test_backed_rule_mappings_not_executed": test_backed,
            "unsupported_inventory_rules": sorted(unexplained),
            "complete": False,
            "verdict": "NOT_RUN",
            "official_compliance_verified": False,
            "ui_verified": False,
        }, sort_keys=True))
        return 1
    if not args.password:
        raise SystemExit("set E2BR3_ADMIN_PASSWORD")
    client = ApiClient(args.base_url, args.timeout)
    events: list[Event] = []
    started = time.monotonic()
    requests = 0
    interrupted: str | None = None

    def add(kind: str, scenario: Scenario | None, classification: str, status: int | None, detail: dict[str, Any]) -> None:
        events.append(Event(
            kind,
            scenario.scenario_id if scenario else None,
            scenario.ordinal if scenario else None,
            scenario.sample_ordinal if scenario else None,
            scenario.generator_family if scenario else None,
            scenario.generation_fingerprint if scenario else None,
            classification,
            status,
            detail,
        ))

    def request(method: str, path: str, payload: dict[str, Any] | None = None) -> tuple[int | None, Any, dict[str, Any]]:
        nonlocal interrupted, requests
        if requests >= args.max_actions:
            interrupted = interrupted or "max_actions"
            return None, None, {}
        if time.monotonic() - started >= args.deadline_seconds:
            interrupted = interrupted or "deadline"
            return None, None, {}
        requests += 1
        status, body, transport = client.request(method, path, payload)
        summary = response_summary(status, body)
        if transport:
            summary["transport_error"] = transport
            interrupted = interrupted or "transport_error"
        if status is not None and status >= 500:
            interrupted = interrupted or "server_error"
        try:
            value = unwrap(json.loads(body))
        except (UnicodeDecodeError, json.JSONDecodeError):
            value = None
        return status, value, summary

    def page_current(case_id: str, page: str, owner: str, row_id: str | None = None) -> tuple[int | None, Any]:
        route = f"/api/cases/{case_id}/editor/pages/{page}"
        status, value, _ = request("GET", f"{route}/rows/{row_id}" if row_id else route)
        if row_id:
            return status, value.get(owner, value) if isinstance(value, dict) else value
        rows = value.get("rows", {}) if isinstance(value, dict) else {}
        current = rows.get(owner) if isinstance(rows, dict) else None
        if isinstance(current, list):
            current = current[0] if current else None
        return status, current

    def audit_logs(case_id: str, owner: str, row_id: str, field: str) -> list[dict[str, Any]]:
        nested_table = next(
            (table for prefix, table in NESTED_AUDIT_TABLES.items() if field.startswith(prefix)),
            None,
        )
        if nested_table:
            status, value, _ = request("GET", f"/api/audit-logs/by-record/cases/{case_id}")
            return [
                item for item in value
                if isinstance(item, dict) and item.get("table_name") == nested_table
            ] if status == 200 and isinstance(value, list) else []
        table = AUDIT_TABLES.get(owner)
        target = f"{table}/{row_id}" if table else f"cases/{case_id}"
        status, value, _ = request("GET", f"/api/audit-logs/by-record/{target}")
        if status != 200:
            return []
        if isinstance(value, list):
            return [item for item in value if isinstance(item, dict)]
        items = value.get("items", value.get("data", [])) if isinstance(value, dict) else []
        return [item for item in items if isinstance(item, dict)] if isinstance(items, list) else []

    def validation(case_id: str, authority: str) -> tuple[int | None, Any, dict[str, Any]]:
        return request("GET", f"/api/cases/{case_id}/validation?authority={authority}")

    needs_meddra_code_binding = any(
        scenario.scenario_id in (
            MEDDRA_CODE_SCENARIO_IDS | MEDDRA_CONTEXT_SCENARIO_IDS
        )
        for scenario in scenarios
    )
    needs_meddra_version_binding = any(
        scenario.scenario_id in MEDDRA_VERSION_SCENARIO_PATHS for scenario in scenarios
    )
    needs_meddra_binding = needs_meddra_code_binding or needs_meddra_version_binding
    needs_whodrug_binding = any(
        scenario.scenario_id in WHODRUG_VERSION_SCENARIOS for scenario in scenarios
    )
    needs_ucum_frequency_binding = any(
        scenario.scenario_id == UCUM_FREQUENCY_SCENARIO_ID
        for scenario in scenarios
    )
    runnable_scenarios = [scenario for scenario in scenarios if not scenario.reference_fixture]
    if (
        runnable_scenarios or needs_meddra_binding or needs_whodrug_binding
        or needs_ucum_frequency_binding
    ):
        status, _, summary = request("POST", "/auth/v1/login", {"email": args.email, "pwd": args.password})
        add("login", None, "PASS" if status == 200 else "FAIL", status, summary)
        if status != 200:
            interrupted = interrupted or "login_failed"
    if needs_meddra_binding and not interrupted:
        status, rows, summary = request(
            "GET",
            "/api/terminology/meddra?q=&limit=1&version=28.1&language=en&level=LLT",
        )
        absent_status, absent_rows, absent_summary = request(
            "GET",
            "/api/terminology/meddra?q=99999999&limit=100&version=28.1&level=LLT",
        )
        try:
            if status != 200 or absent_status != 200:
                raise ValueError("MedDRA terminology lookup failed")
            version, valid_code, invalid_code = meddra_binding(rows, absent_rows)
        except ValueError as error:
            interrupted = interrupted or "meddra_binding_failed"
            add("reference_binding", None, "FAIL", status, {
                **summary,
                "absence_lookup": absent_summary,
                "absence_status": absent_status,
                "reason": str(error),
            })
        else:
            scenarios = bind_meddra_code_scenarios(
                scenarios, version, valid_code, invalid_code,
            )
            runnable_scenarios = [
                scenario for scenario in scenarios if not scenario.reference_fixture
            ]
            add("reference_binding", None, "PASS", status, {
                **summary,
                "absence_lookup": absent_summary,
                "absence_status": absent_status,
                "dictionary": "meddra",
                "version": version,
                "language": "en",
                "level": "LLT",
                "valid_code": valid_code,
                "invalid_code": invalid_code,
                "scenario_count": len({
                    scenario.scenario_id for scenario in scenarios
                    if scenario.scenario_id in (
                        MEDDRA_CODE_SCENARIO_IDS | MEDDRA_CONTEXT_SCENARIO_IDS
                    )
                }),
            })
            if needs_meddra_version_binding:
                release_status, releases, release_summary = request(
                    "GET", "/api/terminology/releases?dictionary=meddra&language=en",
                )
                absent_version_status, absent_version_rows, absent_version_summary = request(
                    "GET", "/api/terminology/meddra?q=&limit=1&version=27.1",
                )
                try:
                    if release_status != 200 or absent_version_status != 200:
                        raise ValueError("MedDRA version availability lookup failed")
                    invalid_version = meddra_unavailable_version(
                        releases, absent_version_rows, version,
                    )
                except ValueError as error:
                    add("reference_binding", None, "FAIL", release_status, {
                        **release_summary,
                        "absence_lookup": absent_version_summary,
                        "absence_status": absent_version_status,
                        "reason": str(error),
                        "scenario_family": "meddra_version_unavailable",
                    })
                else:
                    scenarios = bind_meddra_version_scenarios(
                        scenarios, version, invalid_version, valid_code,
                    )
                    runnable_scenarios = [
                        scenario for scenario in scenarios if not scenario.reference_fixture
                    ]
                    add("reference_binding", None, "PASS", release_status, {
                        **release_summary,
                        "absence_lookup": absent_version_summary,
                        "absence_status": absent_version_status,
                        "dictionary": "meddra",
                        "version": version,
                        "invalid_version": invalid_version,
                        "scenario_family": "meddra_version_unavailable",
                        "scenario_count": len(MEDDRA_VERSION_SCENARIO_PATHS),
                    })
    if needs_ucum_frequency_binding and not interrupted:
        release_status, releases, release_summary = request(
            "GET",
            "/api/terminology/releases?dictionary=ich_constrained_ucum&language=en",
        )
        active_releases = [
            release for release in releases
            if isinstance(release, dict)
            and release.get("dictionary") == "ich_constrained_ucum"
            and release.get("version") == "2.2"
            and str(release.get("language", "")).lower() == "en"
            and release.get("status") == "active"
            and isinstance(
                release.get("loaded_rows", release.get("loadedRows")), int,
            )
            and release.get("loaded_rows", release.get("loadedRows")) >= 9
        ] if isinstance(releases, list) else []
        if release_status != 200 or len(active_releases) != 1:
            interrupted = interrupted or "ucum_frequency_binding_failed"
            add("reference_binding", None, "FAIL", release_status, {
                **release_summary,
                "dictionary": "ich_constrained_ucum",
                "version": "2.2",
                "language": "en",
                "scope": "frequency",
                "reason": "one active loaded ICH constrained UCUM 2.2/en release is required",
            })
        else:
            rebound_scenarios: list[Scenario] = []
            for scenario in scenarios:
                if scenario.scenario_id == UCUM_FREQUENCY_SCENARIO_ID:
                    rebound = replace(scenario, reference_fixture=False)
                    scenario = replace(
                        rebound,
                        generation_fingerprint=scenario_fingerprint(rebound),
                    )
                rebound_scenarios.append(scenario)
            scenarios = rebound_scenarios
            runnable_scenarios = [
                scenario for scenario in scenarios if not scenario.reference_fixture
            ]
            add("reference_binding", None, "PASS", release_status, {
                **release_summary,
                "dictionary": "ich_constrained_ucum",
                "version": "2.2",
                "language": "en",
                "scope": "frequency",
                "loaded_rows": active_releases[0].get(
                    "loaded_rows", active_releases[0].get("loadedRows"),
                ),
                "candidate_source": "controlled artifact required frequency contract",
                "valid_candidate": "d",
                "attempt_allowed_only": True,
                "runtime_validity_verified": False,
                "scenario_count": 1,
            })
    if needs_whodrug_binding and not interrupted:
        release_status, releases, release_summary = request(
            "GET", "/api/terminology/releases?dictionary=whodrug&language=en",
        )
        product_status, products, product_summary = request(
            "GET", "/api/terminology/whodrug?q=&limit=1",
        )
        try:
            if release_status != 200 or product_status != 200:
                raise ValueError("WHODrug terminology lookup failed")
            version, product_code = whodrug_binding(releases, products)
        except ValueError as error:
            interrupted = interrupted or "whodrug_binding_failed"
            add("reference_binding", None, "FAIL", release_status, {
                **release_summary,
                "dictionary": "whodrug",
                "product_lookup": product_summary,
                "product_status": product_status,
                "reason": str(error),
            })
        else:
            scenarios = bind_whodrug_version_scenarios(
                scenarios, version, product_code,
            )
            runnable_scenarios = [
                scenario for scenario in scenarios if not scenario.reference_fixture
            ]
            add("reference_binding", None, "PASS", release_status, {
                **release_summary,
                "product_lookup": product_summary,
                "product_status": product_status,
                "dictionary": "whodrug",
                "version": version,
                "language": "en",
                "product_code": product_code,
                "scenario_count": len({
                    scenario.scenario_id for scenario in scenarios
                    if scenario.scenario_id in WHODRUG_VERSION_SCENARIOS
                }),
            })
    for scenario in scenarios:
        if scenario.reference_fixture:
            add("reference_data", scenario, "BLOCKED_REFERENCE_DATA", None, {
                "expected_code": scenario.expected_code,
                "reason": "authoritative reference-data binding is not implemented",
            })

    if runnable_scenarios:
        if not args.message_sender_identifier or not args.message_sender_identifier.strip():
            raise SystemExit(
                "set E2BR3_DEFAULT_MESSAGE_SENDER or --message-sender-identifier"
            )
        if (
            not args.ich_message_receiver_identifier
            or not args.ich_message_receiver_identifier.strip()
        ):
            raise SystemExit(
                "set E2BR3_DEFAULT_MESSAGE_RECEIVER_ICH "
                "or --ich-message-receiver-identifier"
            )

    year = int(scenario_catalog(args.seed)[0].invalid_value[:4])

    def create_case(scenario: Scenario) -> tuple[int | None, str | None, dict[str, Any]]:
        status, value, summary = request("POST", "/api/cases", {
            "data": {
                "safetyReportIdentification": {
                    "safetyReportId": f"BUSINESS-FUZZ-{scenario.generation_token}-{uuid.uuid4()}"
                },
                "status": "draft",
            }
        })
        return status, object_id(value), summary

    def ci_payload(
        field: str | None = None,
        value: Any = None,
        fixture_scenario: Scenario | None = None,
    ) -> dict[str, Any]:
        payload = {
            "transmissionDate": f"{year}0305120000+0900",
            "reportType": "1",
            "dateFirstReceivedFromSource": f"{year}0303",
            "dateOfMostRecentInformation": f"{year}0304",
        }
        if fixture_scenario:
            for path, fixture_value in fixture_scenario.ci_values:
                set_path(payload, path, fixture_value)
        if fixture_scenario and fixture_scenario.owner == "safetyReportIdentification":
            for path, fixture_value in fixture_scenario.fixture_values:
                set_path(payload, path, fixture_value)
        if field:
            if value is None:
                payload.pop(field, None)
            else:
                payload[field] = value
        return payload

    def drug_payload(scenario: Scenario, value: Any) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "drugCharacterization": "1",
            "medicinalProduct": "Business fuzz product",
            "mpid": f"MPID-{args.seed}",
            "cumulativeDoseValue": 12.5,
            "cumulativeDoseUnit": "mg",
            "drugAuthorizationNumber": f"AUTH-{args.seed}",
            "drugAuthorizationCountry": "KR",
        }
        for path, fixture_value in scenario.fixture_values:
            if fixture_value is None and "." not in path:
                payload.pop(path, None)
            else:
                set_path(payload, path, fixture_value)
        if value is None and "." not in scenario.field:
            payload.pop(scenario.field, None)
        elif value is not None:
            set_path(payload, scenario.field, value)
        return payload

    def reaction_payload(scenario: Scenario | None = None, value: Any = None) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "sequenceNumber": 1,
            "primarySourceReaction": "Business fuzz reaction",
            "reactionLanguage": "eng",
            "reactionMeddraVersionLLT": "26.0",
            "reactionMeddraCodeLLT": "10000001",
            "reactionStartDate": f"{year}0303",
            "reactionEndDate": f"{year}0304",
            "reactionDuration": {"value": "1", "unit": "d"},
            "reactionOutcome": "1",
            "seriousness": {
                "serious": True,
                "criteriaResultsInDeath": True,
                "criteriaLifeThreatening": True,
                "criteriaHospitalization": True,
                "criteriaDisabling": True,
                "criteriaCongenitalAnomaly": True,
                "criteriaOtherMedicallyImportant": True,
            },
        }
        if scenario:
            for path, fixture_value in scenario.fixture_values:
                set_path(payload, path, fixture_value)
            set_path(payload, scenario.field, value)
        return payload

    def test_result_payload(scenario: Scenario, value: Any) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "sequenceNumber": 1,
            "testDate": f"{year}0303",
            "testName": "ALT",
            "testMeddraVersion": "26.0",
            "testMeddraCode": "10000001",
            "testResultCode": "1",
            "testResult": "12.5",
            "testUnit": "mg/dL",
            "testResultUnstructured": "Normal",
        }
        for path, fixture_value in scenario.fixture_values:
            set_path(payload, path, fixture_value)
        set_path(payload, scenario.field, value)
        return payload

    def literature_payload(scenario: Scenario, value: Any) -> dict[str, Any]:
        payload: dict[str, Any] = {"sequenceNumber": 1, "referenceText": "Business literature"}
        for path, fixture_value in scenario.fixture_values:
            set_path(payload, path, fixture_value)
        set_path(payload, scenario.field, value)
        return payload

    def patient_payload(scenario: Scenario, value: Any) -> dict[str, Any]:
        payload: dict[str, Any] = {"patientInitials": "BUSINESS-FUZZ"}
        for path, fixture_value in scenario.fixture_values:
            set_path(payload, path, fixture_value)
        set_path(payload, scenario.field, value)
        return payload

    def medical_history_payload(scenario: Scenario, value: Any) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "sequenceNumber": 1,
            "meddraVersion": "26.0",
            "meddraCode": "10000001",
            "startDate": f"{year}0303",
            "endDate": f"{year}0304",
            "continuing": True,
        }
        for path, fixture_value in scenario.fixture_values:
            set_path(payload, path, fixture_value)
        set_path(payload, scenario.field, value)
        return payload

    def death_info_payload(scenario: Scenario, value: Any) -> dict[str, Any]:
        payload: dict[str, Any] = {"autopsyPerformed": True}
        for path, fixture_value in scenario.fixture_values:
            set_path(payload, path, fixture_value)
        set_path(payload, scenario.field, value)
        return payload

    def death_cause_payload(scenario: Scenario, value: Any) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "meddraVersion": "26.0",
            "meddraCode": "10000001",
            "causeText": "Business fuzz cause",
        }
        for path, fixture_value in scenario.fixture_values:
            set_path(payload, path, fixture_value)
        set_path(payload, scenario.field, value)
        return payload

    def parent_payload(scenario: Scenario, value: Any) -> dict[str, Any]:
        payload: dict[str, Any] = {"parentSex": "2"}
        for path, fixture_value in scenario.fixture_values:
            set_path(payload, path, fixture_value)
        set_path(payload, scenario.field, value)
        return payload

    def parent_history_payload(scenario: Scenario, value: Any) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "meddraVersion": "26.0",
            "meddraCode": "10000001",
            "startDate": f"{year}0303",
            "endDate": f"{year}0304",
            "continuing": True,
        }
        for path, fixture_value in scenario.fixture_values:
            set_path(payload, path, fixture_value)
        set_path(payload, scenario.field, value)
        return payload

    def parent_past_drug_payload(scenario: Scenario, value: Any) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "drugName": "Parent prior drug",
            "mpidVersion": "1",
            "mpid": f"MPID-{args.seed}",
            "startDate": f"{year}0303",
            "endDate": f"{year}0304",
            "indicationMeddraVersion": "26.0",
            "indicationMeddraCode": "10000001",
            "reactionMeddraVersion": "26.0",
            "reactionMeddraCode": "10000001",
        }
        omit_whodrug_unrelated_meddra(payload, scenario)
        for path, fixture_value in scenario.fixture_values:
            set_path(payload, path, fixture_value)
        set_path(payload, scenario.field, value)
        return payload

    def past_drug_payload(scenario: Scenario, value: Any) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "sequenceNumber": 1,
            "drugName": "Prior drug",
            "mpid": f"MPID-{args.seed}",
            "startDate": f"{year}0303",
            "endDate": f"{year}0304",
            "indicationMeddraVersion": "26.0",
            "indicationMeddraCode": "10000001",
            "reactionMeddraVersion": "26.0",
            "reactionMeddraCode": "10000001",
        }
        omit_whodrug_unrelated_meddra(payload, scenario)
        for path, fixture_value in scenario.fixture_values:
            set_path(payload, path, fixture_value)
        set_path(payload, scenario.field, value)
        return payload

    def narrative_payload(scenario: Scenario, value: Any) -> dict[str, Any]:
        if scenario.owner == "senderDiagnoses":
            payload: dict[str, Any] = {
                "sequenceNumber": 1,
                "diagnosisMeddraVersion": "26.0",
                "diagnosisMeddraCode": "10000001",
            }
        else:
            payload = {"sequenceNumber": 1, "summaryText": "Business fuzz summary", "languageCode": "eng"}
        for path, fixture_value in scenario.fixture_values:
            set_path(payload, path, fixture_value)
        set_path(payload, scenario.field, value)
        return payload

    def primary_source_payload(scenario: Scenario, value: Any) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "reporterOrganization": "Business Reporter",
            "reporterCountry": "KR",
            "qualification": "1",
            "primarySourceForRegulatoryPurposes": "1",
        }
        for path, fixture_value in scenario.fixture_values:
            set_path(payload, path, fixture_value)
        set_path(payload, scenario.field, value)
        return payload

    def document_payload(scenario: Scenario, value: Any) -> dict[str, Any]:
        payload: dict[str, Any] = {}
        for path, fixture_value in scenario.fixture_values:
            set_path(payload, path, fixture_value)
        set_path(payload, scenario.field, value)
        return payload

    def sender_payload(scenario: Scenario, value: Any) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "senderType": "1",
            "organizationName": "Business Sender",
            "department": "Safety",
            "personTitle": "Dr",
            "personGivenName": "Business",
            "personFamilyName": "Sender",
            "streetAddress": "1 Test Street",
            "city": "Seoul",
            "state": "Seoul",
            "postcode": "04524",
            "countryCode": "KR",
            "telephone": "+82-2-1234-5678",
            "fax": "+82-2-1234-5679",
            "email": "sender@example.com",
        }
        for path, fixture_value in scenario.fixture_values:
            set_path(payload, path, fixture_value)
        set_path(payload, scenario.field, value)
        return payload

    def study_payload(scenario: Scenario, value: Any) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "studyName": "Business Study",
            "sponsorStudyNumber": f"STUDY-{args.seed}",
            "studyTypeReaction": "1",
        }
        for path, fixture_value in scenario.fixture_values:
            set_path(payload, path, fixture_value)
        set_path(payload, scenario.field, value)
        return payload

    def message_header_payload(case_id: str, scenario: Scenario, value: Any) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "case_id": case_id,
            "batch_sender_identifier": args.message_sender_identifier.strip(),
            "batch_receiver_identifier": args.ich_message_receiver_identifier.strip(),
            "batch_transmission_date": [year, 65, 0, 0, 0, 0, 0, 0, 0],
            "message_number": f"BUSINESS-FUZZ-{case_id}",
            "message_sender_identifier": args.message_sender_identifier.strip(),
            "message_receiver_identifier": args.ich_message_receiver_identifier.strip(),
            "message_date": f"{year}0305000000",
        }
        if scenario.page == "N":
            for path, fixture_value in scenario.fixture_values:
                payload[path] = fixture_value
        for path, fixture_value in scenario.header_values:
            payload[path] = fixture_value
        target = {
            "batchSenderIdentifier": "batch_sender_identifier",
            "batchReceiverIdentifier": "batch_receiver_identifier",
            "batchTransmissionDate": "batch_transmission_date",
            "messageSenderIdentifier": "message_sender_identifier",
            "messageReceiverIdentifier": "message_receiver_identifier",
        }.get(scenario.field)
        if target:
            if value is None:
                payload.pop(target, None)
            else:
                payload[target] = value
        return payload

    def run_edge(scenario: Scenario, edge: str, value: Any) -> None:
        nonlocal interrupted
        status, case_id, summary = create_case(scenario)
        if status != 201 or not case_id:
            add(edge, scenario, "FAIL", status, {**summary, "reason": "case_create_failed"})
            interrupted = interrupted or "case_create_failed"
            return

        status, current = page_current(case_id, "CI", "safetyReportIdentification")
        ci_id = object_id(current)
        ci = ci_payload(
            scenario.field
            if scenario.owner == "safetyReportIdentification"
            and scenario.surface != "singleton"
            else None,
            value,
            scenario,
        )
        if (scenario.page == "SI" and not any(path == "reportType" for path, _ in scenario.ci_values)) or scenario.scenario_id in {
            "c2-study-reporter-organization-required",
            "mfds-relatedness-krct-result-required",
        }:
            ci["reportType"] = "2"
        ci["id"] = ci_id
        status, _, save_summary = request("PATCH", f"/api/cases/{case_id}/editor/pages/CI", {
            "authorities": ["ich"],
            "rows": {"safetyReportIdentification": ci},
        })
        if status != 200 or not ci_id:
            add(edge, scenario, "FAIL", status, {**save_summary, "reason": "ci_fixture_failed"})
            return

        if (
            scenario.page != "N"
            and scenario.surface != "device"
            and not (
                scenario.surface == "topology"
                and scenario.authority in {"fda", "mfds"}
            )
        ):
            status, _, header_summary = request(
                "POST",
                f"/api/cases/{case_id}/message-header",
                {"data": message_header_payload(case_id, scenario, None)},
            )
            if status != 201:
                add(edge, scenario, "FAIL", status, {**header_summary, "reason": "header_fixture_failed"})
                return

        if scenario.page != "SI" and scenario.study_values:
            study = {
                "studyName": "Business Study",
                "sponsorStudyNumber": f"STUDY-{args.seed}",
            }
            for path, fixture_value in scenario.study_values:
                set_path(study, path, fixture_value)
            status, _, study_summary = request(
                "PATCH",
                f"/api/cases/{case_id}/editor/pages/SI",
                {"authorities": [scenario.authority], "rows": {"studyInformation": study}},
            )
            if status != 200:
                add(edge, scenario, "FAIL", status, {**study_summary, "reason": "study_fixture_failed"})
                return

        if scenario.page != "AE" and scenario.reaction_values:
            reaction = reaction_payload()
            for path, fixture_value in scenario.reaction_values:
                set_path(reaction, path, fixture_value)
            status, _, reaction_summary = request(
                "POST",
                f"/api/cases/{case_id}/editor/pages/AE/rows",
                {"authorities": [scenario.authority], "rows": {"reaction": reaction}},
            )
            if status != 201:
                add(edge, scenario, "FAIL", status, {**reaction_summary, "reason": "reaction_fixture_failed"})
                return

        if scenario.surface == "singleton":
            before_status, before_value, before_summary = request(
                "GET", f"/api/audit-logs/by-record/cases/{case_id}",
            )
            if before_status != 200 or not isinstance(before_value, list):
                add(edge, scenario, "FAIL", before_status, {
                    **before_summary,
                    "reason": "singleton_audit_before_failed",
                    "audit_response_is_list": isinstance(before_value, list),
                })
                return
            before_items = before_value
            if scenario.expected_code in {"ICH.C.1.1.REQUIRED", "ICH.C.1.2.REQUIRED"}:
                before_read_status, before_read, before_read_summary = request(
                    "GET", f"/api/cases/{case_id}/editor/pages/CI",
                )
                before_rows = before_read.get("rows") if isinstance(before_read, dict) else None
                before_owner = (
                    before_rows.get("safetyReportIdentification")
                    if isinstance(before_rows, dict) else None
                )
                if before_read_status != 200 or not isinstance(before_owner, dict):
                    add(edge, scenario, "FAIL", before_read_status, {
                        **before_read_summary,
                        "reason": "singleton_ci_before_read_failed",
                    })
                    return
                before_actual = get_path(before_owner, scenario.projection_field)
                if values_equal(before_actual, value):
                    add(edge, scenario, "FAIL", before_read_status, {
                        **before_read_summary,
                        "reason": "singleton_ci_baseline_matches_target",
                        "baseline": redacted(before_actual),
                        "target": redacted(value),
                    })
                    return
                status, saved, save_summary = request(
                    "PATCH",
                    f"/api/cases/{case_id}/editor/pages/CI",
                    {"authorities": ["ich"], "rows": {
                        "safetyReportIdentification": {scenario.field: value},
                    }},
                )
                read_status, read_value, read_summary = request(
                    "GET", f"/api/cases/{case_id}/editor/pages/CI",
                )
                read_rows = read_value.get("rows") if isinstance(read_value, dict) else None
                current = (
                    read_rows.get("safetyReportIdentification")
                    if isinstance(read_rows, dict) else None
                )
                if read_status != 200 or not isinstance(current, dict):
                    add(edge, scenario, "FAIL", read_status, {
                        **read_summary,
                        "reason": "singleton_ci_readback_failed",
                    })
                    return
                actual = get_path(current, scenario.projection_field)
                if scenario.expected_code == "ICH.C.1.1.REQUIRED" and value is None:
                    error = saved.get("error") if isinstance(saved, dict) else None
                    data = error.get("data") if isinstance(error, dict) else None
                    detail = data.get("detail") if isinstance(data, dict) else None
                    after_status, after_value, after_summary = request(
                        "GET", f"/api/audit-logs/by-record/cases/{case_id}",
                    )
                    after_items = after_value if isinstance(after_value, list) else None
                    expected_path = EXPECTED_SCENARIO_ISSUE_PATHS[scenario.scenario_id]
                    unchanged = values_equal(before_actual, actual)
                    expected_rejection = (
                        status == 422
                        and error.get("message") == "CONSTRAINT_VIOLATION"
                        and isinstance(detail, dict)
                        and detail.get("ruleCode") == scenario.expected_code
                        and detail.get("path") == expected_path
                        and unchanged
                        and after_status == 200
                        and isinstance(after_items, list)
                        and len(after_items) == len(before_items)
                    ) if isinstance(error, dict) else False
                    add(
                        edge, scenario,
                        "PASS_INPUT_CONTRACT" if expected_rejection else "FAIL",
                        status,
                        {
                            **save_summary,
                            "expected_code": scenario.expected_code,
                            "expected_path": expected_path,
                            "actual_error": error.get("message") if isinstance(error, dict) else None,
                            "actual_code": detail.get("ruleCode") if isinstance(detail, dict) else None,
                            "actual_path": detail.get("path") if isinstance(detail, dict) else None,
                            "before_read_status": before_read_status,
                            "after_read_status": read_status,
                            "before_readback": redacted(before_actual),
                            "after_readback": redacted(actual),
                            "rejected_write_unchanged": unchanged,
                            "audit_status_after": after_status,
                            "audit_rows_before": len(before_items),
                            "audit_rows_after": len(after_items) if isinstance(after_items, list) else None,
                            "audit_error": after_summary if after_status != 200 else None,
                            "reason": "input_contract_rejected" if expected_rejection else "ci_input_contract_mismatch",
                        },
                    )
                    return
            elif scenario.expected_code == "ICH.D.1.REQUIRED" and value is not None:
                status, _, save_summary = request(
                    "PATCH",
                    f"/api/cases/{case_id}/editor/pages/DM",
                    {"authorities": ["ich"], "rows": {"patientInformation": {"patientInitials": value}}},
                )
                read_status, current = page_current(case_id, "DM", "patientInformation")
                actual = get_path(current, "patientInitials")
            elif scenario.expected_code == "ICH.H.1.REQUIRED" and value is not None:
                status, _, save_summary = request(
                    "PATCH",
                    f"/api/cases/{case_id}/editor/pages/NR",
                    {"authorities": ["ich"], "rows": {"narrative": {"caseNarrative": value}}},
                )
                read_status, current = page_current(case_id, "NR", "narrative")
                actual = get_path(current, "caseNarrative")
            elif scenario.expected_code == "ICH.C.2.r.REQUIRED" and value:
                status, _, save_summary = request(
                    "PATCH",
                    f"/api/cases/{case_id}/editor/pages/RP",
                    {"authorities": ["ich"], "rows": {"primarySources": [{
                        "reporterOrganization": "Business Reporter",
                        "reporterCountry": "KR",
                        "qualification": "1",
                        "primarySourceForRegulatoryPurposes": "1",
                    }]}},
                )
                if status != 200:
                    add(edge, scenario, "FAIL", status, {
                        **save_summary, "reason": "singleton_primary_source_fixture_failed",
                    })
                    return
                read_status, read_value, read_summary = request(
                    "GET", f"/api/cases/{case_id}/editor/pages/RP",
                )
                read_rows = read_value.get("rows") if isinstance(read_value, dict) else None
                collection = read_rows.get("primarySources") if isinstance(read_rows, dict) else None
                if read_status != 200 or not isinstance(collection, list):
                    add(edge, scenario, "FAIL", read_status, {
                        **read_summary,
                        "reason": "singleton_primary_source_readback_failed",
                        "collection_is_list": isinstance(collection, list),
                    })
                    return
                actual = bool(collection)
            elif scenario.expected_code == "ICH.G.k.REQUIRED" and value:
                status, created, save_summary = request(
                    "POST",
                    f"/api/cases/{case_id}/editor/pages/DG/rows",
                    {"authorities": ["ich"], "rows": {"drug": {
                        "drugCharacterization": "1",
                        "medicinalProduct": "Business fuzz product",
                    }}},
                )
                owner_id = created_row_id(created)
                if status != 201 or not owner_id:
                    add(edge, scenario, "FAIL", status, {
                        **save_summary, "reason": "singleton_drug_fixture_failed",
                    })
                    return
                read_status, read_value, read_summary = request(
                    "GET", f"/api/cases/{case_id}/editor/pages/DG",
                )
                read_rows = read_value.get("rows") if isinstance(read_value, dict) else None
                collection = read_rows.get("rows") if isinstance(read_rows, dict) else None
                if read_status != 200 or not isinstance(collection, list):
                    add(edge, scenario, "FAIL", read_status, {
                        **read_summary,
                        "reason": "singleton_drug_readback_failed",
                        "collection_is_list": isinstance(collection, list),
                    })
                    return
                actual = bool(collection)
            elif scenario.expected_code == "ICH.E.i.REQUIRED" and value:
                reaction = reaction_payload()
                for path, fixture_value in scenario.fixture_values:
                    set_path(reaction, path, fixture_value)
                status, created, save_summary = request(
                    "POST",
                    f"/api/cases/{case_id}/editor/pages/AE/rows",
                    {"authorities": ["ich"], "rows": {"reaction": reaction}},
                )
                owner_id = created_row_id(created)
                if status != 201 or not owner_id:
                    add(edge, scenario, "FAIL", status, {
                        **save_summary, "reason": "singleton_reaction_fixture_failed",
                    })
                    return
                read_status, read_value, read_summary = request(
                    "GET", f"/api/cases/{case_id}/editor/pages/AE",
                )
                read_rows = read_value.get("rows") if isinstance(read_value, dict) else None
                collection = read_rows.get("rows") if isinstance(read_rows, dict) else None
                matching_rows = [
                    row for row in collection
                    if isinstance(row, dict) and object_id(row) == owner_id
                ] if isinstance(collection, list) else []
                if (
                    read_status != 200
                    or not isinstance(collection, list)
                    or len(matching_rows) != 1
                ):
                    add(edge, scenario, "FAIL", read_status, {
                        **read_summary,
                        "reason": "singleton_reaction_readback_failed",
                        "collection_is_list": isinstance(collection, list),
                        "matching_row_count": len(matching_rows),
                    })
                    return
                actual = bool(collection)
            else:
                page = {
                    "ICH.D.1.REQUIRED": "DM",
                    "ICH.H.1.REQUIRED": "NR",
                    "ICH.C.2.r.REQUIRED": "RP",
                    "ICH.G.k.REQUIRED": "DG",
                    "ICH.E.i.REQUIRED": "AE",
                }[scenario.expected_code]
                status, read_value, save_summary = request(
                    "GET", f"/api/cases/{case_id}/editor/pages/{page}",
                )
                read_status = status
                read_rows = read_value.get("rows") if isinstance(read_value, dict) else None
                if scenario.expected_code == "ICH.D.1.REQUIRED":
                    actual = (
                        get_path(read_rows.get("patientInformation"), "patientInitials")
                        if isinstance(read_rows, dict)
                        and isinstance(read_rows.get("patientInformation"), dict)
                        else None
                    )
                    shape_valid = (
                        status == 200 and isinstance(read_rows, dict)
                        and "patientInformation" in read_rows
                        and (
                            read_rows.get("patientInformation") is None
                            or isinstance(read_rows.get("patientInformation"), dict)
                        )
                    )
                elif scenario.expected_code == "ICH.H.1.REQUIRED":
                    actual = (
                        get_path(read_rows.get("narrative"), "caseNarrative")
                        if isinstance(read_rows, dict)
                        and isinstance(read_rows.get("narrative"), dict)
                        else None
                    )
                    shape_valid = (
                        status == 200 and isinstance(read_rows, dict)
                        and "narrative" in read_rows
                        and (
                            read_rows.get("narrative") is None
                            or isinstance(read_rows.get("narrative"), dict)
                        )
                    )
                else:
                    key = "primarySources" if scenario.expected_code == "ICH.C.2.r.REQUIRED" else "rows"
                    collection = read_rows.get(key) if isinstance(read_rows, dict) else None
                    shape_valid = status == 200 and isinstance(collection, list)
                    actual = bool(collection) if shape_valid else None
                if not shape_valid:
                    add(edge, scenario, "FAIL", status, {
                        **save_summary,
                        "reason": "singleton_absence_readback_failed",
                        "page": page,
                    })
                    return
            after_status, after_value, after_summary = request(
                "GET", f"/api/audit-logs/by-record/cases/{case_id}",
            )
            if after_status != 200 or not isinstance(after_value, list):
                add(edge, scenario, "FAIL", after_status, {
                    **after_summary,
                    "reason": "singleton_audit_after_failed",
                    "audit_response_is_list": isinstance(after_value, list),
                })
                return
            after_items = after_value
            validation_status, report, validation_summary = validation(case_id, scenario.authority)
            present = scenario.expected_code in issue_codes(report)
            expected_present = edge == "invalid_edge"
            expected_issue_path = EXPECTED_SCENARIO_ISSUE_PATHS.get(
                scenario.scenario_id,
            )
            expected_path_issue_count = complete_issue_count(
                report, scenario.expected_code, expected_issue_path,
            ) if expected_issue_path else 0
            complete_logs = [
                log for log in after_items
                if isinstance(log, dict) and audit_log_complete(log)
            ]
            expected_save_status = (
                201
                if scenario.expected_code in {
                    "ICH.G.k.REQUIRED", "ICH.E.i.REQUIRED",
                } and value
                else 200
            )
            passed = (
                status == expected_save_status
                and read_status == 200
                and values_equal(value, actual)
                and validation_status == 200
                and present == expected_present
                and (not expected_present or issue_complete(report, scenario.expected_code))
                and (
                    expected_issue_path is None
                    or (expected_present and expected_path_issue_count == 1)
                    or (not expected_present and expected_path_issue_count == 0)
                )
                and (
                    len(after_items) > len(before_items)
                    if scenario.expected_code in {
                        "ICH.C.1.1.REQUIRED", "ICH.C.1.2.REQUIRED",
                    }
                    else len(after_items) == len(before_items)
                    if expected_present
                    else len(after_items) > len(before_items)
                )
                and bool(complete_logs)
            )
            add(edge, scenario, "PASS" if passed else "FAIL", status, {
                **save_summary,
                "validation": validation_summary,
                "expected_code": scenario.expected_code,
                "expected_save_status": expected_save_status,
                "expected_code_present": expected_present,
                "actual_code_present": present,
                "expected_issue_path": expected_issue_path,
                "expected_path_issue_count": expected_path_issue_count,
                "readback": redacted(actual),
                "audit_rows_before": len(before_items),
                "audit_rows_after": len(after_items),
                "audit_complete": bool(complete_logs),
                "surface": "server-singleton",
            })
            return

        if scenario.surface == "device":
            device_ci = ci_payload(fixture_scenario=scenario)
            device_ci["id"] = ci_id
            device_ci["combinationProductReportIndicator"] = "1"
            if scenario.expected_code == "FDA.G.K.12.REQUIRED":
                device_ci["localCriteriaReportType"] = "5"
            elif scenario.expected_code == "FDA.W0007":
                device_ci["localCriteriaReportType"] = "4"
            status, _, save_summary = request(
                "PATCH",
                f"/api/cases/{case_id}/editor/pages/CI",
                {"authorities": ["fda"], "rows": {"safetyReportIdentification": device_ci}},
            )
            if status != 200:
                add(edge, scenario, "FAIL", status, {**save_summary, "reason": "device_ci_fixture_failed"})
                return
            status, _, save_summary = request(
                "POST",
                f"/api/cases/{case_id}/message-header",
                {"data": {
                    "case_id": case_id,
                    "batch_sender_identifier": args.message_sender_identifier.strip(),
                    "batch_receiver_identifier": "ZZFDA",
                    "batch_transmission_date": [year, 65, 0, 0, 0, 0, 0, 0, 0],
                    "message_number": f"BUSINESS-DEVICE-{case_id}",
                    "message_sender_identifier": args.message_sender_identifier.strip(),
                    "message_receiver_identifier": "CDER",
                    "message_date": f"{year}0305000000",
                }},
            )
            if status != 201:
                add(edge, scenario, "FAIL", status, {**save_summary, "reason": "device_header_fixture_failed"})
                return

            if scenario.expected_code == "FDA.D.1.R0027":
                status, _, save_summary = request(
                    "PATCH",
                    f"/api/cases/{case_id}/editor/pages/DM",
                    {"authorities": ["fda"], "rows": {"patientInformation": {
                        "patientInitialsNullFlavor": value,
                    }}},
                )
                if status != 200:
                    add(edge, scenario, "FAIL", status, {**save_summary, "reason": "device_patient_fixture_failed"})
                    return
                reaction = reaction_payload()
                reaction["reactionMeddraCodeLLT"] = "10067482"
                status, _, save_summary = request(
                    "POST",
                    f"/api/cases/{case_id}/editor/pages/AE/rows",
                    {"authorities": ["fda"], "rows": {"reaction": reaction}},
                )
                if status != 201:
                    add(edge, scenario, "FAIL", status, {**save_summary, "reason": "device_reaction_fixture_failed"})
                    return

            drug = drug_payload(scenario, None)
            if scenario.expected_code == "FDA.G.k.12.COLLECTION.REQUIRED":
                drug.pop("collectionPresent", None)
            drug["drugCharacterization"] = (
                "4" if scenario.expected_code == "FDA.R0072" else "1"
            )
            if scenario.expected_code == "FDA.R0072":
                drug["fdaOtherCharacterization"] = value
            status, created, save_summary = request(
                "POST",
                f"/api/cases/{case_id}/editor/pages/DG/rows",
                {"authorities": ["fda"], "rows": {"drug": drug}},
            )
            drug_id = created_row_id(created)
            if status != 201 or not drug_id:
                add(edge, scenario, "FAIL", status, {**save_summary, "reason": "device_drug_fixture_failed"})
                return

            if scenario.expected_code == "FDA.G.k.12.COLLECTION.REQUIRED":
                before_audit_status, before_audit_value, before_audit_summary = request(
                    "GET", f"/api/audit-logs/by-record/cases/{case_id}",
                )
                before_audit = before_audit_value if isinstance(before_audit_value, list) else None
                if before_audit_status != 200 or not isinstance(before_audit, list):
                    add(edge, scenario, "FAIL", before_audit_status, {
                        **before_audit_summary,
                        "reason": "device_collection_audit_before_failed",
                    })
                    return
                device_id = None
                if value:
                    status, created, save_summary = request(
                        "POST",
                        f"/api/cases/{case_id}/drugs/{drug_id}/devices",
                        {"data": {
                            "drug_id": drug_id,
                            "sequence_number": 1,
                            "malfunction": True,
                            "device_brand_name": "Business Device",
                        }},
                    )
                    device_id = object_id(created)
                    if status != 201 or not device_id:
                        add(edge, scenario, "FAIL", status, {
                            **save_summary,
                            "reason": "device_collection_fixture_failed",
                        })
                        return
                read_status, read_value, read_summary = request(
                    "GET", f"/api/cases/{case_id}/editor/pages/DG/rows/{drug_id}",
                )
                current = (
                    read_value.get("drug", read_value)
                    if isinstance(read_value, dict) else read_value
                )
                if not value:
                    status = read_status
                    save_summary = {**read_summary, "collection_write": "omitted"}
                devices = current.get("fdaDevices") if isinstance(current, dict) else None
                matching_devices = [
                    device for device in devices
                    if isinstance(device, dict) and object_id(device) == device_id
                ] if isinstance(devices, list) and device_id else []
                collection_matches = (
                    object_id(current) == drug_id
                    and
                    isinstance(devices, list)
                    and (
                        (not value and devices == [])
                        or (value and len(matching_devices) == 1
                            and matching_devices[0].get("malfunction") is True)
                    )
                )
                after_audit_status, after_audit_value, after_audit_summary = request(
                    "GET", f"/api/audit-logs/by-record/cases/{case_id}",
                )
                after_audit = after_audit_value if isinstance(after_audit_value, list) else None
                audit_matches = (
                    after_audit_status == 200
                    and isinstance(after_audit, list)
                    and (
                        (not value and after_audit == before_audit)
                        or (value and len(after_audit) > len(before_audit))
                    )
                )
                validation_status, report, validation_summary = validation(case_id, "fda")
                present = scenario.expected_code in issue_codes(report)
                expected_present = edge == "invalid_edge"
                expected_path = EXPECTED_SCENARIO_ISSUE_PATHS[scenario.scenario_id]
                path_count = complete_issue_count(
                    report, scenario.expected_code, expected_path,
                )
                passed = (
                    read_status == 200
                    and collection_matches
                    and audit_matches
                    and validation_status == 200
                    and present == expected_present
                    and (not expected_present or issue_complete(report, scenario.expected_code))
                    and path_count == (1 if expected_present else 0)
                )
                add(edge, scenario, "PASS" if passed else "FAIL", status, {
                    **save_summary,
                    "validation": validation_summary,
                    "expected_code": scenario.expected_code,
                    "expected_issue_path": expected_path,
                    "expected_code_present": expected_present,
                    "actual_code_present": present,
                    "expected_path_issue_count": path_count,
                    "drug_id": drug_id,
                    "device_id": device_id,
                    "device_count": len(devices) if isinstance(devices, list) else None,
                    "matching_device_count": len(matching_devices),
                    "collection_matches": collection_matches,
                    "audit_status_before": before_audit_status,
                    "audit_status_after": after_audit_status,
                    "audit_rows_before": len(before_audit),
                    "audit_rows_after": len(after_audit) if isinstance(after_audit, list) else None,
                    "audit_matches_edge": audit_matches,
                    "audit_error": after_audit_summary if after_audit_status != 200 else None,
                    "surface": "device-collection",
                })
                return

            malfunction = (
                value if scenario.expected_code == "FDA.G.K.12.REQUIRED" else True
            )
            brand_name = (
                value
                if scenario.expected_code == "FDA.G.k.12.r.4-6.AT_LEAST_ONE"
                else "Business Device"
            )
            status, created, save_summary = request(
                "POST",
                f"/api/cases/{case_id}/drugs/{drug_id}/devices",
                {"data": {
                    "drug_id": drug_id,
                    "sequence_number": 1,
                    "malfunction": malfunction,
                    "device_brand_name": brand_name,
                }},
            )
            device_id = object_id(created)
            if status != 201 or not device_id:
                add(edge, scenario, "FAIL", status, {**save_summary, "reason": "device_fixture_failed"})
                return

            problem_value = (
                value if scenario.expected_code == "FDA.G.K.12.R.3.REQUIRED" else "1234567"
            )
            if malfunction and problem_value is not None:
                status, _, save_summary = request(
                    "POST",
                    f"/api/cases/{case_id}/drugs/{drug_id}/devices/{device_id}/codes",
                    {"data": {"device_id": device_id, "element": "device_problem", "sequence_number": 1, "value_code": problem_value}},
                )
                if status != 201:
                    add(edge, scenario, "FAIL", status, {**save_summary, "reason": "device_problem_fixture_failed"})
                    return
            if scenario.expected_code == "FDA.W0007" and value is not None:
                status, _, save_summary = request(
                    "POST",
                    f"/api/cases/{case_id}/drugs/{drug_id}/devices/{device_id}/codes",
                    {"data": {"device_id": device_id, "element": "remedial_action", "sequence_number": 2, "value_code": value}},
                )
                if status != 201:
                    add(edge, scenario, "FAIL", status, {**save_summary, "reason": "device_remedial_fixture_failed"})
                    return

            read_status, current = page_current(case_id, "DG", "drug", drug_id)
            devices = current.get("fdaDevices", []) if isinstance(current, dict) else []
            device = devices[0] if devices and isinstance(devices[0], dict) else {}
            if scenario.expected_code == "FDA.D.1.R0027":
                read_status, patient = page_current(case_id, "DM", "patientInformation")
                actual = get_path(patient, "patientInitialsNullFlavor")
            elif scenario.expected_code == "FDA.G.K.12.REQUIRED":
                actual = device.get("malfunction")
            elif scenario.expected_code == "FDA.G.k.12.r.4-6.AT_LEAST_ONE":
                actual = device.get("device_brand_name")
            elif scenario.expected_code == "FDA.G.K.12.R.3.REQUIRED":
                codes = device.get("deviceProblemCodes", [])
                actual = codes[0].get("value_code") if codes else None
            elif scenario.expected_code == "FDA.R0072":
                actual = current.get("fda_other_characterization") if isinstance(current, dict) else None
            else:
                codes = device.get("remedialActions", [])
                actual = codes[0].get("value_code") if codes else None

            audit_status, audit_value, _ = request("GET", f"/api/audit-logs/by-record/cases/{case_id}")
            audit_items = (
                audit_value if isinstance(audit_value, list)
                else audit_value.get("items", audit_value.get("data", []))
                if isinstance(audit_value, dict) else []
            )
            complete_logs = [
                log for log in audit_items
                if isinstance(log, dict) and audit_log_complete(log)
            ]
            validation_status, report, validation_summary = validation(case_id, scenario.authority)
            present = scenario.expected_code in issue_codes(report)
            expected_present = edge == "invalid_edge"
            passed = (
                read_status == 200
                and values_equal(value, actual)
                and audit_status == 200
                and bool(complete_logs)
                and validation_status == 200
                and present == expected_present
                and (not expected_present or issue_complete(report, scenario.expected_code))
            )
            add(edge, scenario, "PASS" if passed else "FAIL", status, {
                **save_summary,
                "validation": validation_summary,
                "expected_code": scenario.expected_code,
                "expected_code_present": expected_present,
                "actual_code_present": present,
                "readback": redacted(actual),
                "audit_logs": len(audit_items),
                "audit_complete": bool(complete_logs),
                "surface": "device-subresource",
            })
            return

        if scenario.surface == "topology":
            topology_ci = ci_payload(fixture_scenario=scenario)
            topology_ci["id"] = ci_id
            if scenario.expected_code in {
                "ICH.D.1.1.4.REQUIRED",
                "MFDS.D.1.1.4.REQUIRED",
                "MFDS.C.5.1.r.1.RECEIVER.REQUIRED",
                "MFDS.C.5.1.r.1.NULLFLAVOR.FORBIDDEN",
                "FDA.W0010",
                "FDA.G.k.9.REQUIRED",
            }:
                topology_ci["reportType"] = "2"
            linked_reports: list[dict[str, Any]] = []
            if (
                scenario.expected_code == "FDA.W0001" and value
            ) or scenario.expected_code == "FDA.W0010":
                linked_reports = [{
                    "sequenceNumber": 1,
                    "linkedReportNumber": f"LINKED-{args.seed}",
                }]
            status, _, save_summary = request(
                "PATCH",
                f"/api/cases/{case_id}/editor/pages/CI",
                {"authorities": [scenario.authority], "rows": {
                    "safetyReportIdentification": topology_ci,
                    "linkedReports": linked_reports,
                }},
            )
            if status != 200:
                add(edge, scenario, "FAIL", status, {**save_summary, "reason": "topology_ci_fixture_failed"})
                return

            if scenario.authority in {"mfds", "fda"}:
                if scenario.authority == "mfds":
                    batch_receiver = message_receiver = "MFDS-O-CT"
                elif scenario.expected_code in {"FDA.W0010", "FDA.G.k.9.REQUIRED"}:
                    batch_receiver, message_receiver = "ZZFDA_PREMKT", "CDER_IND"
                else:
                    batch_receiver, message_receiver = "ZZFDA", "CDER"
                status, _, save_summary = request(
                    "POST",
                    f"/api/cases/{case_id}/message-header",
                    {"data": {
                        "case_id": case_id,
                        "batch_sender_identifier": args.message_sender_identifier.strip(),
                        "batch_receiver_identifier": batch_receiver,
                        "batch_transmission_date": [year, 65, 0, 0, 0, 0, 0, 0, 0],
                        "message_number": f"BUSINESS-TOPOLOGY-{case_id}",
                        "message_sender_identifier": args.message_sender_identifier.strip(),
                        "message_receiver_identifier": message_receiver,
                        "message_date": f"{year}0305000000",
                    }},
                )
                if status != 201:
                    add(edge, scenario, "FAIL", status, {**save_summary, "reason": "topology_header_fixture_failed"})
                    return

            patient = {"patientInitials": "BUSINESS-FUZZ"}
            identifiers: list[dict[str, Any]] = []
            if scenario.expected_code in {"FDA.W0001", "FDA.W0002"}:
                patient["patientInitials"] = "AGGREGATE"
            elif scenario.expected_code == "FDA.W0010":
                patient["patientInitials"] = value
            elif scenario.expected_code in {
                "ICH.D.1.1.4.REQUIRED", "MFDS.D.1.1.4.REQUIRED",
            } and value:
                identifiers = [{
                    "sequenceNumber": 1,
                    "identifierTypeCode": "4",
                    "identifierValue": f"STUDY-{args.seed}",
                }]
            status, _, save_summary = request(
                "PATCH",
                f"/api/cases/{case_id}/editor/pages/DM",
                {"authorities": [scenario.authority], "rows": {
                    "patientInformation": patient,
                    "patientIdentifiers": identifiers,
                }},
            )
            if status != 200:
                add(edge, scenario, "FAIL", status, {**save_summary, "reason": "topology_patient_fixture_failed"})
                return

            study = {
                "studyName": "Business topology study",
                "sponsorStudyNumber": f"STUDY-{args.seed}",
                "studyTypeReaction": "1",
            }
            registrations: list[dict[str, Any]] = []
            if scenario.expected_code == "FDA.W0002":
                study["studyTypeReaction"] = value
            elif scenario.expected_code in {"FDA.W0010", "FDA.G.k.9.REQUIRED"}:
                study["fdaIndNumberOccurred"] = "123456"
            elif scenario.expected_code == "MFDS.C.5.1.r.1.RECEIVER.REQUIRED" and value:
                registrations = [{
                    "sequenceNumber": 1,
                    "registrationNumber": f"REG-{args.seed}",
                    "countryCode": "KR",
                }]
            elif scenario.expected_code == "MFDS.C.5.1.r.1.NULLFLAVOR.FORBIDDEN":
                registrations = [{
                    "sequenceNumber": 1,
                    "registrationNumber": None if value else f"REG-{args.seed}",
                    "registrationNumberNullFlavor": value,
                    "countryCode": "KR",
                }]
            status, _, save_summary = request(
                "PATCH",
                f"/api/cases/{case_id}/editor/pages/SI",
                {"authorities": [scenario.authority], "rows": {
                    "studyInformation": study,
                    "studyRegistrationNumbers": registrations,
                }},
            )
            if status != 200:
                add(edge, scenario, "FAIL", status, {**save_summary, "reason": "topology_study_fixture_failed"})
                return

            if scenario.expected_code == "FDA.G.k.9.REQUIRED":
                status, reaction_created, save_summary = request(
                    "POST", f"/api/cases/{case_id}/editor/pages/AE/rows",
                    {"authorities": ["fda"], "rows": {"reaction": reaction_payload()}},
                )
                reaction_id = created_row_id(reaction_created)
                if status != 201 or not reaction_id:
                    add(edge, scenario, "FAIL", status, {
                        **save_summary, "reason": "topology_assessment_reaction_failed",
                    })
                    return
                status, drug_created, save_summary = request(
                    "POST", f"/api/cases/{case_id}/editor/pages/DG/rows",
                    {"authorities": ["fda"], "rows": {"drug": {
                        "drugCharacterization": "1",
                        "medicinalProduct": "Business fuzz product",
                    }}},
                )
                drug_id = created_row_id(drug_created)
                if status != 201 or not drug_id:
                    add(edge, scenario, "FAIL", status, {
                        **save_summary, "reason": "topology_assessment_drug_failed",
                    })
                    return
                before_audit_status, before_audit_value, before_audit_summary = request(
                    "GET", f"/api/audit-logs/by-record/cases/{case_id}",
                )
                before_audit = before_audit_value if isinstance(before_audit_value, list) else None
                if before_audit_status != 200 or not isinstance(before_audit, list):
                    add(edge, scenario, "FAIL", before_audit_status, {
                        **before_audit_summary,
                        "reason": "topology_assessment_audit_before_failed",
                    })
                    return
                if value:
                    status, _, save_summary = request(
                        "PATCH", f"/api/cases/{case_id}/editor/pages/DG/rows/{drug_id}",
                        {"authorities": ["fda"], "rows": {"drug": {
                            "drugReactionAssessments": [{
                                "reactionId": reaction_id,
                                "sourceOfAssessment": "Sponsor",
                                "methodOfAssessment": "FDA",
                                "resultOfAssessment": "Suspected",
                            }],
                        }}},
                    )
                    if status != 200:
                        add(edge, scenario, "FAIL", status, {
                            **save_summary,
                            "reason": "topology_assessment_fixture_failed",
                        })
                        return
                read_status, read_value, read_summary = request(
                    "GET", f"/api/cases/{case_id}/editor/pages/DG/rows/{drug_id}",
                )
                current = (
                    read_value.get("drug", read_value)
                    if isinstance(read_value, dict) else read_value
                )
                if not value:
                    status = read_status
                    save_summary = {**read_summary, "collection_write": "omitted"}
                assessments = (
                    current.get("drugReactionAssessments")
                    if isinstance(current, dict) else None
                )
                matching_assessments = [
                    assessment for assessment in assessments
                    if isinstance(assessment, dict)
                    and assessment.get("reactionId") == reaction_id
                    and assessment.get("sourceOfAssessment") == "Sponsor"
                    and assessment.get("methodOfAssessment") == "FDA"
                    and assessment.get("resultOfAssessment") == "Suspected"
                ] if isinstance(assessments, list) else []
                assessment_identity = (
                    object_identity(matching_assessments[0])
                    if len(matching_assessments) == 1 else {}
                )
                assessment_identity_matches = (
                    assessment_identity.get("reactionId") == reaction_id
                    and "drugReactionAssessmentId" in assessment_identity
                )
                collection_matches = (
                    object_id(current) == drug_id
                    and
                    isinstance(assessments, list)
                    and (
                        (not value and assessments == [])
                        or (value and len(matching_assessments) == 1
                            and assessment_identity_matches)
                    )
                )
                after_audit_status, after_audit_value, after_audit_summary = request(
                    "GET", f"/api/audit-logs/by-record/cases/{case_id}",
                )
                after_audit = after_audit_value if isinstance(after_audit_value, list) else None
                audit_matches = (
                    after_audit_status == 200
                    and isinstance(after_audit, list)
                    and (
                        (not value and after_audit == before_audit)
                        or (value and len(after_audit) > len(before_audit))
                    )
                )
                validation_status, report, validation_summary = validation(case_id, "fda")
                present = scenario.expected_code in issue_codes(report)
                expected_present = edge == "invalid_edge"
                expected_path = EXPECTED_SCENARIO_ISSUE_PATHS[scenario.scenario_id]
                path_count = complete_issue_count(
                    report, scenario.expected_code, expected_path,
                )
                passed = (
                    read_status == 200
                    and collection_matches
                    and audit_matches
                    and validation_status == 200
                    and present == expected_present
                    and (not expected_present or issue_complete(report, scenario.expected_code))
                    and path_count == (1 if expected_present else 0)
                )
                add(edge, scenario, "PASS" if passed else "FAIL", status, {
                    **save_summary,
                    "validation": validation_summary,
                    "expected_code": scenario.expected_code,
                    "expected_issue_path": expected_path,
                    "expected_code_present": expected_present,
                    "actual_code_present": present,
                    "expected_path_issue_count": path_count,
                    "reaction_id": reaction_id,
                    "drug_id": drug_id,
                    "assessment_count": len(assessments) if isinstance(assessments, list) else None,
                    "matching_assessment_count": len(matching_assessments),
                    "assessment_id": (
                        assessment_identity.get("drugReactionAssessmentId")
                    ),
                    "assessment_identity_matches": assessment_identity_matches,
                    "collection_matches": collection_matches,
                    "audit_status_before": before_audit_status,
                    "audit_status_after": after_audit_status,
                    "audit_rows_before": len(before_audit),
                    "audit_rows_after": len(after_audit) if isinstance(after_audit, list) else None,
                    "audit_matches_edge": audit_matches,
                    "audit_error": after_audit_summary if after_audit_status != 200 else None,
                    "surface": "assessment-collection-topology",
                })
                return

            if scenario.expected_code == "FDA.W0001":
                read_status, current = page_current(case_id, "CI", "linkedReports")
                actual = bool(current)
            elif scenario.expected_code == "FDA.W0002":
                read_status, current = page_current(case_id, "SI", "studyInformation")
                actual = get_path(current, "studyTypeReaction")
            elif scenario.expected_code == "FDA.W0010":
                read_status, current = page_current(case_id, "DM", "patientInformation")
                actual = get_path(current, "patientInitials")
            elif scenario.expected_code in {
                "ICH.D.1.1.4.REQUIRED", "MFDS.D.1.1.4.REQUIRED",
            }:
                read_status, current = page_current(case_id, "DM", "patientIdentifiers")
                actual = bool(current)
            elif scenario.expected_code == "MFDS.C.5.1.r.1.RECEIVER.REQUIRED":
                read_status, current = page_current(case_id, "SI", "studyRegistrationNumbers")
                actual = bool(current)
            else:
                read_status, current = page_current(case_id, "SI", "studyRegistrationNumbers")
                actual = get_path(current, "registration_number_null_flavor")

            audit_status, audit_value, _ = request("GET", f"/api/audit-logs/by-record/cases/{case_id}")
            audit_items = (
                audit_value if isinstance(audit_value, list)
                else audit_value.get("items", audit_value.get("data", []))
                if isinstance(audit_value, dict) else []
            )
            complete_logs = [
                log for log in audit_items
                if isinstance(log, dict) and audit_log_complete(log)
            ]
            validation_status, report, validation_summary = validation(case_id, scenario.authority)
            present = scenario.expected_code in issue_codes(report)
            expected_present = edge == "invalid_edge"
            passed = (
                read_status == 200
                and values_equal(value, actual)
                and audit_status == 200
                and bool(complete_logs)
                and validation_status == 200
                and present == expected_present
                and (not expected_present or issue_complete(report, scenario.expected_code))
            )
            add(edge, scenario, "PASS" if passed else "FAIL", status, {
                **save_summary,
                "validation": validation_summary,
                "expected_code": scenario.expected_code,
                "expected_code_present": expected_present,
                "actual_code_present": present,
                "readback": redacted(actual),
                "audit_logs": len(audit_items),
                "audit_complete": bool(complete_logs),
                "surface": "collection-topology",
            })
            return

        contract_rejection_scenarios = {
            "reaction-seriousness-null-flavor-ni-only": (
                "AE", "ICH.E.i.3.2a.NULLFLAVOR.ALLOWED",
                "reactions.0.seriousness.criteriaResultsInDeathNullFlavor",
                "nullFlavor must be one of: NI",
            ),
            "mfds-test-date-null-flavor-vocabulary": (
                "LB", "ICH.F.r.1.NULLFLAVOR.ALLOWED",
                "testResults.0.testDateNullFlavor",
                "nullFlavor must be one of: UNK",
            ),
        }
        contract_before_rows: list[Any] | None = None
        contract_before_audit: list[Any] | None = None
        contract_page_status: int | None = None
        contract_audit_status: int | None = None
        if scenario.scenario_id in contract_rejection_scenarios:
            contract_page = contract_rejection_scenarios[scenario.scenario_id][0]
            contract_page_status, contract_page_value, contract_page_summary = request(
                "GET", f"/api/cases/{case_id}/editor/pages/{contract_page}",
            )
            contract_rows = (
                contract_page_value.get("rows")
                if isinstance(contract_page_value, dict) else None
            )
            contract_before_rows = (
                contract_rows.get("rows") if isinstance(contract_rows, dict) else None
            )
            if contract_page_status != 200 or not isinstance(contract_before_rows, list):
                add(edge, scenario, "FAIL", contract_page_status, {
                    **contract_page_summary,
                    "reason": "input_contract_before_page_failed",
                    "rows_are_list": isinstance(contract_before_rows, list),
                })
                return
            contract_audit_status, contract_audit_value, contract_audit_summary = request(
                "GET", f"/api/audit-logs/by-record/cases/{case_id}",
            )
            contract_before_audit = (
                contract_audit_value if isinstance(contract_audit_value, list) else None
            )
            if contract_audit_status != 200 or not isinstance(contract_before_audit, list):
                add(edge, scenario, "FAIL", contract_audit_status, {
                    **contract_audit_summary,
                    "reason": "input_contract_before_audit_failed",
                    "audit_is_list": isinstance(contract_before_audit, list),
                })
                return

        if scenario.scenario_id == "reaction-hcp-medical-confirmation-omit":
            status, _, save_summary = request("PATCH", f"/api/cases/{case_id}/editor/pages/RP", {
                "authorities": [scenario.authority],
                "rows": {"primarySources": [{
                    "reporterOrganization": "Business HCP",
                    "reporterCountry": "KR",
                    "qualification": "1",
                    "primarySourceForRegulatoryPurposes": "1",
                }]},
            })
            if status != 200:
                add(edge, scenario, "FAIL", status, {**save_summary, "reason": "hcp_fixture_failed"})
                return

        if scenario.page == "N":
            status, created, save_summary = request(
                "POST",
                f"/api/cases/{case_id}/message-header",
                {"data": message_header_payload(case_id, scenario, value)},
            )
            owner_id = object_id(created)
            if status != 201 or not owner_id:
                add(edge, scenario, "FAIL", status, {**save_summary, "reason": "n_fixture_failed"})
                return
            if scenario.field == "batchNumber":
                status, _, save_summary = request(
                    "PUT",
                    f"/api/cases/{case_id}/message-header",
                    {"data": {"batch_number": value}},
                )
                if status != 200:
                    add(edge, scenario, "FAIL", status, {**save_summary, "reason": "n_update_failed"})
                    return
            read_status, current, _ = request("GET", f"/api/cases/{case_id}/message-header")
        elif scenario.page == "CI":
            if scenario.owner == "safetyReportIdentification":
                owner_id = ci_id
                read_status, current = page_current(case_id, "CI", scenario.owner)
            else:
                status, _, save_summary = request("PATCH", f"/api/cases/{case_id}/editor/pages/CI", {
                    "authorities": [scenario.authority],
                    "rows": {scenario.owner: [document_payload(scenario, value)]},
                })
                read_status, current = page_current(case_id, "CI", scenario.owner)
                owner_id = object_id(current)
                if status != 200 or not owner_id:
                    add(edge, scenario, "FAIL", status, {**save_summary, "reason": "ci_document_fixture_failed"})
                    return
        elif scenario.page == "RP":
            sources = [primary_source_payload(scenario, value)]
            if scenario.scenario_id == "c2-primary-source-exactly-once":
                sources.append({
                    "sequenceNumber": 2,
                    "reporterOrganization": "Second Reporter",
                    "reporterCountry": "KR",
                    "qualification": "1",
                    "primarySourceForRegulatoryPurposes": "1",
                })
            status, _, save_summary = request("PATCH", f"/api/cases/{case_id}/editor/pages/RP", {
                "authorities": [scenario.authority],
                "rows": {"primarySources": sources},
            })
            read_status, current = page_current(case_id, "RP", scenario.owner)
            owner_id = object_id(current)
            if status != 200 or not owner_id:
                add(edge, scenario, "FAIL", status, {**save_summary, "reason": "rp_fixture_failed"})
                return
        elif scenario.page == "SD":
            status, _, save_summary = request("PATCH", f"/api/cases/{case_id}/editor/pages/SD", {
                "authorities": [scenario.authority],
                "rows": {"senderInformation": sender_payload(scenario, value)},
            })
            read_status, current = page_current(case_id, "SD", scenario.owner)
            owner_id = object_id(current)
            if status != 200 or not owner_id:
                add(edge, scenario, "FAIL", status, {**save_summary, "reason": "sd_fixture_failed"})
                return
        elif scenario.page == "SI":
            rows = {"studyInformation": study_payload(scenario, value)}
            if scenario.owner == "studyRegistrationNumbers":
                rows = {
                    "studyInformation": {
                        "studyName": "Business Study",
                        "sponsorStudyNumber": f"STUDY-{args.seed}",
                        "studyTypeReaction": "1",
                    },
                    "studyRegistrationNumbers": [{
                        "sequenceNumber": 1,
                        "registrationNumber": f"REG-{args.seed}",
                        scenario.field: value,
                    }],
                }
            status, _, save_summary = request("PATCH", f"/api/cases/{case_id}/editor/pages/SI", {
                "authorities": [scenario.authority],
                "rows": rows,
            })
            read_status, current = page_current(case_id, "SI", scenario.owner)
            owner_id = object_id(current)
            if status != 200 or not owner_id:
                add(edge, scenario, "FAIL", status, {**save_summary, "reason": "si_fixture_failed"})
                return
        elif scenario.page == "DM":
            rows: dict[str, Any] = {"patientInformation": {"patientInitials": "BUSINESS-FUZZ"}}
            if scenario.owner == "patientInformation":
                rows[scenario.owner] = patient_payload(scenario, value)
            elif scenario.owner == "medicalHistoryEpisodes":
                rows[scenario.owner] = [medical_history_payload(scenario, value)]
                if scenario.scenario_id == "d-history-parent-duplicate":
                    rows["parentInfo"] = {"parentSex": "2"}
                    rows["parentMedicalHistory"] = [{"meddraVersion": "26.0", "meddraCode": "10000001"}]
            elif scenario.owner == "deathInfo":
                rows[scenario.owner] = death_info_payload(scenario, value)
            elif scenario.owner in {"reportedCauses", "autopsyCauses"}:
                rows["deathInfo"] = {"dateOfDeath": f"{year}0303", "autopsyPerformed": True}
                rows[scenario.owner] = [death_cause_payload(scenario, value)]
            elif scenario.owner == "parentInfo":
                rows[scenario.owner] = parent_payload(scenario, value)
            elif scenario.owner == "parentMedicalHistory":
                rows["parentInfo"] = {"parentSex": "2"}
                rows[scenario.owner] = [parent_history_payload(scenario, value)]
            else:
                rows["parentInfo"] = {"parentSex": "2"}
                rows[scenario.owner] = [parent_past_drug_payload(scenario, value)]
            d6_before_status: int | None = None
            d6_before: Any = None
            if scenario.scenario_id == "d-lmp-null-flavor-allowed" and edge == "invalid_edge":
                d6_before_status, d6_before = page_current(
                    case_id, "DM", scenario.owner,
                )
            status, saved, save_summary = request("PATCH", f"/api/cases/{case_id}/editor/pages/DM", {
                "authorities": [scenario.authority],
                "rows": rows,
            })
            if scenario.scenario_id == "d-lmp-null-flavor-allowed" and edge == "invalid_edge":
                error = saved.get("error") if isinstance(saved, dict) else None
                data = error.get("data") if isinstance(error, dict) else None
                detail = data.get("detail") if isinstance(data, dict) else None
                expected_path = EXPECTED_SCENARIO_ISSUE_PATHS[scenario.scenario_id]
                d6_after_status, d6_after = page_current(
                    case_id, "DM", scenario.owner,
                )
                before_value = get_path(d6_before, scenario.projection_field)
                after_value = get_path(d6_after, scenario.projection_field)
                before_owner_present = object_id(d6_before) is not None
                after_owner_present = object_id(d6_after) is not None
                unchanged = (
                    d6_before_status == 200
                    and d6_after_status == 200
                    and values_equal(before_value, after_value)
                    and before_owner_present == after_owner_present
                )
                expected_rejection = (
                    status == 422
                    and error.get("message") == "CONSTRAINT_VIOLATION"
                    and isinstance(detail, dict)
                    and detail.get("ruleCode") == scenario.expected_code
                    and detail.get("path") == expected_path
                    and unchanged
                ) if isinstance(error, dict) else False
                add(edge, scenario, "PASS_INPUT_CONTRACT" if expected_rejection else "FAIL", status, {
                    **save_summary,
                    "expected_code": scenario.expected_code,
                    "expected_path": expected_path,
                    "actual_error": error.get("message") if isinstance(error, dict) else None,
                    "actual_code": detail.get("ruleCode") if isinstance(detail, dict) else None,
                    "actual_path": detail.get("path") if isinstance(detail, dict) else None,
                    "before_read_status": d6_before_status,
                    "after_read_status": d6_after_status,
                    "before_readback": redacted(before_value),
                    "after_readback": redacted(after_value),
                    "before_owner_present": before_owner_present,
                    "after_owner_present": after_owner_present,
                    "rejected_write_unchanged": unchanged,
                    "reason": "input_contract_rejected" if expected_rejection else "dm_input_contract_mismatch",
                })
                return
            read_status, current = page_current(case_id, "DM", scenario.owner)
            owner_id = object_id(current)
            if status != 200 or not owner_id:
                add(edge, scenario, "FAIL", status, {**save_summary, "reason": "dm_fixture_failed"})
                return
        elif scenario.page == "NR":
            rows = {"narrative": {"caseNarrative": "Business fuzz narrative"}}
            if scenario.owner == "narrative":
                rows["narrative"][scenario.field] = value
            else:
                rows[scenario.owner] = [narrative_payload(scenario, value)]
            status, _, save_summary = request("PATCH", f"/api/cases/{case_id}/editor/pages/NR", {
                "authorities": [scenario.authority],
                "rows": rows,
            })
            read_status, current = page_current(case_id, "NR", scenario.owner)
            owner_id = object_id(current)
            if status != 200 or not owner_id:
                add(edge, scenario, "FAIL", status, {**save_summary, "reason": "nr_fixture_failed"})
                return
        elif scenario.page == "DH":
            status, _, _ = request("PATCH", f"/api/cases/{case_id}/editor/pages/DM", {
                "authorities": [scenario.authority],
                "rows": {"patientInformation": {"patientInitials": "BUSINESS-FUZZ"}},
            })
            if status != 200:
                add(edge, scenario, "FAIL", status, {"reason": "dh_patient_fixture_failed"})
                return
            status, created, save_summary = request("POST", f"/api/cases/{case_id}/editor/pages/DH/rows", {
                "authorities": [scenario.authority],
                "rows": {"pastDrugHistory": past_drug_payload(scenario, value)},
            })
            owner_id = created_row_id(created)
            if status != 201 or not owner_id:
                add(edge, scenario, "FAIL", status, {**save_summary, "reason": "dh_fixture_failed"})
                return
            read_status, current = page_current(case_id, "DH", scenario.owner, owner_id)
        elif scenario.page == "DG":
            payload = drug_payload(scenario, value)
            if scenario.field.startswith("drugReactionAssessments[]"):
                status, reaction, save_summary = request("POST", f"/api/cases/{case_id}/editor/pages/AE/rows", {
                    "authorities": [scenario.authority],
                    "rows": {"reaction": reaction_payload()},
                })
                reaction_id = created_row_id(reaction)
                if status != 201 or not reaction_id:
                    add(edge, scenario, "FAIL", status, {**save_summary, "reason": "dg_reaction_fixture_failed"})
                    return
                set_path(payload, "drugReactionAssessments[].reactionId", reaction_id)
            status, created, save_summary = request("POST", f"/api/cases/{case_id}/editor/pages/DG/rows", {
                "authorities": [scenario.authority],
                "rows": {"drug": payload},
            })
            owner_id = created_row_id(created)
            if status != 201 or not owner_id:
                add(edge, scenario, "FAIL", status, {**save_summary, "reason": "dg_fixture_failed"})
                return
            read_status, current = page_current(case_id, "DG", scenario.owner, owner_id)
        elif scenario.page == "AE":
            status, created, save_summary = request("POST", f"/api/cases/{case_id}/editor/pages/AE/rows", {
                "authorities": [scenario.authority],
                "rows": {"reaction": reaction_payload(scenario, value)},
            })
            if (
                scenario.scenario_id == "reaction-seriousness-null-flavor-ni-only"
                and edge == "invalid_edge"
            ):
                error = created.get("error") if isinstance(created, dict) else None
                data = error.get("data") if isinstance(error, dict) else None
                detail = data.get("detail") if isinstance(data, dict) else None
                after_page_status, after_page_value, after_page_summary = request(
                    "GET", f"/api/cases/{case_id}/editor/pages/AE",
                )
                after_page_rows = (
                    after_page_value.get("rows")
                    if isinstance(after_page_value, dict) else None
                )
                after_rows = (
                    after_page_rows.get("rows")
                    if isinstance(after_page_rows, dict) else None
                )
                after_audit_status, after_audit_value, after_audit_summary = request(
                    "GET", f"/api/audit-logs/by-record/cases/{case_id}",
                )
                after_audit = after_audit_value if isinstance(after_audit_value, list) else None
                _, contract_code, contract_path, contract_message = contract_rejection_scenarios[
                    scenario.scenario_id
                ]
                unchanged = (
                    after_page_status == 200
                    and isinstance(after_rows, list)
                    and after_rows == contract_before_rows
                    and after_audit_status == 200
                    and isinstance(after_audit, list)
                    and after_audit == contract_before_audit
                )
                expected_rejection = (
                    status == 422
                    and isinstance(error, dict)
                    and error.get("message") == "CONSTRAINT_VIOLATION"
                    and isinstance(detail, dict)
                    and detail.get("ruleCode") == contract_code
                    and detail.get("path") == contract_path
                    and detail.get("message") == contract_message
                    and unchanged
                )
                add(edge, scenario, "PASS_INPUT_CONTRACT" if expected_rejection else "FAIL", status, {
                    **save_summary,
                    "expected_code": scenario.expected_code,
                    "expected_contract_code": contract_code,
                    "expected_contract_path": contract_path,
                    "expected_contract_message": contract_message,
                    "actual_error": error.get("message") if isinstance(error, dict) else None,
                    "actual_code": detail.get("ruleCode") if isinstance(detail, dict) else None,
                    "actual_path": detail.get("path") if isinstance(detail, dict) else None,
                    "actual_message": detail.get("message") if isinstance(detail, dict) else None,
                    "before_page_status": contract_page_status,
                    "after_page_status": after_page_status,
                    "before_row_count": len(contract_before_rows),
                    "after_row_count": len(after_rows) if isinstance(after_rows, list) else None,
                    "before_audit_status": contract_audit_status,
                    "after_audit_status": after_audit_status,
                    "rows_and_audit_unchanged": unchanged,
                    "page_read_error": after_page_summary if after_page_status != 200 else None,
                    "audit_read_error": after_audit_summary if after_audit_status != 200 else None,
                    "reason": "input_contract_rejected" if expected_rejection else "ae_input_contract_mismatch",
                })
                return
            owner_id = created_row_id(created)
            if status != 201 or not owner_id:
                add(edge, scenario, "FAIL", status, {**save_summary, "reason": "ae_fixture_failed"})
                return
            read_status, current = page_current(case_id, "AE", scenario.owner, owner_id)
        elif scenario.page == "LR":
            status, created, save_summary = request("POST", f"/api/cases/{case_id}/editor/pages/LR/rows", {
                "authorities": [scenario.authority],
                "rows": {"literatureReference": literature_payload(scenario, value)},
            })
            if scenario.scenario_id == "c4-literature-base64-format" and edge == "invalid_edge":
                error = created.get("error") if isinstance(created, dict) else None
                data = error.get("data") if isinstance(error, dict) else None
                detail = data.get("detail") if isinstance(data, dict) else None
                expected_rejection = (
                    status == 422
                    and error.get("message") == "CONSTRAINT_VIOLATION"
                    and isinstance(detail, dict)
                    and detail.get("ruleCode") == scenario.expected_code
                    and detail.get("path") == "literatureReferences.0.documentBase64"
                ) if isinstance(error, dict) else False
                add(edge, scenario, "PASS_INPUT_CONTRACT" if expected_rejection else "FAIL", status, {
                    **save_summary,
                    "expected_code": scenario.expected_code,
                    "expected_path": "literatureReferences.0.documentBase64",
                    "actual_error": error.get("message") if isinstance(error, dict) else None,
                    "actual_code": detail.get("ruleCode") if isinstance(detail, dict) else None,
                    "actual_path": detail.get("path") if isinstance(detail, dict) else None,
                    "reason": "input_contract_rejected" if expected_rejection else "lr_input_contract_mismatch",
                })
                return
            owner_id = created_row_id(created)
            if status != 201 or not owner_id:
                add(edge, scenario, "FAIL", status, {**save_summary, "reason": "lr_fixture_failed"})
                return
            read_status, current = page_current(case_id, "LR", scenario.owner, owner_id)
        else:
            status, created, save_summary = request("POST", f"/api/cases/{case_id}/editor/pages/LB/rows", {
                "authorities": [scenario.authority],
                "rows": {"testResult": test_result_payload(scenario, value)},
            })
            if (
                scenario.scenario_id == "mfds-test-date-null-flavor-vocabulary"
                and edge == "invalid_edge"
            ):
                error = created.get("error") if isinstance(created, dict) else None
                data = error.get("data") if isinstance(error, dict) else None
                detail = data.get("detail") if isinstance(data, dict) else None
                after_page_status, after_page_value, after_page_summary = request(
                    "GET", f"/api/cases/{case_id}/editor/pages/LB",
                )
                after_page_rows = (
                    after_page_value.get("rows")
                    if isinstance(after_page_value, dict) else None
                )
                after_rows = (
                    after_page_rows.get("rows")
                    if isinstance(after_page_rows, dict) else None
                )
                after_audit_status, after_audit_value, after_audit_summary = request(
                    "GET", f"/api/audit-logs/by-record/cases/{case_id}",
                )
                after_audit = after_audit_value if isinstance(after_audit_value, list) else None
                _, contract_code, contract_path, contract_message = contract_rejection_scenarios[
                    scenario.scenario_id
                ]
                unchanged = (
                    after_page_status == 200
                    and isinstance(after_rows, list)
                    and after_rows == contract_before_rows
                    and after_audit_status == 200
                    and isinstance(after_audit, list)
                    and after_audit == contract_before_audit
                )
                expected_rejection = (
                    status == 422
                    and isinstance(error, dict)
                    and error.get("message") == "CONSTRAINT_VIOLATION"
                    and isinstance(detail, dict)
                    and detail.get("ruleCode") == contract_code
                    and detail.get("path") == contract_path
                    and detail.get("message") == contract_message
                    and unchanged
                )
                add(edge, scenario, "PASS_INPUT_CONTRACT" if expected_rejection else "FAIL", status, {
                    **save_summary,
                    "expected_code": scenario.expected_code,
                    "expected_contract_code": contract_code,
                    "expected_contract_path": contract_path,
                    "expected_contract_message": contract_message,
                    "actual_error": error.get("message") if isinstance(error, dict) else None,
                    "actual_code": detail.get("ruleCode") if isinstance(detail, dict) else None,
                    "actual_path": detail.get("path") if isinstance(detail, dict) else None,
                    "actual_message": detail.get("message") if isinstance(detail, dict) else None,
                    "before_page_status": contract_page_status,
                    "after_page_status": after_page_status,
                    "before_row_count": len(contract_before_rows),
                    "after_row_count": len(after_rows) if isinstance(after_rows, list) else None,
                    "before_audit_status": contract_audit_status,
                    "after_audit_status": after_audit_status,
                    "rows_and_audit_unchanged": unchanged,
                    "page_read_error": after_page_summary if after_page_status != 200 else None,
                    "audit_read_error": after_audit_summary if after_audit_status != 200 else None,
                    "reason": "input_contract_rejected" if expected_rejection else "lb_input_contract_mismatch",
                })
                return
            owner_id = created_row_id(created)
            if status != 201 or not owner_id:
                add(edge, scenario, "FAIL", status, {**save_summary, "reason": "lb_fixture_failed"})
                return
            read_status, current = page_current(case_id, "LB", scenario.owner, owner_id)

        actual = get_path(current, scenario.projection_field)
        expected_readback = (
            scenario.readback_values[0 if edge == "invalid_edge" else 1]
            if scenario.readback_values is not None
            else value
        )
        logs = audit_logs(case_id, scenario.owner, owner_id, scenario.field)
        complete_logs = [log for log in logs if audit_log_complete(log)]
        field_match = any(
            audit_key_matches(log.get("changedFields", log.get("changed_fields", {})), scenario.projection_field)
            for log in complete_logs
        )
        contract_audit_grew = True
        contract_after_audit_status: int | None = None
        contract_after_audit_count: int | None = None
        if scenario.scenario_id in contract_rejection_scenarios:
            contract_after_audit_status, contract_after_audit_value, _ = request(
                "GET", f"/api/audit-logs/by-record/cases/{case_id}",
            )
            contract_after_audit = (
                contract_after_audit_value
                if isinstance(contract_after_audit_value, list) else None
            )
            contract_after_audit_count = (
                len(contract_after_audit)
                if isinstance(contract_after_audit, list) else None
            )
            contract_audit_grew = (
                contract_after_audit_status == 200
                and isinstance(contract_after_audit, list)
                and isinstance(contract_before_audit, list)
                and len(contract_after_audit) > len(contract_before_audit)
            )
        if scenario.scenario_id in XML_BOUNDARY_SCENARIO_IDS:
            persisted = (
                read_status == 200
                and values_equal(expected_readback, actual)
                and bool(complete_logs)
                and (value is None or field_match)
            )
            add(
                edge,
                scenario,
                "UNVERIFIED_XML_BOUNDARY" if persisted else "FAIL",
                status,
                {
                    **save_summary,
                    "expected_code": scenario.expected_code,
                    "readback": redacted(actual),
                    "audit_logs": len(logs),
                    "audit_complete": bool(complete_logs),
                    "audit_field_match": field_match,
                    "case_validation_surface": False,
                    "reason": (
                        "xml_export_validation_not_executed"
                        if persisted else "xml_boundary_persistence_mismatch"
                    ),
                },
            )
            return
        validation_status, report, validation_summary = validation(case_id, scenario.authority)
        codes = issue_codes(report)
        present = scenario.expected_code in codes
        expected_present = edge == "invalid_edge"
        version_path = MEDDRA_VERSION_SCENARIO_PATHS.get(scenario.scenario_id)
        expected_issue_path = EXPECTED_SCENARIO_ISSUE_PATHS.get(
            scenario.scenario_id, version_path,
        )
        expected_issue_message = EXPECTED_SCENARIO_ISSUE_MESSAGES.get(
            scenario.scenario_id,
        )
        expected_path_issue_count = (
            complete_issue_count(
                report, scenario.expected_code, expected_issue_path,
                expected_issue_message,
            )
            if expected_issue_path else 0
        )
        version_issue_count = (
            complete_issue_count(report, scenario.expected_code, version_path)
            if version_path else 0
        )
        global_version_issue_count = sum(
            isinstance(issue, dict)
            and issue.get("code") == "ICH.MEDDRA.VERSION.UNAVAILABLE"
            for issue in report.get("issues", [])
        ) if isinstance(report, dict) else 0
        investigational_study_only_present = (
            "ICH.G.k.2.5.STUDY.ONLY" in codes
        )
        version_oracle_matches = version_path is None or (
            (edge == "invalid_edge" and version_issue_count == 1
             and global_version_issue_count == 1)
            or (edge == "valid_edge" and global_version_issue_count == 0)
        )
        passed = (
            read_status == 200
            and values_equal(expected_readback, actual)
            and bool(complete_logs)
            and (value is None or field_match)
            and contract_audit_grew
            and validation_status == 200
            and present == expected_present
            and (not expected_present or issue_complete(report, scenario.expected_code))
            and (
                expected_issue_path is None
                or (expected_present and expected_path_issue_count == 1)
                or (not expected_present and expected_path_issue_count == 0)
            )
            and version_oracle_matches
            and (
                scenario.scenario_id != "g-investigational-product-allowed-value"
                or not investigational_study_only_present
            )
            and (
                scenario.scenario_id not in (
                    MEDDRA_CODE_SCENARIO_IDS | MEDDRA_CONTEXT_SCENARIO_IDS
                )
                or "ICH.MEDDRA.VERSION.UNAVAILABLE" not in codes
            )
            and (
                scenario.scenario_id not in WHODRUG_VERSION_SCENARIOS
                or "ICH.MEDDRA.VERSION.UNAVAILABLE" not in codes
            )
            and (
                edge != "valid_edge"
                or scenario.scenario_id not in WHODRUG_VERSION_SCENARIOS
                or WHODRUG_VERSION_SCENARIOS[scenario.scenario_id] not in codes
            )
        )
        add(edge, scenario, "PASS" if passed else "FAIL", status, {
            **save_summary,
            "validation": validation_summary,
            "expected_code": scenario.expected_code,
            "expected_code_present": expected_present,
            "actual_code_present": present,
            "meddra_version_unavailable": "ICH.MEDDRA.VERSION.UNAVAILABLE" in codes,
            "meddra_version_expected_path": version_path,
            "meddra_version_expected_path_issue_count": version_issue_count,
            "meddra_version_global_issue_count": global_version_issue_count,
            "investigational_product_study_only_present": (
                investigational_study_only_present
            ),
            "expected_issue_path": expected_issue_path,
            "expected_issue_message": expected_issue_message,
            "expected_path_issue_count": expected_path_issue_count,
            "whodrug_companion_vocabulary_present": (
                WHODRUG_VERSION_SCENARIOS.get(scenario.scenario_id) in codes
                if scenario.scenario_id in WHODRUG_VERSION_SCENARIOS else False
            ),
            "readback": redacted(actual),
            "expected_normalization": scenario.readback_values is not None,
            "audit_logs": len(logs),
            "audit_complete": bool(complete_logs),
            "audit_field_match": field_match,
            "contract_audit_status": contract_after_audit_status,
            "contract_audit_rows_before": (
                len(contract_before_audit)
                if isinstance(contract_before_audit, list) else None
            ),
            "contract_audit_rows_after": contract_after_audit_count,
            "contract_audit_grew": contract_audit_grew,
        })

    ordered = runnable_scenarios[:]
    random.Random(args.seed).shuffle(ordered)
    for scenario in ordered:
        if interrupted:
            break
        run_edge(scenario, "invalid_edge", scenario.invalid_value)
        if not interrupted:
            run_edge(scenario, "valid_edge", scenario.valid_value)

    if runnable_scenarios and not interrupted:
        status, value, summary = request("GET", "/api/audit-logs/verify-integrity")
        broken = value.get("broken_rows", value.get("brokenRows")) if isinstance(value, dict) else None
        add("audit_chain", None, "PASS" if status == 200 and broken == 0 else "FAIL", status, {
            **summary,
            "broken_rows": broken,
        })

    verified_rules = rules_with_both_edges_passed(scenarios, events)
    passed_edges = {
        (event.scenario_id, event.sample_ordinal, event.kind)
        for event in events
        if event.classification == "PASS"
    }
    verified_meddra_version_fields = sorted(
        scenario_id
        for scenario_id in MEDDRA_VERSION_SCENARIO_PATHS
        if any(scenario.scenario_id == scenario_id for scenario in scenarios)
        and all(
            (scenario.scenario_id, scenario.sample_ordinal, edge) in passed_edges
            for scenario in scenarios if scenario.scenario_id == scenario_id
            for edge in ("invalid_edge", "valid_edge")
        )
    )
    unverified_planned = planned_rules - verified_rules
    supplemental_verified_rules = supplemental_planned_rules & verified_rules
    supplemental_unverified_rules = supplemental_planned_rules - verified_rules
    unverified_inventory = inventory - verified_rules
    complete = bool(scenarios) and not interrupted and not unexplained and not unverified_planned and not supplemental_unverified_rules and not unverified_inventory and all(
        event.classification == "PASS" for event in events
    )

    out_dir = Path(args.artifact_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    artifact = out_dir / f"case-business-validator-{args.seed}.jsonl"
    with artifact.open("w", encoding="utf-8") as handle:
        for event in events:
            handle.write(json.dumps({"seed": args.seed, "commit": commit_sha(), **asdict(event)}, sort_keys=True) + "\n")
        handle.write(json.dumps({
            "kind": "run",
            "seed": args.seed,
            "requests": requests,
            "elapsed_seconds": round(time.monotonic() - started, 3),
            "interrupted": interrupted,
            "scenario_count": len(scenarios),
            "scenario_template_count": len(scenario_templates),
            "samples_per_scenario": args.samples_per_scenario,
            "generator_families": dict(sorted(
                (family, sum(item.generator_family == family for item in scenarios))
                for family in GENERATOR_FAMILIES
            )),
            "catalog_rules": sorted(catalog_covered),
            "catalog_rule_count": len(catalog_covered),
            "planned_rule_count": len(planned_rules),
            "planned_rules": sorted(planned_rules),
            "supplemental_retained_planned_rule_count": len(supplemental_planned_rules),
            "supplemental_retained_planned_rules": sorted(supplemental_planned_rules),
            "supplemental_retained_verified_rules": sorted(supplemental_verified_rules),
            "supplemental_retained_unverified_rules": sorted(supplemental_unverified_rules),
            "xml_boundary_planned_rule_count": len(xml_boundary_planned_rules),
            "xml_boundary_planned_rules": sorted(xml_boundary_planned_rules),
            "unverified_xml_boundary_rule_count": len(xml_boundary_planned_rules),
            "unverified_xml_boundary_rules": sorted(xml_boundary_planned_rules),
            "inventory_catalog_intersection_rule_count": len(inventory & catalog_covered),
            "inventory_catalog_intersection_rules": sorted(inventory & catalog_covered),
            "verified_both_edges_rules": sorted(verified_rules),
            "verified_meddra_version_field_scenarios": verified_meddra_version_fields,
            "verified_meddra_version_field_scenario_count": len(verified_meddra_version_fields),
            "unverified_planned_rules": sorted(unverified_planned),
            "unverified_inventory_rules": sorted(unverified_inventory),
            "inventory_rule_count": len(inventory),
            "raw_uncovered_rules": sorted(raw_uncovered),
            "dispositioned_rules": dispositions,
            "test_backed_rule_mappings_not_executed": test_backed,
            "unsupported_inventory_rules": sorted(unexplained),
            "complete": complete,
            "verdict": "INVENTORIED_API_CHECKS_PASSED" if complete else "INCOMPLETE_OR_FAILED",
            "official_compliance_verified": False,
            "ui_verified": False,
            "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            "artifact": str(artifact),
            "surface": "case-validation-api",
        }, sort_keys=True) + "\n")
    counts: dict[str, int] = {}
    for event in events:
        counts[event.classification] = counts.get(event.classification, 0) + 1
    print(f"events={len(events)} counts={json.dumps(counts, sort_keys=True)} artifact={artifact}")
    return 2 if interrupted else 0 if complete and all(event.classification == "PASS" for event in events) else 1


if __name__ == "__main__":
    sys.exit(main(parser().parse_args()))
