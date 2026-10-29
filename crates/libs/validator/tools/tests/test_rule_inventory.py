import copy
import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

CRATE = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(CRATE / "tools"))
import check_rule_tests


class RuleInventoryTests(unittest.TestCase):
    def test_catalog_matches_implemented_298_rule_baseline(self):
        self.assertEqual(len(check_rule_tests.check_catalog()), 298)


    def test_result_gate_rejects_missing_ignored_failed_duplicate_and_extra_tests(self):
        expected = {"rule_one", "field_ci_path"}
        valid = "test c::rule_one ... ok\ntest field_tests::ci::field_ci_path ... ok\n"
        check_rule_tests.check_test_log(valid, expected)
        for invalid in (
            "test c::rule_one ... ok\n",
            valid.replace("field_ci_path ... ok", "field_ci_path ... ignored"),
            valid.replace("field_ci_path ... ok", "field_ci_path ... FAILED"),
            valid + "test e::rule_one ... ok\n",
            valid + "test e::rule_extra ... ok\n",
        ):
            with self.subTest(log=invalid), self.assertRaises(ValueError):
                check_rule_tests.check_test_log(invalid, expected)

    def test_catalog_gate_rejects_stale_rule_and_field_bindings(self):
        original = json.loads((CRATE / "rules.json").read_text())
        for mutation in ("missing", "duplicate", "target", "test", "issue_code"):
            catalog = copy.deepcopy(original)
            if mutation == "missing":
                catalog["rules"].pop()
            elif mutation == "duplicate":
                catalog["rules"].append(catalog["rules"][0])
            elif mutation == "target":
                catalog["rules"][0]["expectedIssuePaths"] = []
            elif mutation == "test":
                catalog["rules"][0]["testName"] = "rule_wrong"
            else:
                catalog["rules"][0]["expectedIssueCode"] = "WRONG.RULE"
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory:
                crate = Path(directory)
                (crate / "rules.json").write_text(json.dumps(catalog))
                with self.assertRaises(ValueError):
                    check_rule_tests.check_catalog(crate)

    def test_field_json_cannot_drop_or_misbind_a_rule(self):
        for mutation in ("missing", "wrong_rule", "wrong_path"):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory:
                crate = Path(directory)
                shutil.copy(CRATE / "rules.json", crate / "rules.json")
                shutil.copytree(CRATE / "field-rules", crate / "field-rules")
                path = crate / "field-rules/ci.json"
                page = json.loads(path.read_text())
                if mutation == "missing":
                    page["fields"].pop()
                elif mutation == "wrong_rule":
                    page["fields"][0]["ruleCodes"] = ["ICH.H.1.REQUIRED"]
                else:
                    page["fields"][0]["fieldPath"] = "wrong.field"
                path.write_text(json.dumps(page))
                with self.assertRaises(ValueError):
                    check_rule_tests.check_catalog(crate)
