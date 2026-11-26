"""Keep editor input requirements tied to the committed dictionary, not Rust comments."""
from __future__ import annotations

import json
import re
from pathlib import Path


INPUT_PROPERTIES = ("max_length", "allowed_value_constraint", "null_flavors")


def dictionary_entries(root: Path) -> dict:
    entries = {}
    for path in sorted((root / "dictionary").glob("*.json")):
        document = json.loads(path.read_text())
        for entry in document["entries"]:
            if entry["kind"] != "element":
                continue
            code = entry["code"]
            if code in entries:
                raise ValueError(f"duplicate dictionary element: {code}")
            entries[code] = (str(path.relative_to(root)), entry)
    if not entries:
        raise ValueError("editor input checks require the production dictionary")
    return entries


def requirements(entry: dict) -> dict:
    result = {key: entry[key] for key in INPUT_PROPERTIES if key in entry
              and not (key == "allowed_value_constraint"
                       and entry[key]["kind"] == "descriptive")}
    if "max_length" in result:
        result["max_length"] = max(
            [result["max_length"], *entry.get("profile_max_lengths", {}).values()],
            key=length_limit,
        )
    return result


def input_function(code: str) -> str:
    name = re.sub(r"[^a-z0-9]+", "_", code.lower())
    if ".KR." in code:
        name = "mfds_" + name
    section = code.removeprefix("FDA.")[0].lower()
    return f"{section}::{name}"


def binding_function(code: str, entry: dict) -> str:
    # Regional overlays for a physical field are folded into its canonical contract.
    return input_function("D.1" if code == "FDA.D.1" else code)


def length_limit(value: str) -> int:
    if value.isdecimal():
        return int(value)
    match = re.fullmatch(r"\d+\* E2B\(R2\)\s+(\d+)\* E2B\(R3\)", value)
    if match is None:
        raise ValueError(f"unsupported dictionary length: {value!r}")
    return int(match[1])


def check(root: Path) -> list[tuple[str, str, str, str]]:
    """Return named executable input checks; missing definitions always fail."""
    entries = dictionary_entries(root)
    catalog = json.loads((root.parent / "crates/libs/validator/rules.json").read_text())
    known = {row["ruleCode"] for row in catalog["rules"]}
    covered = set()
    tests = []
    for path in sorted((root / "editor-contracts").glob("*.json")):
        if path.name == "schema.json":
            continue
        document = json.loads(path.read_text())
        page = document["pageId"]
        for field in document["fields"]:
            code = field["code"]
            expected = [code] if code in entries else []
            if code == "D.1":
                expected.append("FDA.D.1")
            bindings = field.get("dictionaryInputs", [])
            if [binding["code"] for binding in bindings] != expected:
                raise ValueError(f"{page}/{code}: missing, duplicate or orphan dictionary inputs")
            for binding in bindings:
                source_code = binding["code"]
                source, entry = entries[source_code]
                if binding["source"] != source:
                    raise ValueError(f"{page}/{code}: wrong dictionary source")
                if binding.get("guidance") != entry.get("notes"):
                    raise ValueError(f"{page}/{code}: stale dictionary guidance")
                if source_code in covered:
                    raise ValueError(f"duplicate dictionary binding: {source_code}")
                covered.add(source_code)
                expected_properties = requirements(entry)
                checks = binding["checks"]
                if set(checks) != set(expected_properties):
                    raise ValueError(f"{page}/{code}: dictionary input property coverage differs")
                for prop, requirement in expected_properties.items():
                    spec = checks[prop]
                    label = f"{page}/{source_code}/{prop}"
                    if spec["dictionaryValue"] != requirement:
                        raise ValueError(f"{label}: stale dictionary value")
                    owner = spec["owner"]
                    if owner == "input-contracts":
                        function = binding_function(source_code, entry)
                        if binding.get("function") != function:
                            raise ValueError(f"{label}: wrong input function")
                        if not isinstance(spec.get("ruleCode"), str) or not spec["ruleCode"]:
                            raise ValueError(f"{label}: missing ruleCode")
                        if not spec.get("valid") or not spec.get("invalid"):
                            raise ValueError(f"{label}: positive and negative boundaries required")
                        if any(set(case) != {"value", "nullFlavor"} for case in spec["valid"] + spec["invalid"]):
                            raise ValueError(f"{label}: each boundary requires value and nullFlavor")
                        if prop == "max_length":
                            limit = length_limit(requirement)
                            for group, lengths in (("valid", (limit - 1, limit)), ("invalid", (limit + 1,))):
                                for char in ("X", "한"):
                                    for length in lengths:
                                        case = {"value": {"repeat": char, "count": length}, "nullFlavor": None}
                                        if case not in spec[group]:
                                            raise ValueError(f"{label}: missing ASCII/Unicode length boundary")
                        if prop == "null_flavors":
                            if any({"value": None, "nullFlavor": nf} not in spec["valid"] for nf in requirement):
                                raise ValueError(f"{label}: missing allowed nullFlavor boundary")
                        if prop == "allowed_value_constraint" and requirement["kind"] == "code_set":
                            if any({"value": value, "nullFlavor": None} not in spec["valid"] for value in requirement["values"]):
                                raise ValueError(f"{label}: missing allowed code boundary")
                        tests.append((page, code, source_code + "/" + prop, function))
                    elif owner == "validator":
                        if not spec.get("reason"):
                            raise ValueError(f"{label}: validator ownership requires a reason")
                        codes = spec.get("ruleCodes", [])
                        if not codes or len(codes) != len(set(codes)) or not set(codes) <= known:
                            raise ValueError(f"{label}: missing or unknown validator rules")
                    elif owner == "unresolved":
                        if not isinstance(spec.get("reason"), str) or not spec["reason"].strip():
                            raise ValueError(f"{label}: unresolved requirement needs a reason")
                    else:
                        raise ValueError(f"{label}: unsupported owner {owner!r}")
        for exclusion in document.get("dictionaryExclusions", []):
            code = exclusion["code"]
            if code in covered or code not in entries or not exclusion.get("reason"):
                raise ValueError(f"invalid/duplicate dictionary exclusion: {code}")
            if exclusion["source"] != entries[code][0]:
                raise ValueError(f"wrong exclusion source: {code}")
            # The existing registry explicitly records this transport-only input gap.
            if page != "N" or code != "N.1.1":
                raise ValueError(f"dictionary element must have an editor binding: {code}")
            covered.add(code)
    if covered != set(entries):
        raise ValueError(f"dictionary elements lack editor disposition: {sorted(set(entries) - covered)}")
    if not tests:
        raise ValueError("no executable dictionary input constraints")
    return tests


def render_tests(root: Path) -> str:
    declarations = []
    for page, code, key, function in check(root):
        source_code, prop = key.split("/")
        name = re.sub(r"[^a-z0-9]+", "_", f"{page}_{key}".lower())
        declarations.append(
            f'dictionary_test!({name}, "{page}", "{code}", "{source_code}", '
            f'"{prop}", input_contracts::generated::{function});'
        )
    return "// @generated by registry/tools/generate_editor_contract_web_tests.py.\n" + "\n".join(declarations) + "\n"
