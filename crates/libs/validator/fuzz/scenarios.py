"""Scenario definitions and explicit rule dispositions."""
from __future__ import annotations

import random
from dataclasses import dataclass, replace
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[4]


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
    empty_collections: tuple[str, ...] = ()


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
            (("cumulativeDoseValue", 12.5),),
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
            (("cumulativeDoseUnit", "mg"),),
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
            (("drugAuthorizationNumber", "AUTH-001"),),
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
            "gestationPeriodExposureValue",
            "gestation_period_exposure_value",
            "ICH.G.k.6a.REQUIRED",
            None,
            2,
            (("gestationPeriodExposureUnit", "wk"),),
        ),
        Scenario(
            14,
            "g-gestation-unit-required",
            "ich",
            "DG",
            "drug",
            "gestationPeriodExposureUnit",
            "gestation_period_exposure_unit",
            "ICH.G.k.6b.REQUIRED",
            None,
            "wk",
            (("gestationPeriodExposureValue", 2),),
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
        Scenario(29, "e-reaction-language-required", "ich", "AE", "reaction", "reactionLanguage", "reaction_language", "ICH.E.i.1.1b.REQUIRED", None, "eng", (("primarySourceReaction", "Headache"),)),
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
        Scenario(43, "f-test-date-required", "ich", "LB", "testResult", "testDate", "test_date", "ICH.F.r.1.REQUIRED", None, f"{year}0303", (("testName", "ALT"),)),
        Scenario(44, "f-test-future-date-forbidden", "ich", "LB", "testResult", "testDate", "test_date", "ICH.F.r.1.FUTURE_DATE.FORBIDDEN", "29990303", f"{year}0303"),
        Scenario(45, "f-test-meddra-version-required", "ich", "LB", "testResult", "testMeddraVersion", "test_meddra_version", "ICH.F.r.2.2a.REQUIRED", None, "28.1", (("testMeddraCode", "10088978"),)),
        Scenario(46, "f-test-result-unit-required", "ich", "LB", "testResult", "testUnit", "test_result_unit", "ICH.F.r.3.3.REQUIRED", None, "mg/dL", (("testResult", "12.5"),)),
        Scenario(47, "f-coded-result-required", "ich", "LB", "testResult", "testResultCode", "test_result_code", "ICH.F.r.3.1.REQUIRED", None, "1", (("testName", "ALT"), ("testResult", None), ("testResultUnstructured", None))),
        Scenario(48, "f-value-result-required", "ich", "LB", "testResult", "testResult", "test_result_value", "ICH.F.r.3.2.REQUIRED", None, "12.5", (("testName", "ALT"), ("testResultCode", None), ("testResultUnstructured", None))),
        Scenario(49, "f-unstructured-result-required", "ich", "LB", "testResult", "testResultUnstructured", "result_unstructured", "ICH.F.r.3.4.REQUIRED", None, "Normal", (("testName", "ALT"), ("testResultCode", None), ("testResult", None))),
        Scenario(50, "d-patient-age-exclusive", "ich", "DM", "patientInformation", "patientAgeGroup", "age_group", "ICH.D.2.EXCLUSIVE", "5", None, (("patientBirthDate", f"{year}0303"),)),
        Scenario(51, "d-patient-birth-date-future", "ich", "DM", "patientInformation", "patientBirthDate", "birth_date", "ICH.D.2.1.FUTURE_DATE.FORBIDDEN", "29990303", f"{year}0303"),
        Scenario(52, "d-patient-age-value-required", "ich", "DM", "patientInformation", "patientAge.value", "age_at_time_of_onset", "ICH.D.2.2a.REQUIRED", None, 36.5, (("patientAge.unit", "a"),)),
        Scenario(53, "d-patient-age-unit-required", "ich", "DM", "patientInformation", "patientAge.unit", "age_unit", "ICH.D.2.2b.REQUIRED", None, "a", (("patientAge.value", 36.5),)),
        Scenario(54, "d-patient-gestation-value-required", "ich", "DM", "patientInformation", "gestationPeriod.value", "gestation_period", "ICH.D.2.2.1a.REQUIRED", None, 22, (("gestationPeriod.unit", "wk"),)),
        Scenario(55, "d-patient-gestation-unit-required", "ich", "DM", "patientInformation", "gestationPeriod.unit", "gestation_period_unit", "ICH.D.2.2.1b.REQUIRED", None, "wk", (("gestationPeriod.value", 22),)),
        Scenario(56, "d-patient-lmp-future", "ich", "DM", "patientInformation", "lastMenstrualPeriodDate", "last_menstrual_period_date", "ICH.D.6.FUTURE_DATE.FORBIDDEN", "29990303", f"{year}0303"),
        Scenario(57, "h-diagnosis-meddra-version-required", "ich", "NR", "senderDiagnoses", "diagnosisMeddraVersion", "diagnosis_meddra_version", "ICH.H.3.r.1a.REQUIRED", None, "26.0", (("diagnosisMeddraCode", "10000001"),)),
        Scenario(58, "h-diagnosis-meddra-code-required", "ich", "NR", "senderDiagnoses", "diagnosisMeddraCode", "diagnosis_meddra_code", "ICH.H.3.r.1b.REQUIRED", None, "10000001", (("diagnosisMeddraVersion", "26.0"),)),
        Scenario(59, "h-case-summary-language-required", "ich", "NR", "caseSummaryInformation", "languageCode", "language_code", "ICH.H.5.r.1b.REQUIRED", None, "eng", (("summaryText", "Validation UI summary"),)),
        Scenario(60, "d-medical-history-text-required", "ich", "DM", "patientInformation", "medicalHistoryText", "medical_history_text", "ICH.D.7.2.REQUIRED", None, "No relevant history", empty_collections=("medicalHistoryEpisodes",)),
        Scenario(61, "d-history-meddra-version-required", "ich", "DM", "medicalHistoryEpisodes", "meddraVersion", "meddra_version", "ICH.D.7.1.r.1a.REQUIRED", None, "26.0"),
        Scenario(62, "d-history-meddra-code-required", "ich", "DM", "medicalHistoryEpisodes", "meddraCode", "meddra_code", "ICH.D.7.1.r.1b.REQUIRED", None, "10000001"),
        Scenario(63, "d-history-future-date-forbidden", "ich", "DM", "medicalHistoryEpisodes", "startDate", "start_date", "ICH.D.7.1.r.FUTURE_DATE.FORBIDDEN", "29990303", f"{year}0303"),
        Scenario(64, "d-past-drug-name-required", "ich", "DH", "pastDrugHistory", "drugName", "drug_name", "ICH.D.8.r.1.REQUIRED", None, "Advanced Aspirin", (("startDate", "20200102"),)),
        Scenario(65, "d-past-drug-identifier-exclusive", "ich", "DH", "pastDrugHistory", "phpid", "phpid", "ICH.D.8.MPID_PHPID.EXCLUSIVE", f"PHPID-{suffix}", None),
        Scenario(66, "d-past-drug-future-date-forbidden", "ich", "DH", "pastDrugHistory", "startDate", "start_date", "ICH.D.8.r.FUTURE_DATE.FORBIDDEN", "29990303", f"{year}0303"),
        Scenario(67, "d-past-drug-indication-version-required", "ich", "DH", "pastDrugHistory", "indicationMeddraVersion", "indication_meddra_version", "ICH.D.8.r.6a.REQUIRED", None, "28.1", (("indicationMeddraCode", "10088978"),)),
        Scenario(68, "d-past-drug-indication-code-required", "ich", "DH", "pastDrugHistory", "indicationMeddraCode", "indication_meddra_code", "ICH.D.8.r.6b.REQUIRED", None, "10088978", (("indicationMeddraVersion", "28.1"),)),
        Scenario(69, "d-past-drug-reaction-version-required", "ich", "DH", "pastDrugHistory", "reactionMeddraVersion", "reaction_meddra_version", "ICH.D.8.r.7a.REQUIRED", None, "28.1", (("reactionMeddraCode", "10088978"),)),
        Scenario(70, "d-past-drug-reaction-code-required", "ich", "DH", "pastDrugHistory", "reactionMeddraCode", "reaction_meddra_code", "ICH.D.8.r.7b.REQUIRED", None, "10088978", (("reactionMeddraVersion", "28.1"),)),
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
        Scenario(104, "c2-reporter-country-required", "ich", "RP", "primarySources", "reporterCountry", "reporterCountry", "ICH.C.2.r.3.REQUIRED", None, "KR", (("primarySourceForRegulatoryPurposes", "1"),)),
        Scenario(105, "c2-reporter-qualification-required", "ich", "RP", "primarySources", "qualification", "qualification", "ICH.C.2.r.4.REQUIRED", None, "1"),
        Scenario(106, "c2-primary-source-required", "ich", "RP", "primarySources", "primarySourceForRegulatoryPurposes", "primarySourceForRegulatoryPurposes", "ICH.C.2.r.5.REQUIRED", None, "1"),
        Scenario(107, "c2-primary-source-exactly-once", "ich", "RP", "primarySources", "primarySourceForRegulatoryPurposes", "primarySourceForRegulatoryPurposes", "ICH.C.2.r.5.EXACTLY_ONCE", "1", None),
        Scenario(108, "c3-sender-type-required", "ich", "SD", "senderInformation", "senderType", "senderType", "ICH.C.3.1.REQUIRED", None, "1"),
        Scenario(109, "c3-sender-organization-required", "ich", "SD", "senderInformation", "organizationName", "organizationName", "ICH.C.3.2.REQUIRED", None, "Business Sender", (("senderType", "3"),)),
        Scenario(111, "c5-study-type-reaction-required", "ich", "SI", "studyInformation", "studyTypeReaction", "study_type_reaction", "ICH.C.5.4.REQUIRED", None, "1", ci_values=(("reportType", "2"),)),
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
        Scenario(161, "mfds-relatedness-krct-result-required", "mfds", "DG", "drug", "drugReactionAssessments[].resultOfAssessmentKr2", "drugReactionAssessments[].resultOfAssessmentKr2", "MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED", None, "1", (("drugReactionAssessments[].sourceOfAssessment", "Sponsor"), ("drugReactionAssessments[].methodOfAssessmentKr1", "2")), ci_values=(("reportType", "2"),), header_values=(("batch_receiver_identifier", "MFDS-O-CT"), ("message_receiver_identifier", "MFDS-O-CT"))),
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
            "ICH.F.r.2.2b.REQUIRED", None, "10088978",
            (("testDate", f"{year}0303"), ("testName", None)),
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
