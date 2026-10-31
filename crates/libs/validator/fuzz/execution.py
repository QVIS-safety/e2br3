"""Real API save, readback, validation, audit and cleanup lifecycle."""
from __future__ import annotations

import argparse
import hashlib
import json
import random
import time
import uuid
from dataclasses import asdict, replace
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
    unwrap,
    values_equal,
)
from audit_trail_consistency_fuzzer import audit_key_matches, audit_log_complete
from rbac_rls_blackbox import ApiClient, guard_target
from rule_inventory import discover_business_rule_codes
from scenarios import (
    EXPECTED_SCENARIO_ISSUE_MESSAGES,
    EXPECTED_SCENARIO_ISSUE_PATHS,
    MEDDRA_CODE_SCENARIO_IDS,
    MEDDRA_CONTEXT_SCENARIO_IDS,
    MEDDRA_VERSION_SCENARIO_PATHS,
    SUPPLEMENTAL_RETAINED_SCENARIO_IDS,
    Scenario,
    TEST_BACKED_RULES,
    UCUM_FREQUENCY_SCENARIO_ID,
    WHODRUG_VERSION_SCENARIOS,
    XML_BOUNDARY_SCENARIO_IDS,
    rule_dispositions,
    scenario_catalog,
)
from generation import (
    GENERATOR_FAMILIES,
    generated_scenario,
    scenario_fingerprint,
)
from evidence import (
    Event,
    complete_issue_count,
    issue_codes,
    issue_complete,
    rules_with_both_edges_passed,
    source_hashes,
)
from terminology import (
    bind_meddra_code_scenarios,
    bind_meddra_version_scenarios,
    bind_whodrug_version_scenarios,
    meddra_binding,
    meddra_unavailable_version,
    omit_whodrug_unrelated_meddra,
    whodrug_binding,
)


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
    sources = source_hashes(Path(__file__).resolve().parent)
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
            "runner_sha256": sources["runner.py"],
            "source_sha256": sources,
            "artifact": str(artifact),
            "surface": "case-validation-api",
        }, sort_keys=True) + "\n")
    counts: dict[str, int] = {}
    for event in events:
        counts[event.classification] = counts.get(event.classification, 0) + 1
    print(f"events={len(events)} counts={json.dumps(counts, sort_keys=True)} artifact={artifact}")
    return 2 if interrupted else 0 if complete and all(event.classification == "PASS" for event in events) else 1
