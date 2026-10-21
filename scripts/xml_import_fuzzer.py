#!/usr/bin/env python3
"""Seeded black-box XML import fuzzer using only the backend HTTP API."""

from __future__ import annotations

import argparse
import base64
import collections
import fnmatch
import hashlib
import http.cookiejar
import io
import json
import os
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
import zipfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any
from xml.etree import ElementTree as ET

from rbac_rls_blackbox import SAFE_LOCAL_HOSTS, commit_sha, guard_target


PAGES = ("CI", "RP", "SD", "LR", "SI", "DM", "NR", "DH", "AE", "LB", "DG")
ET.register_namespace("", "urn:hl7-org:v3")
ET.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance")


@dataclass(frozen=True)
class Actor:
	label: str
	email: str
	password_env: str
	product_presave_id: str
	organization_id: str | None = None


@dataclass(frozen=True)
class Sample:
	name: str
	authority: str
	xml: bytes
	kind: str
	expected: str


class Client:
	def __init__(self, base_url: str, timeout: float) -> None:
		self.base_url = base_url.rstrip("/")
		self.timeout = timeout
		self.jar = http.cookiejar.CookieJar()
		self.opener = urllib.request.build_opener(
			urllib.request.HTTPCookieProcessor(self.jar)
		)
		self.requests = 0
		self.request_seconds = 0.0

	def request(
		self,
		method: str,
		path: str,
		body: bytes | dict[str, Any] | None = None,
		content_type: str | None = None,
	) -> tuple[int | None, bytes, str | None]:
		started = time.monotonic()
		data = body if isinstance(body, bytes) else json.dumps(body).encode() if body is not None else None
		headers = {"Accept": "application/json"}
		if data is not None:
			headers["Content-Type"] = content_type or "application/json"
		req = urllib.request.Request(self.base_url + path, data=data, headers=headers, method=method)
		try:
			with self.opener.open(req, timeout=self.timeout) as response:
				return response.status, response.read(), None
		except urllib.error.HTTPError as error:
			return error.code, error.read(), None
		except (OSError, urllib.error.URLError) as error:
			return None, b"", type(error).__name__
		finally:
			self.requests += 1
			self.request_seconds += time.monotonic() - started


def guard_environment(base_url: str, database_url: str | None) -> None:
	guard_target(base_url, False)
	base = urllib.parse.urlparse(base_url)
	if base.port in {None, 8080}:
		raise SystemExit("XML fuzzing requires an explicit non-8080 backend port")
	if not database_url:
		raise SystemExit("set --database-url to the isolated UI database")
	database = urllib.parse.urlparse(database_url)
	database_name = database.path.lstrip("/").split("/", 1)[0]
	if (
		database.scheme not in {"postgres", "postgresql"}
		or database.hostname not in SAFE_LOCAL_HOSTS
		or not database_name.startswith("e2br3_ui_")
		or database_name == "app_db"
	):
		raise SystemExit("refusing non-local or non-isolated database; expected e2br3_ui_<name>")


def local_name(tag: str) -> str:
	return tag.rsplit("}", 1)[-1]


def direct_child(element: ET.Element, name: str) -> ET.Element | None:
	return next((child for child in element if local_name(child.tag) == name), None)


def unique_xml(xml: bytes, token: str) -> bytes:
	root = ET.fromstring(xml)
	report_id = f"ZZ-FUZZ-{token}"[:100]
	porr = next((node for node in root.iter() if local_name(node.tag) == "PORR_IN049016UV"), None)
	if porr is None:
		raise ValueError("PORR_IN049016UV missing")
	message_id = direct_child(porr, "id")
	if message_id is not None:
		message_id.set("extension", report_id)
	investigation = next((node for node in root.iter() if local_name(node.tag) == "investigationEvent"), None)
	if investigation is None:
		raise ValueError("investigationEvent missing")
	case_id = direct_child(investigation, "id")
	if case_id is None:
		raise ValueError("C.1.1 id missing")
	case_id.set("extension", report_id)
	return ET.tostring(root, encoding="utf-8", xml_declaration=True)


def element_with_code(root: ET.Element, code: str) -> ET.Element | None:
	for node in root.iter():
		child = direct_child(node, "code")
		if child is not None and child.get("code") == code:
			return node
	return None


def base64_text(root: ET.Element) -> ET.Element | None:
	return next((node for node in root.iter() if local_name(node.tag) == "text" and node.get("representation") == "B64"), None)


def set_base64_text(node: ET.Element, value: str) -> None:
	children = list(node)
	if children:
		children[-1].tail = value
	else:
		node.text = value


def mutate(xml: bytes, kind: str) -> bytes | None:
	if kind == "malformed_xml":
		return xml[: max(1, len(xml) // 2)]
	if kind == "invalid_utf8":
		return b"\xff\xfe" + xml
	root = ET.fromstring(xml)
	if kind == "wrapper_mismatch":
		porr = next(node for node in root.iter() if local_name(node.tag) == "PORR_IN049016UV")
		message_id = direct_child(porr, "id")
		if message_id is None:
			return None
		message_id.set("extension", "ZZ-FUZZ-MISMATCH")
	elif kind == "c17_ni":
		node = element_with_code(root, "23")
		value = direct_child(node, "value") if node is not None else None
		if value is None:
			return None
		value.attrib.pop("value", None)
		value.set("nullFlavor", "NI")
	elif kind == "value_and_nullflavor":
		node = element_with_code(root, "23")
		value = direct_child(node, "value") if node is not None else None
		if value is None:
			return None
		value.set("value", value.get("value", "true"))
		value.set("nullFlavor", "NI")
	elif kind == "invalid_date":
		control = next((node for node in root.iter() if local_name(node.tag) == "controlActProcess"), None)
		value = direct_child(control, "effectiveTime") if control is not None else None
		if value is None:
			return None
		value.set("value", "not-a-date")
	elif kind.startswith("base64_"):
		text = base64_text(root)
		if text is None:
			return None
		payload = "".join("".join(text.itertext()).split())
		if kind == "base64_invalid_chars":
			set_base64_text(text, "%%%not-base64%%")
		elif kind == "base64_bad_padding":
			set_base64_text(text, "A")
		elif kind == "base64_whitespace":
			set_base64_text(text, " \n\t".join(payload))
		elif kind == "base64_empty":
			set_base64_text(text, "")
		else:
			raise ValueError(f"unknown mutation: {kind}")
	else:
		raise ValueError(f"unknown mutation: {kind}")
	return ET.tostring(root, encoding="utf-8", xml_declaration=True)


def structural_signature(xml: bytes) -> dict[str, int]:
	root = ET.fromstring(xml)
	counts = collections.Counter()
	for node in root.iter():
		code = direct_child(node, "code")
		if code is None:
			continue
		if local_name(node.tag) == "document" and code.get("code") == "1":
			counts["documents"] += 1
		elif local_name(node.tag) == "observation" and code.get("code") == "29":
			counts["reactions"] += 1
		elif local_name(node.tag) == "organizer" and code.get("code") == "4":
			counts["drugs"] += 1
		elif local_name(node.tag) == "organizer" and code.get("code") == "3":
			counts["tests"] += 1
	return {key: counts[key] for key in ("documents", "tests", "drugs", "reactions")}


def identity_signature(xml: bytes) -> tuple[str | None, str | None]:
	root = ET.fromstring(xml)
	investigation = next((node for node in root.iter() if local_name(node.tag) == "investigationEvent"), None)
	control = next((node for node in root.iter() if local_name(node.tag) == "controlActProcess"), None)
	case_id = direct_child(investigation, "id") if investigation is not None else None
	creation = direct_child(control, "effectiveTime") if control is not None else None
	raw_date = creation.get("value") if creation is not None else None
	normalized_date = "".join(character for character in raw_date or "" if character.isdigit())[:14] or None
	return (
		case_id.get("extension") if case_id is not None else None,
		normalized_date,
	)


def preservation_signature(xml: bytes) -> dict[str, collections.Counter[Any]]:
	document = ET.fromstring(xml)
	root = next((node for node in document.iter() if local_name(node.tag) == "investigationEvent"), document)
	null_flavors: collections.Counter[Any] = collections.Counter()
	codes: collections.Counter[Any] = collections.Counter()
	dates: collections.Counter[Any] = collections.Counter()
	attachments: collections.Counter[Any] = collections.Counter()
	for node in root.iter():
		name = local_name(node.tag)
		if null_flavor := node.get("nullFlavor"):
			null_flavors[null_flavor] += 1
		if code := node.get("code"):
			codes[(name, code, node.get("codeSystem"), node.get("codeSystemVersion"))] += 1
		if name in {"birthTime", "availabilityTime", "low", "high", "center", "width"} and (value := node.get("value")):
			dates[(name, value, node.get("unit"))] += 1
		if name == "text" and node.get("representation") == "B64":
			payload = "".join("".join(node.itertext()).split())
			try:
				decoded = base64.b64decode(payload, validate=True)
			except ValueError:
				continue
			attachments[(node.get("mediaType"), hashlib.sha256(decoded).hexdigest())] += 1
	fields: collections.Counter[Any] = collections.Counter()
	def visit(node: ET.Element, path: tuple) -> None:
		text = (node.text or "").strip()
		attributes = tuple(sorted((key, value) for key, value in node.attrib.items()
			if key in {"value", "unit", "nullFlavor", "code", "codeSystem", "codeSystemVersion", "extension", "language"}))
		if text or attributes:
			fields[(path, text, attributes)] += 1
		positions: collections.Counter[str] = collections.Counter()
		for child in node:
			positions[child.tag] += 1
			visit(child, path + ((child.tag, positions[child.tag]),))
	visit(root, ((root.tag, 1),))
	return {"null_flavors": null_flavors, "codes": codes, "dates": dates, "attachments": attachments, "fields": fields}


def multipart(authority: str, product_id: str, filename: str, payload: bytes) -> tuple[bytes, str]:
	boundary = f"XML-FUZZ-{uuid.uuid4().hex}"
	parts = [
		f'--{boundary}\r\nContent-Disposition: form-data; name="format"\r\n\r\n{authority}\r\n',
		f'--{boundary}\r\nContent-Disposition: form-data; name="productPresaveId"\r\n\r\n{product_id}\r\n',
		f'--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="{filename}"\r\nContent-Type: application/zip\r\n\r\n',
	]
	body = "".join(parts).encode() + payload + f"\r\n--{boundary}--\r\n".encode()
	return body, f"multipart/form-data; boundary={boundary}"


def zip_samples(samples: list[Sample]) -> bytes:
	buffer = io.BytesIO()
	with zipfile.ZipFile(buffer, "w", zipfile.ZIP_DEFLATED) as archive:
		for sample in samples:
			archive.writestr(sample.name, sample.xml)
	return buffer.getvalue()


def zip_entries(entries: list[tuple[str, bytes]]) -> bytes:
	buffer = io.BytesIO()
	with zipfile.ZipFile(buffer, "w", zipfile.ZIP_DEFLATED) as archive:
		for name, value in entries:
			archive.writestr(name, value)
	return buffer.getvalue()


def json_value(body: bytes) -> Any:
	try:
		return json.loads(body)
	except (UnicodeDecodeError, json.JSONDecodeError):
		return None


def imported_rows(body: bytes) -> list[dict[str, Any]]:
	value = json_value(body)
	data = value.get("data", {}) if isinstance(value, dict) else {}
	rows = data.get("importedCases", data.get("imported_cases", [])) if isinstance(data, dict) else []
	return [row for row in rows if isinstance(row, dict)]


def reconcile_batch(
	samples: list[Sample],
	rows: list[dict[str, Any]],
) -> tuple[list[tuple[Sample, dict[str, Any]]], list[Sample], list[dict[str, Any]]]:
	pending = {sample.name: sample for sample in samples}
	if len(pending) != len(samples):
		raise ValueError("duplicate source file name in import batch")
	matched: list[tuple[Sample, dict[str, Any]]] = []
	unexpected: list[dict[str, Any]] = []
	for row in rows:
		name = row.get("sourceFileName") or row.get("source_file_name")
		sample = pending.pop(name, None)
		if sample is None:
			unexpected.append(row)
		else:
			matched.append((sample, row))
	return matched, list(pending.values()), unexpected


def archive_probe_status(
	status: int | None,
	transport: str | None,
	rows: list[dict[str, Any]],
) -> str:
	if transport:
		return "request_error"
	if status is not None and status >= 400:
		return "error"
	if rows and all(row.get("status") == "error" for row in rows):
		return "error"
	return "success" if rows else "request_error"


def classify_error(message: str | None, http_status: int | None) -> str:
	text = (message or "").lower()
	if http_status is None:
		return "transport"
	if http_status >= 500 or "panic" in text or "pgdatabaseerror" in text or "sqlx(" in text:
		return "server_or_raw_db"
	for label, needles in (
		("xml_syntax", ("xml parse", "not valid utf-8", "invalid xml")),
		("base64", ("base64", "includeddocument")),
		("input_contract", ("allowed.value", "length.max", "expected format")),
		("duplicate", ("same c.1.1", "import skipped")),
		("database", ("constraint", "database")),
	):
		if any(needle in text for needle in needles):
			return label
	return "other"


def load_config(path: Path) -> tuple[str, list[Actor], dict[str, list[Any]]]:
	config = json.loads(path.read_text())
	actors = [Actor(**item) for item in config["actors"]]
	corpora = {
		authority.lower(): values
		for authority, values in config["corpora"].items()
	}
	return config.get("base_url", "http://localhost:8080"), actors, corpora


def corpus_documents(sources: list[Any]) -> list[tuple[str, bytes, str]]:
	documents: list[tuple[str, bytes, str]] = []
	for source in sources:
		config = {"path": source} if isinstance(source, str) else source
		expected = config.get("expected", "success")
		location = config.get("path") or config.get("url")
		if not location:
			raise ValueError("corpus source requires path or url")
		if config.get("url"):
			with urllib.request.urlopen(location, timeout=120) as response:
				data = response.read()
			path = Path(urllib.parse.urlparse(location).path)
		else:
			path = Path(location)
			if path.is_dir():
				for xml_path in sorted(path.glob("*.xml")):
					documents.append((xml_path.stem, xml_path.read_bytes(), expected))
				continue
			data = path.read_bytes()
		if path.suffix.lower() != ".zip":
			documents.append((path.stem, data, expected))
			continue
		patterns = config.get("members", ["*.xml"])
		with zipfile.ZipFile(io.BytesIO(data)) as archive:
			for member in sorted(archive.namelist()):
				if not member.endswith("/") and any(fnmatch.fnmatch(member, pattern) for pattern in patterns):
					documents.append((Path(member).stem, archive.read(member), expected))
	return documents


def login(client: Client, actor: Actor) -> tuple[int | None, str | None]:
	password = os.environ.get(actor.password_env)
	if password is None:
		return None, f"missing environment variable {actor.password_env}"
	payload: dict[str, Any] = {"email": actor.email, "pwd": password}
	if actor.organization_id:
		payload["organizationId"] = actor.organization_id
	status, body, transport = client.request("POST", "/auth/v1/login", payload)
	return status, transport or (None if status == 200 else body.decode(errors="replace")[:300])


def verify_roundtrip(client: Client, case_id: str, authority: str, source: bytes) -> list[str]:
	failures: list[str] = []
	status, _, transport = client.request("GET", f"/api/cases/{case_id}")
	if status != 200:
		failures.append(f"case_get:{status or transport}")
	for page in PAGES:
		status, _, transport = client.request(
			"GET", f"/api/cases/{case_id}/editor/pages/{page}?authorities={authority}"
		)
		if status != 200:
			failures.append(f"page_{page}:{status or transport}")
	status, exported, transport = client.request("GET", f"/api/cases/{case_id}/export/xml?authority={authority}")
	if status != 200:
		value = json_value(exported)
		detail = value.get("error", {}).get("data", {}).get("detail") if isinstance(value, dict) else None
		failures.append(f"export:{status or transport}:{str(detail)[:500] if detail else ''}")
	else:
		try:
			expected_identity = identity_signature(source)
			actual_identity = identity_signature(exported)
			if actual_identity != expected_identity:
				failures.append(f"roundtrip_identity:{expected_identity}->{actual_identity}")
			expected = structural_signature(source)
			actual = structural_signature(exported)
			for key, count in expected.items():
				if count and actual[key] < count:
					failures.append(f"roundtrip_{key}:{count}->{actual[key]}")
			expected_preservation = preservation_signature(source)
			actual_preservation = preservation_signature(exported)
			for key, expected_values in expected_preservation.items():
				missing = expected_values - actual_preservation[key]
				if missing:
					failures.append(f"roundtrip_{key}:{list(missing.items())[:5]}")
		except ET.ParseError as error:
			failures.append(f"export_xml:{error}")
	return failures


def batches(items: list[Sample], size: int) -> list[list[Sample]]:
	return [items[index:index + size] for index in range(0, len(items), size)]


def run(args: argparse.Namespace) -> int:
	started = time.monotonic()
	base_url, actors, corpora = load_config(args.config)
	guard_environment(base_url, args.database_url)
	if not actors:
		raise ValueError("at least one actor is required")
	clients = {actor.label: Client(base_url, args.timeout) for actor in actors}
	results: list[dict[str, Any]] = []
	created_cases: dict[str, set[str]] = collections.defaultdict(set)
	cleanup_report_ids: dict[str, set[str]] = collections.defaultdict(set)
	for actor in actors:
		status, error = login(clients[actor.label], actor)
		if status != 200:
			raise RuntimeError(f"login failed for {actor.label}: {status} {error}")

	primary = actors[0]
	all_samples: dict[str, list[Sample]] = collections.defaultdict(list)
	for authority, sources in corpora.items():
		for source_name, raw, source_expected in corpus_documents(sources):
			for copy in range(args.copies):
				token = hashlib.sha256(f"{args.seed}:{authority}:{source_name}:{copy}".encode()).hexdigest()[:20].upper()
				try:
					healthy = unique_xml(raw, token)
				except (ET.ParseError, ValueError) as error:
					results.append({"actor": primary.label, "authority": authority, "file": source_name, "kind": "corpus_setup", "status": "error", "message": str(error)})
					continue
				base = Sample(f"healthy-{source_name}-{copy}.xml", authority, healthy, "healthy", source_expected)
				all_samples[authority].append(base)
				if copy < args.mutations_per_seed:
					for kind, mutation_expected in (
						("malformed_xml", "error"), ("invalid_utf8", "error"),
						("base64_invalid_chars", "error"), ("base64_bad_padding", "error"),
						("base64_whitespace", "success"), ("base64_empty", "success"),
						("value_and_nullflavor", "error"),
						("invalid_date", "error"), ("wrapper_mismatch", "success"),
						("c17_ni", "success"),
					):
						mutation_token = hashlib.sha256(
							f"{args.seed}:{authority}:{source_name}:{copy}:{kind}".encode()
						).hexdigest()[:20].upper()
						changed = mutate(unique_xml(raw, mutation_token), kind)
						if changed is not None:
							all_samples[authority].append(Sample(f"mut-{kind}-{source_name}-{copy}.xml", authority, changed, kind, mutation_expected))

	for authority, samples in all_samples.items():
		for batch in batches(samples, args.batch_size):
			payload, content_type = multipart(authority, primary.product_presave_id, f"xml-fuzz-{authority}.zip", zip_samples(batch))
			status, body, transport = clients[primary.label].request("POST", "/api/import/xml", payload, content_type)
			rows = imported_rows(body)
			if not rows:
				for sample in batch:
					try:
						if report_id := identity_signature(sample.xml)[0]:
							cleanup_report_ids[primary.label].add(report_id)
					except ET.ParseError:
						pass
					results.append({"actor": primary.label, "authority": authority, "file": sample.name, "kind": sample.kind, "expected": sample.expected, "status": "request_error", "http": status, "category": classify_error(transport or body.decode(errors="replace")[:500], status)})
				continue
			matched, missing, unexpected = reconcile_batch(batch, rows)
			for row in unexpected:
				results.append({"actor": primary.label, "authority": authority, "file": row.get("sourceFileName") or row.get("source_file_name"), "kind": "unexpected_response_row", "expected": "protocol", "status": "error", "category": "protocol", "message": "response row did not match an uploaded file"})
			for sample in missing:
				try:
					if report_id := identity_signature(sample.xml)[0]:
						cleanup_report_ids[primary.label].add(report_id)
				except ET.ParseError:
					pass
				results.append({"actor": primary.label, "authority": authority, "file": sample.name, "kind": "missing_response_row", "expected": sample.expected, "status": "missing", "category": "protocol", "message": "uploaded file missing from import response"})
			for sample, row in matched:
				name = sample.name
				row_status = row.get("status")
				message = row.get("message")
				case_id = row.get("caseId") or row.get("case_id")
				if case_id:
					created_cases[primary.label].add(case_id)
				else:
					try:
						if report_id := identity_signature(sample.xml)[0]:
							cleanup_report_ids[primary.label].add(report_id)
					except ET.ParseError:
						pass
				roundtrip = verify_roundtrip(clients[primary.label], case_id, authority, sample.xml) if row_status in {"success", "warning"} and case_id and (args.mode == "full" or sample.kind == "healthy") else []
				results.append({"actor": primary.label, "authority": authority, "file": name, "kind": sample.kind, "expected": sample.expected, "status": row_status, "case_id": case_id, "category": classify_error(message, status), "message": message, "roundtrip_failures": roundtrip})

	if args.archive_probes:
		probe_authority = next((key for key, value in all_samples.items() if value), None)
		if probe_authority:
			probe_xml = unique_xml(next(sample.xml for sample in all_samples[probe_authority] if sample.kind == "healthy"), hashlib.sha256(f"{args.seed}:archive".encode()).hexdigest()[:20].upper())
			nested = zip_entries([("inner.xml", probe_xml)])
			archive_probes = (
				("zip_empty", zip_entries([]), "empty.zip", "error"),
				("zip_nested", zip_entries([("nested.zip", nested)]), "nested.zip", "error"),
				("zip_path_name", zip_entries([("../probe.xml", probe_xml)]), "path.zip", "error"),
				("zip_duplicate_name", zip_entries([("one/same.xml", probe_xml), ("two/same.xml", probe_xml)]), "duplicate.zip", "error"),
				("zip_entry_oversize", zip_entries([("large.xml", b" " * (25 * 1024 * 1024 + 1))]), "large.zip", "error"),
				("upload_oversize", b" " * (50 * 1024 * 1024 + 1), "large.xml", "error"),
			)
			for kind, archive, filename, expected in archive_probes:
				payload, content_type = multipart(probe_authority, primary.product_presave_id, filename, archive)
				status, body, transport = clients[primary.label].request("POST", "/api/import/xml", payload, content_type)
				rows = imported_rows(body)
				for row in rows:
					if case_id := row.get("caseId") or row.get("case_id"):
						created_cases[primary.label].add(case_id)
				row_statuses = [row.get("status") for row in rows]
				result_status = archive_probe_status(status, transport, rows)
				results.append({"actor": primary.label, "authority": probe_authority, "file": filename, "kind": kind, "expected": expected, "status": result_status, "http": status, "category": classify_error(transport or body.decode(errors="replace")[:500], status), "message": transport or body.decode(errors="replace")[:500], "row_statuses": row_statuses})

	# Same C.1.1/version must be accepted independently by every configured organization.
	if len(actors) > 1:
		probe_authority = next((key for key, value in all_samples.items() if value), None)
		if probe_authority:
			base_probe = next(sample for sample in all_samples[probe_authority] if sample.kind == "healthy")
			probe = Sample(
				"multi-org-probe.xml",
				probe_authority,
				unique_xml(base_probe.xml, hashlib.sha256(f"{args.seed}:multi-org".encode()).hexdigest()[:20].upper()),
				"multi_org_same_identifier",
				"success",
			)
			for actor in actors:
				payload, content_type = multipart(probe_authority, actor.product_presave_id, f"multi-org-{probe.name}.zip", zip_samples([probe]))
				status, body, transport = clients[actor.label].request("POST", "/api/import/xml", payload, content_type)
				row = (imported_rows(body) or [{}])[0]
				row_status = row.get("status", "request_error")
				case_id = row.get("caseId") or row.get("case_id")
				if case_id:
					created_cases[actor.label].add(case_id)
				else:
					cleanup_report_ids[actor.label].add(identity_signature(probe.xml)[0] or "")
				roundtrip = verify_roundtrip(clients[actor.label], case_id, probe_authority, probe.xml) if row_status in {"success", "warning"} and case_id else []
				results.append({"actor": actor.label, "authority": probe_authority, "file": probe.name, "kind": "multi_org_same_identifier", "expected": "success", "status": row_status, "case_id": case_id, "http": status, "message": row.get("message") or transport, "roundtrip_failures": roundtrip})

	if not args.keep_cases:
		for actor in actors:
			for report_id in sorted(cleanup_report_ids[actor.label] - {""}):
				query = urllib.parse.urlencode({"filters[safety_report_id][$eq]": report_id})
				status, body, transport = clients[actor.label].request("GET", f"/api/cases?{query}")
				value = json_value(body)
				rows = value.get("data", []) if isinstance(value, dict) else []
				if status == 200 and isinstance(rows, list):
					created_cases[actor.label].update(
						row["id"] for row in rows
						if isinstance(row, dict) and isinstance(row.get("id"), str)
					)
				else:
					results.append({"actor": actor.label, "authority": None, "file": None, "kind": "cleanup_lookup", "expected": "success", "status": "error", "http": status, "category": classify_error(transport, status), "message": transport or "case lookup failed"})
			for case_id in sorted(created_cases[actor.label]):
				status, body, transport = clients[actor.label].request(
					"DELETE",
					f"/api/cases/{case_id}",
					{"reason_for_change": "XML import fuzzer cleanup"},
				)
				results.append({
					"actor": actor.label,
					"authority": None,
					"file": None,
					"kind": "cleanup",
					"expected": "success",
					"status": "success" if status == 200 else "error",
					"case_id": case_id,
					"http": status,
					"category": classify_error(transport or body.decode(errors="replace")[:500], status),
					"message": transport or (None if status == 200 else body.decode(errors="replace")[:500]),
				})

	args.output.parent.mkdir(parents=True, exist_ok=True)
	sha = commit_sha()
	request_count = sum(client.requests for client in clients.values())
	request_seconds = sum(client.request_seconds for client in clients.values())
	run_summary = {
		"kind": "run",
		"seed": args.seed,
		"commit": sha,
		"mode": args.mode,
		"copies": args.copies,
		"mutations_per_seed": args.mutations_per_seed,
		"batch_size": args.batch_size,
		"archive_probes": args.archive_probes,
		"keep_cases": args.keep_cases,
		"results": len(results),
		"requests": request_count,
		"request_seconds": round(request_seconds, 3),
		"elapsed_seconds": round(time.monotonic() - started, 3),
	}
	with args.output.open("w") as handle:
		for result in results:
			handle.write(json.dumps({"seed": args.seed, "commit": sha, **result}, ensure_ascii=False) + "\n")
		handle.write(json.dumps(run_summary, ensure_ascii=False) + "\n")
	summary = collections.Counter((item.get("kind"), item.get("status")) for item in results)
	errors = [item for item in results if item.get("category") == "server_or_raw_db" or item.get("roundtrip_failures") or item.get("kind") in {"missing_response_row", "unexpected_response_row"} or (item.get("expected") == "success" and item.get("status") not in {"success", "warning"}) or (item.get("expected") == "error" and item.get("status") != "error")]
	print(json.dumps({"total": len(results), "failures": len(errors), "requests": request_count, "elapsed_seconds": run_summary["elapsed_seconds"], "by_kind_status": {f"{kind}:{status}": count for (kind, status), count in sorted(summary.items())}, "output": str(args.output)}, ensure_ascii=False, indent=2))
	return 1 if errors else 0


def main() -> int:
	parser = argparse.ArgumentParser()
	parser.add_argument("--config", type=Path, required=True)
	parser.add_argument("--mode", choices=("smoke", "full"), default="smoke")
	parser.add_argument("--copies", type=int)
	parser.add_argument("--mutations-per-seed", type=int, default=1)
	parser.add_argument("--batch-size", type=int, default=10)
	parser.add_argument("--seed", type=int, default=20260812)
	parser.add_argument("--timeout", type=float, default=300)
	parser.add_argument("--archive-probes", action=argparse.BooleanOptionalAction)
	parser.add_argument("--database-url", default=os.getenv("SERVICE_DB_URL"))
	parser.add_argument("--keep-cases", action="store_true")
	parser.add_argument("--output", type=Path, default=Path("tmp/xml-import-fuzz/results.jsonl"))
	args = parser.parse_args()
	if args.copies is None:
		args.copies = 1 if args.mode == "smoke" else 10
	if args.archive_probes is None:
		args.archive_probes = args.mode == "full"
	if args.copies < 1 or args.batch_size < 1 or args.mutations_per_seed < 0:
		parser.error("copies/batch-size must be positive and mutations-per-seed non-negative")
	return run(args)


if __name__ == "__main__":
	sys.exit(main())
