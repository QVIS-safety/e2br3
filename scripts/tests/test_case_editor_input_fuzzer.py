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
import presave_case_roundtrip_fuzzer as presave_fuzzer
import rich_case_export_import_fuzzer as rich_fuzzer


class PresaveVerdictTests(unittest.TestCase):
    def test_receiver_local_storage_contract(self) -> None:
        fields = presave_fuzzer.load_fields(["receiver"], unsupported=[])
        self.assertEqual(len(fields), 26)
        self.assertEqual(sum(field["model"] == "ReceiverPresave" for field in fields), 12)
        by_name = {field["backendField"]: field for field in fields}
        count = by_name["nsae_solicited_day_count"]
        for value, expected in [(None, ("accept", None)), (0, ("accept", None)),
                                (-1, ("reject", "INVALID_REQUEST")),
                                (True, ("reject", "INPUT.JSON.INVALID")),
                                (2147483648, ("reject", "INPUT.JSON.INVALID"))]:
            self.assertEqual(presave_fuzzer.presave_expectation(count, value), expected)
        self.assertEqual(presave_fuzzer.presave_expectation(by_name["description"], ""), ("accept", None))
        self.assertEqual(presave_fuzzer.presave_expectation(by_name["organization_name"], ""), ("reject", "INVALID_REQUEST"))
        self.assertIsNone(by_name["organization_name"]["official"])

    def test_native_loader_has_no_case_editor_contract_dependency(self) -> None:
        inventory = []
        excluded = []
        with (
            mock.patch.object(fuzzer, "DEFAULT_CONTRACT", object()),
            mock.patch.object(fuzzer, "DEFAULT_NULL_FLAVOR_PAIRS", object()),
        ):
            fields = presave_fuzzer.load_fields(
                list(presave_fuzzer.SECTIONS), excluded, [], inventory)

        self.assertEqual(len(fields), 143)
        self.assertFalse(excluded)
        self.assertEqual(
            {model: sum(field["model"] == model for field in fields) for model in presave_fuzzer.MODEL_GROUPS},
            {
                "SenderPresave": 12,
                "SenderPresaveGateway": 7,
                "SenderPresaveResponsiblePerson": 7,
                "ReceiverPresave": 12,
                "ReceiverPresaveConsignee": 4,
                "ReceiverPresaveRoute": 10,
                "ReporterPresave": 29,
                "ProductPresave": 21,
                "ProductPresaveActiveSubstance": 8,
                "StudyPresave": 13,
                "StudyPresaveRegistrationNumber": 5,
                "StudyPresaveFdaCrossReportedIndNumber": 3,
                "NarrativePresave": 4,
                "StudyPresaveProduct": 3,
                "StudyPresaveReporter": 5,
            },
        )
        self.assertFalse(any(field["backendField"] == "deleted" for field in fields))
        self.assertEqual(
            {item["disposition"] for item in inventory},
            {"scalar", "collection", "alias", "lifecycle", "server_owned"},
        )
        self.assertEqual(len(inventory), 171)
        self.assertFalse(any(item["disposition"] == "unresolved" for item in inventory))

    def test_official_provenance_does_not_invent_a_presave_validator(self) -> None:
        fields = presave_fuzzer.load_fields(["sender", "receiver"], unsupported=[])
        for code in ["N.1.4", "N.2.r.2", "N.2.r.3"]:
            field = next(item for item in fields if item["code"] == code)
            self.assertIsNotNone(field["official"])
            self.assertIsNone(field["presaveValidator"])
            self.assertEqual(presave_fuzzer.presave_expectation(
                field, "X" * (field["dto"]["width"] + 1)), ("reject", "INVALID_REQUEST"))
            self.assertEqual(presave_fuzzer.presave_expectation(
                field, "A\x00B"), ("reject", "INVALID_REQUEST"))

    def test_native_loader_preserves_serde_and_relationship_shapes(self) -> None:
        fields = presave_fuzzer.load_fields(["product", "study"], unsupported=[])
        by_target = {(field["model"], field["backendField"]): field for field in fields}
        self.assertEqual(
            by_target[("ProductPresave", "preapproval_ip_name")]["apiField"],
            "preApprovalIpName",
        )
        self.assertEqual(
            by_target[("ProductPresave", "brand_name")]["apiField"],
            "drugBrandName",
        )
        self.assertEqual(
            by_target[("ProductPresaveActiveSubstance", "substance_termid_version")]["apiField"],
            "substanceTermIdVersion",
        )
        for field in fields:
            if "Uuid" in field["dto"]["type"]:
                self.assertEqual(len(field["_presaveCandidates"]), 7)
                self.assertIn("00000000-0000-0000-0000-000000000000", field["_presaveCandidates"][:-1])
                self.assertIsNone(field["_presaveCandidates"][-1])
        self.assertTrue(by_target[("ProductPresave", "receiver_presave_id")]["dto"]["nullClear"])
        self.assertTrue(by_target[("ProductPresave", "investigational_product_blinded")]["dto"]["nullClear"])
        self.assertEqual(
            {(field["model"], field["backendField"], field["group"]) for field in fields
             if field["model"] in {"StudyPresaveProduct", "StudyPresaveReporter"}},
            {
                ("StudyPresaveProduct", "sequence_number", "products"),
                ("StudyPresaveProduct", "product_presave_id", "products"),
                ("StudyPresaveProduct", "product_name", "products"),
                ("StudyPresaveReporter", "sequence_number", "reporters"),
                ("StudyPresaveReporter", "reporter_presave_id", "reporters"),
                ("StudyPresaveReporter", "reporter_organization", "reporters"),
                ("StudyPresaveReporter", "reporter_given_name", "reporters"),
                ("StudyPresaveReporter", "reporter_qualification", "reporters"),
            },
        )

    def test_presave_null_clear_follows_only_explicit_deserializers(self) -> None:
        models = presave_fuzzer.prepared_contract(["sender", "product"])
        self.assertTrue(models["SenderPresave"]["sender_type"]["nullClear"])
        self.assertTrue(models["ProductPresave"]["receiver_presave_id"]["nullClear"])
        self.assertFalse(models["SenderPresaveGateway"]["gateway_authority"]["nullClear"])

    def test_presave_null_expectation_respects_clear_and_identity_contracts(self) -> None:
        fields = presave_fuzzer.load_fields(
            ["sender", "receiver", "product", "study"], unsupported=[])
        by_target = {(field["model"], field["backendField"]): field for field in fields}
        self.assertEqual(
            presave_fuzzer.presave_expectation(
                by_target[("SenderPresave", "email")], None),
            ("accept", None),
        )
        self.assertEqual(
            presave_fuzzer.presave_expectation(
                by_target[("SenderPresave", "sender_type")], None),
            ("reject", "INVALID_REQUEST"),
        )
        self.assertEqual(
            presave_fuzzer.presave_expectation(
                by_target[("SenderPresaveGateway", "gateway_authority")], None),
            ("accept", None),
        )
        self.assertEqual(
            presave_fuzzer.presave_expectation(
                by_target[("ProductPresave", "investigational_product_blinded")], None),
            ("accept", None),
        )
        for value in (None, "", "   "):
            self.assertEqual(
                presave_fuzzer.presave_expectation(
                    by_target[("StudyPresave", "sponsor_study_number_kind")], value),
                ("accept", None),
            )
            self.assertEqual(
                presave_fuzzer.presave_expectation(
                    by_target[("ReceiverPresave", "receiver_type")], value),
                ("reject", "PRESAVE.RECEIVER_TYPE.ALLOWED"),
            )
        for target in (
            ("SenderPresave", "country_code"),
            ("StudyPresaveRegistrationNumber", "country_code"),
        ):
            self.assertEqual(
                presave_fuzzer.presave_expectation(by_target[target], "\t\t\t"),
                ("accept", None),
            )

    def test_registry_non_scalars_are_explicitly_accounted_for(self) -> None:
        inventory = []
        excluded = []
        presave_fuzzer.load_fields(
            list(presave_fuzzer.SECTIONS), excluded, [], inventory)
        counts = {kind: sum(item["disposition"] == kind for item in inventory)
                  for kind in {item["disposition"] for item in inventory}}
        self.assertEqual(counts, {
            "scalar": 143,
            "collection": 9,
            "alias": 7,
            "lifecycle": 3,
            "server_owned": 9,
        })
        self.assertFalse(excluded)
        self.assertTrue(all(item["covered_by"] for item in inventory))

    def test_presave_expectations_distinguish_encoding_storage_and_submission(self) -> None:
        field = {"code": "C.2.r.1.1", "official": {
            "dictionary_file": "ich-e2br3.json", "max_length": "50"}}
        self.assertEqual(presave_fuzzer.presave_expectation(field, "한글"), ("accept", None))
        self.assertEqual(presave_fuzzer.presave_expectation(field, ["text"]), ("reject", "INPUT.JSON.INVALID"))
        self.assertEqual(presave_fuzzer.presave_expectation(field, "a" * 51), ("reject", "ICH.C.2.r.1.1.LENGTH.MAX"))
        self.assertEqual(presave_fuzzer.presave_expectation({"code": "G.k.2.5"}, False), ("reject", "ICH.G.k.2.5.ALLOWED.VALUE"))
        self.assertIsNone(presave_fuzzer.presave_expectation({"code": "unknown"}, "value"))
        country = next(field for field in presave_fuzzer.load_fields(["sender"], unsupported=[])
                       if field["code"] == "C.3.4.5")
        self.assertEqual(presave_fuzzer.presave_expectation(country, ""), ("accept", None))
        self.assertEqual(presave_fuzzer.presave_expectation(country, "   "), ("accept", None))
        mfds = {"code": "C.2.r.4.KR.1", "official": {
            "dictionary_file": "mfds-regional.json", "max_length": "1",
            "allowed_values": "1=간호사\n2=기타"}}
        self.assertEqual(presave_fuzzer.presave_expectation(mfds, "2"), ("accept", None))
        self.assertEqual(presave_fuzzer.presave_expectation(mfds, "3"), ("reject", "MFDS.C.2.r.4.KR.1.ALLOWED.VALUE"))
        self.assertEqual(presave_fuzzer.presave_expectation(mfds, "11"), ("reject", "MFDS.C.2.r.4.KR.1.LENGTH.MAX"))

    def test_presave_length_inputs_do_not_fail_unrelated_type_checks(self) -> None:
        from decimal import Decimal
        email = presave_fuzzer.mutation_value({"code": "C.3.4.8"}, random.Random(1), 0, 0)
        self.assertEqual(len(email), 101)
        self.assertEqual(email.count("@"), 1)
        number = presave_fuzzer.mutation_value({"code": "G.k.2.3.r.3a"}, random.Random(1), 0, 0)
        self.assertEqual(Decimal(number), Decimal("11111111111"))
        self.assertEqual(len(number), 11)

    def test_null_comparison_preserves_nested_values_and_identity(self) -> None:
        before = {"parent": {"updatedAt": 1, "updatedBy": "user"}, "children": [{"id": "a", "value": "kept", "updatedAt": 1}]}
        after = {"parent": {"updatedAt": 2, "updatedBy": "user"}, "children": [{"id": "a", "value": "kept", "updatedAt": 2}]}
        self.assertTrue(presave_fuzzer.unchanged_row_values(before, after))
        after["children"][0]["value"] = None
        self.assertFalse(presave_fuzzer.unchanged_row_values(before, after))
        after["children"][0]["value"] = "kept"
        after["parent"]["updatedBy"] = "other"
        self.assertTrue(presave_fuzzer.unchanged_row_values(before, after))
        after["children"][0]["id"] = "other"
        self.assertFalse(presave_fuzzer.unchanged_row_values(before, after))

    def test_readback_and_audit_cannot_hide_ignored_input(self) -> None:
        for candidate, before, actual, complete, changed, expected in (
            (None, "old", "old", False, False, "NOOP_ACCEPTED"),
            (None, "old", None, True, True, "NULL_IGNORE_MISMATCH"),
            (None, "old", "old", True, True, "NULL_IGNORE_MISMATCH"),
            (None, None, None, False, False, "NOOP_ACCEPTED"),
            ("", "old", "old", False, False, "CLEAR_NOT_APPLIED"),
            ("new", "old", "old", False, False, "SAVE_READBACK_MISMATCH"),
            ("new", "old", "new", False, True, "AUDIT_MISMATCH"),
            ("new", "old", "new", True, True, "SAVE_ACCEPTED"),
            ("old", "old", "old", False, False, "NOOP_ACCEPTED"),
            ("", "old", None, True, True, "SAVE_NORMALIZED"),
        ):
            with self.subTest(expected=expected):
                self.assertEqual(presave_fuzzer.saved_value_classification(
                    candidate, before, actual, complete, changed), expected)

    def test_explicit_nullable_patch_requires_clear_and_audit(self) -> None:
        for actual, complete, changed, expected in [
            (None, True, True, "SAVE_ACCEPTED"),
            (True, False, False, "SAVE_READBACK_MISMATCH"),
            (None, False, True, "AUDIT_MISMATCH"),
        ]:
            self.assertEqual(presave_fuzzer.saved_value_classification(
                None, True, actual, complete, changed, null_clears=True), expected)
        country = next(field for field in presave_fuzzer.load_fields(["study"], unsupported=[])
                       if field["code"] == "C.5.1.r.2")
        self.assertEqual(presave_fuzzer.presave_expectation(
            country, "\t\t\t"), ("accept", None))

    def test_partial_and_unknown_campaigns_fail(self) -> None:
        import time
        with tempfile.TemporaryDirectory() as directory:
            args = presave_fuzzer.parser().parse_args([
                "--sections", "reporter", "--seed", "1", "--artifact-dir", directory])
            args.planned_mutations = 1
            args.runner_sha256 = "isolated-test-runner"
            args.contract_sha256 = "isolated-test-contract"
            args.excluded_fields = []
            args.unsupported_contract_pairs = []
            args.interrupted = None
            events = [{"kind": "lifecycle", "classification": "PASS"} for _ in range(3)]
            self.assertEqual(presave_fuzzer.write_artifacts(
                args, events, {}, [], 0, time.monotonic(), None), 1)
            events.append({"kind": "mutation", "classification": "NO_EXPECTATION"})
            self.assertEqual(presave_fuzzer.write_artifacts(
                args, events, {}, [], 0, time.monotonic(), None), 1)
            events[-1]["classification"] = "SAVE_ACCEPTED"
            self.assertEqual(presave_fuzzer.write_artifacts(
                args, events, {}, [], 0, time.monotonic(), None), 0)
            args.interrupted = "deadline"
            self.assertEqual(presave_fuzzer.write_artifacts(
                args, events, {}, [], 0, time.monotonic(), None), 2)


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

    def test_ae_list_projection_row_id(self) -> None:
        reaction_id = "e5f7f72f-f77e-4454-b25a-2caadffc8ee5"
        projection = {
            "caseId": "8e96c341-c00d-4f75-8188-e41a6740b0e4",
            "pageId": "AE",
            "rows": {"rows": [{"id": reaction_id, "sequenceNumber": 1}]},
        }
        self.assertEqual(fuzzer.extract_row_id(projection, "rows"), reaction_id)
        self.assertIsNone(fuzzer.extract_row_id(projection, "reaction"))

    def test_nested_drug_identity_extraction_requires_projected_ids(self) -> None:
        relatedness_id = "11111111-1111-1111-1111-111111111111"
        assessment_id = "22222222-2222-2222-2222-222222222222"
        reaction_id = "33333333-3333-3333-3333-333333333333"
        current = [{
            "id": relatedness_id,
            "drugReactionAssessmentId": assessment_id,
            "reactionId": reaction_id,
            "sourceOfAssessment": "Reporter",
        }]
        identity = fuzzer.object_identity(current)
        self.assertEqual(identity, {
            "id": relatedness_id,
            "drugReactionAssessmentId": assessment_id,
            "reactionId": reaction_id,
        })
        self.assertNotIn(
            "id",
            fuzzer.object_identity([{
                "drugReactionAssessmentId": assessment_id,
                "reactionId": reaction_id,
            }]),
        )

        other_id = "44444444-4444-4444-4444-444444444444"
        rows = [
            {"id": other_id, "source_of_assessment": "Sponsor"},
            {"id": relatedness_id, "source_of_assessment": "Reporter"},
        ]
        selected = fuzzer.row_with_identity(rows, {"id": relatedness_id})
        self.assertEqual(fuzzer.get_path(selected, "source_of_assessment"), "Reporter")
        self.assertIsNone(fuzzer.row_with_identity(rows, {"id": reaction_id}))
        self.assertIsNone(fuzzer.row_with_identity(rows + [rows[1]], {"id": relatedness_id}))
        response = {"drug": {"fdaDevices": [{"id": other_id}]}}
        self.assertEqual(
            fuzzer.replacement_identity(response, "drug", "fdaDevices[]", {"id"}),
            {"id": other_id},
        )
        self.assertEqual(
            fuzzer.replacement_identity(response, "drug", "fdaDevices[]", {"id", "reactionId"}),
            {},
        )
        response["drug"]["fdaDevices"].append({"id": relatedness_id})
        self.assertEqual(
            fuzzer.replacement_identity(response, "drug", "fdaDevices[]", {"id"}),
            {},
        )
        self.assertNotIn(
            "drugReactionAssessmentId",
            fuzzer.object_identity([{
                "id": relatedness_id,
                "reactionId": reaction_id,
            }]),
        )
        self.assertEqual(fuzzer.object_identity([{"id": "not-a-uuid"}]), {})

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
        limits, max_only = fuzzer.load_max_lengths(root)
        self.assertEqual(limits["ICH.C.1.1.LENGTH.MAX"], 100)
        self.assertIn("ICH.G.k.9.i.2.r.1.LENGTH.MAX", max_only)
        self.assertNotIn("FDA.G.k.1.a.LENGTH.MAX", max_only)
        contract = [{"fields": [{
            "authority": "ICH",
            "code": "C.1.11.1",
            "roundTripValue": "base",
            "constraint": {"ruleCode": "ICH.C.1.11.1.ALLOWED.VALUE"},
        }]}]
        self.assertEqual(fuzzer.apply_max_lengths(contract, (limits, max_only)), 1)
        self.assertEqual(contract[0]["fields"][0]["_maxLength"], 1)
        self.assertEqual(fuzzer.candidate_count({"roundTripValue": 1}, 17), 8)
        self.assertEqual(fuzzer.candidate_count({"roundTripValue": ["x"]}, 17), 14)

    def test_max_length_only_expectations_and_projection_empty_noop(self) -> None:
        field = {
            "_maxLength": 60,
            "_maxLengthRule": "ICH.G.k.9.i.2.r.1.LENGTH.MAX",
            "_maxLengthOnly": True,
        }
        self.assertEqual(fuzzer.candidate_expectation(field, 1, None), ("accept", None))
        self.assertEqual(fuzzer.candidate_expectation(field, 3, "x" * 60), ("accept", None))
        self.assertEqual(
            fuzzer.candidate_expectation(field, 3, "  " + "🙂" * 60 + "  "),
            ("accept", None),
        )
        self.assertEqual(
            fuzzer.candidate_expectation(field, 3, "  " + "🙂" * 61 + "  "),
            ("reject", "ICH.G.k.9.i.2.r.1.LENGTH.MAX"),
        )
        self.assertIsNone(fuzzer.candidate_expectation(field, 3, 123))
        self.assertEqual(
            fuzzer.candidate_expectation(field, 7, ["unexpected"]),
            ("reject", "ICH.G.k.9.i.2.r.1.LENGTH.MAX"),
        )
        self.assertEqual(
            fuzzer.candidate_expectation(field, 11, "x" * 61),
            ("reject", "ICH.G.k.9.i.2.r.1.LENGTH.MAX"),
        )
        self.assertEqual(fuzzer.candidate_expectation(field, 12, "\ud800"), ("reject", None))
        self.assertEqual(
            fuzzer.candidate_expectation(field, 13, "\x7f"),
            ("reject", "INPUT.CONTROL_CHAR.REJECTED"),
        )
        field["_maxLengthOnly"] = False
        self.assertIsNone(fuzzer.candidate_expectation(field, 3, "valid length"))
        self.assertEqual(fuzzer.normalized_classification([None], [None], False), "NOOP_ACCEPTED")
        self.assertEqual(
            fuzzer.normalized_classification([None, None], [None, None], False),
            "NOOP_ACCEPTED",
        )

    def test_length_candidates_preserve_known_value_grammar(self) -> None:
        coded = {"code": "C.1.3", "roundTripValue": "1", "_maxLength": 1}
        meddra = {
            "code": "D.10.7.1.r.1b",
            "roundTripValue": "10000001",
            "_maxLength": 8,
        }
        email = {
            "code": "C.3.4.8",
            "roundTripValue": "sender@example.test",
            "_maxLength": 100,
            "constraint": {"invalidValue": "X" * 101},
        }
        self.assertEqual(fuzzer.field_value(coded, random.Random(1), 14), "1")
        self.assertEqual(fuzzer.field_value(meddra, random.Random(1), 14), "10000001")
        for ordinal, length in ((0, 101), (14, 100), (15, 101), (16, 200)):
            value = fuzzer.field_value(email, random.Random(1), ordinal)
            self.assertEqual(len(value), length)
            self.assertEqual(len(value.split("@")), 2)
            self.assertEqual(value.rsplit("@", 1)[1], "a.co")

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
        self.assertEqual(
            fuzzer.mismatched_save_classification(None, "old", "old", False),
            "NOOP_ACCEPTED",
        )
        self.assertEqual(
            fuzzer.mismatched_save_classification(None, "old", "different", False),
            "CLEAR_NOT_APPLIED",
        )
        self.assertEqual(
            fuzzer.mismatched_save_classification("", "old", "old", False),
            "CLEAR_NOT_APPLIED",
        )
        self.assertEqual(
            fuzzer.mismatched_save_classification("", None, None, False),
            "NOOP_ACCEPTED",
        )
        self.assertEqual(
            fuzzer.mismatched_save_classification("new", "old", "old", False),
            "SAVE_READBACK_MISMATCH",
        )
        self.assertEqual(fuzzer.normalized_classification([None], ["old"], True), "SAVE_NORMALIZED")
        self.assertEqual(fuzzer.normalized_classification([None], ["old"], False), "AUDIT_MISMATCH")
        self.assertEqual(
            fuzzer.normalized_classification([None, None], ["old"], True),
            "NORMALIZATION_UNVERIFIED",
        )
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

    def test_encoding_edge_expectation_matches_control_character_contract(self) -> None:
        field = {
            "code": "C.test",
            "roundTripValue": "base",
            "_maxLength": 10,
            "_maxLengthRule": "ICH.C.test.LENGTH.MAX",
            "_maxLengthOnly": True,
        }
        self.assertFalse(fuzzer.contains_rejected_control("0\uffff\ufffe\ufffd"))
        self.assertEqual(fuzzer.candidate_expectation(field, 13, "0\uffff\ufffe\ufffd"), ("accept", None))
        self.assertTrue(fuzzer.contains_rejected_control("0\x7f\uffff"))
        self.assertEqual(
            fuzzer.candidate_expectation(field, 13, "0\x7f\uffff"),
            ("reject", "INPUT.CONTROL_CHAR.REJECTED"),
        )
        field["_maxLength"] = 4
        self.assertEqual(
            fuzzer.candidate_expectation(field, 13, "0\uffff\ufffe\ufffd\U0010ffff"),
            ("reject", "ICH.C.test.LENGTH.MAX"),
        )
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
        pairs = {"DG": [{"value": "unsupported[].name", "nullFlavor": "unsupported[].nameNullFlavor"}]}
        with self.assertRaisesRegex(ValueError, "unresolved NullFlavor pairs"):
            fuzzer.expand_null_flavor_contracts(contract, pairs, {})
        unsupported = []
        self.assertEqual(fuzzer.expand_null_flavor_contracts(contract, pairs, {}, unsupported), 0)
        self.assertEqual(unsupported, ["DG:unsupported[].name"])

    def test_device_null_flavors_use_backend_contract_codes_and_dictionary_values(self) -> None:
        contract = [{"pageId": "DG", "fields": []}]
        pairs = {"DG": [
            {"value": "fdaDevices[].deviceBrandName", "nullFlavor": "fdaDevices[].deviceBrandNameNullFlavor"},
            {"value": "fdaDevices[].commonDeviceName", "nullFlavor": "fdaDevices[].commonDeviceNameNullFlavor"},
        ]}
        allowed = {"FDA.G.k.12.r.4": ["NI"], "FDA.G.k.12.r.5": ["NI"]}
        unsupported = []
        self.assertEqual(fuzzer.expand_null_flavor_contracts(contract, pairs, allowed, unsupported), 2)
        self.assertEqual(unsupported, [])
        by_code = {field["code"]: field for field in contract[0]["fields"]}
        for code, leaf in (("FDA.G.k.12.r.4", "deviceBrandName"),
                           ("FDA.G.k.12.r.5", "commonDeviceName")):
            field = by_code[f"{code}.nullFlavor"]
            self.assertEqual(field["roundTripValue"], "NI")
            self.assertEqual(field["payloadPath"], f"fdaDevices[].{leaf}NullFlavor")
            self.assertEqual(field["nullFlavorPartnerCode"], code)
            self.assertEqual(field["patch"], {"kind": "row", "owner": "drug"})
            sibling = "commonDeviceName" if leaf == "deviceBrandName" else "deviceBrandName"
            self.assertEqual(field["_mutationFixedPayload"], {
                f"fdaDevices[].{sibling}": f"Fuzz {sibling}",
            })
            self.assertEqual(
                fuzzer.candidate_expectation(field, 4, ""),
                ("accept", None),
            )
            self.assertIn(field["payloadPath"], fuzzer.DEVICE_NULL_FLAVOR_PATHS)

        baseline = fuzzer.baseline_for(contract[0]["fields"], minimal=True)
        device = baseline["fdaDevices"][0]
        self.assertEqual(device, {
            "deviceBrandNameNullFlavor": "NI",
            "commonDeviceNameNullFlavor": "NI",
        })

        brand = by_code["FDA.G.k.12.r.4.nullFlavor"]
        mutation = fuzzer.baseline_for([brand])
        for path, value in brand["_mutationFixedPayload"].items():
            fuzzer.set_path(mutation, path, value)
        fuzzer.set_path(mutation, fuzzer.leaf_path(brand["payloadPath"]), None)
        self.assertEqual(mutation["fdaDevices"][0], {
            "deviceBrandNameNullFlavor": None,
            "commonDeviceName": "Fuzz commonDeviceName",
        })
        self.assertEqual(
            fuzzer.normalized_classification(None, "NI", True),
            "SAVE_NORMALIZED",
        )

    def test_setup_callers_forward_explicit_intake_inputs(self) -> None:
        expected = ["--product-key", "PRODUCT-1", "--meddra-version", "28.1",
                    "--meddra-code", "10019211", "--contract", "/tmp/contracts.json",
                    "--null-flavor-pairs", "/tmp/null-flavor-pairs.ts"]
        with tempfile.TemporaryDirectory() as artifact_dir:
            def completed(command, **_kwargs):
                setup_dir = Path(command[command.index("--artifact-dir") + 1])
                setup_dir.mkdir(parents=True, exist_ok=True)
                seed = command[command.index("--seed") + 1]
                (setup_dir / f"case-editor-{seed}.jsonl").write_text(
                    '{"case_id":"11111111-1111-1111-1111-111111111111"}\n',
                    encoding="utf-8",
                )
                return mock.Mock(returncode=0, stdout="", stderr="")

            callers = (
                (ui_fuzzer, ui_fuzzer.setup_case,
                 ui_fuzzer.parser().parse_args([*expected, "--seed", "1", "--artifact-dir", artifact_dir])),
                (presave_fuzzer, presave_fuzzer.setup_case,
                 presave_fuzzer.parser().parse_args([*expected, "--seed", "1", "--artifact-dir", artifact_dir])),
                (rich_fuzzer, lambda args: rich_fuzzer.setup_rich_case(args, 1, Path(artifact_dir)),
                 rich_fuzzer.parser().parse_args([*expected, "--seed", "1", "--artifact-dir", artifact_dir])),
            )
            for module, caller, args in callers:
                with mock.patch.object(module.subprocess, "run", side_effect=completed) as run:
                    caller(args)
                command = run.call_args.args[0]
                self.assertEqual((args.product_key, args.meddra_version, args.meddra_code),
                                 ("PRODUCT-1", "28.1", "10019211"))
                for option in expected[::2]:
                    index = command.index(option)
                    self.assertEqual(command[index + 1], expected[expected.index(option) + 1])

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

    def test_live_intake_requires_explicit_product_and_meddra_inputs(self) -> None:
        with mock.patch.dict(
            fuzzer.os.environ,
            {"E2BR3_PRODUCT_KEY": "", "E2BR3_MEDDRA_VERSION": "", "E2BR3_MEDDRA_CODE": ""},
        ), mock.patch.object(fuzzer, "guard_target"):
            with self.assertRaisesRegex(SystemExit, "product-key"):
                fuzzer.main(fuzzer.parser().parse_args(["--values-per-field", "0"]))
            with self.assertRaisesRegex(SystemExit, "meddra-version"):
                fuzzer.main(fuzzer.parser().parse_args([
                    "--values-per-field", "0", "--product-key", "PRODUCT-1",
                ]))
        with mock.patch.dict(fuzzer.os.environ, {
            "E2BR3_PRODUCT_KEY": "PRODUCT-1",
            "E2BR3_MEDDRA_VERSION": "28.1",
            "E2BR3_MEDDRA_CODE": "10019211",
        }):
            args = fuzzer.parser().parse_args([])
            self.assertEqual(
                (args.product_key, args.meddra_version, args.meddra_code),
                ("PRODUCT-1", "28.1", "10019211"),
            )

    def test_case_bootstrap_uses_intake_and_checks_ui_shell(self) -> None:
        calls = []
        case_id = "11111111-1111-1111-1111-111111111111"

        class Client:
            def __init__(self, *_args):
                pass

            def request(self, method, path, payload=None):
                calls.append((method, path, payload))
                if path == "/auth/v1/login":
                    return 200, '{"data":{}}', None
                if path == "/api/presaves/products":
                    return 200, '{"data":[{"product_id":"PRODUCT-1"}]}', None
                if path == "/api/cases/from-intake":
                    return 201, '{"data":{"case_id":"%s"}}' % case_id, None
                if path == f"/api/cases/{case_id}/editor/shell":
                    return 200, '{"data":{}}', None
                raise AssertionError(f"unexpected request: {method} {path}")

        with tempfile.TemporaryDirectory() as artifact_dir:
            with (
                mock.patch.object(fuzzer, "ApiClient", Client),
                mock.patch.object(fuzzer, "guard_target"),
            ):
                args = fuzzer.parser().parse_args([
                    "--pages", "CI", "--values-per-field", "0", "--no-run-gates",
                    "--max-actions", "4",
                    "--password", "fixture-password",
                    "--artifact-dir", artifact_dir, "--product-key", "PRODUCT-1",
                    "--meddra-version", "28.1", "--meddra-code", "10019211",
                ])
                self.assertEqual(fuzzer.main(args), 2)

        create = next(call for call in calls if call[1] == "/api/cases/from-intake")
        intake = create[2]["data"]
        self.assertEqual(intake["dg_prd_key"], "PRODUCT-1")
        self.assertEqual(intake["reaction_meddra_version"], "28.1")
        self.assertEqual(intake["reaction_meddra_code"], "10019211")
        self.assertNotIn("allow_duplicate_override", intake)
        self.assertEqual(intake["date_first_received_from_source"], intake["date_of_most_recent_information"])
        self.assertIn(("GET", f"/api/cases/{case_id}/editor/shell", None), calls)

    def test_case_bootstrap_rejects_inaccessible_product_before_create(self) -> None:
        calls = []

        class Client:
            def __init__(self, *_args):
                pass

            def request(self, method, path, payload=None):
                calls.append((method, path, payload))
                if path == "/auth/v1/login":
                    return 200, '{"data":{}}', None
                if path == "/api/presaves/products":
                    return 200, '{"data":[{"product_id":"PRODUCT-1"}]}', None
                raise AssertionError(f"unexpected request: {method} {path}")

        with tempfile.TemporaryDirectory() as artifact_dir:
            with (
                mock.patch.object(fuzzer, "ApiClient", Client),
                mock.patch.object(fuzzer, "guard_target"),
            ):
                args = fuzzer.parser().parse_args([
                    "--pages", "CI", "--values-per-field", "0", "--no-run-gates",
                    "--password", "fixture-password", "--artifact-dir", artifact_dir,
                    "--product-key", "MISSING-PRODUCT", "--meddra-version", "28.1",
                    "--meddra-code", "10019211",
                ])
                self.assertEqual(fuzzer.main(args), 2)

        self.assertEqual([path for _, path, _ in calls], [
            "/auth/v1/login", "/api/presaves/products",
        ])

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
        null = next(mutation for mutation in null_flavor["mutations"] if mutation["kind"] == "nullflavor_null")
        conflict = next(mutation for mutation in null_flavor["mutations"] if mutation["kind"] == "nullflavor_with_value")
        self.assertEqual(invalid["expectation"], "reject")
        self.assertEqual(null["expectation"], "accept")
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
