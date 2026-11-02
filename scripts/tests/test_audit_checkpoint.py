import datetime as dt
import importlib.util
from pathlib import Path
import unittest
from types import SimpleNamespace
import tempfile
from unittest import mock
from contextlib import redirect_stdout
import io

spec = importlib.util.spec_from_file_location("audit_checkpoint", Path(__file__).parents[1] / "audit_checkpoint.py")
c = importlib.util.module_from_spec(spec); spec.loader.exec_module(c)
DB = "11111111-1111-1111-1111-111111111111"
ARN = "arn:aws:iam::123456789012:role/checkpoint"


def page(head=2):
    return {"page_no": 1, "total_rows": 2, "verified_ok_rows": 2, "broken_rows": 0,
        "first_broken_id": None, "first_broken_reason": None, "checked_at": "now",
        "checked_from_id": 1, "checked_through_id": head, "snapshot_head_id": head,
        "snapshot_head_hash": str(head) * 64, "next_since_id": None, "has_more": False,
        "legacy_rows": 2}


def report(head=2, continuity=None):
    return {"database_name": "db", "continuity": continuity, "pages": [page(head)]}


def continuity(head=5):
    value = page(head); value.pop("page_no")
    value.update(total_rows=1, verified_ok_rows=1, checked_from_id=head,
                 checked_through_id=head, legacy_rows=1)
    return value


def item(sequence=1, head=1, previous=None):
    body = {"format": 1, "sequence": sequence, "database_uuid": DB, "head_id": head,
        "database_name": "db", "head_hash": str(head) * 64, "writer_arn": ARN,
        "created_at": "2029-01-01T00:00:00Z", "previous": previous,
        "legacy_baseline": {"sha256": "a" * 64, "count": 1, "first_id": 1, "last_id": 1,
                            "cutoff_id": 1}}
    return {"key": f"checkpoints/{DB}/{sequence:020d}.json", "version_id": f"v{sequence}",
        "body": body, "sha256": c.hashlib.sha256(c.canonical(body)).hexdigest(), "writer_arn": ARN,
        "retention_mode": "COMPLIANCE", "retain_until": dt.datetime(2030, 1, 1, tzinfo=dt.timezone.utc)}


def qualified_values():
    baseline = {"sha256": "a" * 64, "count": 10, "first_id": 1, "last_id": 10, "cutoff_id": 10}
    defects = [{"id": 5, "stored_prev_hash": "b" * 64, "expected_prev_hash": "c" * 64,
                "stored_predecessor_id": 7, "expected_predecessor_id": 4}]
    disposition = c.build_disposition(DB, "db", 10, "d" * 64, baseline, defects, "preserved historical fork",
                                      "e" * 64, "f" * 64)
    post = page(12)
    post.update(total_rows=2, verified_ok_rows=2, checked_from_id=11, checked_through_id=12,
                legacy_rows=0, snapshot_head_hash="1" * 64)
    report_value = {"database_name": "db", "continuity": None, "pages": [post],
        "boundary": {"min_v1_id": 1, "max_v1_id": 10, "v1_count": 10,
            "min_v2_id": 11, "max_v2_id": 12, "v2_count": 2, "v1_after_cutoff": 0,
            "unknown_version_count": 0, "v2_at_or_before_cutoff": 0, "snapshot_head_id": 12,
            "cutoff_id": 10, "cutoff_hash": "d" * 64}}
    return disposition, baseline, defects, report_value


def qualified_item(sequence=1, head=12, previous=None):
    disposition, baseline, _, _ = qualified_values()
    value = item(sequence, head, previous)
    value["body"].update(format=2, head_hash="1" * 64, legacy_baseline=baseline,
                         legacy_disposition=disposition)
    value["sha256"] = c.hashlib.sha256(c.canonical(value["body"])).hexdigest()
    return value


class CheckpointGuards(unittest.TestCase):
    def validate(self, values):
        return c.validate_chain(values, DB, ARN, dt.datetime(2029, 1, 1, tzinfo=dt.timezone.utc))

    def test_valid_chain(self):
        first = item(); link = {"key": first["key"], "version_id": "v1", "sha256": first["sha256"],
            "head_id": 1, "head_hash": "1" * 64}
        self.assertEqual(self.validate([first, item(2, 2, link)])["version_id"], "v2")

    def test_missing_or_replayed_sequence_rejected(self):
        for values in ([item(2, 2)], [item(), item()]):
            with self.assertRaisesRegex(RuntimeError, "sequence"):
                self.validate(values)

    def test_tamper_rejected(self):
        value = item(); value["body"]["head_id"] = 9
        with self.assertRaisesRegex(RuntimeError, "body hash"):
            self.validate([value])

    def test_wrong_format_type_rejected(self):
        value = item(); value["body"]["sequence"] = "1"
        with self.assertRaisesRegex(RuntimeError, "format or sequence type"):
            self.validate([value])

    def test_untrusted_head_cannot_reach_sql(self):
        value = item(); value["body"]["head_id"] = "1); DROP TABLE audit_logs;--"
        value["sha256"] = c.hashlib.sha256(c.canonical(value["body"])).hexdigest()
        with self.assertRaisesRegex(RuntimeError, "invalid audit head"):
            self.validate([value])

    def test_identity_rejected(self):
        value = item(); value["writer_arn"] = "arn:other"
        with self.assertRaisesRegex(RuntimeError, "dedicated writer"):
            self.validate([value])

    def test_retention_rejected(self):
        for mode, date in (("GOVERNANCE", dt.datetime(2030, 1, 1, tzinfo=dt.timezone.utc)),
                           ("COMPLIANCE", dt.datetime(2028, 1, 1, tzinfo=dt.timezone.utc))):
            value = item(); value["retention_mode"] = mode; value["retain_until"] = date
            with self.assertRaisesRegex(RuntimeError, "retention"):
                self.validate([value])

    def test_broken_link_rejected(self):
        with self.assertRaisesRegex(RuntimeError, "link"):
            self.validate([item(), item(2, 1, {})])

    def test_delete_marker_and_old_version_rejected(self):
        with self.assertRaisesRegex(RuntimeError, "delete markers"):
            c.listed_versions({"DeleteMarkers": [{"Key": "x"}]})
        with self.assertRaisesRegex(RuntimeError, "multiple versions"):
            c.listed_versions({"Versions": [{"IsLatest": False}]})

    def test_bucket_requires_object_lock_and_versioning(self):
        c.validate_bucket({"ObjectLockConfiguration": {"ObjectLockEnabled": "Enabled"}},
                          {"Status": "Enabled"})
        for lock, versioning in (({}, {"Status": "Enabled"}),
                                 ({"ObjectLockConfiguration": {"ObjectLockEnabled": "Enabled"}}, {})):
            with self.assertRaisesRegex(RuntimeError, "Object Lock|versioning"):
                c.validate_bucket(lock, versioning)

    def test_retained_put_has_explicit_sha256_checksum(self):
        args = SimpleNamespace(bucket="bucket", writer_arn=ARN, bucket_owner="123456789012")
        command = c.put_arguments(args, "key", "/tmp/body", "2030-01-01T00:00:00Z", "a" * 64)
        index = command.index("--checksum-algorithm")
        self.assertEqual(command[index:index + 2], ["--checksum-algorithm", "SHA256"])

    def test_principal_normalization(self):
        self.assertEqual(c.normalized_principal({"Account": "123456789012",
            "Arn": "arn:aws:sts::123456789012:assumed-role/checkpoint/session"}),
            ("123456789012", ARN))
        user = "arn:aws:iam::123456789012:user/dedicated"
        self.assertEqual(c.normalized_principal({"Account": "123456789012", "Arn": user})[1], user)
        with self.assertRaisesRegex(RuntimeError, "IAM user or assumed role"):
            c.normalized_principal({"Account": "123456789012", "Arn": "arn:aws:sts::123456789012:federated-user/x"})

    def test_healthy_report(self):
        self.assertEqual(c.verify_report(report(), None), (2, "2" * 64, 2))

    def test_missing_and_wrong_type_rejected(self):
        for mutation in (lambda p: p.pop("broken_rows"), lambda p: p.update(total_rows="2")):
            value = report(); mutation(value["pages"][0])
            with self.assertRaisesRegex(RuntimeError, "fields|types"):
                c.verify_report(value, None)

    def test_count_and_broken_rejected(self):
        value = report(); value["pages"][0]["verified_ok_rows"] = 1
        with self.assertRaisesRegex(RuntimeError, "counts"):
            c.verify_report(value, None)
        value = report(); value["pages"][0].update(broken_rows=1, verified_ok_rows=1,
            first_broken_id=1, first_broken_reason="bad")
        with self.assertRaisesRegex(RuntimeError, "broken"):
            c.verify_report(value, None)
        value = report(); value["pages"][0]["legacy_rows"] = -1
        with self.assertRaisesRegex(RuntimeError, "legacy count"):
            c.verify_report(value, None)

    def test_partial_rejected(self):
        value = report(); value["pages"][0].update(has_more=True, next_since_id=3)
        with self.assertRaisesRegex(RuntimeError, "partial"):
            c.verify_report(value, None)

    def test_prior_head_change_and_rollback_rejected(self):
        old = item(head=5)
        prior = continuity(5)
        value = report(4, prior); value["pages"][0].update(total_rows=4, verified_ok_rows=4, legacy_rows=4)
        with self.assertRaisesRegex(RuntimeError, "rolled back"):
            c.verify_report(value, old)
        value = report(6, dict(prior, snapshot_head_hash="x" * 64))
        with self.assertRaisesRegex(RuntimeError, "exact checkpointed head"):
            c.verify_report(value, old)

    def test_incomplete_continuity_rejected(self):
        value = continuity(); value.pop("total_rows")
        with self.assertRaisesRegex(RuntimeError, "incomplete"):
            c.verify_report(report(6, value), item(head=5))

    def test_sql_genesis_and_legacy_stream(self):
        sql = c.verification_sql(10)
        self.assertIn("verify_audit_log_hash_chain_range(NULL, NULL, 10000)", sql)
        self.assertIn("FROM audit_logs WHERE hash_version = 1 ORDER BY id", sql)
        self.assertEqual(sql.count("jsonb_build_array(old_values IS NULL, old_values)"), 1)
        self.assertEqual(sql.count("jsonb_build_array(new_values IS NULL, new_values)"), 1)
        self.assertEqual(sql.count("jsonb_build_array(changed_fields IS NULL, changed_fields)"), 1)
        self.assertIn("REPEATABLE READ READ ONLY", sql)
        self.assertNotIn("COPY (", sql)

    def test_stream_protocol_preserves_json_escaping(self):
        row = [1, 7] + [None] * 16
        row[9] = "tab\tbackslash\\unicode 한글\nnewline"
        raw = c.json.dumps(row, ensure_ascii=False)
        kind, parsed, canonical = c.parse_stream_line("LEGACY\t" + raw + "\n")
        self.assertEqual((kind, parsed, canonical), ("legacy", row, raw))
        with self.assertRaisesRegex(RuntimeError, "unexpected output"):
            c.parse_stream_line("COPY 1\n")

    def test_legacy_shape_distinguishes_sql_null_from_json_null(self):
        sql_null = [1, 7] + [None] * 16
        json_null = list(sql_null)
        sql_null[12] = [True, None]
        json_null[12] = [False, None]
        left = c.json.dumps(sql_null, separators=(",", ":"))
        right = c.json.dumps(json_null, separators=(",", ":"))
        self.assertNotEqual(c.hashlib.sha256((left + "\n").encode()).digest(),
                            c.hashlib.sha256((right + "\n").encode()).digest())
        self.assertEqual(c.parse_stream_line("LEGACY\t" + left + "\n")[1][12], [True, None])

    def test_naive_retention_time_rejected(self):
        with self.assertRaisesRegex(RuntimeError, "timezone"):
            c.parse_time("2030-01-01T00:00:00")

    def test_unanchored_verify_only_bootstrap_rejected(self):
        with self.assertRaisesRegex(RuntimeError, "no independent anchor"):
            c.main(["--bucket", "b", "--bucket-owner", "123456789012", "--region", "r",
                "--profile", "p", "--writer-arn", ARN, "--database-uuid", DB,
                "--database-name", "db", "--retention-days", "1", "--bootstrap", "--verify-only"])

    def test_database_url_becomes_libpq_environment_without_argv_secret(self):
        secret = "p@ss/word"
        url = "postgresql://audit:p%40ss%2Fword@db.example:5433/safety%20db?sslmode=require&connect_timeout=5"
        env = c.database_environment(url, {"PGHOST": "stale", "PGHOSTADDR": "127.0.0.2",
            "PGSERVICE": "wrong", "PGSERVICEFILE": "/tmp/wrong", "PGPASSFILE": "/tmp/pass",
            "OTHER": "kept"})
        self.assertEqual({key: env[key] for key in
            ("PGHOST", "PGPORT", "PGDATABASE", "PGUSER", "PGPASSWORD", "PGSSLMODE", "PGCONNECT_TIMEOUT")},
            {"PGHOST": "db.example", "PGPORT": "5433", "PGDATABASE": "safety db",
             "PGUSER": "audit", "PGPASSWORD": secret, "PGSSLMODE": "require", "PGCONNECT_TIMEOUT": "5"})
        command = ["psql", "-X", "-q", "-A", "-t", "-v", "ON_ERROR_STOP=1"]
        self.assertNotIn(secret, " ".join(command))
        for key in ("PGHOSTADDR", "PGSERVICE", "PGSERVICEFILE", "PGPASSFILE"):
            self.assertNotIn(key, env)

    def test_database_url_rejects_unknown_parameter(self):
        with self.assertRaisesRegex(RuntimeError, "unsupported"):
            c.database_environment("postgres://user@host/db?fallback=true", {})
        for url in ("postgres:///db", "postgres://host/db"):
            with self.assertRaisesRegex(RuntimeError, "hostname and username"):
                c.database_environment(url, {})

    def test_qualified_disposition_and_epoch(self):
        disposition, baseline, defects, value = qualified_values()
        self.assertEqual(c.verify_qualified_report(value, None, disposition, baseline, defects),
                         (12, "1" * 64))

    def test_qualified_rejects_defect_and_baseline_changes(self):
        disposition, baseline, defects, value = qualified_values()
        with self.assertRaisesRegex(RuntimeError, "defect records"):
            c.verify_qualified_report(value, None, disposition, baseline, defects + [dict(defects[0], id=6)])
        with self.assertRaisesRegex(RuntimeError, "defect records"):
            c.verify_qualified_report(value, None, disposition, baseline, [])
        changed = dict(baseline, sha256="0" * 64)
        with self.assertRaisesRegex(RuntimeError, "baseline"):
            c.verify_qualified_report(value, None, disposition, changed, defects)

    def test_qualified_rejects_boundary_changes_and_empty_v2(self):
        disposition, baseline, defects, value = qualified_values()
        for field, replacement in (("cutoff_hash", "0" * 64), ("v1_after_cutoff", 1),
                                   ("v2_at_or_before_cutoff", 1), ("unknown_version_count", 1),
                                   ("v2_count", 0)):
            broken = c.json.loads(c.json.dumps(value)); broken["boundary"][field] = replacement
            with self.assertRaisesRegex(RuntimeError, "boundary"):
                c.verify_qualified_report(broken, None, disposition, baseline, defects)

    def test_qualified_rejects_incomplete_v2_coverage(self):
        disposition, baseline, defects, value = qualified_values()
        skipped = c.json.loads(c.json.dumps(value)); skipped["pages"][0]["checked_from_id"] = 12
        with self.assertRaisesRegex(RuntimeError, "every version-2 row"):
            c.verify_qualified_report(skipped, None, disposition, baseline, defects)
        short = c.json.loads(c.json.dumps(value)); short["boundary"]["v2_count"] = 3
        with self.assertRaisesRegex(RuntimeError, "every version-2 row"):
            c.verify_qualified_report(short, None, disposition, baseline, defects)

    def test_qualified_rejects_v2_break_and_legacy_payload_mismatch(self):
        disposition, baseline, defects, value = qualified_values()
        broken = c.json.loads(c.json.dumps(value)); broken["pages"][0].update(
            broken_rows=1, verified_ok_rows=1, first_broken_id=11, first_broken_reason="bad")
        with self.assertRaisesRegex(RuntimeError, "broken"):
            c.verify_qualified_report(broken, None, disposition, baseline, defects)
        row = [1, 5, "b" * 64, "a" * 64] + [None] * 14
        checked = {"recomputed_entry_hash": "0" * 64, "expected_prev_hash": "c" * 64,
                   "stored_predecessor_id": 7, "expected_predecessor_id": 4}
        with self.assertRaisesRegex(RuntimeError, "payload hash mismatch"):
            c.checked_legacy_defect(row, checked)

    def test_qualified_sql_is_bounded_and_single_legacy_stream(self):
        sql = c.qualified_verification_sql(10, None)
        self.assertIn("verify_audit_log_hash_chain_range(11, NULL, 10000)", sql)
        self.assertEqual(sql.count("'LEGACY_CHECKED'"), 1)
        self.assertIn("REPEATABLE READ READ ONLY", sql)
        self.assertIn("SELECT stream_line FROM (\nWITH ordered AS (", sql)
        self.assertNotIn(";\nWITH ordered AS (", sql)
        self.assertIn("::text AS stream_line FROM checked ORDER BY id\n) legacy_stream;", sql)
        self.assertNotIn(") legacy_stream ORDER BY id", sql)

    def test_disposition_export_is_private_and_never_overwrites(self):
        disposition, _, _, _ = qualified_values()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "disposition.json"
            c.write_disposition(path, disposition)
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
            self.assertEqual(c.json.loads(path.read_text()), disposition)
            with self.assertRaises(FileExistsError):
                c.write_disposition(path, disposition)

    def test_qualified_checkpoint_chain_retains_disposition(self):
        first = qualified_item()
        link = {"key": first["key"], "version_id": "v1", "sha256": first["sha256"],
                "head_id": 12, "head_hash": first["body"]["head_hash"]}
        second = qualified_item(2, 13, link)
        self.assertEqual(self.validate([first, second])["body"]["format"], 2)
        changed = qualified_item(2, 13, link)
        changed["body"]["legacy_disposition"]["root_cause"] = "changed"
        changed["sha256"] = c.hashlib.sha256(c.canonical(changed["body"])).hexdigest()
        with self.assertRaisesRegex(RuntimeError, "changes the retained legacy disposition"):
            self.validate([first, changed])

    def test_qualified_checkpoint_rejects_format_downgrade(self):
        first = qualified_item()
        link = {"key": first["key"], "version_id": "v1", "sha256": first["sha256"],
                "head_id": 12, "head_hash": first["body"]["head_hash"]}
        downgraded = item(2, 13, link)
        downgraded["body"]["head_hash"] = "2" * 64
        downgraded["sha256"] = c.hashlib.sha256(c.canonical(downgraded["body"])).hexdigest()
        with self.assertRaisesRegex(RuntimeError, "changes checkpoint format"):
            self.validate([first, downgraded])

    def test_main_rechecks_expiry_after_database_scan_in_both_modes(self):
        argv = ["--bucket", "b", "--bucket-owner", "123456789012", "--region", "r",
            "--profile", "p", "--writer-arn", ARN, "--database-uuid", DB,
            "--database-name", "db", "--retention-days", "1"]
        for retained, qualified in ((item(), False), (qualified_item(), True)):
            fake = mock.Mock()
            fake.identity.return_value = {"Account": "123456789012",
                "Arn": "arn:aws:sts::123456789012:assumed-role/checkpoint/session"}
            fake.load.return_value = [retained]
            times = iter((dt.datetime(2029, 1, 1, tzinfo=dt.timezone.utc),
                          dt.datetime(2031, 1, 1, tzinfo=dt.timezone.utc)))

            def timed_validate(objects, db_uuid, writer_arn):
                now = next(times)
                return c.validate_chain(objects, db_uuid, writer_arn, now), now

            if qualified:
                disposition, baseline, defects, value = qualified_values()
                prior = continuity(12); prior.update(snapshot_head_hash="1" * 64, legacy_rows=0)
                value["continuity"] = prior
                database_patch = mock.patch.object(c, "read_qualified_database",
                    return_value=(value, baseline, defects))
            else:
                value = report(1, continuity(1)); value["pages"][0].update(
                    total_rows=1, verified_ok_rows=1, checked_through_id=1, legacy_rows=1,
                    snapshot_head_hash="1" * 64)
                value["continuity"].update(snapshot_head_hash="1" * 64)
                database_patch = mock.patch.object(c, "read_database",
                    return_value=(value, retained["body"]["legacy_baseline"]))
            with self.subTest(qualified=qualified), mock.patch.object(c, "S3", return_value=fake), \
                    mock.patch.object(c, "validate_chain_now", side_effect=timed_validate), database_patch, \
                    self.assertRaisesRegex(RuntimeError, "active COMPLIANCE retention"):
                c.main(argv)
            fake.put.assert_not_called()

    def test_main_rechecks_expiry_after_s3_readback_before_success(self):
        retained = item()
        fake = mock.Mock()
        fake.identity.return_value = {"Account": "123456789012",
            "Arn": "arn:aws:sts::123456789012:assumed-role/checkpoint/session"}
        fake.load.side_effect = ([retained], [retained])
        fake.put.return_value = {"VersionId": "new-version"}
        times = iter((dt.datetime(2029, 1, 1, tzinfo=dt.timezone.utc),
                      dt.datetime(2029, 1, 2, tzinfo=dt.timezone.utc),
                      dt.datetime(2031, 1, 1, tzinfo=dt.timezone.utc)))

        def timed_validate(objects, db_uuid, writer_arn):
            now = next(times)
            return c.validate_chain(objects, db_uuid, writer_arn, now), now

        value = report(2, continuity(1)); value["pages"][0].update(
            total_rows=2, verified_ok_rows=2, checked_from_id=1, checked_through_id=2,
            legacy_rows=1, snapshot_head_hash="2" * 64)
        value["continuity"].update(snapshot_head_hash="1" * 64)
        output = io.StringIO()
        with mock.patch.object(c, "S3", return_value=fake), \
                mock.patch.object(c, "validate_chain_now", side_effect=timed_validate), \
                mock.patch.object(c, "read_database",
                    return_value=(value, retained["body"]["legacy_baseline"])), \
                redirect_stdout(output), self.assertRaisesRegex(RuntimeError, "active COMPLIANCE retention"):
            c.main(["--bucket", "b", "--bucket-owner", "123456789012", "--region", "r",
                "--profile", "p", "--writer-arn", ARN, "--database-uuid", DB,
                "--database-name", "db", "--retention-days", "1"])
        fake.put.assert_called_once()
        self.assertEqual(output.getvalue(), "")

    def test_disposition_override_requires_bootstrap(self):
        with self.assertRaisesRegex(RuntimeError, "only for bootstrap"):
            c.main(["--bucket", "b", "--bucket-owner", "123456789012", "--region", "r",
                "--profile", "p", "--writer-arn", ARN, "--database-uuid", DB,
                "--database-name", "db", "--retention-days", "1",
                "--legacy-disposition", "/tmp/disposition.json"])

    def test_disposition_rejects_wrong_identity_and_bounds(self):
        disposition, _, _, _ = qualified_values()
        for update in ({"database_uuid": "not-a-uuid"},
                       {"legacy_baseline": dict(disposition["legacy_baseline"], last_id=9)}):
            broken = c.json.loads(c.json.dumps(disposition)); broken.update(update)
            with self.assertRaisesRegex(RuntimeError, "UUID|cutoff"):
                c.validate_disposition(broken)


if __name__ == "__main__":
    unittest.main()
