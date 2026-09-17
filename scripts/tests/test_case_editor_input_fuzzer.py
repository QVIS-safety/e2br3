#!/usr/bin/env python3

import random
import sys
import tempfile
import unicodedata
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import case_editor_input_fuzzer as fuzzer
import case_editor_ui_fuzzer as ui_fuzzer
import rbac_rls_blackbox


class CaseEditorInputFuzzerTests(unittest.TestCase):
    def test_nested_rows_and_list_readback(self) -> None:
        self.assertEqual(
            fuzzer.nested_root("fdaCrossReportedIndNumbers[].indNumber"),
            "fdaCrossReportedIndNumbers[]",
        )
        self.assertEqual(
            fuzzer.nested_root("drugReactionAssessments[].sourceOfAssessment"),
            "drugReactionAssessments[]",
        )
        self.assertIsNone(fuzzer.nested_root("raceCodes[]"))
        self.assertIsNone(fuzzer.nested_root("[].registrationNumber"))
        self.assertTrue(fuzzer.values_equal(["\t\n"], ["\t\n"]))
        self.assertTrue(fuzzer.values_equal(64.5, ["64.50"]))
        self.assertFalse(fuzzer.values_equal(64.5, ["64.50", "99"]))
        self.assertFalse(fuzzer.values_equal(["x"], ["y"]))

    def test_unicode_and_exact_length_candidates(self) -> None:
        field = {"roundTripValue": "base", "_maxLength": 4}
        self.assertEqual(fuzzer.candidate_count(field, 99), 17)
        values = [fuzzer.field_value(field, fuzzer.candidate_rng(1, field, ordinal, 0), ordinal) for ordinal in range(8, 17)]
        self.assertNotEqual(values[0], unicodedata.normalize("NFC", values[0]))
        self.assertTrue(any(char in values[1] for char in "\u202b\u202e\u2067"))
        self.assertTrue(any(char in values[2] for char in "\u200b\u200c\u200d\ufeff\u2060"))
        self.assertGreater(len(values[3]), 64)
        self.assertGreaterEqual(ord(values[4]), 0xD800)
        self.assertLessEqual(ord(values[4]), 0xDFFF)
        self.assertTrue(any(char in values[5] for char in "\ufffd\ufffe\uffff\U0010ffff"))
        self.assertEqual(len(values[6]), 4)
        self.assertEqual(len(values[7]), 5)
        self.assertGreater(len(values[8]), 5)
        self.assertEqual(fuzzer.candidate_expectation(field, 14), ("length_boundary", None))
        self.assertEqual(fuzzer.candidate_expectation(field, 16), ("reject", None))

    def test_generated_max_lengths_and_candidate_caps(self) -> None:
        root = Path(__file__).resolve().parents[2]
        limits = fuzzer.load_max_lengths(root)
        self.assertEqual(limits["ICH.C.1.1.LENGTH.MAX"], 100)
        contract = [{"fields": [{
            "authority": "ICH",
            "code": "C.1.11.1",
            "roundTripValue": "base",
            "constraint": {"ruleCode": "ICH.C.1.11.1.ALLOWED.VALUE"},
        }]}]
        self.assertEqual(fuzzer.apply_max_lengths(contract, limits), 1)
        self.assertEqual(contract[0]["fields"][0]["_maxLength"], 1)
        self.assertEqual(fuzzer.candidate_count({"roundTripValue": 1}, 17), 8)
        self.assertEqual(fuzzer.candidate_count({"roundTripValue": ["x"]}, 17), 14)

    def test_generated_identifier_boolean_rules_and_expectations(self) -> None:
        root = Path(__file__).resolve().parents[2]
        identifiers = fuzzer.load_generated_rules(root, fuzzer.IDENTIFIER_RE)
        booleans = fuzzer.load_generated_rules(root, fuzzer.BOOLEAN_RE)
        field = {
            "authority": "ICH",
            "code": "D.10.8.r.2b",
            "roundTripValue": "base",
            "constraint": {"invalidValue": "bad", "ruleCode": "ICH.D.10.8.r.2b.ALLOWED.VALUE"},
        }
        boolean = {
            "authority": "ICH",
            "code": "C.1.9.1",
            "roundTripValue": True,
            "constraint": {"invalidValue": "not-a-boolean", "ruleCode": "ICH.C.1.9.1.ALLOWED.VALUE"},
        }
        contract = [{"fields": [field, boolean]}]
        self.assertEqual(fuzzer.apply_generated_rules(contract, identifiers, booleans), (1, 1))
        self.assertEqual(fuzzer.candidate_count(field, 99), 18)
        self.assertTrue(any(char in fuzzer.field_value(field, random.Random(1), 17) for char in "\t\n\r"))
        self.assertEqual(
            fuzzer.candidate_expectation(field, 17),
            ("reject", "ICH.D.10.8.r.2b.ALLOWED.VALUE"),
        )
        self.assertFalse(fuzzer.field_value(boolean, random.Random(1), 3))
        self.assertTrue(fuzzer.field_value(boolean, random.Random(1), 5))
        self.assertFalse(fuzzer.field_value(boolean, random.Random(1), 6))
        self.assertEqual(fuzzer.candidate_expectation(boolean, 3), ("accept", None))
        self.assertEqual(fuzzer.candidate_expectation(boolean, 5), ("accept_or_forbidden", None))
        self.assertEqual(fuzzer.candidate_expectation(boolean, 6), ("accept", None))
        self.assertEqual(
            fuzzer.candidate_expectation(boolean, 2),
            ("reject", "ICH.C.1.9.1.ALLOWED.VALUE"),
        )

    def test_camel_case_rule_code_and_exact_max_expectation(self) -> None:
        self.assertEqual(
            fuzzer.summarize_error_detail({"ruleCode": "ICH.C.1.LENGTH.MAX", "message": "bad"})["error_rule_code"],
            "ICH.C.1.LENGTH.MAX",
        )
        self.assertIsNone(fuzzer.expectation_error(("reject", "RULE"), 422, "RULE"))
        self.assertIsNotNone(fuzzer.expectation_error(("reject", "RULE"), 422, "OTHER"))
        self.assertIsNotNone(fuzzer.expectation_error(("length_boundary", "LENGTH"), 422, "FORMAT"))
        self.assertIsNone(fuzzer.expectation_error(("length_boundary", "LENGTH"), 200, None))
        self.assertIsNotNone(fuzzer.expectation_error(("length_boundary", "LENGTH"), 422, "LENGTH"))
        self.assertIsNone(fuzzer.expectation_error(("accept_or_forbidden", None), 403, None))

    def test_normalization_requires_audit_only_when_value_changes(self) -> None:
        self.assertEqual(fuzzer.normalized_classification("old", "old", False), "CLEAR_NOT_APPLIED")
        self.assertEqual(fuzzer.normalized_classification("new", "old", True), "NORMALIZATION_UNVERIFIED")
        self.assertEqual(fuzzer.normalized_classification(None, "old", True), "SAVE_NORMALIZED")
        self.assertEqual(fuzzer.normalized_classification(None, "old", False), "AUDIT_MISMATCH")
        self.assertEqual(fuzzer.normalized_classification(None, None, False), "NOOP_ACCEPTED")
        self.assertEqual(fuzzer.audit_field_key("safetyReportId"), "safety_report_id")
        self.assertEqual(fuzzer.audit_field_key("reporterCountry"), "country_code")
        self.assertTrue(fuzzer.audit_key_matches(
            {"drug_additional_info_codes_json.unexpected": {"old": None, "new": 1}},
            "drug_additional_info_codes_json",
        ))
        self.assertFalse(fuzzer.audit_key_matches(
            {"reporter_email_backup": {"old": None, "new": "x"}},
            "reporterEmail",
        ))
        self.assertIn("drug_additional_info_codes_json", fuzzer.UNFILTERED_AUDIT_FIELDS)
        self.assertTrue(fuzzer.audit_log_complete({
            "user_id": "u", "organization_id": "o", "created_at": "t", "action": "UPDATE",
            "changed_fields": {"json_field.child": {"old": 1, "new": 2}},
            "old_values": {"json_field": {"child": 1}}, "new_values": {"json_field": {"child": 2}},
            "prev_hash": "p", "entry_hash": "e",
        }))

    def test_commit_sha_is_cached(self) -> None:
        rbac_rls_blackbox.commit_sha.cache_clear()
        with mock.patch.object(
            rbac_rls_blackbox.subprocess,
            "check_output",
            return_value="abc123\n",
        ) as check_output:
            self.assertEqual(rbac_rls_blackbox.commit_sha(), "abc123")
            self.assertEqual(rbac_rls_blackbox.commit_sha(), "abc123")
        check_output.assert_called_once()
        rbac_rls_blackbox.commit_sha.cache_clear()

    def test_api_verdict_never_promotes_setup_missing_or_unknown_checks(self) -> None:
        saved = fuzzer.Event("mutation", "D.1", "DM", "patient", "one", "SAVE_ACCEPTED", 200, {})
        baseline = fuzzer.Event("baseline", None, "DM", "patient", None, "PASS", 200, {})
        self.assertEqual(fuzzer.run_verdict([baseline], 0, None, [])["verdict"], "INCOMPLETE_OR_FAILED")
        self.assertEqual(fuzzer.run_verdict([saved], 2, None, [])["executed_mutations"], 1)
        for planned, interrupted, unsupported in [(2, None, []), (1, "deadline", []), (1, None, ["DG:device"])]:
            self.assertEqual(fuzzer.run_verdict([saved], planned, interrupted, unsupported)["verdict"], "INCOMPLETE_OR_FAILED")
        for classification in ["NO_EXPECTATION", "CLEAR_NOT_APPLIED", "NORMALIZATION_UNVERIFIED", "BOUNDARY_BLOCKED", "AUTHORIZATION_BLOCKED", "FUTURE_UNRECOGNIZED_STATUS"]:
            event = fuzzer.Event("mutation", "D.1", "DM", "patient", "one", classification, 200, {})
            self.assertEqual(fuzzer.run_verdict([event], 1, None, [])["verdict"], "INCOMPLETE_OR_FAILED")
        result = fuzzer.run_verdict([saved], 1, None, [])
        self.assertEqual(result["verdict"], "SELECTED_API_CHECKS_PASSED")
        self.assertFalse(result["official_compliance_verified"])
        self.assertFalse(result["ui_verified"])
        created = fuzzer.Event("create", None, None, None, None, "PASS", 201, {})
        setup = fuzzer.run_verdict([created, baseline], 0, None, ["DG:device"], setup_only=True)
        self.assertEqual(setup["verdict"], "BASELINE_ONLY")
        self.assertEqual(setup["executed_mutations"], 0)
        self.assertEqual(setup["unsupported_null_flavor_pairs"], ["DG:device"])
        gate = fuzzer.Event("validator_gate", None, None, None, None, "GATE_PASS", None, {})
        self.assertEqual(fuzzer.run_verdict([saved, gate], 1, None, [])["verdict"], "SELECTED_API_CHECKS_PASSED")
        gate.classification = "GATE_BLOCKED"
        self.assertEqual(fuzzer.run_verdict([saved, gate], 1, None, [])["verdict"], "INCOMPLETE_OR_FAILED")

    def test_unmapped_pairs_are_explicit_and_other_callers_still_fail_closed(self) -> None:
        contract = [{"pageId": "DG", "fields": []}]
        pairs = {"DG": [{"value": "fdaDevices[].deviceBrandName", "nullFlavor": "fdaDevices[].brandNF"}]}
        with self.assertRaisesRegex(ValueError, "unresolved NullFlavor pairs"):
            fuzzer.expand_null_flavor_contracts(contract, pairs, {})
        unsupported = []
        self.assertEqual(fuzzer.expand_null_flavor_contracts(contract, pairs, {}, unsupported), 0)
        self.assertEqual(unsupported, ["DG:fdaDevices[].deviceBrandName"])

    def test_seeded_samples_are_reproducible_and_vary(self) -> None:
        field = {"authority": "ICH", "code": "H.1", "payloadPath": "caseNarrative", "roundTripValue": "base"}
        first = fuzzer.field_value(field, fuzzer.candidate_rng(11, field, 5, 0), 5)
        repeated = fuzzer.field_value(field, fuzzer.candidate_rng(11, field, 5, 0), 5)
        another_seed = fuzzer.field_value(field, fuzzer.candidate_rng(12, field, 5, 0), 5)
        another_sample = fuzzer.field_value(field, fuzzer.candidate_rng(11, field, 5, 1), 5, 1)
        self.assertEqual(first, repeated)
        self.assertNotEqual(first, another_seed)
        self.assertNotEqual(first, another_sample)
        self.assertEqual(fuzzer.candidate_sample_count(field, 0, 3), 1)
        self.assertEqual(fuzzer.candidate_sample_count(field, 5, 3), 3)

    def test_generated_candidates_have_no_generic_fallback(self) -> None:
        field = {
            "authority": "ICH",
            "code": "H.1",
            "frontendPath": "narrative.caseNarrative",
            "payloadPath": "caseNarrative",
            "roundTripValue": "base",
            "constraint": {"invalidValue": "bad"},
        }
        candidates = [
            fuzzer.field_value(field, fuzzer.candidate_rng(11, field, 8, sample), 8, sample)
            for sample in range(3)
        ]
        self.assertEqual(len(set(candidates)), 3)
        self.assertEqual(len({
            fuzzer.candidate_fingerprint(field, 8, sample, candidate)
            for sample, candidate in enumerate(candidates)
        }), 3)
        with self.assertRaises(ValueError):
            fuzzer.field_value(field, random.Random(1), 18)
        self.assertEqual(fuzzer.parser().parse_args([]).samples_per_category, 3)
        self.assertEqual(fuzzer.parser().parse_args(["--field", "H.1"]).field, ["H.1"])

    def test_nullflavor_error_candidates_and_value_conflict(self) -> None:
        field = {
            "code": "D.1.nullFlavor",
            "payloadPath": "patientBirthDateNullFlavor",
            "roundTripValue": "UNK",
            "constraint": {"status": "verified", "invalidValue": "ZZZ"},
            "_allowedNullFlavors": ["UNK"],
            "_nullFlavorPartnerPath": "patientBirthDate",
            "_nullFlavorPartnerValue": "20000101",
        }
        rng = random.Random(1)
        self.assertEqual(fuzzer.candidate_count(field, 17), 14)
        for ordinal in (0, 5, 8, 9, 10, 11, 12):
            self.assertTrue(fuzzer.nullflavor_invalid_candidate(field, fuzzer.field_value(field, rng, ordinal)))
        mutation = {"patientBirthDateNullFlavor": "UNK"}
        self.assertTrue(fuzzer.add_nullflavor_partner(field, mutation, 13))
        self.assertEqual(mutation["patientBirthDate"], "20000101")
        self.assertEqual(fuzzer.candidate_kind(field, 13), "nullflavor_with_value")

    def test_complete_baseline_prefers_values_over_null_flavors(self) -> None:
        value = {
            "code": "D.2.1", "payloadPath": "patientBirthDate", "roundTripValue": "20000101",
            "constraint": {"status": "verified"},
        }
        null_flavor = {
            "code": "D.2.1.nullFlavor", "payloadPath": "patientBirthDateNullFlavor", "roundTripValue": "UNK",
            "constraint": {"status": "verified"}, "_allowedNullFlavors": ["UNK"],
        }
        self.assertEqual(fuzzer.baseline_for([value, null_flavor]), {"patientBirthDate": "20000101"})
        self.assertEqual(
            fuzzer.baseline_for(
                [{"code": "G.k.2.5", "payloadPath": "investigationalProductBlinded", "roundTripValue": True,
                  "constraint": {"status": "verified"}}],
                safe=True,
            ),
            {"investigationalProductBlinded": False},
        )
        self.assertTrue(fuzzer.parser().parse_args(["--complete-baseline"]).complete_baseline)

    def test_meddra_baseline_override_updates_only_meddra_fields(self) -> None:
        contract = [{"fields": [
            {"code": "E.i.2.1a", "frontendPath": "reactionMeddraVersionLLT", "roundTripValue": "26.0"},
            {"code": "E.i.2.1b", "frontendPath": "reactionMeddraCodeLLT", "roundTripValue": "10000001"},
            {"code": "E.i.1.1b", "frontendPath": "reactionLanguage", "roundTripValue": "eng"},
        ]}]
        self.assertEqual(fuzzer.apply_meddra_baseline(contract, "28.1", "10000001"), 1)
        self.assertEqual(contract[0]["fields"][0]["roundTripValue"], "28.1")
        self.assertEqual(contract[0]["fields"][1]["roundTripValue"], "10000001")
        self.assertEqual(contract[0]["fields"][2]["roundTripValue"], "eng")

    def test_ui_plan_covers_all_fields_and_seeded_candidates(self) -> None:
        args = ui_fuzzer.parser().parse_args(["--seed", "2026081401", "--dry-run"])
        plan = ui_fuzzer.build_plan(args)
        self.assertGreaterEqual(plan["fieldCount"], 297)
        self.assertGreaterEqual(plan["mutationCount"], 10_000)
        self.assertEqual(len({field["code"] for field in plan["fields"]}), plan["fieldCount"])
        null_flavor = next(field for field in plan["fields"] if field["nullFlavor"])
        invalid = next(mutation for mutation in null_flavor["mutations"] if mutation["kind"] == "nullflavor_unknown")
        conflict = next(mutation for mutation in null_flavor["mutations"] if mutation["kind"] == "nullflavor_with_value")
        self.assertEqual(invalid["expectation"], "reject")
        self.assertEqual(conflict["expectation"], "reject")
        shards = [
            ui_fuzzer.build_plan(ui_fuzzer.parser().parse_args([
                "--seed", "2026081401", "--shard-count", "3", "--shard-index", str(index), "--dry-run",
            ]))
            for index in range(3)
        ]
        self.assertEqual(sum(shard["fieldCount"] for shard in shards), plan["fieldCount"])
        self.assertEqual(sum(shard["mutationCount"] for shard in shards), plan["mutationCount"])

    def test_ui_wrapper_fails_incomplete_or_empty_runs(self) -> None:
        with tempfile.TemporaryDirectory() as artifact_dir:
            args = ui_fuzzer.parser().parse_args([
                "--seed", "1", "--case-id", "case-id", "--pwcli", "unused",
                "--artifact-dir", artifact_dir,
            ])
            plan = {"fieldCount": 1, "mutationCount": 1, "fields": []}
            with (
                mock.patch.object(ui_fuzzer, "build_plan", return_value=plan),
                mock.patch.object(ui_fuzzer, "guard_target"),
            ):
                for classification in ("NOT_RUN", "UNRENDERABLE", "NO_EXPECTATION"):
                    incomplete = {
                        "counts": {classification: 1},
                        "results": [{"classification": classification}],
                    }
                    with mock.patch.object(ui_fuzzer, "run_browser", return_value=incomplete):
                        self.assertEqual(ui_fuzzer.main(args), 1)

            with mock.patch.object(
                ui_fuzzer,
                "build_plan",
                return_value={"fieldCount": 0, "mutationCount": 0, "fields": []},
            ):
                self.assertEqual(ui_fuzzer.main(args), 1)


if __name__ == "__main__":
    unittest.main()
