import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from ci_test_scope import changed_paths, select_tests


class TestSelectionTests(unittest.TestCase):
    def test_validator_only_changes_skip_other_tests(self):
        for suffix in ("src/lib.rs", "rules.json", "field-rules/ci.json", "tools/check_rule_tests.py"):
            with self.subTest(path=suffix):
                self.assertEqual(select_tests(["crates/libs/validator/" + suffix]),
                                 {"validator": True, "backend": False})

    def test_unrelated_changes_skip_validator(self):
        for path in ("crates/services/web-server/src/lib.rs", "crates/libs/xml/src/lib.rs",
                     "registry/execution-ledger.json", "db/migrations/change.sql",
                     "db/bootstrap/10-triggers.sql", "README.md"):
            with self.subTest(path=path):
                self.assertEqual(select_tests([path]), {"validator": False, "backend": True})

    def test_shared_inputs_and_mixed_changes_run_both(self):
        for path in (
            "crates/libs/lib-core/src/model.rs", "crates/libs/lib-auth/src/lib.rs",
            "crates/libs/lib-utils/src/lib.rs", "crates/libs/input-contracts/src/lib.rs",
            "registry/editor-contracts/si.json", "registry/sections/c-safety-report.json",
            "registry/vocabularies/iso639-2.json", "Cargo.toml", "Cargo.lock",
            ".cargo/config.toml", "rust-toolchain.toml", "rustfmt.toml",
            ".github/workflows/ci.yml", "scripts/ci_test_scope.py",
            "scripts/tests/test_ci_test_scope.py",
        ):
            with self.subTest(path=path):
                self.assertEqual(select_tests([path]), {"validator": True, "backend": True})
        self.assertEqual(select_tests(["crates/libs/validator/src/lib.rs", "README.md"]),
                         {"validator": True, "backend": True})

    def test_no_changed_files_selects_no_tests(self):
        self.assertEqual(select_tests([]), {"validator": False, "backend": False})


class GitDiffTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.git("init", "-q")
        self.git("config", "user.name", "CI scope test")
        self.git("config", "user.email", "ci-scope@example.invalid")
        self.git("config", "commit.gpgsign", "false")
        self.git("config", "core.hooksPath", "/dev/null")
        self.write("README.md", "initial")
        self.base = self.commit()

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, stderr=subprocess.PIPE).decode().strip()

    def write(self, path, content):
        file = self.root / path
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(content)

    def commit(self):
        self.git("add", "-A")
        self.git("commit", "-qm", "test change")
        return self.git("rev-parse", "HEAD")

    def test_push_includes_every_commit_and_handles_spaces(self):
        self.write("crates/libs/validator/src/a file.rs", "validator")
        self.commit()
        self.write("crates/services/web-server/src/lib.rs", "backend")
        head = self.commit()
        paths = changed_paths(self.base, head, False, self.root)
        self.assertEqual(set(paths), {"crates/libs/validator/src/a file.rs",
                                     "crates/services/web-server/src/lib.rs"})
        self.assertEqual(select_tests(paths), {"validator": True, "backend": True})

    def test_pull_request_ignores_unmerged_base_branch_changes(self):
        self.write("crates/services/web-server/src/lib.rs", "base-only change")
        base_tip = self.commit()
        self.git("checkout", "-qb", "feature", self.base)
        self.write("crates/libs/validator/src/lib.rs", "validator")
        head = self.commit()
        self.assertEqual(changed_paths(base_tip, head, True, self.root),
                         ["crates/libs/validator/src/lib.rs"])

    def test_renames_across_boundary_and_deletions_select_both(self):
        self.write("crates/libs/validator/src/lib.rs", "validator")
        base = self.commit()
        self.git("mv", "crates/libs/validator/src/lib.rs", "moved.rs")
        head = self.commit()
        self.assertEqual(set(changed_paths(base, head, False, self.root)),
                         {"crates/libs/validator/src/lib.rs", "moved.rs"})
        (self.root / "moved.rs").unlink()
        deleted = self.commit()
        self.assertEqual(changed_paths(head, deleted, False, self.root), ["moved.rs"])

    def test_new_branch_checks_entire_tree_and_empty_diff_selects_nothing(self):
        self.write("crates/libs/validator/src/lib.rs", "validator")
        head = self.commit()
        paths = changed_paths("0" * 40, head, False, self.root)
        self.assertEqual(select_tests(paths), {"validator": True, "backend": True})
        self.assertEqual(changed_paths(head, head, False, self.root), [])

    def test_missing_history_fails_instead_of_skipping_tests(self):
        with self.assertRaises(subprocess.CalledProcessError):
            changed_paths("1" * 40, self.base, False, self.root)
        with self.assertRaises(ValueError):
            changed_paths("0" * 40, self.base, True, self.root)

    def test_cli_reads_push_and_pr_events_and_writes_job_outputs(self):
        self.write("crates/libs/validator/src/lib.rs", "validator")
        head = self.commit()
        for name, event in (
            ("push", {"before": self.base, "after": head}),
            ("pull_request", {"pull_request": {"base": {"sha": self.base}, "head": {"sha": head}}}),
        ):
            with self.subTest(event=name):
                event_file = self.root / "event.json"
                event_file.write_text(json.dumps(event))
                output = self.root / (name + "-output")
                result = subprocess.run(
                    [sys.executable, str(Path(__file__).resolve().parents[1] / "ci_test_scope.py")],
                    cwd=self.root, check=True, capture_output=True, text=True,
                    env={**os.environ, "GITHUB_EVENT_NAME": name,
                         "GITHUB_EVENT_PATH": str(event_file), "GITHUB_OUTPUT": str(output)},
                )
                self.assertEqual(output.read_text(), "validator=true\nbackend=false\n")
                self.assertEqual(json.loads(result.stdout)["tests"],
                                 {"validator": True, "backend": False})
