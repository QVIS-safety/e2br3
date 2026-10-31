"""Bind scenarios to explicitly supplied terminology releases."""
from __future__ import annotations
from dataclasses import replace
from typing import Any
from scenarios import (
    MEDDRA_CODE_SCENARIO_IDS,
    MEDDRA_CONTEXT_SCENARIO_IDS,
    MEDDRA_FIELDS_BY_OWNER,
    MEDDRA_VERSION_SCENARIO_PATHS,
    Scenario,
    WHODRUG_CODE_FIELDS,
    WHODRUG_VERSION_SCENARIOS,
)
from generation import (
    scenario_fingerprint,
)


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
