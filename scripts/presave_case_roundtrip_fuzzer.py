#!/usr/bin/env python3
"""Seeded presave save/readback/audit fuzzer with a Case Edit import plan."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import re
import subprocess
import sys
import time
import unicodedata
from collections import Counter
from decimal import Decimal, InvalidOperation
from pathlib import Path
from typing import Any

import case_editor_input_fuzzer as candidates
from audit_trail_consistency_fuzzer import audit_soft_delete
from rbac_rls_blackbox import ApiClient, commit_sha, guard_target, response_summary


ROOT = Path(__file__).resolve().parents[1]
REGISTRY = ROOT / "registry/presaves/sections"
SECTIONS = {
    "sender": ("c-sender.json", "senders", "sender", "SD", "Import Sender"),
    "receiver": ("c-receiver.json", "receivers", "receiver", "SD", "Import Receiver"),
    "reporter": ("c-reporter.json", "reporters", "reporter", "RP", "Import Reporter"),
    "product": ("g-product.json", "products", "product", "DG", "Import Product"),
    "study": ("c-study.json", "studies", "study", "SI", "Import"),
    "narrative": ("h-narrative.json", "narratives", "narrative", "NR", "Import Record"),
}
MODEL_GROUPS = {
    "SenderPresave": ("sender", "sender_presaves"),
    "SenderPresaveGateway": ("gateways", "sender_presave_gateways"),
    "SenderPresaveResponsiblePerson": ("responsiblePersons", "sender_presave_responsible_persons"),
    "ReceiverPresave": ("receiver", "receiver_presaves"),
    "ReceiverPresaveConsignee": ("consignees", "receiver_presave_consignees"),
    "ReceiverPresaveRoute": ("routes", "receiver_presave_routes"),
    "ReporterPresave": ("reporter", "reporter_presaves"),
    "ProductPresave": ("product", "product_presaves"),
    "ProductPresaveActiveSubstance": ("activeSubstances", "product_presave_active_substances"),
    "StudyPresave": ("study", "study_presaves"),
    "StudyPresaveRegistrationNumber": ("registrationNumbers", "study_presave_registration_numbers"),
    "StudyPresaveFdaCrossReportedIndNumber": ("fdaCrossReportedInds", "study_presave_fda_cross_reported_ind_numbers"),
    "NarrativePresave": ("narrative", "narrative_presaves"),
    "StudyPresaveProduct": ("products", "study_presave_products"),
    "StudyPresaveReporter": ("reporters", "study_presave_reporters"),
}
GROUPS = {
    "sender": ("sender", "gateways", "responsiblePersons"),
    "receiver": ("receiver", "consignees", "routes"),
    "reporter": ("reporter",),
    "product": ("product", "activeSubstances"),
    "study": ("study", "products", "registrationNumbers", "fdaCrossReportedInds", "reporters"),
    "narrative": ("narrative",),
}
API_FIELD_ALIASES = {
    "substance_termid_version": "substanceTermIdVersion",
    "substance_termid": "substanceTermId",
    "strength_value": "substanceStrengthValue",
    "strength_unit": "substanceStrengthUnit",
}


def camel(value: str) -> str:
    head, *tail = value.split("_")
    return head + "".join(part[:1].upper() + part[1:] for part in tail)


def api_field(value: str) -> str:
    return API_FIELD_ALIASES.get(value, camel(value))


def prepared_contract(section_names: list[str], unsupported: list[str] | None = None) -> dict[str, dict[str, Any]]:
    """Discover writable Presave fields from their DTOs, never Case Edit contracts."""
    source = (ROOT / 'crates/libs/lib-core/src/model/presave.rs').read_text()
    schema = (ROOT / 'db/bootstrap/01-safetydb-schema.sql').read_text()
    models = {}
    for model, (group, table) in MODEL_GROUPS.items():
        if not any(group in GROUPS[section] for section in section_names):
            continue
        body = re.search(r'pub struct ' + model + r'ForUpdate \{(.*?)\n\}', source, re.S)
        table_body = re.search(r'CREATE TABLE IF NOT EXISTS ' + table + r' \((.*?)\n\);', schema, re.S)
        if body is None or table_body is None:
            raise ValueError(f'Missing Presave DTO/schema: {model}')
        fields = {}
        for match in re.finditer(r'((?:\s*#\[[^\n]+\]\s*)*)pub (\w+): ([^,\n]+),', body[1]):
            attrs, name, rust_type = match.groups()
            rename = re.search(r'rename = "([^"]+)"', attrs)
            column = re.search(r'^\s*' + name + r' ([^\n]+)', table_body[1], re.M)
            if column is None:
                raise ValueError(f'Missing Presave column: {model}.{name}')
            width = re.search(r'VARCHAR\((\d+)\)', column[1])
            fields[name] = {'type': rust_type, 'api': rename[1] if rename else api_field(name),
                            'width': int(width[1]) if width else None,
                            'nullClear': any(deserializer in attrs for deserializer in (
                                'deserialize_patch_option', 'deserialize_clearable_text'))}
        models[model] = fields
    return models


def load_fields(section_names: list[str], excluded: list[dict[str, Any]] | None = None,
                unsupported: list[str] | None = None, inventory: list[dict[str, Any]] | None = None) -> list[dict[str, Any]]:
    models = prepared_contract(section_names, unsupported)
    dictionary = {}
    for filename in ('ich-e2br3.json', 'fda-regional.json', 'mfds-regional.json'):
        document = json.loads((ROOT / 'registry/dictionary' / filename).read_text())
        for entry in document['entries']:
            dictionary[entry['code']] = {**entry, 'dictionary_file': filename, 'source_file': document['source']}
    fields = []
    for section in section_names:
        registry_file, _, parent_group, page, _ = SECTIONS[section]
        registry = json.loads((REGISTRY / registry_file).read_text())
        by_target = {}
        for row in registry:
            backend = row.get('backend', {})
            by_target.setdefault((backend.get('model'), backend.get('field')), []).append(row)
        for model, dto in models.items():
            group, table = MODEL_GROUPS[model]
            if group not in GROUPS[section]:
                continue
            for name, metadata in dto.items():
                if name == 'deleted':
                    continue  # Destructive transitions use the isolated lifecycle campaign below.
                rows = by_target.get((model, name), [])
                code = rows[0]['e2br3_code'] if rows else f'local.presave.{section}.{model}.{name}'
                frontend = rows[0].get('frontend', {}).get('field') if rows else None
                source_code = code if code in dictionary else 'MFDS.' + code
                source = dictionary.get(source_code)
                rust_type = metadata['type']
                if rust_type not in {'Option<String>', 'Option<bool>', 'Option<i32>', 'Option<Decimal>', 'Option<Uuid>', 'Option<Option<bool>>', 'Option<Option<Uuid>>'}:
                    raise ValueError(f'Unsupported Presave DTO type: {model}.{name}: {rust_type}')
                validator_linked = source is not None and model not in {'SenderPresaveGateway', 'ReceiverPresaveRoute', 'ReceiverPresaveConsignee', 'StudyPresaveProduct', 'StudyPresaveReporter'}
                is_nf = name.endswith('_null_flavor')
                baseline_value = 'FZ'
                if 'bool' in rust_type:
                    baseline_value = True if code == 'G.k.2.5' else False
                elif 'i32' in rust_type:
                    baseline_value = 1
                elif 'Decimal' in rust_type:
                    baseline_value = 1
                elif 'Uuid' in rust_type:
                    baseline_value = None  # Resolved from real test records before mutation.
                elif source and validator_linked:
                    allowed = source.get('allowed_value_constraint', {})
                    if allowed.get('kind') == 'code_set':
                        baseline_value = allowed['values'][0]
                    elif source.get('vocabulary') == 'ISO3166' or name == 'country_code' or name.endswith('_country'):
                        baseline_value = 'KR'
                    elif source.get('data_type') == 'N':
                        baseline_value = '1'
                    elif source.get('dictionary_file') == 'mfds-regional.json' and '=' in source.get('allowed_values', ''):
                        baseline_value = source['allowed_values'].split('=', 1)[0].strip()
                if code in {'FDA.C.5.5a', 'FDA.C.5.6.r'}:
                    baseline_value = 'IND123456'
                if code == 'FDA.C.5.5b':
                    baseline_value = 'ANDA123456'
                if code == 'G.k.2.3.r.3b':
                    baseline_value = 'mg'
                if 'email' in name:
                    baseline_value = 'presave@example.test'
                if name in {'gateway_authority', 'authority'}:
                    baseline_value = 'fda'
                if name == 'condition_operator':
                    baseline_value = 'Equal'
                if name == 'receiver_type':
                    baseline_value = 'Regulatory Authority'
                if name == 'sponsor_study_number_kind':
                    baseline_value = 'STUDY_NO'
                field = {'code': code, 'authority': rows[0].get('authority', 'LOCAL') if rows else 'LOCAL',
                         'section': section, 'pageId': page, 'model': model, 'group': group,
                         'parentGroup': parent_group, 'auditTable': table, 'backendField': name,
                         'apiField': metadata['api'], 'payloadPath': metadata['api'],
                         'frontendField': frontend, 'frontendPath': frontend or metadata['api'],
                         'roundTripValue': baseline_value, 'official': source, 'dto': metadata,
                         'presaveValidator': source_code if validator_linked else None,
                         'baselineBasis': 'dependency' if 'Uuid' in rust_type else 'official_representation' if validator_linked else 'dto_and_db',
                         'constraint': {'status': 'verified', 'invalidValue': {}, 'ruleCode': 'INPUT.JSON.INVALID'}}
                width = int(source['max_length']) if validator_linked and str(source.get('max_length', '')).isdigit() else metadata['width']
                if width:
                    field['_maxLength'] = width
                    field['_maxLengthRule'] = ('MFDS.' if source and source['dictionary_file'] == 'mfds-regional.json' else 'ICH.') + code + '.LENGTH.MAX' if validator_linked else 'INVALID_REQUEST'
                if 'bool' in rust_type:
                    field['_presaveCandidates'] = [{}, None, True, False, '', 0, 1, 'true']
                elif 'i32' in rust_type:
                    field['_presaveCandidates'] = [{}, None, 1, 0, -1, 2147483647, 2147483648, 1.5, '1', True]
                elif 'Uuid' in rust_type:
                    field['_presaveCandidates'] = [{}, None, '', 'not-a-uuid', 1, '00000000-0000-0000-0000-000000000000', None]  # Last slot is resolved from the real dependency record.
                if is_nf:
                    field['roundTripValue'] = ''
                fields.append(field)
        # Containers and aliases are accounted for, never miscounted as missing input tests.
        collection_targets = {"regulatorGateways": "gateways", "senderPersons": "responsiblePersons",
            "consignees": "consignees", "routes": "routes", "activeSubstances": "activeSubstances",
            "fdaCrossReportedIndNumbers": "fdaCrossReportedInds", "studyProducts": "products",
            "studyRegistrationNumbers": "registrationNumbers", "studyReporters": "reporters"}
        alias_targets = {"defaultSender": "isDefault", "isDefaultSender": "isDefault",
            "sender": "senderPresaveId", "receiver": "receiverPresaveId", "productId": "productPresaveId",
            "productName": "products.productName", "studyProductName": "products.productName"}
        for row in registry:
            model = row.get("backend", {}).get("model")
            name = row.get("backend", {}).get("field")
            target = (model, name)
            frontend = row.get("frontend", {}).get("field")
            matched = [f for f in fields if f["section"] == section and (f["model"], f["backendField"]) == target]
            disposition, link = "unresolved", None
            if matched:
                disposition, link = "scalar", matched[0]["code"]
            elif frontend in collection_targets:
                disposition, link = "collection", collection_targets[frontend]
            elif frontend in alias_targets:
                disposition, link = "alias", alias_targets[frontend]
            elif frontend == section + "Deleted" or name == "deleted":
                disposition, link = "lifecycle", section
            elif model in models and name and name.endswith("_presave_id") and name not in models[model]:
                disposition, link = "server_owned", "parent route ID supplied by details API"
            item = {"section": section, "code": row["e2br3_code"], "disposition": disposition,
                    "target": target, "covered_by": link}
            if inventory is not None:
                inventory.append(item)
            if disposition == "unresolved" and excluded is not None:
                excluded.append({**item, "reason": "unmapped Presave registry entry"})
    by_target = {(f['model'], f['backendField']): f for f in fields}
    for field in fields:
        name = field['backendField']
        if not name.endswith('_null_flavor'):
            continue
        partner = by_target.get((field['model'], name.removesuffix('_null_flavor')))
        if partner is None or partner['official'] is None:
            raise ValueError(f'No authoritative NullFlavor pair: {field["code"]}')
        allowed = partner['official'].get('null_flavors')
        if not allowed:
            raise ValueError(f'No authoritative NullFlavor codes: {field["code"]}')
        field['official'] = partner['official']
        field['presaveValidator'] = partner['presaveValidator']
        field['_allowedNullFlavors'] = allowed
        field['roundTripValue'] = allowed[0]
        field['nullFlavorPartnerCode'] = partner['code']
        partner['nullFlavorPartnerCode'] = field['code']
        prefix = '' if partner['code'].startswith('FDA.') else 'ICH.'
        field['constraint'] = {'status': 'verified', 'invalidValue': 'BADNF',
                               'ruleCode': prefix + partner['code'] + '.NULLFLAVOR.ALLOWED'}
    return fields

def mutation_value(field: dict[str, Any], rng: Any, ordinal: int, sample: int) -> Any:
    if "_presaveCandidates" in field:
        if field["section"] == "receiver" and field["backendField"] == "organization_name" and ordinal in {2, 4}:
            return f"FZ-RECEIVER-{rng.randrange(1_000_000_000)}"
        return copy.deepcopy(field["_presaveCandidates"][ordinal])
    # Isolate length from email syntax / Decimal decoding (ICH C.3.4.8, G.k.2.3.r.3a).
    if ordinal == 0 and field["code"] == "C.3.4.8":
        return "a" * 60 + "@" + "b" * 36 + ".com"
    if ordinal == 0 and field["code"] == "G.k.2.3.r.3a":
        return "11111111111"
    if field["code"] == "C.3.4.8" and ordinal in {14, 15, 16}:
        limit = int(field["official"]["max_length"])
        length = (limit, limit + 1, limit + 64)[ordinal - 14]
        return "a@" + "b" * (length - 7) + ".test"
    return candidates.field_value(field, rng, ordinal, sample)


def presave_expectation(field: dict[str, Any], value: Any) -> tuple[str, str | None] | None:
    """Storage/JSON contract, with source-backed representation limits.

    Submission conformance, vocabulary membership and conditional business rules
    are not inferred from a successful Presave write.
    """
    dtype = field.get("dto", {}).get("type", "")
    if value is None:
        if field.get("dto", {}).get("nullClear") and dtype == "Option<String>":
            value = ""
        else:
            return "accept", None
    name = field.get("backendField", "")
    if "Uuid" in dtype:
        if not isinstance(value, str) or not re.fullmatch(r"[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}", value):
            return "reject", "INPUT.JSON.INVALID"
        if value == "00000000-0000-0000-0000-000000000000":
            return "reject", None  # Referential/authorization rejection, exact code recorded.
        return "accept", None
    if "i32" in dtype:
        if type(value) is not int or not -2147483648 <= value <= 2147483647:
            return "reject", "INPUT.JSON.INVALID"
        return ("reject", "INVALID_REQUEST") if name.endswith("_day_count") and value < 0 else ("accept", None)
    if "bool" in dtype and field["code"] != "G.k.2.5":
        return ("accept", None) if type(value) is bool else ("reject", "INPUT.JSON.INVALID")
    if field["code"] == "G.k.2.5":
        if not isinstance(value, bool):
            return "reject", "INPUT.JSON.INVALID"
        return ("accept", None) if value else ("reject", "ICH.G.k.2.5.ALLOWED.VALUE")
    if field["code"] == "G.k.2.3.r.3a":
        try:
            if isinstance(value, (bool, dict, list)):
                return "reject", "INPUT.JSON.INVALID"
            number = Decimal(str(value))
            if not number.is_finite():
                return "reject", "INPUT.JSON.INVALID"
        except InvalidOperation:
            return "reject", "INPUT.JSON.INVALID"
        return (("reject", "ICH.G.k.2.3.r.3a.LENGTH.MAX")
                if len(str(number)) > 10 else ("accept", None))
    if not isinstance(value, str) or any(0xD800 <= ord(c) <= 0xDFFF for c in value):
        return "reject", "INPUT.JSON.INVALID"
    checked = bool(field.get("presaveValidator")) or ("dto" not in field and field.get("official") is not None) or name in {
        "organization_name_notation", "additional_information"} or (
        field.get("section") == "receiver" and name in {"organization_name", "receiver_type"})
    if any(unicodedata.category(c) == "Cc" and c not in "\t\n\r" for c in value):
        if checked:
            return "reject", "INPUT.CONTROL_CHAR.REJECTED"
        if "\x00" in value:
            return "reject", "INVALID_REQUEST"
    local_enum = {"gateway_authority": {"ich", "fda", "mfds"}, "authority": {"fda", "mfds"},
                  "condition_operator": {"Equal"}, "sponsor_study_number_kind": {"STUDY_NO", "PROTOCOL_NO"},
                  "receiver_type": {"Regulatory Authority", "Original Manufacturer"}}
    if name in local_enum and value not in local_enum[name]:
        clearable_blank = field.get("dto", {}).get("nullClear") and not value.strip()
        if not clearable_blank or name == "receiver_type":
            return "reject", "PRESAVE.RECEIVER_TYPE.ALLOWED" if name == "receiver_type" else "INVALID_REQUEST"
    if name == "organization_name_notation" and len(value) > 50:
        return "reject", "PRESAVE.SENDER.ORGANIZATION_NAME_NOTATION.LENGTH.MAX"
    if not field.get("official") or ("dto" in field and not field.get("presaveValidator")):
        if not field.get("dto"):
            return None
        if field.get("section") == "receiver" and name == "organization_name" and not value.strip():
            return "reject", "INVALID_REQUEST"
        width = field.get("dto", {}).get("width")
        if width and len(value.rstrip(" ")) > width:
            return "reject", "INVALID_REQUEST"
        return "accept", None
    if candidates.is_nullflavor_field(field):
        if not value.strip():
            return "accept", None
        if value not in field["_allowedNullFlavors"]:
            return "reject", field["constraint"]["ruleCode"]
        return "accept", None
    if not value.strip():
        # Legacy DTOs without the clearable-text deserializer retain whitespace.
        # PostgreSQL rejects their overlong non-space whitespace (e.g. tabs).
        if (not field.get("dto", {}).get("nullClear")
                and field["code"] in {"C.3.4.5", "C.5.1.r.2"}
                and len(value.rstrip(" ")) > 2):
            return "reject", "INVALID_REQUEST"
        # App-specific Presave identities, not authority submission conformance.
        required = {"C.3.1", "C.3.2", "C.2.r.1.2", "C.2.r.2.1", "C.2.r.4",
                    "C.5.2", "C.5.3", "C.5.4", "H.1"}
        return ("reject", "INVALID_REQUEST") if field["code"] in required else ("accept", None)
    if field["code"] == "C.3.4.8":
        parts = value.split("@")
        if (len(parts) != 2 or not parts[0] or any(c.isspace() for c in value)
                or "." not in parts[1] or not all(parts[1].split("."))):
            return "reject", "ICH.C.3.4.8.FORMAT"
    source = field.get("official")
    if not source:
        return None
    limit = source.get("max_length")
    if limit is None or not str(limit).isdigit():
        return None
    if len(value) > int(limit):
        prefix = "MFDS." if source["dictionary_file"] == "mfds-regional.json" else "ICH."
        code = field["code"] if field["code"].startswith("FDA.") else prefix + field["code"]
        return "reject", code + ".LENGTH.MAX"
    allowed = source.get("allowed_value_constraint", {})
    if source["dictionary_file"] == "mfds-regional.json":
        # User-designated workbook: representation only, not submission eligibility.
        entries = source.get("allowed_values", "").splitlines()
        if entries and all("=" in entry and entry.split("=", 1)[0].isdigit() for entry in entries):
            if value not in [entry.split("=", 1)[0] for entry in entries]:
                return "reject", "MFDS." + field["code"] + ".ALLOWED.VALUE"
    if allowed.get("kind") == "code_set" and value not in allowed["values"]:
        return "reject", "ICH." + field["code"] + ".ALLOWED.VALUE"
    if allowed.get("identifier_profile") and any(c in "\t\n\r" for c in value):
        return "reject", "ICH." + field["code"] + ".ALLOWED.VALUE"
    return "accept", None


def empty_rows(section: str) -> dict[str, Any]:
    parent = SECTIONS[section][2]
    return {group: {} if group == parent else [] for group in GROUPS[section]}


def group_row(rows: dict[str, Any], field: dict[str, Any]) -> dict[str, Any]:
    value = rows[field["group"]]
    if isinstance(value, list):
        if not value:
            value.append({"sequenceNumber": 1, "deleted": False})
        return value[0]
    return value


def get_value(rows: dict[str, Any], field: dict[str, Any]) -> Any:
    value = rows.get(field["group"])
    if isinstance(value, list):
        value = value[0] if value else {}
    return value.get(field["apiField"]) if isinstance(value, dict) else None


def set_value(rows: dict[str, Any], field: dict[str, Any], value: Any) -> None:
    group_row(rows, field)[field["apiField"]] = value


def parent_record_id(record_id: str, rows: dict[str, Any], field: dict[str, Any]) -> str | None:
    if field["group"] == field["parentGroup"]:
        return record_id
    row = group_row(rows, field)
    value = row.get("id")
    return value if isinstance(value, str) else None


def merge_child_ids(state: dict[str, Any], actual: dict[str, Any]) -> None:
    for group, value in state.items():
        if not isinstance(value, list) or not value:
            continue
        actual_rows = actual.get(group)
        if isinstance(actual_rows, list) and actual_rows and isinstance(actual_rows[0], dict):
            if isinstance(actual_rows[0].get("id"), str):
                value[0]["id"] = actual_rows[0]["id"]


def baseline(section: str, fields: list[dict[str, Any]], seed: int) -> dict[str, Any]:
    rows = empty_rows(section)
    for field in fields:
        if field["section"] != section or candidates.is_nullflavor_field(field):
            continue
        set_value(rows, field, copy.deepcopy(field["roundTripValue"]))
    identity = {
        "sender": ("organizationName", f"FZ-SD-{seed}"),
        "receiver": ("organizationName", f"FZ-SR-{seed}"),
        "reporter": ("organization", f"FZ-RP-{seed}"),
        "study": ("studyName", f"FZ-SI-{seed}"),
        "product": ("medicinalProduct", f"FZ-DG-{seed}"),
        "narrative": ("caseNarrative", f"FZ-NR-{seed}"),
    }[section]
    rows[SECTIONS[section][2]][identity[0]] = identity[1]
    if section == "sender":
        rows["gateways"][0].update(sequenceNumber=1, gatewayAuthority="fda", senderIdentifier="FZ-SENDER", isDefaultForAuthority=False)
        rows["responsiblePersons"][0].update(sequenceNumber=1, personGivenName="FZ Person", isDefault=False)
    if section == "receiver":
        rows["receiver"]["receiverType"] = "Regulatory Authority"
        rows["consignees"][0].update(sequenceNumber=1, name="FZ Consignee")
        rows["routes"][0].update(sequenceNumber=1, authority="fda", receiverLabel="FDA",
                                messageReceiverIdentifier="FZ", conditionPage="CI", conditionFieldCode="C.1.1",
                                conditionOperator="Equal", conditionValueCode="FZ", conditionValueLabel="FZ")
    if section == "product":
        rows["product"]["preApprovalIpName"] = f"FZ-IP-{seed}"
    if section == "narrative":
        rows["narrative"]["templateTitle"] = identity[1]
    return rows


def transfer_baseline(section: str, fields: list[dict[str, Any]], seed: int) -> dict[str, Any]:
    rows = baseline(section, fields, seed)
    if section == "reporter" and rows["reporter"].get("qualificationKr1"):
        rows["reporter"]["qualification"] = "3"
    if section == "study" and rows["study"].get("studyTypeReactionKr1"):
        rows["study"]["studyTypeReaction"] = "3"
    if section == "product":
        # MPID and PhPID are individually valid but mutually exclusive in Case Edit.
        rows["product"]["phpidVersion"] = ""
        rows["product"]["phpid"] = ""
        assert not (rows["product"].get("mpid") and rows["product"].get("phpid"))
    return rows


def unwrap(value: Any) -> Any:
    if isinstance(value, dict) and "data" in value:
        return value["data"]
    return value


def object_id(value: Any) -> str | None:
    return candidates.object_id(value)


def logs_changed(before: list[dict[str, Any]], after: list[dict[str, Any]]) -> list[dict[str, Any]]:
    before_ids = {str(log.get("id")) for log in before}
    return [log for log in after if str(log.get("id")) not in before_ids]


def error_detail(value: Any) -> tuple[str | None, str | None]:
    error = value.get("error", {}) if isinstance(value, dict) else {}
    data = error.get("data", {}) if isinstance(error, dict) else {}
    detail = data.get("detail", {}) if isinstance(data, dict) else {}
    if not isinstance(detail, dict):
        details = error.get("details", {}) if isinstance(error, dict) else {}
        detail = details.get("detail") if isinstance(details, dict) else None
    if not isinstance(detail, dict) or not detail:
        message = error.get("message") if isinstance(error, dict) else None
        return (message, None) if message == "INVALID_REQUEST" else (None, None)
    return detail.get("ruleCode") or detail.get("rule_code"), detail.get("path")


def unchanged_row_values(before: Any, after: Any) -> bool:
    # base/utils.rs refreshes updatedAt/updatedBy even for a no-op save.
    # Business values, identities and creation metadata must all survive.
    def without_update_time(value: dict[str, Any]) -> dict[str, Any]:
        return {key: item for key, item in value.items()
                if key not in {"updatedAt", "updated_at", "updatedBy", "updated_by"}}
    return json.loads(json.dumps(before), object_hook=without_update_time) == json.loads(
        json.dumps(after), object_hook=without_update_time
    )


def saved_value_classification(candidate: Any, before: Any, actual: Any,
                               audit_complete: bool, audit_changed: bool, *, null_clears: bool = False) -> str:
    changed = not candidates.values_equal(actual, before)
    if candidate is None and not null_clears:
        return "NOOP_ACCEPTED" if not changed and not audit_changed else "NULL_IGNORE_MISMATCH"
    if (changed and not audit_complete) or (not changed and audit_changed):
        return "AUDIT_MISMATCH"
    if candidates.values_equal(candidate, actual):
        return "SAVE_ACCEPTED" if changed else "NOOP_ACCEPTED"
    if candidates.is_blank_candidate(candidate):
        return candidates.normalized_classification(actual, before, audit_complete)
    return "SAVE_READBACK_MISMATCH"


def run(args: argparse.Namespace) -> int:
    args.runner_sha256 = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    args.contract_sha256 = hashlib.sha256(b"".join(
        path.read_bytes() for path in sorted(REGISTRY.glob("*.json"))
    ) + (ROOT / "crates/libs/lib-core/src/model/presave.rs").read_bytes()).hexdigest()
    guard_target(args.base_url, False)
    requested_sections = [value.strip().lower() for value in args.sections.split(",") if value.strip()]
    unknown = set(requested_sections) - set(SECTIONS)
    if unknown:
        raise SystemExit(f"unknown sections: {', '.join(sorted(unknown))}")
    if not requested_sections or args.values_per_field < 1 or args.samples_per_category < 1:
        raise SystemExit("select at least one section, value and sample")
    section_names = [section for section in SECTIONS if section in requested_sections]
    args.excluded_fields = []
    args.unsupported_contract_pairs = []
    args.coverage_inventory = []
    fields = load_fields(section_names, args.excluded_fields, args.unsupported_contract_pairs, args.coverage_inventory)
    if args.field:
        unknown_fields = set(args.field) - {field["code"] for field in fields}
        if unknown_fields:
            raise SystemExit(f"unsupported selected fields: {', '.join(sorted(unknown_fields))}")
    args.planned_mutations = sum(
        candidates.candidate_sample_count(field, ordinal, args.samples_per_category)
        for field in fields if not args.field or field["code"] in args.field
        for ordinal in range(min(args.values_per_field, len(field["_presaveCandidates"]))
                             if "_presaveCandidates" in field else candidates.candidate_count(field, args.values_per_field))
    )
    args.interrupted = None
    if args.dry_run:
        counts = Counter(field["section"] for field in fields)
        print(json.dumps({"verdict": "NOT_RUN", "fields": len(fields),
                          "planned_mutations": args.planned_mutations, "sections": dict(counts),
                          "excluded_fields": args.excluded_fields,
                          "unsupported_contract_pairs": args.unsupported_contract_pairs}, sort_keys=True))
        return 1 if args.excluded_fields or args.unsupported_contract_pairs or not args.planned_mutations else 0
    if not args.planned_mutations:
        raise SystemExit("no supported mutations selected; no API requests were sent")

    client = ApiClient(args.base_url, args.timeout)
    started = time.monotonic()
    requests = 0
    events: list[dict[str, Any]] = []
    records: dict[str, dict[str, Any]] = {}
    sha = commit_sha()

    def request(method: str, path: str, payload: dict[str, Any] | None = None) -> tuple[int | None, Any, dict[str, Any]]:
        nonlocal requests
        if args.interrupted:
            return None, None, {"interrupted": args.interrupted}
        if requests >= args.max_actions or time.monotonic() - started >= args.deadline_seconds:
            args.interrupted = "max_actions" if requests >= args.max_actions else "deadline"
            return None, None, {"interrupted": "limit"}
        requests += 1
        status, body, transport = client.request(method, path, payload)
        try:
            parsed = json.loads(body)
        except (json.JSONDecodeError, UnicodeDecodeError):
            parsed = None
        summary = response_summary(status, body)
        if transport:
            summary["transport_error"] = transport
            args.interrupted = "transport_error"
        return status, unwrap(parsed), summary

    def audit_logs(table: str, record_id: str | None) -> list[dict[str, Any]]:
        if not record_id:
            event(None, "AUDIT_UNAVAILABLE", None, kind="audit_read", reason="missing record id")
            return []
        status, value, _ = request("GET", f"/api/audit-logs/by-record/{table}/{record_id}")
        if status != 200 or not isinstance(value, list):
            event(None, "AUDIT_UNAVAILABLE", status, kind="audit_read", table=table)
            return []
        return value

    def event(field: dict[str, Any] | None, classification: str, status: int | None, **detail: Any) -> None:
        events.append({
            "kind": "mutation" if field else detail.pop("kind", "setup"),
            "seed": args.seed,
            "commit": sha,
            "section": field.get("section") if field else detail.pop("section", None),
            "field": field.get("code") if field else None,
            "ordinal": detail.pop("ordinal", None),
            "sample": detail.pop("sample", None),
            "classification": classification,
            "http_status": status,
            **detail,
        })

    status, _, summary = request("POST", "/auth/v1/login", {"email": args.email, "pwd": args.password})
    event(None, "PASS" if status == 200 else "FAIL", status, kind="login", response=summary)
    if status != 200:
        return write_artifacts(args, events, records, fields, requests, started, None)

    if args.prepare_case and "product" in section_names:
        status, profile, summary = request("GET", "/api/users/me")
        scope = profile.get("scope", {}) if isinstance(profile, dict) else {}
        blind_allowed = scope.get("accessBlindAllowed") is True
        event(
            None,
            "PASS" if status == 200 and blind_allowed else "FAIL",
            status,
            kind="case_blind_permission",
            section="product",
            blind_allowed=blind_allowed,
            response=summary,
        )

    support_sender_id: str | None = None
    support_product_id: str | None = None

    def ensure_sender_dependency(owner: str) -> str | None:
        nonlocal support_sender_id
        sender_id = records.get("sender", {}).get("id") or support_sender_id
        if sender_id:
            return sender_id
        support = {
            "sender": {
                "senderType": "1",
                "organizationName": f"FZ-SUPPORT-SD-{args.seed}",
            },
            "gateways": [],
            "responsiblePersons": [],
        }
        dep_status, dep_body, dep_summary = request(
            "POST", "/api/presaves/senders", {"data": {"rows": support}}
        )
        support_sender_id = object_id(dep_body)
        event(
            None,
            "PASS" if dep_status == 201 and support_sender_id else "FAIL",
            dep_status,
            kind="dependency",
            section=owner,
            response=dep_summary,
        )
        return support_sender_id

    def ensure_product_dependency() -> str | None:
        nonlocal support_product_id
        product_id = records.get("product", {}).get("id") or support_product_id
        if product_id:
            return product_id
        support = {
            "product": {
                "senderPresaveId": ensure_sender_dependency("study"),
                "productId": f"FZ-SUPPORT-PRODUCT-{args.seed}",
                "medicinalProduct": f"FZ-SUPPORT-DG-{args.seed}",
            },
            "activeSubstances": [],
        }
        dep_status, dep_body, dep_summary = request(
            "POST", "/api/presaves/products", {"data": {"rows": support}}
        )
        support_product_id = object_id(dep_body)
        event(
            None,
            "PASS" if dep_status == 201 and support_product_id else "FAIL",
            dep_status,
            kind="dependency",
            section="study",
            response=dep_summary,
        )
        return support_product_id

    setup_sections = set(section_names)
    if "study" in setup_sections:
        setup_sections.update({"product", "reporter"})
    if "product" in setup_sections:
        setup_sections.update({"sender", "receiver"})
    setup_fields = fields if setup_sections == set(section_names) else load_fields(list(setup_sections))
    for section in (name for name in SECTIONS if name in setup_sections):
        _, plural, _, _, _ = SECTIONS[section]
        state = baseline(section, setup_fields, args.seed)
        for field in setup_fields:
            if field["section"] == section and "Uuid" in field["dto"]["type"]:
                owner = field["backendField"].removesuffix("_presave_id")
                if owner in records:
                    set_value(state, field, records[owner]["id"])
        if section == "product":
            state["product"]["senderPresaveId"] = ensure_sender_dependency("product")
            state["product"]["productId"] = f"FZ-PRODUCT-{args.seed}"
        if section == "study":
            state["study"]["productPresaveId"] = ensure_product_dependency()
        status, created, summary = request("POST", f"/api/presaves/{plural}", {"data": {"rows": state}})
        record_id = object_id(created)
        event(None, "PASS" if status == 201 and record_id else "FAIL", status, kind="create", section=section, response=summary)
        if status != 201 or not record_id:
            continue
        status, actual, summary = request("GET", f"/api/presaves/{plural}/{record_id}" + ("" if section in {"reporter", "narrative"} else "/details"))
        actual_rows = actual.get("rows", {}) if isinstance(actual, dict) else {}
        if status != 200 or not isinstance(actual_rows, dict):
            event(None, "FAIL", status, kind="readback", section=section, response=summary)
            continue
        merge_child_ids(state, actual_rows)
        records[section] = {"id": record_id, "state": state, "baseline": copy.deepcopy(state), "actual": actual_rows}

    dependency_records = records
    records = {section: record for section, record in records.items() if section in section_names}
    for field in fields:
        if "Uuid" in field["dto"]["type"]:
            owner = field["backendField"].removesuffix("_presave_id")
            if owner in dependency_records:
                field["_presaveCandidates"][-1] = dependency_records[owner]["id"]
            else:
                event(None, "FAIL", None, kind="dependency", section=field["section"],
                      reason=f"missing real {owner} record for relationship mutation")
    by_code = {field["code"]: field for field in fields}
    for field in fields:
        if args.interrupted:
            break
        if args.field and field["code"] not in args.field:
            continue
        section = field["section"]
        record = records.get(section)
        if not record:
            continue
        _, plural, _, _, _ = SECTIONS[section]
        endpoint = f"/api/presaves/{plural}/{record['id']}" + ("" if section in {"reporter", "narrative"} else "/details")
        method = "PATCH" if section in {"reporter", "narrative"} else "PUT"
        restored = copy.deepcopy(record["baseline"])
        for companion in fields:
            if companion["section"] == section and candidates.is_nullflavor_field(companion):
                set_value(restored, companion, "")
        reset_status, _, reset_summary = request(method, endpoint, {"data": {"rows": restored}})
        reset_get, reset_body, _ = request("GET", endpoint)
        reset_rows = reset_body.get("rows") if isinstance(reset_body, dict) else None
        reset_ok = (reset_status == 200 and reset_get == 200 and isinstance(reset_rows, dict)
                    and all(candidates.values_equal(get_value(restored, item), get_value(reset_rows, item))
                            or (get_value(restored, item) == "" and get_value(reset_rows, item) is None)
                            for item in fields if item["section"] == section))
        event(None, "PASS" if reset_ok else "FAIL", reset_status, kind="field_baseline",
              section=section, target_field=field["code"], response=reset_summary)
        if not reset_ok:
            continue
        record["state"], record["actual"] = restored, reset_rows
        for ordinal in range(min(args.values_per_field, len(field["_presaveCandidates"]))
                             if "_presaveCandidates" in field else candidates.candidate_count(field, args.values_per_field)):
            for sample in range(candidates.candidate_sample_count(field, ordinal, args.samples_per_category)):
                rng = candidates.candidate_rng(args.seed, field, ordinal, sample)
                candidate = mutation_value(field, rng, ordinal, sample)
                before_rows = copy.deepcopy(record["actual"])
                before_value = get_value(before_rows, field)
                attempted = copy.deepcopy(record["state"])
                set_value(attempted, field, candidate)
                partner = by_code.get(field.get("nullFlavorPartnerCode"))
                partner_cleared_by_nf = False
                # ProductPresaveForUpdate explicitly distinguishes omission from JSON null.
                null_clears = field["dto"]["nullClear"]
                if candidate is not None and partner and partner["section"] == section:
                    if candidates.is_nullflavor_field(field):
                        if ordinal == 13 or candidate == "":
                            set_value(attempted, partner, partner["roundTripValue"])
                        else:
                            # Match UI: changing NF alone lets the pair trigger clear the old value.
                            group_row(attempted, partner).pop(partner["apiField"], None)
                            partner_cleared_by_nf = True
                    else:
                        set_value(attempted, partner, "")
                target_id = parent_record_id(record["id"], record["actual"], field)
                logs_before = audit_logs(field["auditTable"], target_id)
                omission_check = None
                if candidate is None:
                    omitted = copy.deepcopy(attempted)
                    group_row(omitted, field).pop(field["apiField"], None)
                    omitted_status, _, _ = request(method, endpoint, {"data": {"rows": omitted}})
                    omitted_get, omitted_body, _ = request("GET", endpoint)
                    omitted_rows = omitted_body.get("rows") if isinstance(omitted_body, dict) else None
                    omitted_logs = audit_logs(field["auditTable"], target_id)
                    omission_check = {
                        "status": omitted_status, "readback_status": omitted_get,
                        "rows_unchanged": unchanged_row_values(before_rows, omitted_rows),
                        "audit_unchanged": not logs_changed(logs_before, omitted_logs),
                    }
                status, body, summary = request(method, endpoint, {"data": {"rows": attempted}})
                rule, error_path = error_detail(body)
                status_get, actual, _ = request("GET", endpoint)
                actual_rows = actual.get("rows", {}) if isinstance(actual, dict) else {}
                actual_value = get_value(actual_rows, field) if isinstance(actual_rows, dict) else None
                logs_after = audit_logs(field["auditTable"], target_id)
                changed = logs_changed(logs_before, logs_after)
                expectation = presave_expectation(field, candidate)
                if field["code"] == "H.1" and candidates.is_blank_candidate(candidate):
                    additional = before_rows["narrative"].get("additionalInformation")
                    if isinstance(additional, str) and additional.strip():
                        expectation = ("accept", None)  # Either text supplies Narrative identity.
                expectation_basis = "unproven" if expectation is None else "presave_storage_contract"
                if expectation and expectation[1] == "INPUT.JSON.INVALID":
                    expectation_basis = "json_dto"
                elif expectation and expectation[1] == "INVALID_REQUEST":
                    expectation_basis = "presave_identity"
                elif expectation and expectation[1] and expectation[1].endswith(".LENGTH.MAX"):
                    expectation_basis = "official_max_length"
                if field.get("official") and field["official"]["dictionary_file"] == "mfds-regional.json":
                    expectation_basis = "user_designated_mfds_workbook_and_storage_contract"
                if ordinal == 0 and "_presaveCandidates" not in field and field["code"] not in {"G.k.2.5", "G.k.2.3.r.3a", "C.3.4.8"}:
                    expectation = ("reject", field["constraint"]["ruleCode"])
                    expectation_basis = "presave_dto_representation"
                if candidates.is_nullflavor_field(field) and ordinal == 13:
                    expectation = ("reject", "INPUT_CONTRACT.NULLFLAVOR.PAIR")
                expectation_failure = candidates.expectation_error(expectation, status, rule)
                partner_classification = None
                day_count_clear_verified = None
                if status == 200 and status_get == 200:
                    matched = [
                        log for log in changed
                        if candidates.audit_key_matches(log.get("changed_fields", log.get("changedFields", {})), field["backendField"])
                    ]
                    complete = any(candidates.audit_log_complete(log) for log in matched)
                    classification = saved_value_classification(
                        candidate, before_value, actual_value, complete,
                        bool(changed) if candidate is None else bool(matched), null_clears=null_clears
                    )
                    if candidate is not None and partner and partner["section"] == section:
                        partner_logs = [log for log in changed if candidates.audit_key_matches(
                            log.get("changed_fields", log.get("changedFields", {})), partner["backendField"])]
                        partner_classification = saved_value_classification(
                            "" if partner_cleared_by_nf else get_value(attempted, partner), get_value(before_rows, partner),
                            get_value(actual_rows, partner),
                            any(candidates.audit_log_complete(log) for log in partner_logs), bool(partner_logs))
                        if partner_classification not in {"SAVE_ACCEPTED", "NOOP_ACCEPTED", "SAVE_NORMALIZED"}:
                            classification = "PARTNER_" + partner_classification
                    if (section == "receiver" and field["backendField"].endswith("_not_applicable")
                            and candidate is True):
                        count_name = field["backendField"].removesuffix("_not_applicable") + "_day_count"
                        count_field = next(item for item in fields if item["section"] == section
                                           and item["backendField"] == count_name)
                        count_logs = [log for log in changed if candidates.audit_key_matches(
                            log.get("changed_fields", log.get("changedFields", {})), count_name)]
                        day_count_clear_verified = (get_value(actual_rows, count_field) is None
                            and (get_value(before_rows, count_field) is None
                                 or any(candidates.audit_log_complete(log) for log in count_logs)))
                        if not day_count_clear_verified:
                            classification = "DAY_COUNT_CLEAR_MISMATCH"
                        set_value(attempted, count_field, None)
                    expected_null_rows = copy.deepcopy(before_rows)
                    if candidate is None and null_clears:
                        set_value(expected_null_rows, field, None)
                    if candidate is None and (
                        omission_check != {"status": 200, "readback_status": 200,
                                           "rows_unchanged": True, "audit_unchanged": True}
                        or not unchanged_row_values(expected_null_rows, actual_rows)
                    ):
                        classification = "NULL_OMISSION_MISMATCH"
                    if expectation_failure and classification in {"SAVE_ACCEPTED", "NOOP_ACCEPTED", "SAVE_NORMALIZED"}:
                        classification = "EXPECTATION_MISMATCH"
                    if candidate is not None or null_clears:
                        record["state"] = attempted
                    record["actual"] = actual_rows
                    merge_child_ids(record["state"], actual_rows)
                elif status in {400, 409, 422} and status_get == 200:
                    unchanged = actual_rows == before_rows
                    classification = (
                        "CONSTRAINT_REJECTED"
                        if unchanged and not changed and not expectation_failure and rule
                        else "FAIL"
                    )
                else:
                    classification = "FAIL"
                if (expectation and expectation[0] == "length_boundary" and status in {400, 409, 422}
                        and rule != expectation[1] and actual_rows == before_rows and not changed):
                    classification = "BOUNDARY_BLOCKED"
                if expectation is None and classification in {"SAVE_ACCEPTED", "NOOP_ACCEPTED", "SAVE_NORMALIZED", "CONSTRAINT_REJECTED"}:
                    classification = "NO_EXPECTATION"
                event(
                    field,
                    classification,
                    status,
                    ordinal=ordinal,
                    sample=sample,
                    candidate_kind="local_storage_contract" if "_presaveCandidates" in field else candidates.candidate_kind(field, ordinal),
                    candidate=candidates.redacted(candidate),
                    before=candidates.redacted(before_value),
                    readback=candidates.redacted(actual_value),
                    generation_fingerprint=candidates.candidate_fingerprint(field, ordinal, sample, candidate),
                    expected=expectation[0] if expectation else None,
                    expectation_scope="presave_storage_and_json_representation",
                    expectation_basis=expectation_basis,
                    official_source=field.get("official"),
                    presave_validator=field.get("presaveValidator"),
                    baseline_basis=field.get("baselineBasis"),
                    expected_rule=expectation[1] if expectation else None,
                    actual_rule=rule,
                    error_path=error_path,
                    expectation_failure=expectation_failure,
                    readback_status=status_get,
                    audit_new_logs=len(changed),
                    response=summary,
                    omission_check=omission_check,
                    null_contract="clear" if null_clears else "ignore",
                    partner_classification=partner_classification,
                    partner_cleared_by_nf=partner_cleared_by_nf,
                    day_count_clear_verified=day_count_clear_verified,
                )
                if status is None:
                    break
            if status is None:
                break

    for section, record in records.items():
        if args.interrupted:
            break
        _, plural, _, _, _ = SECTIONS[section]
        endpoint = f"/api/presaves/{plural}/{record['id']}" + ("" if section in {"reporter", "narrative"} else "/details")
        method = "PATCH" if section in {"reporter", "narrative"} else "PUT"
        attempted = transfer_baseline(section, fields, args.seed) if args.prepare_case else copy.deepcopy(record["baseline"])
        for field in fields:
            if field["section"] == section and candidates.is_nullflavor_field(field):
                set_value(attempted, field, "")
        parent = SECTIONS[section][2]
        for key in ("senderPresaveId", "productId", "productPresaveId"):
            value = record["state"].get(parent, {}).get(key)
            if value is not None:
                attempted[parent][key] = value
        merge_child_ids(attempted, record["actual"])
        status, _, _ = request(method, endpoint, {"data": {"rows": attempted}})
        status_get, actual, _ = request("GET", endpoint)
        actual_rows = actual.get("rows", {}) if isinstance(actual, dict) else {}
        section_fields = [field for field in fields if field["section"] == section]
        passed = (
            status == 200
            and status_get == 200
            and all(
                candidates.values_equal(get_value(attempted, field), get_value(actual_rows, field))
                or (get_value(attempted, field) == "" and get_value(actual_rows, field) is None)
                for field in section_fields
            )
        )
        event(
            None,
            "PASS" if passed else "FAIL",
            status,
            kind="transfer_baseline",
            section=section,
            readback_status=status_get,
        )
        if passed:
            record["state"] = attempted
            record["actual"] = actual_rows

    # Two rows exercise child identity, sparse edits, deletion and sibling isolation.
    for section, record in records.items():
        if args.interrupted:
            break
        _, plural, parent, _, _ = SECTIONS[section]
        endpoint = f"/api/presaves/{plural}/{record['id']}/details"
        for group, key in (("gateways", "senderIdentifier"), ("responsiblePersons", "personGivenName"),
                           ("consignees", "name"), ("routes", "receiverLabel"),
                           ("activeSubstances", "substanceName"),
                           ("registrationNumbers", "registrationNumber"),
                           ("fdaCrossReportedInds", "indNumber"), ("products", "productName"), ("reporters", "reporterGivenName")):
            if not record["state"].get(group):
                continue
            rows = copy.deepcopy(record["state"])
            sibling = copy.deepcopy(record["actual"][group][0])
            new_row = copy.deepcopy(rows[group][0])
            new_row.pop("id", None)
            for field_name in list(new_row):
                if field_name.endswith("NullFlavor") and new_row[field_name] == "":
                    del new_row[field_name]
            new_row.update(sequenceNumber=2, deleted=False)
            new_row[key] = "IND765432" if key == "indNumber" else "FZ Second Row"
            rows[group].append(new_row)
            status, _, summary = request("PUT", endpoint, {"data": {"rows": rows}})
            get_status, body, _ = request("GET", endpoint)
            actual = body["rows"] if get_status == 200 and isinstance(body, dict) else None
            added = ([item for item in actual[group] if item["id"] != sibling["id"]
                      and not item.get("deleted", False)] if actual else [])
            table = next(table for mapped_group, table in MODEL_GROUPS.values() if mapped_group == group)
            create_logs = audit_logs(table, added[0]["id"]) if len(added) == 1 else []
            create_audit_ok = any(candidates.audit_log_complete(log, "CREATE")
                and candidates.audit_key_matches(log.get("changed_fields", log.get("changedFields", {})),
                                                 candidates.snake(key)) for log in create_logs)
            kept = next((item for item in actual[group] if item["id"] == sibling["id"]), None) if actual else None
            passed = (status == 200 and len(added) == 1 and added[0][key] == new_row[key]
                      and unchanged_row_values(sibling, kept) and create_audit_ok)
            event(None, "PASS" if passed else "FAIL", status, kind="child_lifecycle",
                  section=section, group=group, action="add_second", response=summary,
                  audit_complete=create_audit_ok)
            if not passed:
                continue
            child_id = added[0]["id"]
            sibling = copy.deepcopy(kept)
            for action, patch in (("edit_second", {key: "IND876543" if key == "indNumber" else "FZ Edited Row"}),
                                  ("delete_second", {"deleted": True})):
                before_logs = audit_logs(table, child_id)
                status, _, summary = request("PUT", endpoint, {"data": {"rows": {
                    parent: {}, group: [{"id": child_id, **patch}]}}})
                get_status, body, _ = request("GET", endpoint)
                actual = body["rows"] if get_status == 200 and isinstance(body, dict) else None
                child = next((item for item in actual[group] if item["id"] == child_id), None) if actual else None
                kept = next((item for item in actual[group] if item["id"] == sibling["id"]), None) if actual else None
                delta = logs_changed(before_logs, audit_logs(table, child_id))
                audit_ok = any(
                    (candidates.audit_log_complete(log, "DELETE") or
                     (candidates.audit_log_complete(log, "UPDATE") and audit_soft_delete(log)))
                    if action == "delete_second" else
                    (candidates.audit_log_complete(log, "UPDATE") and candidates.audit_key_matches(
                        log.get("changed_fields", log.get("changedFields", {})), candidates.snake(key)))
                    for log in delta)
                passed = (status == 200 and get_status == 200 and sibling == kept
                          and audit_ok
                          and ((child is None or child.get("deleted") is True) if action == "delete_second"
                               else child is not None and child[key] == patch[key]))
                event(None, "PASS" if passed else "FAIL", status, kind="child_lifecycle",
                      section=section, group=group, action=action, child_id=child_id, response=summary,
                      audit_complete=audit_ok, sibling_preserved=sibling == kept)
                if not passed:
                    break
            if actual:
                record["actual"] = actual

    # Archive an independent test record, never a record used by another section.
    for section, record in records.items():
        if args.interrupted:
            break
        _, plural, parent, _, _ = SECTIONS[section]
        collection = f"/api/presaves/{plural}"
        suffix = "" if section in {"reporter", "narrative"} else "/details"
        sibling_endpoint = f"{collection}/{record['id']}{suffix}"
        sibling_status, sibling_before, _ = request("GET", sibling_endpoint)
        if sibling_status != 200 or not isinstance(sibling_before, dict) or "rows" not in sibling_before:
            event(None, "FAIL", sibling_status, kind="lifecycle_setup", section=section)
            continue
        clone_rows = baseline(section, fields, args.seed + 1)
        for key in ("senderPresaveId", "productPresaveId"):
            if key in record["state"][parent]:
                clone_rows[parent][key] = record["state"][parent][key]
        if section == "product":
            clone_rows[parent]["productId"] = f"FZ-LIFECYCLE-PRODUCT-{args.seed}"
        if section == "study":
            clone_rows[parent]["sponsorStudyNumber"] = f"FZ-LIFECYCLE-STUDY-{args.seed}"
        status, created, summary = request("POST", collection, {"data": {"rows": clone_rows}})
        clone_id = object_id(created)
        event(None, "PASS" if status == 201 and clone_id else "BLOCKED_LIFECYCLE_CREATION",
              status, kind="lifecycle_setup", section=section, record_id=clone_id, response=summary)
        if status != 201 or not clone_id:
            continue
        root_endpoint = f"{collection}/{clone_id}"
        table = next(table for group, table in MODEL_GROUPS.values() if group == parent)
        for action, deleted in (("delete", True), ("restore", False), ("delete_again", True)):
            if args.interrupted:
                break
            logs_before = audit_logs(table, clone_id)
            if deleted:
                status, _, summary = request("DELETE", root_endpoint)
            else:
                data = {"rows": {parent: {"deleted": False}}} if section in {"reporter", "narrative"} else {"deleted": False}
                status, _, summary = request("PATCH", root_endpoint, {"data": data})
            detail_status, detail, _ = request("GET", root_endpoint + suffix)
            list_status, listing, _ = request("GET", collection)
            sibling_status, sibling_after, _ = request("GET", sibling_endpoint)
            detail_parent = detail.get("rows", {}).get(parent) if isinstance(detail, dict) else None
            # Receiver, Product and Study lists return models; others return section rows.
            listed_parents = [
                item if section in {"receiver", "product", "study"} else item.get("rows", {}).get(parent)
                for item in listing if isinstance(item, dict)
            ] if isinstance(listing, list) else []
            listed = [item for item in listed_parents
                      if isinstance(item, dict) and item.get("id") == clone_id]
            logs_after = audit_logs(table, clone_id)
            fresh_logs = logs_changed(logs_before, logs_after)
            audit_ok = any(
                candidates.audit_log_complete(log)
                and (audit_soft_delete(log) if deleted else any(
                    candidates.snake(str(key).split(".")[-1]) == "deleted"
                    and isinstance(delta, dict) and delta.get("old") is True and delta.get("new") is False
                    for key, delta in log.get("changedFields", log.get("changed_fields", {})).items()
                )) for log in fresh_logs
            )
            passed = (
                status == (204 if deleted else 200) and detail_status == 200 and list_status == 200
                and isinstance(detail_parent, dict) and detail_parent.get("id") == clone_id
                and detail_parent.get("deleted") is deleted
                and len(listed) == 1 and listed[0].get("deleted") is deleted
                and sibling_status == 200 and sibling_after == sibling_before and audit_ok
            )
            event(None, "PASS" if passed else "FAIL", status, kind="lifecycle", section=section,
                  action=action, record_id=clone_id, expected_deleted=deleted,
                  detail_status=detail_status, list_status=list_status,
                  detail_deleted=detail_parent.get("deleted") if isinstance(detail_parent, dict) else None,
                  list_matches=len(listed), list_deleted=[item.get("deleted") for item in listed],
                  sibling_preserved=sibling_status == 200 and sibling_after == sibling_before,
                  audit_complete=audit_ok, response=summary)
            if not passed:
                break

    case_id = None
    if args.prepare_case and records and not args.interrupted and all(
        item["classification"] in {"PASS", "SAVE_ACCEPTED", "NOOP_ACCEPTED", "SAVE_NORMALIZED", "CONSTRAINT_REJECTED"}
        for item in events
    ):
        try:
            case_id = setup_case(args)
        except subprocess.CalledProcessError as error:
            event(None, "FAIL", None, kind="case_setup", exit_code=error.returncode)
    status, integrity, summary = request("GET", "/api/audit-logs/verify-integrity")
    broken = (
        integrity.get("brokenRows", integrity.get("broken_rows"))
        if isinstance(integrity, dict)
        else None
    )
    event(None, "PASS" if status == 200 and broken == 0 else "FAIL", status, kind="audit_chain", response=summary, broken_rows=broken)
    return write_artifacts(args, events, records, fields, requests, started, case_id)


def setup_case(args: argparse.Namespace) -> str | None:
    if not args.product_key or not args.meddra_version or not args.meddra_code:
        raise SystemExit("--prepare-case requires --product-key, --meddra-version, and --meddra-code")
    setup_dir = Path(args.artifact_dir) / "case-setup"
    command = [
        sys.executable,
        str(ROOT / "scripts/case_editor_input_fuzzer.py"),
        "--base-url", args.base_url,
        "--email", args.email,
        "--password", args.password,
        "--seed", str(args.seed),
        "--pages", "DG",
        "--field", "G.k.2.2",
        "--values-per-field", "0",
        "--samples-per-category", "1",
        "--artifact-dir", str(setup_dir),
        "--no-run-gates",
        "--product-key", args.product_key,
        "--meddra-version", args.meddra_version,
        "--meddra-code", args.meddra_code,
        "--contract", args.contract,
        "--null-flavor-pairs", args.null_flavor_pairs,
    ]
    subprocess.run(command, cwd=ROOT, check=True)
    artifact = setup_dir / f"case-editor-{args.seed}.jsonl"
    result = json.loads(artifact.read_text(encoding="utf-8").splitlines()[-1])
    return result.get("case_id")


def write_artifacts(
    args: argparse.Namespace,
    events: list[dict[str, Any]],
    records: dict[str, dict[str, Any]],
    fields: list[dict[str, Any]],
    requests: int,
    started: float,
    case_id: str | None,
) -> int:
    out = Path(args.artifact_dir)
    out.mkdir(parents=True, exist_ok=True)
    artifact = out / f"presave-roundtrip-{args.seed}.jsonl"
    counts = Counter(event["classification"] for event in events)
    executed = sum(event["kind"] == "mutation" for event in events)
    lifecycle_executed = sum(event["kind"] == "lifecycle" for event in events)
    section_names = {name.strip().lower() for name in args.sections.split(",") if name.strip()}
    child_executed = sum(event["kind"] == "child_lifecycle" for event in events)
    child_planned = sum(3 * {"sender": 2, "receiver": 2, "product": 1, "study": 4,
                            "reporter": 0, "narrative": 0}[section] for section in section_names)
    allowed = {"PASS", "SAVE_ACCEPTED", "NOOP_ACCEPTED", "SAVE_NORMALIZED", "CONSTRAINT_REJECTED"}
    complete = (
        args.planned_mutations > 0 and executed == args.planned_mutations
        and lifecycle_executed == 3 * len(section_names)
        and child_executed == child_planned
        and not args.interrupted and not args.excluded_fields and not args.unsupported_contract_pairs
        and all(item["classification"] in allowed for item in events)
        and (not args.prepare_case or bool(case_id))
    )
    run_event = {
        "kind": "run",
        "seed": args.seed,
        "case_id": case_id,
        "fields": len(fields),
        "events": len(events),
        "requests": requests,
        "elapsed_seconds": round(time.monotonic() - started, 3),
        "counts": dict(counts),
        "surface": "presave-api+case-edit-ui-plan",
        "verdict": "SELECTED_PRESAVE_API_CHECKS_PASSED" if complete else "INCOMPLETE_OR_FAILED",
        "planned_mutations": args.planned_mutations,
        "executed_mutations": executed,
        "planned_lifecycle_steps": 3 * len(section_names),
        "executed_lifecycle_steps": lifecycle_executed,
        "planned_child_lifecycle_steps": child_planned,
        "executed_child_lifecycle_steps": child_executed,
        "excluded_fields": args.excluded_fields,
        "coverage_inventory": getattr(args, "coverage_inventory", []),
        "unsupported_contract_pairs": args.unsupported_contract_pairs,
        "interrupted": args.interrupted,
        "ui_verified": False,
        "official_compliance_verified": False,
        "not_covered": [
            "fields absent from the registry or supported input contract",
            "role/organization combinations and in-use deletion conflicts",
            "browser controls, messages, unsaved navigation and Case import",
        ],
        "runner_sha256": args.runner_sha256,
        "contract_sha256": args.contract_sha256,
    }
    with artifact.open("w", encoding="utf-8") as handle:
        for event in events:
            handle.write(json.dumps(event, ensure_ascii=True, sort_keys=True) + "\n")
        handle.write(json.dumps(run_event, ensure_ascii=True, sort_keys=True) + "\n")
    plan_records = []
    for section, record in records.items():
        _, _, _, page, import_button = SECTIONS[section]
        expected = []
        for field in fields:
            if field["section"] != section:
                continue
            expected_value = get_value(record["actual"], field)
            if field["code"] == "local.receiver.2":
                expected_value = {
                    "Original Manufacturer": "1",
                    "Regulatory Authority": "2",
                }.get(expected_value, expected_value)
            expected.append({
                "code": field["code"],
                "authority": field.get("authority", "ICH").lower(),
                "projectionPath": field.get("projectionPath"),
                "value": expected_value,
                "auditField": field["backendField"],
            })
        identity_field = {
            "sender": "organizationName",
            "receiver": "organizationName",
            "reporter": "organization",
            "study": "studyName",
            "product": "medicinalProduct",
            "narrative": "templateTitle",
        }[section]
        identity = record["actual"][SECTIONS[section][2]].get(identity_field)
        plan_records.append({
            "section": section,
            "recordId": record["id"],
            "page": page,
            "importButton": import_button,
            "identity": identity,
            "expected": expected,
        })
    plan = {
        "schemaVersion": 2,
        "seed": args.seed,
        "caseId": case_id,
        "coverageMode": "random-presave-fuzz+transfer-safe-case-baseline",
        "executionStatus": "NOT_RUN",
        "records": plan_records,
    }
    plan_path = out / f"presave-ui-plan-{args.seed}.json"
    plan_path.write_text(json.dumps(plan, ensure_ascii=True, indent=2, sort_keys=True), encoding="utf-8")
    print(f"verdict={run_event['verdict']} mutations={executed}/{args.planned_mutations} counts={dict(counts)} fields={len(fields)} requests={requests} artifact={artifact} ui_plan={plan_path}")
    return 2 if args.interrupted else 0 if complete else 1


def parser() -> argparse.ArgumentParser:
    value = argparse.ArgumentParser()
    value.add_argument("--base-url", default=os.getenv("E2BR3_BASE_URL", "http://127.0.0.1:8080"))
    value.add_argument("--email", default=os.getenv("E2BR3_ADMIN_EMAIL", "demo.cro.admin@example.com"))
    value.add_argument("--password", default=os.getenv("E2BR3_ADMIN_PASSWORD", "welcome"))
    value.add_argument("--seed", type=int, default=int(time.time()))
    value.add_argument("--sections", default=",".join(SECTIONS))
    value.add_argument("--field", action="append", help="mutate only this field code; section baselines remain complete")
    value.add_argument("--prepare-case", action="store_true", help="also prepare a Case for the unexecuted UI import plan")
    value.add_argument("--values-per-field", type=int, default=candidates.IDENTIFIER_CANDIDATES)
    value.add_argument("--samples-per-category", type=int, default=1)
    value.add_argument("--artifact-dir", default="tmp/rbac-rls-fuzz/presave-case-roundtrip")
    value.add_argument("--max-actions", type=int, default=30000)
    value.add_argument("--deadline-seconds", type=float, default=1800)
    value.add_argument("--timeout", type=float, default=15)
    value.add_argument("--dry-run", action="store_true")
    value.add_argument("--product-key", default=os.getenv("E2BR3_PRODUCT_KEY"))
    value.add_argument("--meddra-version", default=os.getenv("E2BR3_MEDDRA_VERSION"))
    value.add_argument("--meddra-code", default=os.getenv("E2BR3_MEDDRA_CODE"))
    value.add_argument("--contract", default=str(candidates.DEFAULT_CONTRACT))
    value.add_argument("--null-flavor-pairs", default=str(candidates.DEFAULT_NULL_FLAVOR_PAIRS))
    return value


if __name__ == "__main__":
    sys.exit(run(parser().parse_args()))
