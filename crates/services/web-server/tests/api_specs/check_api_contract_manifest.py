#!/usr/bin/env python3
"""Generate/check the static API operation and Rust DTO source manifest."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import re
import sys
from collections import defaultdict
from pathlib import Path


HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
WEB = REPO / "crates/services/web-server"
OUT = HERE / "api-contract-manifest.json"
BASELINE_COMMIT = "18f9b0e6e1f54cde3b88fb6e7c4e0ab8f0949ed5"
PRIMITIVES = {
	"bool", "char", "f32", "f64", "i8", "i16", "i32", "i64", "i128",
	"isize", "str", "u8", "u16", "u32", "u64", "u128", "usize", "String",
	"Vec", "Option", "Box", "HashMap", "BTreeMap", "HashSet", "Cow", "Arc",
	"Pin", "PathBuf",
}
CONTRACT_WRAPPERS = {
	"Body", "HeaderMap", "Json", "Multipart", "Path", "Query", "Request",
	"Response", "Result", "StatusCode",
}
EXTERNAL_SIMPLE = {
	"Date": "time::Date", "DateTime": "chrono::DateTime",
	"Decimal": "rust_decimal::Decimal", "NaiveDate": "chrono::NaiveDate",
	"OffsetDateTime": "time::OffsetDateTime", "Uuid": "uuid::Uuid",
	"JsonValue": "serde_json::Value", "Value": "serde_json::Value",
}
EXTERNAL_ROOTS = {
	"axum", "chrono", "core", "http", "modql", "rust_decimal", "serde_json",
	"sqlx", "std", "time", "tower_cookies", "uuid",
}
IN_REPO_ROOTS = {
	"crate", "lib_auth", "lib_core", "lib_rest_core", "lib_utils", "lib_web",
	"validator", "xml",
}


def sha256(path: Path) -> str:
	return hashlib.sha256(path.read_bytes()).hexdigest()


def rel(path: Path) -> str:
	return path.resolve().relative_to(REPO).as_posix()


def line_at(text: str, offset: int) -> int:
	return text.count("\n", 0, offset) + 1


def source_ref(path: Path, text: str, offset: int) -> dict:
	return {"source": rel(path), "line": line_at(text, offset)}


def mask_rust(text: str) -> str:
	"""Blank comments and string contents while retaining offsets/newlines."""
	out = list(text)
	i = 0
	while i < len(text):
		if text.startswith("//", i):
			end = text.find("\n", i)
			end = len(text) if end < 0 else end
			for j in range(i, end): out[j] = " "
			i = end
		elif text.startswith("/*", i):
			depth, j = 1, i + 2
			while j < len(text) and depth:
				if text.startswith("/*", j): depth += 1; j += 2
				elif text.startswith("*/", j): depth -= 1; j += 2
				else: j += 1
			if depth: raise ValueError("unclosed block comment")
			for k in range(i, j):
				if out[k] != "\n": out[k] = " "
			i = j
		elif text[i] == '"':
			j, escaped = i + 1, False
			while j < len(text):
				if escaped: escaped = False
				elif text[j] == "\\": escaped = True
				elif text[j] == '"': j += 1; break
				j += 1
			for k in range(i, j):
				if out[k] != "\n": out[k] = " "
			i = j
		elif text[i] == "r" and i + 1 < len(text) and text[i + 1] in {'"', '#'}:
			m = re.match(r'r(#{0,16})"', text[i:])
			if not m: i += 1; continue
			closing = '"' + m.group(1)
			j = text.find(closing, i + len(m.group(0)))
			if j < 0: raise ValueError("unclosed raw string")
			j += len(closing)
			for k in range(i, j):
				if out[k] != "\n": out[k] = " "
			i = j
		else:
			i += 1
	return "".join(out)


def matching(mask: str, start: int) -> int:
	pairs = {"(": ")", "[": "]", "{": "}", "<": ">"}
	opener = mask[start]
	closer = pairs[opener]
	depth = 1
	for i in range(start + 1, len(mask)):
		if mask[i] == opener: depth += 1
		elif mask[i] == closer:
			depth -= 1
			if depth == 0: return i
	raise ValueError(f"unclosed {opener} at {start}")


def split_top(text: str, delimiter: str = ",") -> list[str]:
	mask = mask_rust(text)
	depth = 0
	start = 0
	parts = []
	for i, ch in enumerate(mask):
		if ch in "([{<": depth += 1
		elif ch in ")]}>": depth -= 1
		elif ch == delimiter and depth == 0:
			if text[start:i].strip(): parts.append(text[start:i].strip())
			start = i + 1
	if text[start:].strip(): parts.append(text[start:].strip())
	return parts


def normalize_rust(value: str) -> str:
	return re.sub(r"\s+", " ", value.strip()).replace(" ,", ",")


def calls(text: str, marker: str):
	mask = mask_rust(text)
	for match in re.finditer(re.escape(marker), mask):
		start = match.end()
		end = matching(mask, start - 1)
		yield match.start(), split_top(text[start:end])


def extract_routes(path: Path, prefix: str, text: str | None = None, line_offset: int = 0) -> list[dict]:
	text = path.read_text() if text is None else text
	records = []
	for marker in (".route(", "rest_collection_item_routes("):
		for offset, parts in calls(text, marker):
			pairs = [(parts[0], parts[1])] if marker == ".route(" else (
				[(parts[0], parts[2]), (parts[1], parts[3])] if len(parts) == 4 else []
			)
			for route_path, router in pairs:
				if not route_path.startswith('"'): continue
				route_path = json.loads(route_path)
				for m in re.finditer(
					r"(?<![A-Za-z0-9_])(get|post|put|patch|delete)\(\s*([A-Za-z0-9_:]+)",
					router,
				):
					records.append({
						"method": m.group(1).upper(), "path": prefix + route_path,
						"handler": m.group(2), "route": {
							**source_ref(path, text, offset),
							"line": line_at(text, offset) + line_offset,
						},
					})
	return records


def routes() -> list[dict]:
	records = []
	route_dir = WEB / "src/web/rest/routes"
	module_file = route_dir / "mod.rs"
	module_text = module_file.read_text()
	modules = set(re.findall(r"^pub mod ([A-Za-z_][A-Za-z0-9_]*);", module_text, re.M))
	module_sources = {path.stem: path for path in route_dir.glob("*.rs") if path.name != "mod.rs"}
	if modules != set(module_sources):
		raise ValueError(f"route module/file mismatch: modules={sorted(modules)}, files={sorted(module_sources)}")
	api_sources = [WEB / "src/web/rest/mod.rs", *(path for name, path in module_sources.items() if name != "submissions")]
	for path in api_sources:
		records.extend(extract_routes(path, "/api"))
	submissions = module_sources["submissions"]
	text = submissions.read_text()
	marker = "pub fn routes_submissions_internal"
	public, internal = text.split(marker, 1)
	records.extend(extract_routes(submissions, "/api", public))
	records.extend(extract_routes(
		submissions, "/internal", marker + internal,
		text[:text.index(marker)].count("\n"),
	))
	login_source = WEB / "src/web/routes_login.rs"
	records.extend(extract_routes(login_source, "/auth/v1"))
	root_source = WEB / "src/lib.rs"
	records.extend(extract_routes(root_source, ""))

	active_route_sources = [*api_sources, submissions]
	declared = set()
	for source in active_route_sources:
		declared.update(re.findall(r"\bpub fn (routes[A-Za-z0-9_]*)\s*\(", mask_rust(source.read_text())))
	composed = set()
	for source in (WEB / "src/web/routes_rest.rs", WEB / "src/web/routes_internal.rs"):
		composed.update(re.findall(r"\brest::(routes[A-Za-z0-9_]*)\s*\(", mask_rust(source.read_text())))
	if declared != composed:
		raise ValueError(f"route function composition mismatch: uncomposed={sorted(declared-composed)}, undeclared={sorted(composed-declared)}")
	for source in [*active_route_sources, login_source, root_source]:
		text_source = source.read_text(); masked = mask_rust(text_source)
		if source == root_source and re.search(r"\.(?:nest_service|route_service)\s*\(", masked):
			raise ValueError(f"unsupported root service route registration in {rel(source)}")
		if re.search(r"\.nest\s*\(", masked) and source == root_source:
			nests = set(re.findall(r"\.nest\(\s*\"([^\"]+)\"", text_source))
			if nests != {"/api", "/auth/v1", "/internal"}:
				raise ValueError(f"unsupported root nest set: {sorted(nests)}")
		elif re.search(r"\.(?:nest|nest_service|route_service)\s*\(", masked):
			raise ValueError(f"unsupported nested/service route registration in {rel(source)}")
		if source != root_source:
			expected_handlers = len(re.findall(r"(?<![A-Za-z0-9_])(get|post|put|patch|delete)\(\s*[A-Za-z0-9_:]+", masked))
			actual_handlers = sum(1 for record in records if record["route"]["source"] == rel(source))
			if expected_handlers != actual_handlers:
				raise ValueError(f"unparsed route handlers in {rel(source)}: source={expected_handlers}, extracted={actual_handlers}")
	keys = [(r["method"], r["path"]) for r in records]
	if len(keys) != len(set(keys)): raise ValueError("duplicate route method/path")
	protected = set((WEB / "tests/authz/protected_route_inventory.snapshot").read_text().splitlines())
	actual = {f'{r["method"]} {r["path"]}' for r in records if r["path"].startswith("/api/")}
	if actual != protected:
		raise ValueError(f"protected snapshot mismatch: missing={sorted(protected-actual)}, extra={sorted(actual-protected)}")
	return sorted(records, key=lambda r: (r["path"], r["method"]))


def production_rust_files() -> list[Path]:
	return sorted(path for path in (REPO / "crates").rglob("*.rs") if "src" in path.parts)


def function_signature(path: Path, text: str, mask: str, match: re.Match) -> dict:
	open_paren = mask.find("(", match.end() - 1)
	close_paren = matching(mask, open_paren)
	i = close_paren + 1
	while i < len(mask) and mask[i].isspace(): i += 1
	if mask.startswith("->", i):
		i += 2
		depth = 0
		while i < len(mask):
			ch = mask[i]
			if ch in "([<": depth += 1
			elif ch in ")]>": depth -= 1
			elif ch == "{" and depth == 0: break
			i += 1
	else:
		while i < len(mask) and mask[i] != "{": i += 1
	start = match.start()
	return {
		"kind": "direct", "declaration": source_ref(path, text, start),
		"signature": normalize_rust(text[start:i]),
	}


def function_index(files: list[Path]) -> dict[str, list[dict]]:
	index = defaultdict(list)
	pattern = re.compile(r"\b(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(")
	for path in files:
		text = path.read_text()
		mask = mask_rust(text)
		for match in pattern.finditer(mask):
			entry = function_signature(path, text, mask, match)
			entry["name"] = match.group(1)
			index[match.group(1)].append(entry)
	return index


def macro_definitions(files: list[Path]) -> tuple[dict[str, list[dict]], dict[Path, list[tuple[int, int]]]]:
	defs = defaultdict(list)
	ranges = defaultdict(list)
	for path in files:
		text = path.read_text(); mask = mask_rust(text)
		for m in re.finditer(r"\bmacro_rules!\s*([A-Za-z_][A-Za-z0-9_]*)\s*", mask):
			i = m.end()
			while i < len(mask) and mask[i].isspace(): i += 1
			if i >= len(mask) or mask[i] not in "{([": continue
			end = matching(mask, i)
			defs[m.group(1)].append({
				"name": m.group(1), "path": path, "text": text[i + 1:end],
				"mask": mask[i + 1:end], "offset": i + 1,
				"declaration": source_ref(path, text, m.start()),
			})
			ranges[path].append((m.start(), end + 1))
	return defs, ranges


def macro_invocations(files: list[Path], definition_ranges: dict) -> list[dict]:
	result = []
	pat = re.compile(r"(?<![A-Za-z0-9_$])((?:[A-Za-z_][A-Za-z0-9_]*::)*([A-Za-z_][A-Za-z0-9_]*))!\s*([({])")
	for path in files:
		text = path.read_text(); mask = mask_rust(text)
		for m in pat.finditer(mask):
			if any(a <= m.start() < b for a, b in definition_ranges.get(path, [])): continue
			start = m.end() - 1; end = matching(mask, start)
			result.append({
				"name": m.group(2), "qualified": m.group(1), "path": path,
				"body": text[start + 1:end], "offset": m.start(),
				"invocation": source_ref(path, text, m.start()),
			})
	return result


def named_macro_mapping(definition: dict, invocation: dict) -> dict[str, str]:
	mapping = {}
	label_vars = dict(re.findall(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*:\s*\$([A-Za-z_][A-Za-z0-9_]*)\s*:", definition["text"]))
	for part in split_top(invocation["body"]):
		m = re.match(r"([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(.*)\Z", part, re.S)
		if m and m.group(1) in label_vars:
			mapping[label_vars[m.group(1)]] = m.group(2).strip()
	return mapping


def positional_macro_mappings(definition: dict, invocation: dict) -> list[dict[str, str]]:
	text, mask = definition["text"], definition["mask"]
	value_parts = split_top(invocation["body"])
	candidates = []
	depth = 0
	for start, ch in enumerate(mask):
		if ch in "({[":
			if depth == 0:
				end = matching(mask, start)
				j = end + 1
				while j < len(mask) and mask[j].isspace(): j += 1
				if mask.startswith("=>", j):
					pattern_parts = split_top(text[start + 1:end])
					mapping = {}
					for pattern, value in zip(pattern_parts, value_parts):
						vars_ = re.findall(r"\$([A-Za-z_][A-Za-z0-9_]*)\s*:[A-Za-z_]+", pattern)
						if len(vars_) == 1: mapping[vars_[0]] = value.strip()
					candidates.append(mapping)
			depth += 1
		elif ch in ")}]": depth -= 1
	return candidates or [{}]


def macro_signature_template(definition: dict, variable: str) -> tuple[str, int] | None:
	text, mask = definition["text"], definition["mask"]
	pat = re.compile(r"\bpub(?:\([^)]*\))?\s+async\s+fn\s+\$" + re.escape(variable) + r"\s*\(")
	m = pat.search(mask)
	if not m: return None
	open_paren = mask.find("(", m.end() - 1); close = matching(mask, open_paren)
	i = close + 1
	while i < len(mask) and mask[i].isspace(): i += 1
	if mask.startswith("->", i):
		i += 2; depth = 0
		while i < len(mask):
			ch = mask[i]
			if ch in "([<": depth += 1
			elif ch in ")]>": depth -= 1
			elif ch == "{" and depth == 0: break
			i += 1
	else:
		while i < len(mask) and mask[i] != "{": i += 1
	return text[m.start():i], m.start()


def expand_signature(template: str, mapping: dict[str, str], macro_crate: str) -> str:
	value = template.replace("$crate", macro_crate)
	for name, replacement in sorted(mapping.items(), key=lambda item: -len(item[0])):
		value = re.sub(r"\$" + re.escape(name) + r"\b", replacement, value)
	if re.search(r"\$[A-Za-z_]", value):
		raise ValueError(f"unexpanded macro variable in signature: {normalize_rust(value)}")
	return normalize_rust(value)


def generated_case_handler(handler: str, invocations: list[dict], definitions: dict) -> dict | None:
	roles = {
		"create_": "create", "get_": "get", "list_": "list", "update_": "update", "delete_": "delete",
	}
	for prefix, role in roles.items():
		if not handler.startswith(prefix): continue
		for inv in invocations:
			if inv["name"] != "generate_case_rest_fns": continue
			definition = definitions[inv["name"]][0]
			mapping = named_macro_mapping(definition, inv)
			suffix = mapping.get("suffix")
			if not suffix: continue
			expected = f"list_{suffix}s" if role == "list" else f"{prefix}{suffix}"
			if expected != handler: continue
			entity = mapping["entity"]; create = mapping["for_create"]; update = mapping["for_update"]
			base = (
				"axum::extract::State(mm): axum::extract::State<ModelManager>, "
				"ctx_w: lib_web::middleware::mw_auth::CtxW, "
				"snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW, "
			)
			if role == "create":
				sig = f"pub async fn {handler}({base}axum::extract::Path(case_id): axum::extract::Path<Uuid>, axum::Json(params): axum::Json<ParamsForCreate<{create}>>) -> Result<(axum::http::StatusCode, axum::Json<DataRestResult<{entity}>>)>"
			elif role == "list":
				sig = f"pub async fn {handler}({base}axum::extract::Path(case_id): axum::extract::Path<Uuid>) -> Result<(axum::http::StatusCode, axum::Json<DataRestResult<Vec<{entity}>>>)>"
			elif role == "update":
				sig = f"pub async fn {handler}({base}axum::extract::Path((case_id, id)): axum::extract::Path<(Uuid, Uuid)>, axum::Json(params): axum::Json<ParamsForUpdate<{update}>>) -> Result<(axum::http::StatusCode, axum::Json<DataRestResult<{entity}>>)>"
			elif role == "delete":
				sig = f"pub async fn {handler}({base}axum::extract::Path((case_id, id)): axum::extract::Path<(Uuid, Uuid)>) -> Result<axum::http::StatusCode>"
			else:
				sig = f"pub async fn {handler}({base}axum::extract::Path((case_id, id)): axum::extract::Path<(Uuid, Uuid)>) -> Result<(axum::http::StatusCode, axum::Json<DataRestResult<{entity}>>)>"
			return {
				"kind": "macro", "signature": sig, "invocation": inv["invocation"],
				"macro": definition["declaration"], "macro_name": inv["name"],
			}
	return None


def resolve_handler(handler_path: str, direct: dict, invocations: list[dict], definitions: dict) -> dict:
	handler = handler_path.split("::")[-1]
	candidates = direct.get(handler, [])
	web_candidates = [c for c in candidates if c["declaration"]["source"].startswith("crates/services/web-server/src/")]
	if handler_path.startswith("handlers_login::"):
		web_candidates = [c for c in candidates if "/lib-web/src/handlers/handlers_login.rs" in c["declaration"]["source"]]
	if handler == "health":
		web_candidates = [c for c in candidates if c["declaration"]["source"] == "crates/services/web-server/src/lib.rs"]
	if len(web_candidates) == 1: return web_candidates[0]
	if len(web_candidates) > 1:
		raise ValueError(f"ambiguous direct handler {handler_path}: {web_candidates}")

	case_generated = generated_case_handler(handler, invocations, definitions)
	if case_generated: return case_generated

	matching_invocations = [
		inv for inv in invocations if re.search(r"\b" + re.escape(handler) + r"\b", mask_rust(inv["body"]))
	]
	resolved = []
	for inv in matching_invocations:
		for definition in definitions.get(inv["name"], []):
			named = named_macro_mapping(definition, inv)
			for positional in positional_macro_mappings(definition, inv):
				mapping = {**positional, **named}
				for variable, value in mapping.items():
					if normalize_rust(value) != handler: continue
					template = macro_signature_template(definition, variable)
					if not template: continue
					macro_crate = "lib_rest_core" if "/lib-rest-core/" in str(definition["path"]) else "crate"
					try:
						signature = expand_signature(template[0], mapping, macro_crate)
					except ValueError:
						continue
					resolved.append({
						"kind": "macro", "signature": signature,
						"invocation": inv["invocation"], "macro": definition["declaration"],
						"macro_name": inv["name"],
					})
	unique = {r["signature"] + json.dumps(r["invocation"], sort_keys=True): r for r in resolved}
	if len(unique) != 1:
		raise ValueError(f"unresolved/ambiguous macro handler {handler_path}: {list(unique.values())}")
	return next(iter(unique.values()))


def signature_parts(signature: str) -> tuple[list[dict], str]:
	mask = mask_rust(signature)
	open_paren = mask.find("("); close = matching(mask, open_paren)
	params = []
	for raw in split_top(signature[open_paren + 1:close]):
		piece_mask = mask_rust(raw); depth = 0; colon = -1
		for i, ch in enumerate(piece_mask):
			if ch in "([{<": depth += 1
			elif ch in ")]}>": depth -= 1
			elif ch == ":" and depth == 0 and not (
				(i + 1 < len(piece_mask) and piece_mask[i + 1] == ":") or
				(i > 0 and piece_mask[i - 1] == ":")
			):
				colon = i; break
		if colon >= 0:
			params.append({"pattern": normalize_rust(raw[:colon]), "type": normalize_rust(raw[colon + 1:])})
		else:
			params.append({"pattern": None, "type": normalize_rust(raw)})
	after = signature[close + 1:].strip()
	response = normalize_rust(after[2:]) if after.startswith("->") else "()"
	return params, response


def coverage() -> dict[tuple[str, str], dict]:
	path = HERE / "coverage.tsv"
	rows = {}
	with path.open(newline="") as handle:
		reader = csv.DictReader(handle, delimiter="\t")
		for line, row in enumerate(reader, 2):
			key = (row["method"], row["path"])
			if key in rows: raise ValueError(f"duplicate coverage row {key}")
			rows[key] = {**row, "source": {"source": rel(path), "line": line}}
	return rows


def check_spec_ref(value: str) -> None:
	file_name, anchor = value.split("#", 1)
	path = HERE / file_name
	if not path.is_file(): raise ValueError(f"missing spec file {file_name}")
	text = path.read_text()
	if f'id="{anchor}"' in text:
		return
	headings = []
	for line in text.splitlines():
		if not line.startswith("#"): continue
		heading = line.lstrip("#").strip().lower()
		slug = re.sub(r"[^\w\- ]", "", heading).replace(" ", "-")
		headings.append(slug)
	if anchor not in headings: raise ValueError(f"missing spec anchor {value}")


def declaration_index(files: list[Path]) -> dict[str, list[dict]]:
	index = defaultdict(list)
	generic_owners = defaultdict(list)
	pattern = re.compile(r"(?P<attrs>(?:\s*#\[[^\]]*\])*)\s*(?:pub(?:\([^)]*\))?\s+)?(?P<kind>struct|enum|type)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)")
	for path in files:
		text = path.read_text(); mask = mask_rust(text)
		for m in pattern.finditer(mask):
			kind, name = m.group("kind"), m.group("name")
			start = m.start("kind"); i = m.end()
			while i < len(mask) and mask[i].isspace(): i += 1
			# Skip generic parameters and where clauses until the declaration body/alias.
			depth = 0
			while i < len(mask):
				ch = mask[i]
				if ch == "<": depth += 1
				elif ch == ">": depth -= 1
				elif depth == 0 and ch in "{(;=": break
				i += 1
			if i >= len(mask): continue
			if mask[i] in "{(":
				end = matching(mask, i) + 1
				while end < len(mask) and mask[end].isspace(): end += 1
				if end < len(mask) and mask[end] == ";": end += 1
			elif mask[i] == "=":
				end = mask.find(";", i)
				if end < 0: continue
				end += 1
			else:
				end = i + 1
			attrs = re.findall(r"#\[[^\]]*\]", text[m.start("attrs"):m.start("kind")])
			entry = {
				"name": name, "kind": kind, "declaration": source_ref(path, text, start),
				"attributes": [normalize_rust(a) for a in attrs],
				"serde_attributes": [normalize_rust(a) for a in attrs if a.lstrip().startswith("#[serde")],
				"raw": text[start:end].strip(), "field_types": [],
			}
			header = text[m.end():i]
			if "<" in header:
				hmask = mask_rust(header); generic_start = hmask.find("<")
				generic_end = matching(hmask, generic_start)
				for parameter in split_top(header[generic_start + 1:generic_end]):
					pm = re.match(r"(?:const\s+)?([A-Za-z_][A-Za-z0-9_]*)", parameter.strip())
					if pm:
						entry.setdefault("generic_parameters", []).append(pm.group(1))
						generic_owners[(rel(path), pm.group(1))].append({
							"owner": name, **source_ref(path, text, start),
						})
			if kind == "struct" and mask[i] == "{":
				body = text[i + 1:matching(mask, i)]
				for field in split_top(body):
					fm = re.match(r"(?P<attrs>(?:\s*#\[[^\]]*\])*)\s*(?:pub(?:\([^)]*\))?\s+)?(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*:\s*(?P<type>.*)\Z", field, re.S)
					if not fm: continue
					fattrs = [normalize_rust(a) for a in re.findall(r"#\[[^\]]*\]", fm.group("attrs"))]
					entry.setdefault("fields", []).append({
						"name": fm.group("name"), "type": normalize_rust(fm.group("type")),
						"attributes": fattrs,
						"serde_attributes": [a for a in fattrs if a.startswith("#[serde")],
					})
					entry["field_types"].append(normalize_rust(fm.group("type")))
			elif kind == "struct" and mask[i] == "(":
				body = text[i + 1:matching(mask, i)]
				entry["tuple_fields"] = []
				for field in split_top(body):
					fattrs = [normalize_rust(a) for a in re.findall(r"#\[[^\]]*\]", field)]
					field_type = re.sub(r"^(?:\s*#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?", "", field).strip()
					entry["tuple_fields"].append({
						"type": normalize_rust(field_type), "attributes": fattrs,
						"serde_attributes": [a for a in fattrs if a.startswith("#[serde")],
					})
					entry["field_types"].append(normalize_rust(field_type))
			elif kind == "type":
				target = text[i + 1:end - 1].strip()
				entry["target"] = normalize_rust(target); entry["field_types"].append(entry["target"])
			elif kind == "enum" and mask[i] == "{":
				body = text[i + 1:matching(mask, i)]
				entry["variants"] = []
				for variant in split_top(body):
					normalized = normalize_rust(variant)
					entry["variants"].append(normalized)
					without_attrs = re.sub(r"^(?:\s*#\[[^\]]*\]\s*)*", "", variant)
					vm = re.match(r"[A-Za-z_][A-Za-z0-9_]*\s*(.*)\Z", without_attrs, re.S)
					if not vm: continue
					payload = vm.group(1).strip()
					if payload.startswith("(") and payload.endswith(")"):
						entry["field_types"].extend(normalize_rust(v) for v in split_top(payload[1:-1]))
					elif payload.startswith("{") and payload.endswith("}"):
						for field in split_top(payload[1:-1]):
							fm = re.match(r"(?:\s*#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?[A-Za-z_][A-Za-z0-9_]*\s*:\s*(.*)\Z", field, re.S)
							if fm: entry["field_types"].append(normalize_rust(fm.group(1)))
			index[name].append(entry)
	for (source, name), owners in generic_owners.items():
		index[name].append({
			"name": name, "kind": "generic_parameter",
			"declaration": {"source": source, "line": owners[0]["line"]},
			"owners": owners, "attributes": [], "serde_attributes": [],
			"raw": f"generic parameter {name} declared by {', '.join(owner['owner'] for owner in owners)}",
			"field_types": [],
		})
	# authorization_id! is the only reachable macro-generated DTO declaration.
	ids_path = REPO / "crates/libs/lib-core/src/authorization/ids.rs"
	ids_text = ids_path.read_text()
	for invocation in re.finditer(r"\bauthorization_id!\(([A-Za-z_][A-Za-z0-9_]*)\);", ids_text):
		name = invocation.group(1)
		index[name].append({
			"name": name, "kind": "struct",
			"declaration": source_ref(ids_path, ids_text, invocation.start()),
			"macro": {"source": rel(ids_path), "line": 28},
			"attributes": ["#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]", "#[serde(transparent)]"],
			"serde_attributes": ["#[serde(transparent)]"],
			"raw": f"authorization_id!({name}) => pub struct {name}(String);",
			"field_types": ["String"],
			"tuple_fields": [{"type": "String", "attributes": [], "serde_attributes": []}],
		})
	return index


def add_generated_dto_declarations(index: dict, definitions: dict, invocations: list[dict]) -> None:
	"""Materialize DTO structs emitted by the single-row presave macro."""
	macro_name = "generate_single_row_presave_rest_fns"
	definition = definitions[macro_name][0]
	for invocation in (item for item in invocations if item["name"] == macro_name):
		mapping = named_macro_mapping(definition, invocation)
		specs = (
			("rows", "#[derive(Debug, Serialize)]", [(mapping["row"], mapping["entity"])]),
			("details", "#[derive(Debug, Serialize)]", [("rows", mapping["rows"])]),
			("create_request", "#[derive(Deserialize)]", [("rows", mapping["create_rows"])]),
			("create_rows", "#[derive(Deserialize)]", [(mapping["row"], mapping["for_create"])]),
			("update_request", "#[derive(Deserialize)]", [("rows", mapping["update_rows"])]),
			("update_rows", "#[derive(Deserialize)]", [(mapping["row"], mapping["for_update"])]),
		)
		for variable, derive, fields in specs:
			name = mapping[variable]
			serde_attrs = [] if variable in {"rows", "details"} else ["#[serde(rename_all = \"camelCase\", deny_unknown_fields)]"]
			attributes = [derive, *serde_attrs]
			raw_fields = ", ".join(f"pub {field}: {field_type}" for field, field_type in fields)
			index[name].append({
				"name": name, "kind": "struct", "declaration": invocation["invocation"],
				"macro": definition["declaration"], "attributes": attributes,
				"serde_attributes": serde_attrs,
				"raw": f'{" ".join(attributes)} pub struct {name} {{ {raw_fields} }}',
				"fields": [{
					"name": field, "type": field_type, "attributes": [], "serde_attributes": [],
				} for field, field_type in fields],
				"field_types": [field_type for _, field_type in fields],
			})


def type_tokens(value: str) -> list[str]:
	value = re.sub(r"'[_A-Za-z][_A-Za-z0-9]*", "", value)
	return re.findall(r"\b[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*\b", value)


def expand_use_tree(tree: str, prefix: str = "") -> list[tuple[str, str]]:
	tree = tree.strip()
	mask = mask_rust(tree)
	depth = 0
	brace = -1
	for i, ch in enumerate(mask):
		if ch == "{" and depth == 0: brace = i; break
		if ch in "([<": depth += 1
		elif ch in ")]>": depth -= 1
	if brace >= 0:
		end = matching(mask, brace)
		base = tree[:brace].strip().removesuffix("::")
		combined = "::".join(part for part in (prefix, base) if part)
		result = []
		for item in split_top(tree[brace + 1:end]):
			result.extend(expand_use_tree(item, combined))
		return result
	if tree in {"self", "super", "crate"} or tree.endswith("::*"):
		return []
	match = re.match(r"(.+?)\s+as\s+([A-Za-z_][A-Za-z0-9_]*)\Z", tree)
	path, alias = (match.group(1).strip(), match.group(2)) if match else (tree, tree.split("::")[-1])
	full = "::".join(part for part in (prefix, path) if part)
	return [(alias, full)]


def import_index(files: list[Path]) -> dict[str, dict[str, dict]]:
	result = defaultdict(dict)
	for path in files:
		text = path.read_text(); mask = mask_rust(text)
		for match in re.finditer(r"\buse\s+", mask):
			i, depth = match.end(), 0
			while i < len(mask):
				ch = mask[i]
				if ch in "({[<": depth += 1
				elif ch in ")}]>": depth -= 1
				elif ch == ";" and depth == 0: break
				i += 1
			if i >= len(mask): continue
			for alias, full in expand_use_tree(text[match.end():i]):
				result[rel(path)][alias] = {
					"rust_type": full, "source": source_ref(path, text, match.start()),
				}
	return result


def resolve_type(name: str, context_source: str, declarations: dict, imports: dict) -> tuple[dict | None, dict | None]:
	simple = name.split("::")[-1]
	root = name.split("::")[0]
	if "::" in name and simple in CONTRACT_WRAPPERS:
		return None, {"rust_type": name, "origin": "framework_or_result_wrapper"}
	if "::" in name and root in EXTERNAL_ROOTS:
		return None, {"rust_type": name, "origin": "external"}
	candidates = declarations.get(simple, [])
	local = [c for c in candidates if c["declaration"]["source"] == context_source]
	if len(local) == 1: return local[0], None
	if "::" in name:
		parts = name.split("::")
		qualified = []
		for candidate in candidates:
			source = candidate["declaration"]["source"].replace("-", "_")
			if all(part in source for part in parts[1:-1]): qualified.append(candidate)
		if len(qualified) == 1: return qualified[0], None
	if simple in PRIMITIVES:
		return None, {"rust_type": name, "origin": "primitive"}
	if "::" not in name and simple in imports.get(context_source, {}):
		imported = imports[context_source][simple]
		import_root = imported["rust_type"].split("::")[0]
		if import_root in EXTERNAL_ROOTS:
			return None, {
				"rust_type": name, "origin": "external_import_alias",
				"canonical_type": imported["rust_type"], "import": imported["source"],
			}
		if import_root in IN_REPO_ROOTS:
			return resolve_type(imported["rust_type"], context_source, declarations, imports)
	if simple in CONTRACT_WRAPPERS and name.split("::")[0] not in IN_REPO_ROOTS:
		return None, {"rust_type": name, "origin": "framework_or_result_wrapper"}
	if len(candidates) == 1: return candidates[0], None
	if len(candidates) > 1:
		raise ValueError(f"ambiguous in-repo type {name} from {context_source}: {[c['declaration'] for c in candidates]}")
	if root in EXTERNAL_ROOTS:
		return None, {"rust_type": name, "origin": "external"}
	if "::" not in name and simple in EXTERNAL_SIMPLE:
		return None, {
			"rust_type": name, "origin": "external_prelude_type",
			"canonical_type": EXTERNAL_SIMPLE[simple],
		}
	if root in IN_REPO_ROOTS:
		raise ValueError(f"in-repo type has no parsed declaration: {name} from {context_source}")
	# Multiple declarations with identical raw wire shape are still distinct contracts.
	raise ValueError(f"unresolved type {name} from {context_source}; add an exact declaration/import rule")


def resolved_type_refs(value: str, context_source: str, declarations: dict, imports: dict) -> list[dict]:
	refs = []
	seen = set()
	for token in type_tokens(value):
		if token in seen: continue
		seen.add(token)
		declaration, terminal = resolve_type(token, context_source, declarations, imports)
		if declaration is None:
			refs.append(terminal)
		else:
			refs.append({
				"rust_type": token, "origin": "in_repo_declaration",
				"declaration_id": f'{declaration["declaration"]["source"]}:{declaration["declaration"]["line"]}:{declaration["name"]}',
			})
	return refs


def dto_graph(seed_values: list[tuple[str, str]], declarations: dict, imports: dict) -> tuple[list[dict], list[dict]]:
	queue = list(seed_values)
	seen_refs = set()
	selected = {}
	terminals = {}
	while queue:
		value, context = queue.pop(0)
		for token in type_tokens(value):
			key = (token, context)
			if key in seen_refs: continue
			seen_refs.add(key)
			decl, terminal = resolve_type(token, context, declarations, imports)
			if decl is None:
				terminal_key = json.dumps(terminal, sort_keys=True)
				terminals[terminal_key] = terminal
				continue
			decl_id = f'{decl["declaration"]["source"]}:{decl["declaration"]["line"]}:{decl["name"]}'
			if decl_id in selected: continue
			copy = {k: v for k, v in decl.items() if k != "field_types"}
			copy["id"] = decl_id
			copy["nested_type_refs"] = [
				ref
				for nested in decl["field_types"]
				for ref in resolved_type_refs(nested, decl["declaration"]["source"], declarations, imports)
			]
			selected[decl_id] = copy
			for nested in decl["field_types"]:
				queue.append((nested, decl["declaration"]["source"]))
	return [selected[k] for k in sorted(selected)], [terminals[k] for k in sorted(terminals)]


def shared_contracts() -> dict:
	return {
		"root-request-context": {
			"meaning": "All routes pass request stamping, cookie context resolution, and response mapping.",
			"refs": [{"source": "crates/services/web-server/src/lib.rs", "line": 88}, {"source": "crates/libs/lib-web/src/middleware/mw_res_map.rs", "line": 170}],
		},
		"api-auth-db-context": {
			"meaning": "/api routes require an authenticated DB context; handler extractors and authorization helpers add operation-specific checks.",
			"refs": [{"source": "crates/services/web-server/src/lib.rs", "line": 76}, {"source": "crates/libs/lib-web/src/middleware/mw_db_ctx.rs", "line": 8}],
		},
		"api-csrf-origin": {
			"meaning": "State-changing cookie-authenticated /api requests are subject to Origin validation.",
			"refs": [{"source": "crates/services/web-server/src/lib.rs", "line": 82}, {"source": "crates/services/web-server/src/lib.rs", "line": 100}],
		},
		"login-context": {
			"meaning": "Login/logoff/refresh share root context; refresh additionally requires the DB-context layer declared by the login router.",
			"refs": [{"source": "crates/services/web-server/src/web/routes_login.rs", "line": 8}],
		},
		"internal-handler-auth": {
			"meaning": "Internal routes have no user-auth route middleware; each handler's declared header/token checks are authoritative.",
			"refs": [{"source": "crates/services/web-server/src/web/routes_internal.rs", "line": 6}, {"source": "crates/services/web-server/src/web/rest/submission_rest.rs", "line": 512}],
		},
		"result-error-map": {
			"meaning": "Declared Result errors are converted by the shared response mapper; route specs remain authoritative for operation-specific errors.",
			"refs": [{"source": "crates/services/web-server/src/lib.rs", "line": 93}, {"source": "crates/libs/lib-web/src/middleware/mw_res_map.rs", "line": 170}],
		},
	}


def build_manifest() -> dict:
	files = production_rust_files()
	direct = function_index(files)
	definitions, definition_ranges = macro_definitions(files)
	invocations = macro_invocations(files, definition_ranges)
	imports = import_index(files)
	coverage_rows = coverage()
	operations = []
	seeds = []
	current_routes = routes()
	if set(coverage_rows) != {(r["method"], r["path"]) for r in current_routes}:
		raise ValueError("coverage.tsv method/path set differs from router set")
	for route in current_routes:
		key = (route["method"], route["path"])
		cov = coverage_rows[key]
		check_spec_ref(cov["spec"])
		handler = resolve_handler(route["handler"], direct, invocations, definitions)
		params, response = signature_parts(handler["signature"])
		macro_source = handler.get("macro", {}).get("source", "")
		context = (
			macro_source if macro_source.endswith("/section_presave_rest/shared.rs")
			else handler.get("declaration", handler.get("invocation"))["source"]
		)
		wire = [p for p in params if re.search(r"\b(Path|Query|Json|Multipart|HeaderMap|Request)\b", p["type"])]
		for param in wire: seeds.append((param["type"], context))
		seeds.append((response, context))
		refs = ["root-request-context", "result-error-map"]
		if route["path"].startswith("/api/"):
			refs.extend(["api-auth-db-context", "api-csrf-origin"])
		elif route["path"].startswith("/auth/v1/"): refs.append("login-context")
		elif route["path"].startswith("/internal/"): refs.append("internal-handler-auth")
		operations.append({
			**route, "coverage": cov, "handler_contract": handler,
			"parameters": params, "wire_request_parameters": wire,
			"declared_response": response, "shared_contract_refs": refs,
			"type_resolution_source": context,
			"dynamic_value_note": (
				"Json<Value> is a dynamic Rust wire value; field keys and branch semantics remain in the linked human spec and handler source, not an inferred schema."
				if any(re.search(r"\bValue\b", p["type"]) for p in wire) or re.search(r"\bValue\b", response)
				else None
			),
		})
	declarations = declaration_index(files)
	add_generated_dto_declarations(declarations, definitions, invocations)
	dtos, terminals = dto_graph(seeds, declarations, imports)
	for operation in operations:
		context = operation["type_resolution_source"]
		operation["wire_request_type_refs"] = [{
			"parameter": parameter,
			"refs": resolved_type_refs(parameter["type"], context, declarations, imports),
		} for parameter in operation["wire_request_parameters"]]
		operation["response_type_refs"] = resolved_type_refs(
			operation["declared_response"], context, declarations, imports,
		)
	shared = shared_contracts()
	referenced_paths = {
		*(op["route"]["source"] for op in operations),
		*(op["coverage"]["source"]["source"] for op in operations),
		*(op["handler_contract"].get("declaration", op["handler_contract"].get("invocation"))["source"] for op in operations),
		*(op["handler_contract"]["macro"]["source"] for op in operations if "macro" in op["handler_contract"]),
		*(dto["declaration"]["source"] for dto in dtos),
		*(ref["source"] for contract in shared.values() for ref in contract["refs"]),
		*(HERE / op["coverage"]["spec"].split("#", 1)[0] for op in operations),
		WEB / "tests/authz/protected_route_inventory.snapshot",
		WEB / "src/web/rest/routes/mod.rs",
		WEB / "src/web/routes_rest.rs",
		WEB / "src/web/routes_internal.rs",
		Path(__file__),
	}
	for operation in operations:
		for evidence in operation["coverage"]["evidence"].split(";"):
			if evidence != "-":
				referenced_paths.add(WEB / evidence.split("::", 1)[0])
	normalized_paths = {rel(p) if isinstance(p, Path) else p for p in referenced_paths}
	return {
		"schema_version": 1,
		"baseline_commit": BASELINE_COMMIT,
		"baseline_route_count": 436,
		"operation_count": len(operations),
		"runtime": "NOT_RUN",
		"scope": {
			"claim": "Static source inventory of explicit routes, declared handler wire types, reachable Rust DTO declarations, and links to human behavioral specifications/test evidence.",
			"non_claims": [
				"This is not an OpenAPI or inferred JSON schema.",
				"Option and other Rust wrappers do not imply JSON null, clear, omission, or default behavior.",
				"Source inventory does not prove runtime behavior, persistence, authorization branches, status codes, or test passage.",
				"Json<Value> and other dynamic values have no statically inferred field schema here; use each operation's linked human spec and handler source.",
			],
		},
		"content_hashes": {path: sha256(REPO / path) for path in sorted(normalized_paths)},
		"shared_contracts": shared,
		"operations": operations,
		"rust_types": dtos,
		"external_and_primitive_types": terminals,
	}


def self_check() -> None:
	fixture = 'fn f(Path((a, b)): Path<(Uuid, Uuid)>, Json(v): Json<Vec<String>>) -> Result<Json<Value>>'
	params, response = signature_parts(fixture)
	assert [p["type"] for p in params] == ["Path<(Uuid, Uuid)>", "Json<Vec<String>>"]
	assert response == "Result<Json<Value>>"
	qualified, qualified_response = signature_parts(
		"fn f(axum::extract::Path((case_id, row_id)): axum::extract::Path<(uuid::Uuid, uuid::Uuid)>, axum::Json(input): axum::Json<Input>) -> axum::Json<Output>"
	)
	assert qualified == [
		{"pattern": "axum::extract::Path((case_id, row_id))", "type": "axum::extract::Path<(uuid::Uuid, uuid::Uuid)>"},
		{"pattern": "axum::Json(input)", "type": "axum::Json<Input>"},
	]
	assert qualified_response == "axum::Json<Output>"
	assert split_top('A<B, C>, D<(E, F)>') == ['A<B, C>', 'D<(E, F)>']
	assert split_top('A<r#"x,y"#, /* ignored, comma */ B>, C') == ['A<r#"x,y"#, /* ignored, comma */ B>', 'C']
	masked = mask_rust('a /* { */ "}" { x }')
	assert matching(masked, masked.index("{")) == masked.rindex("}")
	assert expand_signature("pub async fn $name(Json<$body>) -> $result", {
		"name": "create", "body": "Input", "result": "Result<Json<Output>>",
	}, "crate") == "pub async fn create(Json<Input>) -> Result<Json<Output>>"
	try:
		resolve_type("DefinitelyMissing", "fixture.rs", {}, {})
	except ValueError as error:
		assert "unresolved type" in str(error)
	else:
		raise AssertionError("unknown unqualified types must fail closed")


def main() -> int:
	parser = argparse.ArgumentParser()
	parser.add_argument("--write", action="store_true", help="replace the checked-in manifest")
	args = parser.parse_args()
	self_check()
	manifest = build_manifest()
	encoded = json.dumps(manifest, indent=2, ensure_ascii=False, sort_keys=False) + "\n"
	if args.write:
		OUT.write_text(encoded)
		print(f"wrote {OUT}: {manifest['operation_count']} operations, {len(manifest['rust_types'])} Rust types")
		return 0
	if not OUT.exists():
		print(f"missing {OUT}; run {Path(__file__).name} --write", file=sys.stderr)
		return 1
	if OUT.read_text() != encoded:
		print(f"stale {OUT}; run {Path(__file__).name} --write and review the diff", file=sys.stderr)
		return 1
	print(f"OK: {manifest['operation_count']} operations, {len(manifest['rust_types'])} Rust types, runtime NOT_RUN")
	return 0


if __name__ == "__main__":
	raise SystemExit(main())
