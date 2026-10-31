from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any


REQUIRED_FIELD_KEYS = (
    "code",
    "authority",
    "frontendPath",
    "payloadPath",
    "projectionPath",
    "patch",
    "roundTripValue",
)
STAGE_STATUSES = {"verified", "not_applicable", "blocked"}


def _editor_status(row: dict[str, Any]) -> Any:
    return row.get("editor_status", row.get("status"))


def load_editor_contract(root: Path, page_id: str) -> dict[str, Any]:
    path = root / "editor-contracts" / f"{page_id.lower()}.json"
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"could not load {page_id} editor contract: {error}") from error
    if not isinstance(payload, dict):
        raise ValueError(f"{path}: editor contract must be an object")
    return payload


def _registry_frontend_paths(row: dict[str, Any]) -> set[str]:
    frontend = row.get("frontend")
    if not isinstance(frontend, dict):
        return set()
    section = frontend.get("section")
    field = frontend.get("field")
    if not isinstance(section, str) or not isinstance(field, str):
        return set()
    return {
        f"{section}.{candidate.strip()}".replace("[]", "")
        for candidate in field.split("/")
        if candidate.strip()
    }


def _validate_stage(code: str, name: str, stage: Any, result: Any) -> None:
    if not isinstance(stage, dict):
        result.add(f"{code} missing {name} evidence")
        return
    status = stage.get("status")
    if status not in STAGE_STATUSES:
        result.add(
            f"{code} {name}.status must be one of {sorted(STAGE_STATUSES)}"
        )
        return
    if status in {"not_applicable", "blocked"} and not stage.get("reason"):
        result.add(f"{code} {name} {status} requires reason")
    if status == "verified":
        evidence_key = "ruleCode" if name == "constraint" else "issuePath"
        if not stage.get(evidence_key):
            result.add(f"{code} {name} verified requires {evidence_key}")
        if name == "constraint" and "invalidValue" not in stage:
            result.add(f"{code} constraint verified requires invalidValue")


def _validate_control(
    code: str, control: Any, result: Any, round_trip_value: Any = None
) -> None:
    if control is None:
        return
    if not isinstance(control, dict):
        result.add(f"{code} control must be an object")
        return
    if control.get("kind") == "async_autocomplete":
        for name in ("queryValue", "optionValue", "optionDataValue", "optionLabel", "selectedLabel"):
            if not isinstance(control.get(name), str) or not control[name].strip():
                result.add(f"{code} async_autocomplete control requires {name}")
        effects = control.get("selectionEffects")
        if effects is not None and (
            not isinstance(effects, list)
            or not effects
            or any(
                not isinstance(effect, dict)
                or set(effect) != {"frontendPath", "value"}
                or not isinstance(effect.get("frontendPath"), str)
                or not effect["frontendPath"].strip()
                or not isinstance(effect.get("value"), str)
                or not effect["value"].strip()
                for effect in effects
            )
            or len({effect["frontendPath"] for effect in effects}) != len(effects)
        ):
            result.add(f"{code} async_autocomplete selectionEffects requires unique frontendPath/value pairs")
        return
    if control.get("kind") == "string_checkbox":
        if not isinstance(control.get("checkedValue"), str) or not control["checkedValue"]:
            result.add(f"{code} string_checkbox control requires checkedValue")
        if not isinstance(control.get("uncheckedValue"), str):
            result.add(f"{code} string_checkbox control requires uncheckedValue")
        return
    if control.get("kind") not in {"native_select", "radio_group", "radix_select"}:
        return
    kind = control["kind"]
    options = control.get("options")
    dynamic_options = control.get("dynamicOptions")
    if kind == "radix_select":
        if (options is None) == (dynamic_options is None):
            result.add(
                f"{code} radix_select control requires exactly one of options or dynamicOptions"
            )
            return
        if dynamic_options is not None:
            if not isinstance(dynamic_options, dict):
                result.add(f"{code} radix_select dynamicOptions must be an object")
                return
            for name in ("source", "valuePath", "labelPath"):
                value = dynamic_options.get(name)
                if not isinstance(value, str) or not value.strip():
                    result.add(f"{code} radix_select dynamicOptions requires {name}")
            return
    if not isinstance(options, list) or not options:
        result.add(f"{code} {kind} control requires options")
        return
    values: set[str] = set()
    for option in options:
        if not isinstance(option, dict):
            result.add(f"{code} {kind} option must be an object")
            continue
        value = option.get("value")
        label = option.get("label")
        if not isinstance(value, str) or not value:
            result.add(f"{code} {kind} option value is required")
        elif value in values:
            result.add(f"{code} {kind} option value {value!r} is duplicated")
        else:
            values.add(value)
        if not isinstance(label, str) or not label:
            result.add(f"{code} {kind} option label is required")
    if round_trip_value is not None and round_trip_value not in values:
        result.add(
            f"{code} roundTripValue {round_trip_value!r} is not declared by its {kind} control"
        )


def _validate_persistence(code: str, field: dict, result: Any) -> None:
    if field.get("execution", {}).get("status") == "blocked":
        return
    persistence = field.get("persistence")
    if not isinstance(persistence, dict) or set(persistence) != {"clear", "blank", "omission", "audit"}:
        result.add(f"{code} requires explicit clear/blank/omission/audit persistence contract")
        return
    omission = persistence["omission"]
    valid_omission = omission == "preserve" or isinstance(omission, dict) and (
        set(omission) == {"effect", "value"} and omission["effect"] == "store" or
        set(omission) == {"effect", "status", "error", "detail"} and omission["effect"] == "reject" and
        omission["status"] == 400 and omission["error"] == "INVALID_REQUEST" and bool(omission["detail"])
    )
    if not valid_omission:
        result.add(f"{code} omission requires preserve or an explicit replacement/rejection outcome")
    for operation in ("clear", "blank"):
        outcome = persistence[operation]
        if not isinstance(outcome, dict):
            result.add(f"{code} {operation} requires an explicit outcome")
            continue
        effect = outcome.get("effect")
        keys = {"store": {"effect", "input", "value"}, "reject": {"effect", "input", "status", "ruleCode"},
                "not_applicable": {"effect", "reason"}}
        if effect not in keys or set(outcome) != keys[effect]:
            result.add(f"{code} invalid {operation} outcome")
        elif effect == "reject" and (outcome["status"] != 422 or not outcome["ruleCode"]):
            result.add(f"{code} {operation} rejection requires 422 and a ruleCode")
        elif effect == "not_applicable" and (operation != "blank" or not outcome["reason"] or isinstance(field.get("roundTripValue"), str)):
            result.add(f"{code} only non-string blank input may be not_applicable with a reason")
    audit = persistence["audit"]
    if not isinstance(audit, dict) or set(audit) != {"table", "column", "record"}:
        result.add(f"{code} audit requires table, column and record")
        return
    for key in ("table", "column"):
        if not isinstance(audit[key], str) or not re.fullmatch(r"[a-z_][a-z0-9_]*", audit[key]):
            result.add(f"{code} invalid audit {key}")
    record = audit["record"]
    if not isinstance(record, dict) or not (
        record == {"source": "case"} or
        set(record) == {"source", "path"} and record["source"] == "projection" and isinstance(record["path"], str) and record["path"]
    ):
        result.add(f"{code} audit requires a case ID or an explicit projection ID path")


def validate_editor_contract(
    registry_rows: list[dict[str, Any]],
    contract: dict[str, Any],
    result: Any,
) -> None:
    page_id = contract.get("pageId")
    if not isinstance(page_id, str) or not page_id:
        result.add("editor contract missing pageId")
        return
    fields = contract.get("fields")
    if not isinstance(fields, list):
        result.add(f"{page_id} editor contract fields must be an array")
        return

    rows_by_code = {
        row.get("e2br3_code"): row
        for row in registry_rows
        if isinstance(row, dict) and isinstance(row.get("e2br3_code"), str)
    }
    fields_by_code: dict[str, dict[str, Any]] = {}
    for field in fields:
        if not isinstance(field, dict):
            result.add(f"{page_id} editor contract field must be an object")
            continue
        code = field.get("code")
        if not isinstance(code, str) or not code:
            result.add(f"{page_id} editor contract field missing code")
            continue
        if code in fields_by_code:
            result.add(f"{page_id} editor contract duplicate code {code}")
            continue
        fields_by_code[code] = field
        if "businessValidation" in field:
            result.add(f"{code} businessValidation belongs in the validator crate")
        if page_id != "N":
            _validate_persistence(code, field, result)
        _validate_control(
            code, field.get("control"), result, field.get("roundTripValue")
        )
        scenario_controls = field.get("scenarioControls")
        expected_scenario_controls = {
            "mfds-g-product-required": {
                "kind": "mfds_product_direct_input",
                "productionReference": "mfdsProduct.item_seq",
            }
        }
        expected_substance_scenario_controls = {
            "mfds-g-substance-version-required": {
                "kind": "mfds_foreign_substance_direct_input",
                "productionReference": "whoSubstance.code",
            }
        }
        expected_substance_render_prerequisites = [{
            "code": "G.k.2.4",
            "frontendPath": "drugs[].obtainDrugCountry",
            "value": "KR",
        }]
        if code == "G.k.2.1.KR.1b":
            if scenario_controls != expected_scenario_controls:
                result.add(
                    f"{code} scenarioControls must declare the exact domestic MFDS product input"
                )
        elif code == "G.k.2.3.r.1.KR.1b":
            if scenario_controls != expected_substance_scenario_controls:
                result.add(
                    f"{code} scenarioControls must declare the exact foreign MFDS substance input"
                )
            if field.get("renderPrerequisites") != expected_substance_render_prerequisites:
                result.add(
                    f"{code} renderPrerequisites must declare the exact domestic drug country"
                )
        elif scenario_controls is not None:
            result.add(f"{code} scenarioControls is unsupported")
        payload_path = field.get("payloadPath")
        if not isinstance(payload_path, str) or not payload_path:
            result.add(f"{code} missing payloadPath evidence")
        patch = field.get("patch")
        if not isinstance(patch, dict):
            result.add(f"{code} patch must be an object")
        else:
            if patch.get("kind") not in {"row", "rows"}:
                result.add(f"{code} patch.kind must be row or rows")
            owner = patch.get("owner")
            if not isinstance(owner, str) or not owner:
                result.add(f"{code} patch.owner is required")
        row = rows_by_code.get(code)
        if row is None:
            result.add(f"{code} editor contract has no registry row")
            continue
        if field.get("authority") != row.get("authority"):
            result.add(
                f"{code} editor contract authority {field.get('authority')} does not match registry {row.get('authority')}"
            )
        frontend_path = field.get("frontendPath")
        registry_paths = _registry_frontend_paths(row)
        if isinstance(frontend_path, str) and frontend_path.replace("[]", "") not in registry_paths:
            result.add(
                f"{code} frontend path {frontend_path} does not match registry {sorted(registry_paths)}"
            )
        if _editor_status(row) == "complete":
            for key in REQUIRED_FIELD_KEYS:
                if key not in field:
                    result.add(f"{code} missing {key} evidence")
            _validate_stage(code, "constraint", field.get("constraint"), result)

    for row in registry_rows:
        if (
            row.get("editor_page") == page_id
            and _editor_status(row) == "complete"
            and row.get("e2br3_code") not in fields_by_code
        ):
            result.add(
                f"{row.get('e2br3_code')} complete but missing from {page_id} editor contract"
            )

    for field in fields_by_code.values():
        prerequisites = field.get("renderPrerequisites")
        if prerequisites is None:
            continue
        if not isinstance(prerequisites, list) or not prerequisites:
            result.add(f"{field['code']} renderPrerequisites must be a non-empty array")
            continue
        for prerequisite in prerequisites:
            if not isinstance(prerequisite, dict):
                result.add(f"{field['code']} render prerequisite must be an object")
                continue
            matches = [candidate for candidate in fields
                       if isinstance(candidate, dict)
                       and candidate.get("code") == prerequisite.get("code")
                       and candidate.get("frontendPath") == prerequisite.get("frontendPath")
                       and candidate.get("patch") == field.get("patch")]
            if len(matches) != 1:
                result.add(
                    f"{field['code']} render prerequisite must match one exact same-owner contract field"
                )
