"""Seeded mutations of the existing scenario catalog."""
from __future__ import annotations

import base64
import hashlib
import json
import random
import re
from dataclasses import replace
from typing import Any
from case_editor_input_fuzzer import snake
from scenarios import (
    Scenario,
)


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
