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
    def test_catalog_matches_implemented_business_rules(self):
        check_rule_tests.check_catalog()

    def test_discovery_includes_collector_dynamic_and_retained_business_codes(self):
        from rule_inventory import discover_business_rule_codes
        codes = discover_business_rule_codes()
        self.assertTrue({
            "FDA.C.1.6.1.r.2.FILE_NAME.REQUIRED",
            "FDA.C.1.6.1.r.2.MEDIA_TYPE.MATCH",
            "FDA.C.4.r.2.FILE_NAME.REQUIRED",
            "FDA.C.4.r.2.MEDIA_TYPE.MATCH",
            "MFDS.D.2.3.ALLOWED.VALUE",
            "ICH.D.7.1.r.6.ALLOWED.VALUE",
            "ICH.D.7.3.ALLOWED.VALUE",
            "ICH.D.10.2.2b.ALLOWED.VALUE",
            "ICH.G.k.2.5.ALLOWED.VALUE",
        } <= codes)
        consistency = {code for code in codes if code.endswith(".MEDDRA.VERSION.CONSISTENT")}
        self.assertEqual(len(consistency), 12)
        self.assertNotIn("ICH.C.1", codes)
        self.assertNotIn("FDA.C.1", codes)
        self.assertNotIn("ICH.MEDDRA.VERSION.UNAVAILABLE", codes)


    def test_result_gate_rejects_missing_ignored_failed_duplicate_and_extra_tests(self):
        expected = {"c::rule_one", "d::rule_two"}
        valid = "test c::rule_one ... ok\ntest d::rule_two ... ok\n"
        check_rule_tests.check_test_log(valid, expected)
        for invalid in (
            "test c::rule_one ... ok\n",
            valid.replace("rule_two ... ok", "rule_two ... ignored"),
            valid.replace("rule_two ... ok", "rule_two ... FAILED"),
            valid + "test c::rule_one ... ok\n",
            valid + "test e::rule_extra ... ok\n",
            valid.replace("d::rule_two", "e::rule_two"),
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

    def test_catalog_gate_rejects_invalid_authority_modules(self):
        original = json.loads((CRATE / "rules.json").read_text())
        rule = original["rules"][0]
        authority = rule["ruleCode"].split(".", 1)[0].lower()
        other = "ich" if authority == "fda" else "fda"
        for module in (
            "case::sections::c::rule_tests",
            f"case::sections::c::rule_tests::{other}",
            f"case::sections::z::rule_tests::{authority}",
        ):
            catalog = copy.deepcopy(original)
            catalog["rules"][0]["testModule"] = module
            with self.subTest(module=module), tempfile.TemporaryDirectory() as directory:
                crate = Path(directory)
                (crate / "rules.json").write_text(json.dumps(catalog))
                with self.assertRaisesRegex(ValueError, "invalid Rust test module"):
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
