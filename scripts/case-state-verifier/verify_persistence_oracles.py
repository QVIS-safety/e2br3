#!/usr/bin/env python3
"""Verify Case snapshots, blocked source deletion, and audit integrity."""

import hashlib
import http.cookiejar
import json
import os
import re
import subprocess
import uuid
import urllib.error
import urllib.request
from pathlib import Path


ROOT = Path(__file__).parent
BASE = os.environ["API_URL"]
DB = os.environ["STATE_VERIFIER_DB"]
seed = json.loads((ROOT / "seed-result.json").read_text())
source = (Path(__file__).resolve().parents[2] / "crates/services/web-server/src/bootstrap.rs").read_text()


def constant(name):
    match = re.search(rf'const {name}: &str = "([^"]+)";', source)
    if not match:
        raise RuntimeError(f"missing bootstrap value {name}")
    return match.group(1)


jar = http.cookiejar.CookieJar()
opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))


def request(method, path, data=None, expected=(200,)):
    req = urllib.request.Request(
        BASE + path,
        data=json.dumps(data).encode() if data is not None else None,
        headers={
            "Content-Type": "application/json",
            "x-e2br3-reason-for-change": "Presave import persistence oracle",
            "x-e2br3-change-category": "Edited Data",
        },
        method=method,
    )
    try:
        response = opener.open(req, timeout=30)
    except urllib.error.HTTPError as error:
        response = error
    raw = response.read()
    body = json.loads(raw) if raw else None
    if response.status not in expected:
        raise AssertionError(f"{method} {path}: {response.status} {body}")
    return response.status, body


def sql_json(sql):
    password = subprocess.run(
        ["docker", "exec", os.environ["POSTGRES_CONTAINER"], "printenv", "POSTGRES_PASSWORD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    env = {**os.environ, "PGPASSWORD": password}
    result = subprocess.run(
        [
            "psql", "-h", "127.0.0.1", "-U", "oracle_migration_admin",
            "-d", DB, "-X", "-A", "-t", "-c", sql,
        ],
        check=True,
        capture_output=True,
        text=True,
        env=env,
    ).stdout.strip()
    return json.loads(result)


case_id = seed["caseId"]
source_ids = [seed[k]["id"] for k in ["sender", "receiver", "reporter", "product", "study", "narrative"]]
snapshot_sql = f"""
SELECT jsonb_build_object(
  'sender', (SELECT to_jsonb(x) - ARRAY['updated_at','updated_by'] FROM sender_information x WHERE case_id='{case_id}'),
  'receiver', (SELECT to_jsonb(x) - ARRAY['updated_at','updated_by'] FROM receiver_information x WHERE case_id='{case_id}'),
  'reporters', (SELECT COALESCE(jsonb_agg(to_jsonb(x) - ARRAY['updated_at','updated_by'] ORDER BY sequence_number),'[]') FROM primary_sources x WHERE case_id='{case_id}' AND NOT deleted),
  'study', (SELECT to_jsonb(x) - ARRAY['updated_at','updated_by'] FROM study_information x WHERE case_id='{case_id}'),
  'drugs', (SELECT COALESCE(jsonb_agg(to_jsonb(x) - ARRAY['updated_at','updated_by'] ORDER BY sequence_number),'[]') FROM drug_information x WHERE case_id='{case_id}' AND NOT deleted),
  'narrative', (SELECT to_jsonb(x) - ARRAY['updated_at','updated_by'] FROM narrative_information x WHERE case_id='{case_id}')
);
"""
audit_sql = f"""
SELECT jsonb_build_object(
  'caseAudit', (SELECT count(*) FROM audit_logs WHERE record_id IN (
    SELECT id FROM sender_information WHERE case_id='{case_id}' UNION ALL
    SELECT id FROM receiver_information WHERE case_id='{case_id}' UNION ALL
    SELECT id FROM primary_sources WHERE case_id='{case_id}' UNION ALL
    SELECT id FROM study_information WHERE case_id='{case_id}' UNION ALL
    SELECT id FROM drug_information WHERE case_id='{case_id}' UNION ALL
    SELECT id FROM narrative_information WHERE case_id='{case_id}'
  )),
  'sourceAudit', (SELECT count(*) FROM audit_logs WHERE record_id = ANY(ARRAY[{','.join(repr(x) + '::uuid' for x in source_ids)}]))
);
"""

request("POST", "/auth/v1/login", {
    "email": constant("DEMO_CRO_ADMIN_EMAIL"),
    "pwd": constant("INITIAL_PASSWORD"),
})
before = sql_json(snapshot_sql)
audit_before = sql_json(audit_sql)

delete_results = []
for kind, plural in [
    ("sender", "senders"), ("receiver", "receivers"),
    ("reporter", "reporters"), ("product", "products"),
    ("study", "studies"), ("narrative", "narratives"),
]:
    status, body = request(
        "DELETE", f"/api/presaves/{plural}/{seed[kind]['id']}", expected=(409,)
    )
    delete_results.append({"kind": kind, "status": status, "code": body.get("error", {}).get("code")})

after = sql_json(snapshot_sql)
audit_after = sql_json(audit_sql)
if before != after:
    raise AssertionError("Case snapshot changed after rejected Presave deletion")
if audit_before != audit_after:
    raise AssertionError(f"audit changed after rejected deletion: {audit_before} -> {audit_after}")

mutation_results = []
mutation_marker = uuid.uuid4().hex[:8].upper()
for kind, plural, row_key, field, method, suffix in [
    ("sender", "senders", "sender", "organizationName", "PUT", "/details"),
    ("receiver", "receivers", "receiver", "organizationName", "PUT", "/details"),
    ("reporter", "reporters", "reporter", "reporterFamilyName", "PATCH", ""),
    ("product", "products", "product", "medicinalProduct", "PUT", "/details"),
    ("study", "studies", "study", "studyName", "PUT", "/details"),
    ("narrative", "narratives", "narrative", "caseNarrative", "PATCH", ""),
]:
    changed = f"SOURCE-MUTATED-{kind.upper()}-{mutation_marker}"
    status, body = request(
        method,
        f"/api/presaves/{plural}/{seed[kind]['id']}{suffix}",
        {"data": {"rows": {row_key: {field: changed}}}},
    )
    actual = body.get("data", {}).get("rows", {}).get(row_key, {}).get(field)
    if actual != changed:
        raise AssertionError(f"{kind} source mutation readback: {actual!r}")
    mutation_results.append({"kind": kind, "status": status, "field": field})

mutation_snapshot = sql_json(snapshot_sql)
mutation_audit = sql_json(audit_sql)
if mutation_snapshot != after:
    raise AssertionError("Case snapshot changed after source Presave mutation")
if mutation_audit["caseAudit"] != audit_after["caseAudit"]:
    raise AssertionError(f"Case audit changed after source mutation: {audit_after} -> {mutation_audit}")
if mutation_audit["sourceAudit"] != audit_after["sourceAudit"] + len(mutation_results):
    raise AssertionError(f"source mutations were not audited exactly once: {audit_after} -> {mutation_audit}")

_, trail = request("GET", f"/api/cases/{case_id}/audit-trail")
_, integrity = request("GET", "/api/audit-logs/verify-integrity")
result = {
    "completed": True,
    "caseSnapshotSha256": hashlib.sha256(json.dumps(before, sort_keys=True).encode()).hexdigest(),
    "auditCounts": mutation_audit,
    "caseAuditTrailEntries": len(trail.get("data", [])),
    "integrity": integrity,
    "deleteResults": delete_results,
    "sourceMutationResults": mutation_results,
}
(ROOT / "persistence-oracles.json").write_text(json.dumps(result, indent=2))
print(json.dumps(result, indent=2))
