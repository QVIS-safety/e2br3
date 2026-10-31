#!/usr/bin/env python3
"""Seeded black-box fuzzer for persisted Case business-validator rules.

Each scenario proves both edges of one rule: a valid persisted baseline does
not emit the rule, one field mutation emits it, and restoring that field
removes it. Save readback and audit evidence are checked on both mutations.
"""

from __future__ import annotations

import argparse
import os
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
SCRIPTS = ROOT / "scripts"
VALIDATOR_TOOLS = ROOT / "crates/libs/validator/tools"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))
if str(VALIDATOR_TOOLS) not in sys.path:
    sys.path.insert(0, str(VALIDATOR_TOOLS))

from audit_trail_consistency_fuzzer import audit_key_matches
from rule_inventory import (
    GENERATED_BUSINESS_RULE_CODES,
    INPUT_ONLY_SUFFIXES,
    RETAINED_ALLOWED_VALUE_RULES,
    RULE_RE,
    discover_business_rule_codes,
)

from scenarios import (
    DISPOSITION_GROUPS,
    EXPECTED_SCENARIO_ISSUE_MESSAGES,
    EXPECTED_SCENARIO_ISSUE_PATHS,
    MEDDRA_CODE_SCENARIO_IDS,
    MEDDRA_CONTEXT_SCENARIO_IDS,
    MEDDRA_FIELDS_BY_OWNER,
    MEDDRA_VERSION_SCENARIO_PATHS,
    SUPPLEMENTAL_RETAINED_SCENARIO_IDS,
    Scenario,
    TEST_BACKED_RULES,
    UCUM_FREQUENCY_SCENARIO_ID,
    WHODRUG_CODE_FIELDS,
    WHODRUG_VERSION_SCENARIOS,
    XML_BOUNDARY_SCENARIO_IDS,
    device_integration_scenarios,
    mirror_warning_scenarios,
    reference_required_scenarios,
    reference_vocabulary_scenarios,
    rule_dispositions,
    scenario_catalog,
    singleton_integration_scenarios,
    topology_integration_scenarios,
)

from generation import (
    DATE_TEXT_RE,
    GENERATOR_FAMILIES,
    generated_scenario,
    generator_family,
    scenario_fingerprint,
)

from evidence import (
    Event,
    complete_issue_count,
    issue_codes,
    issue_complete,
    rules_with_both_edges_passed,
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

from execution import main


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


if __name__ == "__main__":
    sys.exit(main(parser().parse_args()))
