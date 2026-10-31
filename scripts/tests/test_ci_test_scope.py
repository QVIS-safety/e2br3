import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from ci_test_scope import changed_paths


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
        self.assertEqual(changed_paths(head, head, False, self.root), [])

    def test_missing_history_fails_instead_of_skipping_tests(self):
        with self.assertRaises(subprocess.CalledProcessError):
            changed_paths("1" * 40, self.base, False, self.root)
        with self.assertRaises(ValueError):
            changed_paths("0" * 40, self.base, True, self.root)

    def test_event_reader_uses_event_revisions_instead_of_checkout_sha(self):
        self.write("crates/libs/validator/src/lib.rs", "validator")
        head = self.commit()
        for name, event in (
            ("push", {"before": self.base, "after": head}),
            ("pull_request", {"pull_request": {"base": {"sha": self.base}, "head": {"sha": head}}}),
            ("workflow_dispatch", {}),
        ):
            with self.subTest(event=name):
                event_file = self.root / "event.json"
                event_file.write_text(json.dumps(event))
                result = subprocess.run(
                    [sys.executable, "-c", "import json; from ci_test_scope import event_revisions; print(json.dumps(event_revisions()))"],
                    cwd=self.root, check=True, capture_output=True, text=True,
                    env={**os.environ, "GITHUB_EVENT_NAME": name,
                         "GITHUB_EVENT_PATH": str(event_file),
                         "GITHUB_SHA": head if name == "workflow_dispatch" else "wrong-checkout-sha",
                         "PYTHONPATH": str(Path(__file__).resolve().parents[1])},
                )
                expected_base = "0" * 40 if name == "workflow_dispatch" else self.base
                self.assertEqual(json.loads(result.stdout), [name, expected_base, head])
