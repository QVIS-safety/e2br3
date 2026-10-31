"""Validation issue and completed-edge evidence checks."""
from __future__ import annotations

import hashlib
from pathlib import Path
from dataclasses import dataclass
from typing import Any
from scenarios import (
    MEDDRA_VERSION_SCENARIO_PATHS,
    Scenario,
)


def source_hashes(root: Path) -> dict[str, str]:
    return {
        name: hashlib.sha256((root / name).read_bytes()).hexdigest()
        for name in (
            "runner.py", "scenarios.py", "generation.py",
            "evidence.py", "terminology.py", "execution.py",
        )
    }


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
