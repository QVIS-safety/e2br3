#!/usr/bin/env python3
"""Create a rich Case, optionally mix seeded NullFlavors, and roundtrip XML.

Example: --null-flavor-rate 0.3 --seed 20261012 --rounds 5
Baselines choose compatible FDA postmarket/MFDS CT values; conditionally required
fields are excluded from NullFlavor mixing and recorded in the plan.
Only eligible contract-backed fields are mixed. Optional transport verification:
--submit --authorities fda,mfds --sender-presave-id UUID
--fda-receiver 'FDA(Postmarket)' --mfds-receiver 'MFDS(CT)'
Submission uses the backend's configured gateway; ACK1 is not regulatory acceptance.
Submit mode retains all Cases, including ambiguous/failed attempts."""

from __future__ import annotations

import argparse
import copy
import math
import hashlib
import json
import os
import subprocess
import sys
import time
import urllib.error
import uuid
from pathlib import Path
from typing import Any

import case_editor_input_fuzzer as editor
from case_editor_input_fuzzer import object_id
from rbac_rls_blackbox import ApiClient, guard_target, response_summary
from xml_import_fuzzer import (
	identity_signature,
	multipart,
	preservation_signature,
	structural_signature,
	unique_xml,
)


ROOT = Path(__file__).resolve().parents[1]
PAGES = ("CI", "RP", "SD", "LR", "SI", "DM", "DH", "NR", "AE", "LB", "DG")


def json_value(body: bytes) -> Any:
	try:
		return json.loads(body)
	except (UnicodeDecodeError, json.JSONDecodeError):
		return None


def imported_case_id(body: bytes) -> str | None:
	value = json_value(body)
	data = value.get("data", value) if isinstance(value, dict) else {}
	rows = data.get("importedCases", data.get("imported_cases", [])) if isinstance(data, dict) else []
	if isinstance(rows, list):
		for row in rows:
			if isinstance(row, dict):
				case_id = row.get("caseId") or row.get("case_id")
				if isinstance(case_id, str):
					return case_id
	case_id = data.get("caseId") or data.get("case_id") if isinstance(data, dict) else None
	return case_id if isinstance(case_id, str) else None


def fingerprint(value: str) -> str:
	return hashlib.sha256(value.encode()).hexdigest()[:12]


def summary(status: int | None, body: bytes, transport: str | None = None) -> dict[str, Any]:
	value = response_summary(status, body)
	if transport:
		value["transport_error"] = transport
	return value


def null_flavor_rate(value: str) -> float:
	rate = float(value)
	if not math.isfinite(rate) or not 0 <= rate <= 1:
		raise argparse.ArgumentTypeError("NullFlavor rate must be between 0 and 1")
	return rate


def mix_null_flavors(contract, pairs, allowed, seed, rate):
	"""Annotate complete baselines; preserve original contract test candidates."""
	mixed = copy.deepcopy(contract)
	# A complete Case must choose one representation of mutually exclusive fields.
	omit = {"D.2.1", "D.2.2.1a", "D.2.2.1b", "D.2.3", "D.7.1.r.6",
		"C.1.11.1", "C.1.11.2", "C.5.4", "FDA.C.5.5a", "FDA.C.5.5b", "FDA.C.5.6.r",
		"D.10.2.2a", "D.10.2.2b", "D.10.8.r.3a", "D.10.8.r.3b",
		"G.k.2.1.2a", "G.k.2.1.2b", "G.k.2.5", "E.i.8", "FDA.G.k.1.a", "C.4.r.2"}
	for page in mixed:
		for field in page["fields"]:
			if field["payloadPath"].startswith("fdaCrossReportedIndNumbers[]"):
				field["_richOmitRoot"] = "fdaCrossReportedIndNumbers"
			if field["code"] in omit or (page["pageId"] == "DH" and "phpid" in field["payloadPath"].lower()):
				field["_richValue"] = None
			elif field["code"] in {"D.7.3", "D.9.3"}:
				field["_richValue"] = True
			elif field["code"] == "C.1.2":
				# shortcut: legacy message_date holds 14 digits; offset coverage needs a header schema fix.
				field["_richValue"] = "20260722120000"
			if field["code"] == "G.k.9.i.2.r.1":
				field.setdefault("_fixedPayload", {})["drugReactionAssessments[].methodOfAssessmentKr1"] = "2"
	expanded = copy.deepcopy(mixed)
	unsupported = []
	editor.expand_null_flavor_contracts(expanded, pairs, allowed, unsupported)
	plan = {"seed": seed, "rate": rate, "selected": [], "skipped": [], "unsupported": unsupported}
	# These owners are overwritten by the existing authority setup/repair steps.
	protected = {"safetyReportIdentification", "senderInformation", "studyInformation", "studyRegistrationNumbers", "patientIdentifiers"}
	for page in expanded:
		if page["pageId"] not in PAGES:
			continue
		originals = editor.contract_rows(mixed, page["pageId"])
		for nf in page["fields"]:
			if not editor.is_nullflavor_field(nf) or "_allowedNullFlavors" not in nf:
				continue
			matches = [f for f in originals if f["payloadPath"] == nf["_nullFlavorPartnerPath"] and f["patch"] == nf["patch"] and not editor.is_nullflavor_field(f)]
			identity = {"page": page["pageId"], "code": nf["code"], "path": nf["payloadPath"]}
			if len(matches) != 1:
				plan["skipped"].append({**identity, "reason": "no unique concrete baseline field"})
				continue
			field = matches[0]
			if ("_richValue" in field and field["_richValue"] is None) or field["code"] in {"C.2.r.4", "D.2.2a", "D.2.2b", "D.10.6", "D.9.3", "G.k.9.i.2.r.1", "G.k.9.i.2.r.2", "G.k.9.i.2.r.3"}:
				plan["skipped"].append({**identity, "reason": "conditional baseline requirement"})
				continue
			if field["patch"]["owner"] in protected or field["payloadPath"] == "senderComments":
				plan["skipped"].append({**identity, "reason": "authority setup overwrites field"})
				continue
			rng = editor.candidate_rng(seed, field, 0, 0)
			if rng.random() >= rate:
				continue
			token = rng.choice(nf["_allowedNullFlavors"])
			field["_richNullFlavor"] = {"path": nf["payloadPath"], "value": token}
			plan["selected"].append({**identity, "value_path": field["payloadPath"], "null_flavor": token})
	return mixed, plan


def setup_rich_case(args: argparse.Namespace, seed: int, output_dir: Path) -> tuple[str | None, dict[str, Any]]:
	if not args.product_key or not args.meddra_version or not args.meddra_code:
		raise SystemExit("rich case setup requires --product-key, --meddra-version, and --meddra-code")
	setup_dir = output_dir / f"round-{seed}" / "setup"
	setup_dir.mkdir(parents=True, exist_ok=True)
	contract_path = Path(args.contract) if args.contract else editor.DEFAULT_CONTRACT
	pairs_path = Path(args.null_flavor_pairs) if args.null_flavor_pairs else editor.DEFAULT_NULL_FLAVOR_PAIRS
	mixed, plan = mix_null_flavors(
		json.loads(contract_path.read_text()), editor.load_null_flavor_pairs(pairs_path),
		editor.load_dictionary_null_flavors(ROOT), seed, args.null_flavor_rate,
	)
	generated_contract = setup_dir / "mixed-contract.json"
	generated_contract.write_text(json.dumps(mixed, ensure_ascii=False, indent=2))
	(setup_dir / "null-flavor-plan.json").write_text(json.dumps(plan, ensure_ascii=False, indent=2))
	command = [
		sys.executable,
		str(ROOT / "scripts/case_editor_input_fuzzer.py"),
		"--base-url", args.base_url,
		"--email", args.email,
		"--password", args.password,
		"--seed", str(seed),
		"--pages", ",".join(PAGES),
		"--complete-baseline",
		"--values-per-field", "0",
		"--samples-per-category", "1",
		"--product-key", args.product_key,
		"--meddra-version", args.meddra_version,
		"--meddra-code", args.meddra_code,
		"--artifact-dir", str(setup_dir),
		"--no-run-gates",
		"--no-verify-global-audit-chain",
	]
	command.extend(["--contract", str(generated_contract.resolve())])
	if args.null_flavor_pairs:
		command.extend(["--null-flavor-pairs", args.null_flavor_pairs])
	try:
		process = subprocess.run(
			command,
			cwd=ROOT,
			capture_output=True,
			text=True,
			timeout=args.setup_timeout,
			check=False,
		)
	except (OSError, subprocess.TimeoutExpired) as error:
		return None, {"status": "FAIL", "reason": type(error).__name__}
	(setup_dir / "stdout.log").write_text(process.stdout, encoding="utf-8")
	(setup_dir / "stderr.log").write_text(process.stderr, encoding="utf-8")
	artifact = setup_dir / f"case-editor-{seed}.jsonl"
	case_id = None
	if artifact.exists():
		try:
			case_id = json.loads(artifact.read_text(encoding="utf-8").splitlines()[-1]).get("case_id")
		except (IndexError, json.JSONDecodeError):
			pass
	return case_id, {
		"status": "PASS" if process.returncode == 0 and case_id else "FAIL",
		"exit_code": process.returncode,
		"case_id": case_id,
		"stdout_fingerprint": fingerprint(process.stdout),
		"stderr_fingerprint": fingerprint(process.stderr),
		"artifact": str(artifact),
		"null_flavor_plan": plan,
	}


def create_import_product(client: ApiClient, token: str, sender_id: str | None = None) -> tuple[str | None, str | None, list[dict[str, Any]]]:
	results: list[dict[str, Any]] = []
	if sender_id is None:
		sender_payload = {
			"data": {
				"rows": {
					"sender": {"senderType": "1", "organizationName": f"RICH-IMPORT-{token}"},
			"gateways": [
				{
					"sequenceNumber": index,
					"gatewayAuthority": authority,
					"senderIdentifier": f"RICH-{authority.upper()[:3]}-01",
					"isDefaultForAuthority": True,
					"deleted": False,
				}
				for index, authority in enumerate(("ich", "fda", "mfds"), 1)
			],
					"responsiblePersons": [],
				}
			}
		}
		status, body, transport = client.request("POST", "/api/presaves/senders", sender_payload)
		results.append({"kind": "support_sender", "status": status, "response": summary(status, body, transport)})
		sender_id = object_id(json_value(body))
		if status != 201 or not sender_id:
			return None, None, results
	product_payload = {
		"data": {
			"rows": {
				"product": {
					"senderPresaveId": sender_id,
					"productId": f"RICH-IMPORT-PRODUCT-{token}",
					"medicinalProduct": f"Rich Import Product {token}",
				},
				"activeSubstances": [],
			}
		}
	}
	status, body, transport = client.request("POST", "/api/presaves/products", product_payload)
	results.append({"kind": "support_product", "status": status, "response": summary(status, body, transport)})
	return (object_id(json_value(body)) if status == 201 else None), sender_id, results


def link_case_sender(client: ApiClient, case_id: str, sender_id: str) -> dict[str, Any]:
	status, body, transport = client.request(
		"PATCH",
		f"/api/cases/{case_id}/editor/pages/SD",
		{
			"authorities": ["ich", "fda", "mfds"],
			"rows": {"senderInformation": {"sourceSenderPresaveId": sender_id}},
		},
	)
	return {"kind": "case_sender_link", "status": status, "response": summary(status, body, transport)}


def configure_case_authority_types(client: ApiClient, case_id: str) -> dict[str, Any]:
	status, body, transport = client.request(
		"PUT",
		f"/api/cases/{case_id}",
		{
			"data": {"fda_report_type": "4", "mfds_report_type": "1"},
			"reason_for_change": "rich export/import roundtrip setup",
		},
	)
	return {"kind": "case_authority_types", "status": status, "response": summary(status, body, transport)}


def configure_case_identification(client: ApiClient, case_id: str) -> dict[str, Any]:
	status, body, transport = client.request(
		"PATCH",
		f"/api/cases/{case_id}/editor/pages/CI",
		{
			"authorities": ["ich", "fda", "mfds"],
			"rows": {
				"safetyReportIdentification": {
					"reportType": "1",
					"localCriteriaReportType": "2",
					"combinationProductReportIndicator": "false",
					"otherCaseIdentifiersExist": True,
					"fulfilExpeditedCriteria": False,
					"additionalDocumentsAvailable": False,
				}
			},
		},
	)
	return {"kind": "case_identification", "status": status, "response": summary(status, body, transport)}


def configure_mfds_ct_case(client: ApiClient, case_id: str, seed: int) -> list[dict[str, Any]]:
	"""Fill the MFDS CT topology without requiring a local KR product catalog."""
	existing = {}
	for page, owner in [("SI", "studyRegistrationNumbers"), ("DM", "patientIdentifiers")]:
		status, body, transport = client.request("GET", f"/api/cases/{case_id}/editor/pages/{page}")
		value = json_value(body)
		if status != 200 or transport or not isinstance(value, dict) or owner not in value.get("rows", {}):
			return [{"kind": "mfds_ct_read", "status": "FAIL", "response": summary(status, body, transport)}]
		existing[owner] = [row for row in value["rows"][owner] if not row["deleted"]]
	registrations = existing["studyRegistrationNumbers"]
	registration_key = {"id": registrations[0]["id"]} if registrations else {"sequenceNumber": 1}
	identifiers = existing["patientIdentifiers"]
	study_ids = [row for row in identifiers if row["identifier_type_code"] == "4"]
	identifier_key = {"id": study_ids[0]["id"]} if study_ids else {"sequenceNumber": max([row["sequence_number"] for row in identifiers], default=0) + 1}
	patches = [
		(
			"mfds_ct_identification",
			"CI",
			{"safetyReportIdentification": {"reportType": "2", "fulfilExpeditedCriteria": True}},
		),
		(
			"mfds_ct_study",
			"SI",
			{
				"studyInformation": {
					"studyName": "Rich CT study",
					"sponsorStudyNumber": f"STUDY-{seed}",
					"studyTypeReaction": "1",
				},
				"studyRegistrationNumbers": [{
					**registration_key,
					"registrationNumber": f"REG-{seed}",
					"countryCode": "KR",
				}],
			},
		),
		(
			"mfds_ct_patient_identifier",
			"DM",
			{
				"patientInformation": {},
				"patientIdentifiers": [{
					**identifier_key,
					"identifierTypeCode": "4",
					"identifierValue": f"STUDY-{seed}",
					"identifierValueNullFlavor": None,
				}],
			},
		),
	]
	results: list[dict[str, Any]] = []
	for kind, page, rows in patches:
		status, body, transport = client.request(
			"PATCH",
			f"/api/cases/{case_id}/editor/pages/{page}",
			{"authorities": ["mfds"], "rows": rows},
		)
		results.append({"kind": kind, "status": status, "response": summary(status, body, transport)})
	return results


def configure_case_narrative(client: ApiClient, case_id: str) -> dict[str, Any]:
	status, body, transport = client.request(
		"PATCH",
		f"/api/cases/{case_id}/editor/pages/NR",
		{
			"authorities": ["ich", "fda", "mfds"],
			"rows": {"narrative": {"senderComments": "발신자 의견"}},
		},
	)
	return {"kind": "case_narrative", "status": status, "response": summary(status, body, transport)}


def validation(client: ApiClient, case_id: str, authority: str) -> dict[str, Any]:
	status, body, transport = client.request("GET", f"/api/cases/{case_id}/validation?authority={authority}")
	value = json_value(body)
	data = value.get("data", {}) if isinstance(value, dict) else {}
	return {"status": status, "ok": data.get("ok") if isinstance(data, dict) else None, "issues": data.get("issues") if isinstance(data, dict) else None, "response": summary(status, body, transport)}


def prepare_receiver(client: ApiClient, args: argparse.Namespace, case_id: str, authority: str) -> dict[str, Any]:
	status, body, transport = client.request("PUT", f"/api/cases/{case_id}/submission-receiver", {
		"data": {"authority": authority, "receiver_label": getattr(args, f"{authority}_receiver", None)}})
	return {"kind": "prepare_receiver", "case_id": case_id, "authority": authority, "status": status, "response": summary(status, body, transport)}


def export_xml(client: ApiClient, case_id: str, authority: str) -> tuple[int | None, bytes, dict[str, Any]]:
	status, body, transport = client.request("GET", f"/api/cases/{case_id}/export/xml?authority={authority}")
	return status, body, summary(status, body, transport)


def compare_xml(source: bytes, exported: bytes) -> list[str]:
	failures: list[str] = []
	try:
		if identity_signature(source) != identity_signature(exported):
			failures.append("identity")
		before = structural_signature(source)
		after = structural_signature(exported)
		for key, count in before.items():
			if after[key] < count:
				failures.append(f"structure:{key}:{count}->{after[key]}")
		before = preservation_signature(source)
		after = preservation_signature(exported)
		for key, values in before.items():
			if values - after[key]:
				failures.append(f"preservation:{key}")
	except Exception as error:  # noqa: BLE001 - report malformed export as a test failure
		failures.append(f"compare:{type(error).__name__}")
	return failures


def delete_case(client: ApiClient, case_id: str) -> dict[str, Any]:
	status, body, transport = client.request("DELETE", f"/api/cases/{case_id}", {"reason_for_change": "rich export/import fuzzer cleanup"})
	return {"status": status, "case_id": case_id, "response": summary(status, body, transport)}


def submit_case(client: ApiClient, args: argparse.Namespace, case_id: str, authority: str, output_dir: Path) -> list[dict[str, Any]]:
	result = {"kind": "submission", "case_id": case_id, "authority": authority,
		"status": "FAIL", "reason": "not submitted", "events": [],
		"idempotency_key": f"rich-{authority}-{case_id}", "regulatory_acceptance": "not_verified"}
	# Persist intent before dispatch; a lost response must never trigger a blind resend.
	with (output_dir / f"submission-{case_id}.jsonl").open("x", encoding="utf-8") as journal:
		def record(stage, status, body, transport):
			result["events"].append({"stage": stage, "response": summary(status, body, transport)})
			journal.write(json.dumps(result, ensure_ascii=True) + "\n")
			journal.flush()
			os.fsync(journal.fileno())

		status, body, transport = client.request("PUT", f"/api/cases/{case_id}/submission-receiver", {
			"data": {"authority": authority, "receiver_label": getattr(args, f"{authority}_receiver")}})
		record("receiver", status, body, transport)
		if status != 200 or transport:
			return [result]
		check = validation(client, case_id, authority)
		if check["status"] != 200 or check["ok"] is not True:
			result["reason"] = "validation failed after receiver selection; not submitted"
			result["validation"] = check
			record("validation_failed", check["status"], b"", None)
			return [result]
		result["reason"] = "dispatch outcome unknown; inspect Case submission history before any retry"
		record("dispatch_intent", None, b"", None)
		deadline = time.monotonic() + args.ack_timeout
		status, body, transport = client.request("POST", f"/api/cases/{case_id}/submissions/{authority}", {
			"reason_for_change": "rich Case fuzzer transport verification",
			"e_signature": {"meaning": "execute submission", "password": args.password}},
			extra_headers={"X-Idempotency-Key": result["idempotency_key"]})
		record("dispatch_response", status, body, transport)
		if status not in {200, 201, 202} or transport:
			return [result]
		submission_id = None
		while True:
			value = json_value(body)
			data = value.get("data") if isinstance(value, dict) else None
			if not isinstance(data, dict) or data.get("case_id") != case_id or not data.get("id"):
				result["reason"] = "invalid submission response; no resend"
				break
			if submission_id is not None and data["id"] != submission_id:
				result["reason"] = "submission ID changed during polling; no resend"
				break
			try:
				submission_id = str(uuid.UUID(data["id"]))
			except (ValueError, TypeError, AttributeError):
				result["reason"] = "invalid submission ID; no resend"
				break
			result.update({"submission_id": submission_id, "submission_status": data.get("status"),
				"gateway": data.get("gateway"), "remote_submission_id": data.get("remote_submission_id")})
			if data.get("gateway") not in {"as2-submitter-http", f"as2-submitter-http-{authority}"}:
				result["reason"] = "submission gateway is not AS2"
				break
			acks = [data.get(f"ack{level}") for level in range(1, 5)]
			if data.get("status") == "rejected" or any(isinstance(ack, dict) and ack.get("success") is False for ack in acks):
				result["reason"] = "submission rejected or negative ACK"
				break
			ack1 = data.get("ack1")
			if data.get("status") in {"ack1_received", "ack2_received", "ack3_received", "ack4_received"} and isinstance(ack1, dict) and ack1.get("level") == 1 and ack1.get("success") is True and data.get("remote_submission_id"):
				result.update({"status": "PASS", "reason": "ACK1 confirmed; regulatory acceptance not verified", "ack1": ack1})
				break
			remaining = deadline - time.monotonic()
			if remaining <= 0:
				result["reason"] = "ACK1 timeout; retained for reconciliation, no resend"
				break
			record("pending_ack1", status, body, transport)
			time.sleep(min(2, remaining))
			status, body, transport = client.request("GET", f"/api/submissions/{submission_id}")
			if status != 200 or transport:
				result["reason"] = "submission polling failed; no resend"
				break
		record("outcome", status, body, transport)
	return [result]


def run(args: argparse.Namespace) -> int:
	guard_target(args.base_url, args.allow_remote)
	authorities = [item.strip().lower() for item in args.authorities.split(",") if item.strip()]
	unknown = set(authorities) - {"ich", "fda", "mfds"}
	if unknown or not authorities:
		raise SystemExit(f"invalid authorities: {', '.join(sorted(unknown)) or 'none'}")
	if args.submit:
		if args.rounds <= 0 or len(authorities) != len(set(authorities)):
			raise SystemExit("--submit requires positive rounds and unique authorities")
		if set(authorities) - {"fda", "mfds"}:
			raise SystemExit("--submit requires --authorities fda,mfds (or one of them)")
		if not args.sender_presave_id:
			raise SystemExit("--submit requires --sender-presave-id with configured authority gateways")
		uuid.UUID(args.sender_presave_id)
		for authority in authorities:
			if not getattr(args, f"{authority}_receiver"):
				raise SystemExit(f"--submit requires --{authority}-receiver")
		if not math.isfinite(args.ack_timeout) or args.ack_timeout <= 0:
			raise SystemExit("--ack-timeout must be finite and positive")
	output_dir = Path(args.artifact_dir)
	output_dir.mkdir(parents=True, exist_ok=True)
	client = ApiClient(args.base_url, args.timeout)
	results: list[dict[str, Any]] = []
	case_ids: list[str] = []
	status, body, transport = client.request("POST", "/auth/v1/login", {"email": args.email, "pwd": args.password})
	results.append({"kind": "login", "status": status, "response": summary(status, body, transport)})
	if status != 200:
		return write_results(args, results)
	product_id, sender_id, product_results = create_import_product(client, f"{args.seed}-{uuid.uuid4().hex[:8]}", args.sender_presave_id if args.submit else None)
	results.extend(product_results)
	if not product_id:
		results.append({"kind": "support_product", "status": "FAIL", "reason": "no product ID returned"})
		return write_results(args, results)
	for round_no in range(args.rounds):
		seed = args.seed + round_no
		source_id, setup = setup_rich_case(args, seed, output_dir)
		results.append({"kind": "rich_case_setup", "round": round_no, **setup})
		if not source_id:
			continue
		case_ids.append(source_id)
		if setup["status"] != "PASS":
			continue
		if sender_id:
			results.append(link_case_sender(client, source_id, sender_id))
		results.append(configure_case_authority_types(client, source_id))
		results.append(configure_case_identification(client, source_id))
		for authority in authorities:
			preparation_start = len(results)
			if authority == "mfds":
				results.extend(configure_mfds_ct_case(client, source_id, seed))
				results.append(configure_case_narrative(client, source_id))
			else:
				results.append(configure_case_identification(client, source_id))
			results.append(prepare_receiver(client, args, source_id, authority))
			if any(not isinstance(item["status"], int) or not 200 <= item["status"] < 300 for item in results[preparation_start:]):
				continue
			check = validation(client, source_id, authority)
			source_check = {"kind": "source_validation", "round": round_no, "authority": authority, "case_id": source_id, **check}
			results.append(source_check)
			if check["status"] != 200 or check["ok"] is not True:
				continue
			status, source_xml, export_summary = export_xml(client, source_id, authority)
			results.append({"kind": "source_export", "round": round_no, "authority": authority, "case_id": source_id, "status": status, "response": export_summary})
			if status != 200:
				continue
			authority_dir = output_dir / f"round-{seed}" / authority
			authority_dir.mkdir(parents=True, exist_ok=True)
			(source_path := authority_dir / "source.xml").write_bytes(source_xml)
			try:
				import_xml = unique_xml(source_xml, f"{seed}-{authority}-{uuid.uuid4().hex[:10]}")
			except (ValueError, UnicodeError) as error:
				results.append({"kind": "import_prepare", "round": round_no, "authority": authority, "status": "FAIL", "reason": type(error).__name__})
				continue
			(import_path := authority_dir / "import.xml").write_bytes(import_xml)
			payload, content_type = multipart(authority, product_id, f"rich-{authority}-{seed}.xml", import_xml)
			status, body, transport = client.request("POST", "/api/import/xml", payload, content_type)
			import_id = imported_case_id(body)
			if import_id:
				case_ids.append(import_id)
			results.append({"kind": "import", "round": round_no, "authority": authority, "status": status if import_id else "FAIL", "http_status": status, "case_id": import_id, "response": json_value(body)})
			if status != 200 or not import_id:
				continue
			preparation_start = len(results)
			if sender_id:
				results.append(link_case_sender(client, import_id, sender_id))
				results.append(configure_case_authority_types(client, import_id))
			results.append(prepare_receiver(client, args, import_id, authority))
			preparation_ok = all(isinstance(item["status"], int) and 200 <= item["status"] < 300 for item in results[preparation_start:])
			if not preparation_ok:
				continue
			import_check = validation(client, import_id, authority)
			results.append({"kind": "import_validation", "round": round_no, "authority": authority, "case_id": import_id, **import_check})
			status, exported, export_summary = export_xml(client, import_id, authority)
			failures = compare_xml(import_xml, exported) if status == 200 else [f"reexport:{status}"]
			results.append({"kind": "reexport_compare", "round": round_no, "authority": authority, "case_id": import_id, "status": "PASS" if status == 200 and not failures else "FAIL", "http_status": status, "failures": failures, "response": export_summary})
			if status == 200:
				(authority_dir / "reexport.xml").write_bytes(exported)
			if args.submit:
				if not preparation_ok or failures or import_check["status"] != 200 or import_check["ok"] is not True:
					results.append({"kind": "submission", "case_id": import_id, "authority": authority, "status": "FAIL", "reason": "validation or XML roundtrip failed; not submitted"})
				else:
					results.extend(submit_case(client, args, import_id, authority, authority_dir))
	if args.submit:
		completed = sum(item.get("kind") == "submission" for item in results)
		if completed != args.rounds * len(authorities):
			results.append({"kind": "submission_coverage", "status": "FAIL", "expected": args.rounds * len(authorities), "completed": completed})
	if not args.keep_cases and not args.submit:
		for case_id in sorted(set(case_ids)):
			results.append({"kind": "cleanup", **delete_case(client, case_id)})
	return write_results(args, results)


def write_results(args: argparse.Namespace, results: list[dict[str, Any]]) -> int:
	path = Path(args.artifact_dir) / f"rich-case-export-import-{args.seed}.jsonl"
	path.parent.mkdir(parents=True, exist_ok=True)
	with path.open("w", encoding="utf-8") as handle:
		for result in results:
			handle.write(json.dumps(result, ensure_ascii=True, sort_keys=True) + "\n")
	failed = [
		item for item in results
		if item.get("status") in {"FAIL", "error"}
		or item.get("kind") == "reexport_compare" and item.get("status") != "PASS"
		or isinstance(item.get("status"), int) and not 200 <= item["status"] < 300
		or item.get("status") is None
		or item.get("kind", "").endswith("validation") and item.get("ok") is not True
	]
	counts = {kind: sum(1 for item in results if item.get("kind") == kind) for kind in sorted({item.get("kind") for item in results})}
	print(json.dumps({"results": len(results), "failed": len(failed), "counts": counts, "artifact": str(path)}, ensure_ascii=True))
	return 1 if failed else 0


def parser() -> argparse.ArgumentParser:
	parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
	parser.add_argument("--base-url", default=os.getenv("E2BR3_BASE_URL", "http://127.0.0.1:8216"))
	parser.add_argument("--email", default=os.getenv("E2BR3_ADMIN_EMAIL", "demo.cro.admin@example.com"))
	parser.add_argument("--password", default=os.getenv("E2BR3_ADMIN_PASSWORD", "welcome"))
	parser.add_argument("--authorities", default="ich,fda,mfds")
	parser.add_argument("--null-flavor-rate", type=null_flavor_rate, default=0.0, help="probability per eligible baseline field (0..1); reproducible with --seed")
	parser.add_argument("--rounds", type=int, default=1)
	parser.add_argument("--seed", type=int, default=20260819)
	parser.add_argument("--timeout", type=float, default=30)
	parser.add_argument("--setup-timeout", type=float, default=600)
	parser.add_argument("--artifact-dir", default="tmp/rich-case-export-import")
	parser.add_argument("--contract", default=os.getenv("E2BR3_EDITOR_CONTRACT"))
	parser.add_argument("--null-flavor-pairs", default=os.getenv("E2BR3_NULL_FLAVOR_PAIRS"))
	parser.add_argument("--product-key", default=os.getenv("E2BR3_PRODUCT_KEY"))
	parser.add_argument("--meddra-version", default=os.getenv("E2BR3_MEDDRA_VERSION"))
	parser.add_argument("--meddra-code", default=os.getenv("E2BR3_MEDDRA_CODE"))
	parser.add_argument("--submit", action="store_true", help="submit imported FDA/MFDS Cases and wait for ACK1; retains all Cases")
	parser.add_argument("--sender-presave-id", help="existing sender with configured submission gateways; required for --submit")
	parser.add_argument("--fda-receiver", help="exact configured FDA receiver label")
	parser.add_argument("--mfds-receiver", help="exact configured MFDS receiver label")
	parser.add_argument("--ack-timeout", type=float, default=120, help="seconds to wait for ACK1; never resends")
	parser.add_argument("--keep-cases", action="store_true")
	parser.add_argument("--allow-remote", action="store_true")
	return parser


if __name__ == "__main__":
	try:
		sys.exit(run(parser().parse_args()))
	except KeyboardInterrupt:
		sys.exit(2)
