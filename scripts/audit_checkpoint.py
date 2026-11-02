#!/usr/bin/env python3
"""Publish independently retained audit-chain checkpoints to S3 Object Lock."""

import argparse
import datetime as dt
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
import uuid
import urllib.parse


def fail(message):
    raise RuntimeError(message)


def run_json(command, env=None):
    try:
        result = subprocess.run(command, env=env, text=True, capture_output=True)
    except OSError as error:
        fail(f"cannot execute {command[0]}: {error}")
    if result.returncode:
        fail(f"command failed ({command[0]} {command[1]}): {result.stderr.strip()}")
    try:
        value = json.loads(result.stdout)
        if not isinstance(value, dict):
            fail(f"unexpected JSON type from {command[0]} {command[1]}")
        return value
    except json.JSONDecodeError as error:
        fail(f"invalid JSON from {command[0]} {command[1]}: {error}")


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def validate_baseline(value):
    if (not isinstance(value, dict) or set(value) != {"sha256", "count", "first_id", "last_id", "cutoff_id"}
            or not isinstance(value["sha256"], str) or not re.fullmatch(r"[0-9a-f]{64}", value["sha256"])
            or type(value["count"]) is not int or value["count"] < 0 or type(value["cutoff_id"]) is not int):
        fail("checkpoint has an invalid legacy baseline")
    if value["count"] == 0:
        if value["first_id"] is not None or value["last_id"] is not None:
            fail("empty legacy baseline has row bounds")
    elif type(value["first_id"]) is not int or type(value["last_id"]) is not int or value["first_id"] > value["last_id"]:
        fail("legacy baseline has invalid row bounds")


def validate_disposition(value):
    fields = {"format", "database_uuid", "database_name", "historical_status", "cutoff_id", "cutoff_hash", "legacy_baseline",
        "defects", "defects_sha256", "root_cause", "evidence_sha256", "migration_sha256"}
    if not isinstance(value, dict) or set(value) != fields or type(value["format"]) is not int or value["format"] != 1:
        fail("legacy disposition has an unexpected shape")
    try:
        uuid.UUID(value["database_uuid"])
    except (ValueError, TypeError, AttributeError):
        fail("legacy disposition has an invalid database UUID")
    if not isinstance(value["database_name"], str) or not value["database_name"]:
        fail("legacy disposition has an invalid database name")
    if (value["historical_status"] != "known_broken_preserved"
            or type(value["cutoff_id"]) is not int or value["cutoff_id"] < 1
            or not isinstance(value["cutoff_hash"], str)
            or not re.fullmatch(r"[0-9a-fA-F]{64}", value["cutoff_hash"])):
        fail("legacy disposition has an invalid boundary")
    validate_baseline(value["legacy_baseline"])
    if (value["legacy_baseline"]["count"] < 1 or type(value["legacy_baseline"]["first_id"]) is not int
            or value["legacy_baseline"]["first_id"] < 1
            or value["legacy_baseline"]["last_id"] != value["cutoff_id"]
            or value["legacy_baseline"]["cutoff_id"] != value["cutoff_id"]):
        fail("legacy disposition baseline cutoff disagrees with its boundary")
    if not isinstance(value["defects"], list) or len(value["defects"]) == 0:
        fail("legacy disposition must preserve at least one defect")
    defect_fields = {"id", "stored_prev_hash", "expected_prev_hash",
        "stored_predecessor_id", "expected_predecessor_id"}
    last_id = None
    for defect in value["defects"]:
        if not isinstance(defect, dict) or set(defect) != defect_fields or type(defect["id"]) is not int:
            fail("legacy disposition has an invalid defect record")
        for key in ("stored_prev_hash", "expected_prev_hash"):
            if not isinstance(defect[key], str) or not re.fullmatch(r"[0-9a-fA-F]{64}", defect[key]):
                fail("legacy disposition has an invalid defect hash")
        for key in ("stored_predecessor_id", "expected_predecessor_id"):
            if type(defect[key]) is not int or defect[key] < 1 or defect[key] > value["cutoff_id"]:
                fail("legacy disposition has an invalid predecessor ID")
        if defect["id"] < 1 or defect["id"] > value["cutoff_id"]:
            fail("legacy disposition has a defect outside its cutoff")
        if defect["stored_prev_hash"] == defect["expected_prev_hash"]:
            fail("legacy disposition contains a non-defect")
        if last_id is not None and defect["id"] <= last_id:
            fail("legacy disposition defects are not in strict ID order")
        last_id = defect["id"]
    expected_digest = hashlib.sha256(canonical(value["defects"])).hexdigest()
    if value["defects_sha256"] != expected_digest:
        fail("legacy disposition defect digest does not match its records")
    for key in ("evidence_sha256", "migration_sha256"):
        if not isinstance(value[key], str) or not re.fullmatch(r"[0-9a-f]{64}", value[key]):
            fail(f"legacy disposition has an invalid {key}")
    if not isinstance(value["root_cause"], str) or not value["root_cause"].strip():
        fail("legacy disposition root cause is required")


def validate_chain(objects, db_uuid, writer_arn, now):
    previous = None
    for sequence, item in enumerate(objects, 1):
        if not isinstance(item, dict) or not isinstance(item.get("body"), dict):
            fail(f"checkpoint {sequence} is not a JSON object")
        body = item["body"]
        expected_fields = {"format", "sequence", "database_uuid", "database_name", "head_id",
            "head_hash", "legacy_baseline", "writer_arn", "created_at", "previous"}
        if body.get("format") == 2:
            expected_fields.add("legacy_disposition")
        if set(body) != expected_fields:
            fail(f"checkpoint {sequence} has missing or unexpected fields")
        if body.get("format") not in (1, 2) or type(body.get("sequence")) is not int:
            fail(f"checkpoint {sequence} has an unsupported format or sequence type")
        if body["format"] == 2:
            validate_disposition(body["legacy_disposition"])
            if body["legacy_baseline"] != body["legacy_disposition"]["legacy_baseline"]:
                fail(f"checkpoint {sequence} changes its disposition baseline")
            if (body["database_uuid"] != body["legacy_disposition"]["database_uuid"]
                    or body["database_name"] != body["legacy_disposition"]["database_name"]):
                fail(f"checkpoint {sequence} disposition belongs to another database")
        if (type(body.get("head_id")) is not int or body["head_id"] < 1
                or not isinstance(body.get("head_hash"), str)
                or not re.fullmatch(r"[0-9a-fA-F]{64}", body["head_hash"])):
            fail(f"checkpoint {sequence} has an invalid audit head")
        if (not isinstance(body["database_name"], str) or not body["database_name"]
                or not isinstance(body["writer_arn"], str) or not isinstance(body["created_at"], str)):
            fail(f"checkpoint {sequence} has invalid identity metadata")
        parse_time(body["created_at"])
        validate_baseline(body.get("legacy_baseline"))
        expected_key = f"checkpoints/{db_uuid}/{sequence:020d}.json"
        if item["key"] != expected_key or body.get("sequence") != sequence:
            fail("checkpoint sequence is missing, duplicated, or out of order")
        digest = hashlib.sha256(canonical(body)).hexdigest()
        if (not isinstance(item.get("version_id"), str) or not item["version_id"]
                or item.get("sha256") != digest):
            fail(f"checkpoint {sequence} body hash does not match immutable metadata")
        if item.get("writer_arn") != writer_arn or body.get("writer_arn") != writer_arn:
            fail(f"checkpoint {sequence} was not written by the dedicated writer")
        if item.get("retention_mode") != "COMPLIANCE" or item.get("retain_until") <= now:
            fail(f"checkpoint {sequence} lacks active COMPLIANCE retention")
        if body.get("database_uuid") != db_uuid:
            fail(f"checkpoint {sequence} belongs to another database identity")
        if previous is None:
            if body.get("previous") is not None:
                fail("bootstrap checkpoint has a previous link")
        else:
            if body["format"] != previous["body"]["format"]:
                fail(f"checkpoint {sequence} changes checkpoint format")
            if body.get("legacy_baseline") != previous["body"].get("legacy_baseline"):
                fail(f"checkpoint {sequence} changes the enrolled legacy baseline")
            if body.get("legacy_disposition") != previous["body"].get("legacy_disposition"):
                fail(f"checkpoint {sequence} changes the retained legacy disposition")
            expected = {
                "key": previous["key"], "version_id": previous["version_id"],
                "sha256": previous["sha256"], "head_id": previous["body"]["head_id"],
                "head_hash": previous["body"]["head_hash"],
            }
            if body.get("previous") != expected:
                fail(f"checkpoint {sequence} does not link to the prior retained object")
            if body["head_id"] <= previous["body"]["head_id"]:
                fail(f"checkpoint {sequence} does not advance the audit head")
        previous = item
    return previous


def utc_now():
    return dt.datetime.now(dt.timezone.utc)


def validate_chain_now(objects, db_uuid, writer_arn, clock=utc_now):
    now = clock()
    return validate_chain(objects, db_uuid, writer_arn, now), now


def normalized_principal(identity):
    account, arn = identity.get("Account"), identity.get("Arn")
    if not isinstance(account, str) or not isinstance(arn, str):
        fail("STS identity is missing Account or Arn")
    sts_prefix = f"arn:aws:sts::{account}:assumed-role/"
    if arn.startswith(sts_prefix):
        parts = arn[len(sts_prefix):].split("/")
        if len(parts) < 2 or not all(parts):
            fail("STS assumed-role ARN is malformed")
        return account, f"arn:aws:iam::{account}:role/{'/'.join(parts[:-1])}"
    if arn.startswith(f"arn:aws:iam::{account}:user/"):
        return account, arn
    fail("checkpoint writer must be an explicitly configured IAM user or assumed role")


def listed_versions(listing):
    if listing.get("DeleteMarkers"):
        fail("checkpoint prefix contains delete markers")
    versions = listing.get("Versions", [])
    if not isinstance(versions, list) or any(not item.get("IsLatest") for item in versions):
        fail("checkpoint key has multiple versions; possible overwrite or rollback")
    return versions


def validate_bucket(lock, versioning):
    if lock.get("ObjectLockConfiguration", {}).get("ObjectLockEnabled") != "Enabled":
        fail("S3 bucket does not have Object Lock enabled")
    if versioning.get("Status") != "Enabled":
        fail("S3 bucket versioning is not enabled")


def put_arguments(args, key, source, retain_until, digest):
    return ["s3api", "put-object", "--bucket", args.bucket, "--key", key,
        "--body", source, "--if-none-match", "*", "--checksum-algorithm", "SHA256",
        "--object-lock-mode", "COMPLIANCE", "--object-lock-retain-until-date", retain_until,
        "--metadata", f"sha256={digest},writer-arn={args.writer_arn}",
        "--expected-bucket-owner", args.bucket_owner]


def verification_sql(previous_id):
    continuity = "NULL::jsonb" if previous_id is None else (
        f"(SELECT to_jsonb(c) FROM verify_audit_log_hash_chain_range({previous_id},{previous_id},1) c)")
    return f"""
BEGIN TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY;
WITH RECURSIVE first_page AS (
  SELECT v.* FROM verify_audit_log_hash_chain_range(NULL, NULL, 10000) v
), pages AS (
  SELECT 1 page_no, f.* FROM first_page f
  UNION ALL
  SELECT p.page_no + 1, v.* FROM pages p
  CROSS JOIN LATERAL verify_audit_log_hash_chain_range(
    p.next_since_id, p.snapshot_head_id, 10000) v
  WHERE p.has_more
), result AS (
  SELECT jsonb_build_object(
    'database_name', current_database(),
    'continuity', {continuity},
    'pages', jsonb_agg(to_jsonb(p) ORDER BY page_no)
  ) value FROM pages p
)
SELECT 'REPORT' || chr(9) || value::text FROM result;
SELECT 'LEGACY' || chr(9) || jsonb_build_array(
  hash_version, id, prev_hash::text, entry_hash::text, organization_id,
  table_name, record_id, action, user_id, reason_for_change, change_category,
  e_signature_id,
  jsonb_build_array(old_values IS NULL, old_values),
  jsonb_build_array(new_values IS NULL, new_values),
  jsonb_build_array(changed_fields IS NULL, changed_fields), ip_address::text,
  user_agent, to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
)::text
FROM audit_logs WHERE hash_version = 1 ORDER BY id;
/* The unaligned SELECT preserves JSON escaping; COPY text would escape the protocol tab. */
COMMIT;
"""


def qualified_verification_sql(cutoff_id, previous_id):
    if type(cutoff_id) is not int or cutoff_id < 1:
        fail("qualified checkpoint cutoff must be a positive integer")
    continuity = "NULL::jsonb" if previous_id is None else (
        f"(SELECT to_jsonb(c) FROM verify_audit_log_hash_chain_range({previous_id},{previous_id},1) c)")
    return f"""
BEGIN TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY;
WITH RECURSIVE boundary AS MATERIALIZED (
  SELECT min(id) FILTER (WHERE hash_version = 1) AS min_v1_id,
         max(id) FILTER (WHERE hash_version = 1) AS max_v1_id,
         count(*) FILTER (WHERE hash_version = 1) AS v1_count,
         min(id) FILTER (WHERE hash_version = 2) AS min_v2_id,
         max(id) FILTER (WHERE hash_version = 2) AS max_v2_id,
         count(*) FILTER (WHERE hash_version = 2) AS v2_count,
         count(*) FILTER (WHERE hash_version IS NULL OR hash_version NOT IN (1, 2)) AS unknown_version_count,
         count(*) FILTER (WHERE hash_version = 1 AND id > {cutoff_id}) AS v1_after_cutoff,
         count(*) FILTER (WHERE hash_version = 2 AND id <= {cutoff_id}) AS v2_at_or_before_cutoff,
         max(id) AS snapshot_head_id
    FROM audit_logs
), first_page AS (
  SELECT v.* FROM verify_audit_log_hash_chain_range({cutoff_id + 1}, NULL, 10000) v
), pages AS (
  SELECT 1 page_no, f.* FROM first_page f
  UNION ALL
  SELECT p.page_no + 1, v.* FROM pages p
  CROSS JOIN LATERAL verify_audit_log_hash_chain_range(
    p.next_since_id, p.snapshot_head_id, 10000) v
  WHERE p.has_more
), result AS (
  SELECT jsonb_build_object(
    'database_name', current_database(), 'continuity', {continuity},
    'boundary', (SELECT to_jsonb(b) || jsonb_build_object(
      'cutoff_id', {cutoff_id},
      'cutoff_hash', (SELECT entry_hash::text FROM audit_logs WHERE id = {cutoff_id})) FROM boundary b),
    'pages', jsonb_agg(to_jsonb(p) ORDER BY page_no)
  ) value FROM pages p
)
SELECT 'QUALIFIED_REPORT' || chr(9) || value::text FROM result;
SELECT stream_line FROM (
WITH ordered AS (
  SELECT a.*,
         lag(a.id) OVER (ORDER BY a.id) AS expected_predecessor_id,
         lag(a.entry_hash::text) OVER (ORDER BY a.id) AS expected_prev_hash
    FROM audit_logs a
), checked AS (
  SELECT o.*,
         coalesce(o.expected_prev_hash, repeat('0', 64)) AS checked_expected_prev_hash,
         encode(public.digest(concat_ws('|',
           coalesce(o.id::text, ''), coalesce(o.prev_hash::text, ''), o.table_name,
           o.record_id::text, o.action, o.user_id::text, coalesce(o.reason_for_change, ''),
           coalesce(o.change_category, ''), coalesce(o.e_signature_id::text, ''),
           coalesce(o.old_values::text, 'null'), coalesce(o.new_values::text, 'null'),
           coalesce(o.changed_fields::text, 'null'), coalesce(o.ip_address::text, ''),
           coalesce(o.user_agent, ''),
           to_char(o.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
         ), 'sha256'), 'hex') AS recomputed_entry_hash
    FROM ordered o WHERE o.hash_version = 1
)
SELECT id, 'LEGACY_CHECKED' || chr(9) || jsonb_build_object(
  'canonical', jsonb_build_array(
    hash_version, id, prev_hash::text, entry_hash::text, organization_id,
    table_name, record_id, action, user_id, reason_for_change, change_category,
    e_signature_id, jsonb_build_array(old_values IS NULL, old_values),
    jsonb_build_array(new_values IS NULL, new_values),
    jsonb_build_array(changed_fields IS NULL, changed_fields), ip_address::text,
    user_agent, to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
  )::text,
  'expected_predecessor_id', expected_predecessor_id,
  'expected_prev_hash', checked_expected_prev_hash,
  'stored_predecessor_id', CASE
    WHEN prev_hash::text = checked_expected_prev_hash THEN expected_predecessor_id
    ELSE (SELECT id FROM audit_logs p WHERE p.entry_hash = checked.prev_hash LIMIT 1)
  END,
  'recomputed_entry_hash', recomputed_entry_hash
)::text AS stream_line FROM checked ORDER BY id
) legacy_stream;
COMMIT;
"""


def verify_report(report, previous):
    if set(report) != {"database_name", "continuity", "pages"} or not isinstance(report["pages"], list):
        fail("database verification report has an unexpected shape")
    pages = report["pages"]
    if len(pages) == 0:
        fail("database verification returned no pages")
    required = {"page_no", "total_rows", "verified_ok_rows", "broken_rows", "first_broken_id",
        "first_broken_reason", "checked_at", "checked_from_id", "checked_through_id",
        "snapshot_head_id", "snapshot_head_hash", "next_since_id", "has_more", "legacy_rows"}
    head_id, head_hash = pages[0].get("snapshot_head_id"), pages[0].get("snapshot_head_hash")
    if type(head_id) is not int or not isinstance(head_hash, str) or not re.fullmatch(r"[0-9a-fA-F]{64}", head_hash):
        fail("cannot checkpoint an empty audit log")
    expected_from = pages[0].get("checked_from_id")
    legacy_rows = 0
    for number, page in enumerate(pages, 1):
        if set(page) != required:
            fail("database verification page has missing or unexpected fields")
        integer_fields = ("page_no", "total_rows", "verified_ok_rows", "broken_rows",
                          "checked_from_id", "checked_through_id", "snapshot_head_id", "legacy_rows")
        if any(type(page[field]) is not int for field in integer_fields) or type(page["has_more"]) is not bool:
            fail("database verification page has invalid field types")
        if (not isinstance(page["checked_at"], str)
                or (page["next_since_id"] is not None and type(page["next_since_id"]) is not int)):
            fail("database verification page has invalid field types")
        if page["page_no"] != number or page["snapshot_head_id"] != head_id or page["snapshot_head_hash"] != head_hash:
            fail("verification pages do not share one frozen audit head")
        if page["total_rows"] <= 0 or page["total_rows"] != page["verified_ok_rows"] + page["broken_rows"]:
            fail("database verification page row counts are inconsistent")
        if page["legacy_rows"] < 0 or page["legacy_rows"] > page["total_rows"]:
            fail("database verification page legacy count is inconsistent")
        if page["broken_rows"] != 0 or page["first_broken_id"] is not None or page["first_broken_reason"] is not None:
            fail("audit range contains broken rows")
        if page["checked_from_id"] != expected_from or page["checked_through_id"] < page["checked_from_id"]:
            fail("database verification pages are discontinuous")
        legacy_rows += page["legacy_rows"]
        expected_from = page["next_since_id"]
        if page["has_more"] != (page["next_since_id"] is not None):
            fail("database verification continuation fields disagree")
    if pages[-1]["has_more"] or pages[-1]["next_since_id"] is not None or pages[-1]["checked_through_id"] != head_id:
        fail("audit range verification is partial")
    if previous:
        old = previous["body"]
        continuity = report.get("continuity")
        continuity_fields = required - {"page_no"}
        if not isinstance(continuity, dict) or set(continuity) != continuity_fields:
            fail("previous checkpoint continuity report is incomplete")
        if (any(type(continuity[field]) is not int for field in
                ("total_rows", "verified_ok_rows", "broken_rows", "checked_from_id",
                 "checked_through_id", "snapshot_head_id", "legacy_rows"))
                or type(continuity["has_more"]) is not bool
                or continuity["total_rows"] != 1 or continuity["verified_ok_rows"] != 1
                or continuity["broken_rows"] != 0 or continuity["first_broken_id"] is not None
                or continuity["first_broken_reason"] is not None
                or not isinstance(continuity["checked_at"], str)
                or continuity["checked_from_id"] != old["head_id"]
                or continuity["checked_through_id"] != old["head_id"]
                or continuity["next_since_id"] is not None or continuity["has_more"]
                or continuity["legacy_rows"] not in (0, 1)):
            fail("previous checkpoint head no longer verifies")
        if continuity.get("snapshot_head_id") != old["head_id"] or continuity.get("snapshot_head_hash") != old["head_hash"]:
            fail("database no longer contains the exact checkpointed head")
        if head_id < old["head_id"]:
            fail("database audit head rolled back")
    return head_id, head_hash, legacy_rows


def verify_qualified_report(report, previous, disposition, legacy, defects):
    if not isinstance(report, dict) or set(report) != {"database_name", "continuity", "boundary", "pages"}:
        fail("qualified database verification report has an unexpected shape")
    boundary_fields = {"min_v1_id", "max_v1_id", "v1_count", "min_v2_id", "max_v2_id",
        "v2_count", "unknown_version_count", "v1_after_cutoff", "v2_at_or_before_cutoff", "snapshot_head_id",
        "cutoff_id", "cutoff_hash"}
    boundary = report["boundary"]
    if not isinstance(boundary, dict) or set(boundary) != boundary_fields:
        fail("qualified database boundary report has an unexpected shape")
    integer_fields = boundary_fields - {"cutoff_hash"}
    if any(type(boundary[field]) is not int for field in integer_fields):
        fail("qualified database boundary has invalid field types")
    if (boundary["cutoff_id"] != disposition["cutoff_id"]
            or boundary["cutoff_hash"] != disposition["cutoff_hash"]
            or boundary["min_v1_id"] != legacy["first_id"]
            or boundary["max_v1_id"] != disposition["cutoff_id"]
            or boundary["v1_count"] != legacy["count"]
            or boundary["unknown_version_count"] != 0
            or boundary["v1_after_cutoff"] != 0 or boundary["v2_at_or_before_cutoff"] != 0
            or boundary["v2_count"] < 1 or boundary["min_v2_id"] <= disposition["cutoff_id"]
            or boundary["max_v2_id"] != boundary["snapshot_head_id"]):
        fail("qualified database epoch boundary changed or is not separated")
    head_id, head_hash, legacy_rows = verify_report(
        {"database_name": report["database_name"], "continuity": report["continuity"],
         "pages": report["pages"]}, previous)
    if legacy_rows != 0 or head_id != boundary["snapshot_head_id"]:
        fail("qualified post-boundary verification includes legacy rows or the wrong head")
    if (report["pages"][0]["checked_from_id"] != boundary["min_v2_id"]
            or sum(page["total_rows"] for page in report["pages"]) != boundary["v2_count"]):
        fail("qualified post-boundary verifier does not cover every version-2 row")
    if legacy != disposition["legacy_baseline"]:
        fail("qualified legacy canonical baseline changed")
    if defects != disposition["defects"]:
        fail("qualified legacy defect records changed")
    if hashlib.sha256(canonical(defects)).hexdigest() != disposition["defects_sha256"]:
        fail("qualified legacy defect digest changed")
    return head_id, head_hash


def build_disposition(database_uuid, database_name, cutoff_id, cutoff_hash, legacy, defects, root_cause,
                      evidence_sha256, migration_sha256):
    value = {"format": 1, "database_uuid": database_uuid, "database_name": database_name,
        "historical_status": "known_broken_preserved",
        "cutoff_id": cutoff_id, "cutoff_hash": cutoff_hash, "legacy_baseline": legacy,
        "defects": defects, "defects_sha256": hashlib.sha256(canonical(defects)).hexdigest(),
        "root_cause": root_cause, "evidence_sha256": evidence_sha256,
        "migration_sha256": migration_sha256}
    validate_disposition(value)
    return value


def write_disposition(path, value):
    validate_disposition(value)
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "wb") as output:
        output.write(canonical(value))


def prepare_disposition(args, cutoff_id, root_cause, evidence_sha256, migration_sha256, path):
    report, legacy, defects = read_qualified_database(args, cutoff_id, None)
    boundary = report.get("boundary")
    if not isinstance(boundary, dict) or report.get("database_name") != args.database_name:
        fail("qualified database boundary report is missing")
    value = build_disposition(args.database_uuid, args.database_name,
                              cutoff_id, boundary.get("cutoff_hash"), legacy, defects,
                              root_cause, evidence_sha256, migration_sha256)
    verify_qualified_report(report, None, value, legacy, defects)
    write_disposition(path, value)
    return {"historical_status": value["historical_status"], "historical_broken_rows": len(defects),
            "cutoff_id": cutoff_id, "cutoff_hash": value["cutoff_hash"]}


class S3:
    def __init__(self, args):
        self.args = args
        self.base = [args.aws, "--profile", args.profile, "--region", args.region]

    def call(self, *parts):
        return run_json(self.base + list(parts) + ["--output", "json"])

    def identity(self):
        return self.call("sts", "get-caller-identity")

    def preflight(self):
        validate_bucket(
            self.call("s3api", "get-object-lock-configuration", "--bucket", self.args.bucket,
                      "--expected-bucket-owner", self.args.bucket_owner),
            self.call("s3api", "get-bucket-versioning", "--bucket", self.args.bucket,
                      "--expected-bucket-owner", self.args.bucket_owner))

    def load(self):
        listing = self.call("s3api", "list-object-versions", "--bucket", self.args.bucket,
                            "--prefix", f"checkpoints/{self.args.database_uuid}/",
                            "--expected-bucket-owner", self.args.bucket_owner)
        versions = listed_versions(listing)
        objects = []
        for item in sorted(versions, key=lambda value: value["Key"]):
            key, version = item["Key"], item["VersionId"]
            with tempfile.NamedTemporaryFile() as output:
                try:
                    result = subprocess.run(self.base + ["s3api", "get-object", "--bucket", self.args.bucket,
                        "--key", key, "--version-id", version, "--expected-bucket-owner",
                        self.args.bucket_owner, output.name, "--output", "json"], text=True,
                        capture_output=True)
                except OSError as error:
                    fail(f"cannot execute {self.args.aws}: {error}")
                if result.returncode:
                    fail(f"cannot read retained checkpoint {key}: {result.stderr.strip()}")
                metadata = json.loads(result.stdout)
                output.seek(0)
                body = json.load(output)
            retention = self.call("s3api", "get-object-retention", "--bucket", self.args.bucket,
                "--key", key, "--version-id", version, "--expected-bucket-owner", self.args.bucket_owner)["Retention"]
            objects.append({"key": key, "version_id": version, "body": body,
                "sha256": metadata.get("Metadata", {}).get("sha256"),
                "writer_arn": metadata.get("Metadata", {}).get("writer-arn"),
                "retention_mode": retention.get("Mode"),
                "retain_until": parse_time(retention.get("RetainUntilDate"))})
        return objects

    def put(self, key, body, retain_until):
        data, digest = canonical(body), hashlib.sha256(canonical(body)).hexdigest()
        with tempfile.NamedTemporaryFile() as source:
            source.write(data); source.flush()
            return self.call(*put_arguments(self.args, key, source.name, retain_until, digest))


def parse_time(value):
    if not isinstance(value, str) or not value:
        fail("retention timestamp is missing")
    parsed = dt.datetime.fromisoformat(value.replace("Z", "+00:00"))
    if parsed.tzinfo is None or parsed.utcoffset() is None:
        fail("retention timestamp has no timezone")
    return parsed


def parse_stream_line(line):
    if line.startswith("REPORT\t"):
        return "report", json.loads(line.split("\t", 1)[1]), None
    if line.startswith("LEGACY\t"):
        raw = line.split("\t", 1)[1].rstrip("\n")
        row = json.loads(raw)
        if not isinstance(row, list) or len(row) != 18 or row[0] != 1 or type(row[1]) is not int:
            fail("legacy audit stream has an invalid canonical row")
        return "legacy", row, raw
    if line.strip():
        fail("database verification returned unexpected output")
    return "blank", None, None


def parse_qualified_line(line):
    if line.startswith("QUALIFIED_REPORT\t"):
        return "report", json.loads(line.split("\t", 1)[1]), None
    if line.startswith("LEGACY_CHECKED\t"):
        value = json.loads(line.split("\t", 1)[1])
        fields = {"canonical", "expected_predecessor_id", "expected_prev_hash",
            "stored_predecessor_id", "recomputed_entry_hash"}
        if not isinstance(value, dict) or set(value) != fields or not isinstance(value["canonical"], str):
            fail("qualified legacy stream has an invalid row")
        row = json.loads(value["canonical"])
        if (not isinstance(row, list) or len(row) != 18 or type(row[0]) is not int or row[0] != 1
                or type(row[1]) is not int or row[1] < 1):
            fail("qualified legacy stream has an invalid canonical row")
        for hash_value in (row[2], row[3], value["expected_prev_hash"], value["recomputed_entry_hash"]):
            if not isinstance(hash_value, str) or not re.fullmatch(r"[0-9a-fA-F]{64}", hash_value):
                fail("qualified legacy stream has an invalid hash")
        for key in ("expected_predecessor_id", "stored_predecessor_id"):
            if value[key] is not None and (type(value[key]) is not int or value[key] < 1):
                fail("qualified legacy stream has an invalid predecessor ID")
        return "legacy_checked", (row, value), value["canonical"]
    if line.strip():
        fail("qualified database verification returned unexpected output")
    return "blank", None, None


def checked_legacy_defect(row, checked):
    if row[3] != checked["recomputed_entry_hash"]:
        fail(f"legacy audit payload hash mismatch at id {row[1]}")
    if row[2] == checked["expected_prev_hash"]:
        return None
    return {"id": row[1], "stored_prev_hash": row[2],
        "expected_prev_hash": checked["expected_prev_hash"],
        "stored_predecessor_id": checked["stored_predecessor_id"],
        "expected_predecessor_id": checked["expected_predecessor_id"]}


def database_environment(url, base):
    parsed = urllib.parse.urlsplit(url)
    if parsed.scheme not in ("postgres", "postgresql") or parsed.fragment:
        fail("AUDIT_CHECKPOINT_DATABASE_URL must be a PostgreSQL URL without a fragment")
    if not parsed.path.startswith("/") or len(parsed.path) == 1:
        fail("AUDIT_CHECKPOINT_DATABASE_URL must name a database")
    if "/" in parsed.path[1:]:
        fail("AUDIT_CHECKPOINT_DATABASE_URL database name must be one path segment")
    if parsed.hostname is None or parsed.username is None:
        fail("AUDIT_CHECKPOINT_DATABASE_URL must include an explicit hostname and username")
    env = base.copy()
    for name in ("PGDATABASE", "PGHOST", "PGHOSTADDR", "PGPORT", "PGUSER", "PGPASSWORD",
                 "PGPASSFILE", "PGSERVICE", "PGSERVICEFILE", "PGSSLMODE",
                 "PGSSLROOTCERT", "PGSSLCERT", "PGSSLKEY", "PGCONNECT_TIMEOUT",
                 "PGAPPNAME", "PGOPTIONS", "PGTARGETSESSIONATTRS"):
        env.pop(name, None)
    env["PGDATABASE"] = urllib.parse.unquote(parsed.path[1:])
    env["PGHOST"] = parsed.hostname
    if parsed.port is not None:
        env["PGPORT"] = str(parsed.port)
    env["PGUSER"] = urllib.parse.unquote(parsed.username)
    if parsed.password is not None:
        env["PGPASSWORD"] = urllib.parse.unquote(parsed.password)
    query_names = {"sslmode": "PGSSLMODE", "sslrootcert": "PGSSLROOTCERT",
        "sslcert": "PGSSLCERT", "sslkey": "PGSSLKEY", "connect_timeout": "PGCONNECT_TIMEOUT",
        "application_name": "PGAPPNAME", "options": "PGOPTIONS",
        "target_session_attrs": "PGTARGETSESSIONATTRS"}
    for key, values in urllib.parse.parse_qs(parsed.query, keep_blank_values=True).items():
        if key not in query_names or len(values) != 1 or values[0] == "":
            fail(f"unsupported or ambiguous PostgreSQL URL parameter: {key}")
        env[query_names[key]] = values[0]
    env["PGCLIENTENCODING"] = "UTF8"
    return env


def read_database(args, previous_id):
    base_env = os.environ.copy()
    url = base_env.pop("AUDIT_CHECKPOINT_DATABASE_URL", None)
    if not url:
        fail("AUDIT_CHECKPOINT_DATABASE_URL is required")
    env = database_environment(url, base_env)
    digest, count, first_id, last_id, report = hashlib.sha256(), 0, None, None, None
    with tempfile.TemporaryFile(mode="w+") as errors:
        try:
            # FETCH_COUNT makes psql use a cursor instead of buffering the full legacy SELECT.
            process = subprocess.Popen([args.psql, "-X", "-q", "-A", "-t", "-v", "ON_ERROR_STOP=1",
                "-v", "FETCH_COUNT=1000"],
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=errors, env=env, text=True)
        except OSError as error:
            fail(f"cannot execute {args.psql}: {error}")
        process.stdin.write(verification_sql(previous_id)); process.stdin.close()
        try:
            for line in process.stdout:
                kind, value, raw = parse_stream_line(line)
                if kind == "report":
                    if report is not None:
                        fail("database verification returned multiple reports")
                    report = value
                elif kind == "legacy":
                    row = value
                    if last_id is not None and row[1] <= last_id:
                        fail("legacy audit stream is not in strict ID order")
                    digest.update(raw.encode("utf-8") + b"\n")
                    count += 1; first_id = row[1] if first_id is None else first_id; last_id = row[1]
        except (RuntimeError, ValueError, UnicodeError):
            process.terminate()
            process.wait()
            raise
        code = process.wait()
        if code:
            errors.seek(0)
            fail(f"database verification failed: {errors.read().strip()}")
    if report is None:
        fail("database verification did not return exactly one JSON report")
    return report, {"sha256": digest.hexdigest(), "count": count,
                    "first_id": first_id, "last_id": last_id}


def read_qualified_database(args, cutoff_id, previous_id):
    base_env = os.environ.copy()
    url = base_env.pop("AUDIT_CHECKPOINT_DATABASE_URL", None)
    if not url:
        fail("AUDIT_CHECKPOINT_DATABASE_URL is required")
    env = database_environment(url, base_env)
    digest, count, first_id, last_id, report, defects = hashlib.sha256(), 0, None, None, None, []
    with tempfile.TemporaryFile(mode="w+") as errors:
        try:
            process = subprocess.Popen([args.psql, "-X", "-q", "-A", "-t", "-v", "ON_ERROR_STOP=1",
                "-v", "FETCH_COUNT=1000"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                stderr=errors, env=env, text=True)
        except OSError as error:
            fail(f"cannot execute {args.psql}: {error}")
        process.stdin.write(qualified_verification_sql(cutoff_id, previous_id)); process.stdin.close()
        try:
            for line in process.stdout:
                kind, value, raw = parse_qualified_line(line)
                if kind == "report":
                    if report is not None:
                        fail("qualified database verification returned multiple reports")
                    report = value
                elif kind == "legacy_checked":
                    row, checked = value
                    if last_id is not None and row[1] <= last_id:
                        fail("qualified legacy stream is not in strict ID order")
                    defect = checked_legacy_defect(row, checked)
                    if defect is not None:
                        defects.append(defect)
                    digest.update(raw.encode("utf-8") + b"\n")
                    count += 1; first_id = row[1] if first_id is None else first_id; last_id = row[1]
        except (RuntimeError, ValueError, UnicodeError):
            process.terminate(); process.wait(); raise
        code = process.wait()
        if code:
            errors.seek(0)
            fail(f"qualified database verification failed: {errors.read().strip()}")
    if report is None:
        fail("qualified database verification did not return exactly one report")
    return report, {"sha256": digest.hexdigest(), "count": count, "first_id": first_id,
        "last_id": last_id, "cutoff_id": cutoff_id}, defects


def main(argv=None):
    parser = argparse.ArgumentParser()
    for name in ("bucket", "bucket-owner", "region", "profile", "writer-arn", "database-uuid", "database-name"):
        parser.add_argument("--" + name, required=True)
    parser.add_argument("--retention-days", required=True, type=int)
    parser.add_argument("--bootstrap", action="store_true")
    parser.add_argument("--verify-only", action="store_true")
    parser.add_argument("--legacy-disposition")
    parser.add_argument("--aws", default="aws")
    parser.add_argument("--psql", default="psql")
    args = parser.parse_args(argv)
    uuid.UUID(args.database_uuid)
    if args.retention_days < 1:
        fail("retention-days must be positive")
    if args.bootstrap and args.verify_only:
        fail("--bootstrap cannot be combined with --verify-only because no independent anchor exists")
    if args.legacy_disposition and not args.bootstrap:
        fail("--legacy-disposition is accepted only for bootstrap; later runs use the retained copy")
    s3 = S3(args)
    account, principal = normalized_principal(s3.identity())
    if account != args.bucket_owner or principal != args.writer_arn:
        fail("active AWS identity is not the configured dedicated checkpoint writer")
    s3.preflight()
    objects = s3.load()
    if args.bootstrap != (not objects):
        fail("bootstrap is required exactly once on an empty checkpoint prefix")
    previous, _ = validate_chain_now(objects, args.database_uuid, args.writer_arn)
    disposition = previous["body"].get("legacy_disposition") if previous else None
    if args.legacy_disposition:
        try:
            with open(args.legacy_disposition, encoding="utf-8") as source:
                disposition = json.load(source)
        except OSError as error:
            fail(f"cannot read legacy disposition: {error}")
        validate_disposition(disposition)
    if disposition is not None and (disposition["database_uuid"] != args.database_uuid
                                    or disposition["database_name"] != args.database_name):
        fail("legacy disposition belongs to another configured database")
    if disposition is not None:
        report, legacy, defects = read_qualified_database(
            args, disposition["cutoff_id"], previous["body"]["head_id"] if previous else None)
        if report.get("database_name") != args.database_name:
            fail("connected PostgreSQL database name does not match configuration")
        head_id, head_hash = verify_qualified_report(report, previous, disposition, legacy, defects)
        previous, publication_now = validate_chain_now(
            objects, args.database_uuid, args.writer_arn)
        if previous and head_id == previous["body"]["head_id"]:
            print(json.dumps({"verified": False, "unchanged": True,
                "historical_status": "known_broken_preserved",
                "historical_broken_rows": len(disposition["defects"]),
                "post_boundary_verified": True, "sequence": previous["body"]["sequence"],
                "head_id": head_id, "head_hash": head_hash}, sort_keys=True))
            return
        if args.verify_only:
            print(json.dumps({"verified": False, "unchanged": False, "published": False,
                "historical_status": "known_broken_preserved",
                "historical_broken_rows": len(disposition["defects"]),
                "post_boundary_verified": True, "head_id": head_id,
                "head_hash": head_hash}, sort_keys=True))
            return
        sequence = len(objects) + 1
        body = {"format": 2, "sequence": sequence, "database_uuid": args.database_uuid,
            "database_name": args.database_name, "head_id": head_id, "head_hash": head_hash,
            "legacy_baseline": legacy, "legacy_disposition": disposition,
            "writer_arn": args.writer_arn, "created_at": publication_now.isoformat().replace("+00:00", "Z"),
            "previous": None if not previous else {"key": previous["key"],
                "version_id": previous["version_id"], "sha256": previous["sha256"],
                "head_id": previous["body"]["head_id"], "head_hash": previous["body"]["head_hash"]}}
        key = f"checkpoints/{args.database_uuid}/{sequence:020d}.json"
        retain_until = (publication_now + dt.timedelta(days=args.retention_days)).isoformat().replace("+00:00", "Z")
        result = s3.put(key, body, retain_until)
        version_id = result.get("VersionId")
        if not version_id:
            fail("S3 did not return a VersionId for the retained qualified checkpoint")
        stored = s3.load()
        validate_chain_now(stored, args.database_uuid, args.writer_arn)
        saved = stored[sequence - 1] if len(stored) >= sequence else None
        if (saved is None or saved["version_id"] != version_id or saved["body"] != body
                or saved["sha256"] != hashlib.sha256(canonical(body)).hexdigest()
                or saved["retain_until"] < parse_time(retain_until)):
            fail("the retained qualified checkpoint cannot be read back at its expected sequence")
        print(json.dumps({"verified": False, "historical_status": "known_broken_preserved",
            "historical_broken_rows": len(disposition["defects"]),
            "post_boundary_verified": True, "sequence": sequence, "head_id": head_id,
            "head_hash": head_hash, "key": key, "version_id": version_id,
            "retained_until": retain_until}, sort_keys=True))
        return
    report, legacy = read_database(args, previous["body"]["head_id"] if previous else None)
    if report.get("database_name") != args.database_name:
        fail("connected PostgreSQL database name does not match configuration")
    head_id, head_hash, legacy_rows = verify_report(report, previous)
    legacy["cutoff_id"] = previous["body"]["legacy_baseline"]["cutoff_id"] if previous else head_id
    if legacy_rows != legacy["count"]:
        fail("range verifier legacy count does not match the canonical legacy stream")
    if previous and legacy != previous["body"].get("legacy_baseline"):
        fail("legacy audit metadata changed after checkpoint enrollment")
    previous, publication_now = validate_chain_now(
        objects, args.database_uuid, args.writer_arn)
    if previous and head_id == previous["body"]["head_id"]:
        print(json.dumps({"verified": True, "unchanged": True, "sequence": previous["body"]["sequence"],
            "head_id": head_id, "head_hash": head_hash}, sort_keys=True))
        return
    if args.verify_only:
        print(json.dumps({"verified": True, "unchanged": False, "published": False,
            "head_id": head_id, "head_hash": head_hash}, sort_keys=True))
        return
    sequence = len(objects) + 1
    body = {"format": 1, "sequence": sequence, "database_uuid": args.database_uuid,
        "database_name": args.database_name, "head_id": head_id, "head_hash": head_hash,
        "legacy_baseline": legacy,
        "writer_arn": args.writer_arn, "created_at": publication_now.isoformat().replace("+00:00", "Z"),
        "previous": None if not previous else {"key": previous["key"],
            "version_id": previous["version_id"], "sha256": previous["sha256"],
            "head_id": previous["body"]["head_id"], "head_hash": previous["body"]["head_hash"]}}
    key = f"checkpoints/{args.database_uuid}/{sequence:020d}.json"
    retain_until = (publication_now + dt.timedelta(days=args.retention_days)).isoformat().replace("+00:00", "Z")
    result = s3.put(key, body, retain_until)
    version_id = result.get("VersionId")
    if not version_id:
        fail("S3 did not return a VersionId for the retained checkpoint")
    stored = s3.load()
    validate_chain_now(stored, args.database_uuid, args.writer_arn)
    saved = stored[sequence - 1] if len(stored) >= sequence else None
    if (saved is None or saved["version_id"] != version_id or saved["body"] != body
            or saved["sha256"] != hashlib.sha256(canonical(body)).hexdigest()
            or saved["retain_until"] < parse_time(retain_until)):
        fail("the retained checkpoint cannot be read back at its expected sequence")
    print(json.dumps({"verified": True, "sequence": sequence, "head_id": head_id,
        "head_hash": head_hash, "key": key, "version_id": version_id,
        "retained_until": retain_until}, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (KeyError, ValueError, RuntimeError) as error:
        print(f"audit checkpoint failed: {error}", file=sys.stderr)
        sys.exit(1)
