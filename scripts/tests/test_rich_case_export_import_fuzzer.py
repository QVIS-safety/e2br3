import argparse
import copy
import json
import tempfile
from contextlib import ExitStack
from unittest.mock import patch
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import case_editor_input_fuzzer as editor
import rich_case_export_import_fuzzer as rich


class RichNullFlavorTests(unittest.TestCase):
    def test_seeded_mixture_clears_values_and_preserves_contract(self):
        fields = [{"code": f"D.{i}", "payloadPath": f"field{i}",
                   "frontendPath": f"field{i}", "roundTripValue": "original",
                   "authority": "ICH", "patch": {"kind": "row", "owner": "patientInformation"},
                   "constraint": {"status": "verified"}} for i in range(40)]
        contract = [{"pageId": "DM", "fields": fields}]
        pairs = {"DM": [{"value": f"field{i}", "nullFlavor": f"field{i}NullFlavor"} for i in range(40)]}
        allowed = {f"D.{i}": ["UNK", "MSK"] for i in range(40)}
        before = copy.deepcopy(contract)
        a, plan = rich.mix_null_flavors(contract, pairs, allowed, 12, .3)
        self.assertEqual((a, plan), rich.mix_null_flavors(contract, pairs, allowed, 12, .3))
        self.assertNotEqual(plan["selected"], rich.mix_null_flavors(contract, pairs, allowed, 13, .3)[1]["selected"])
        self.assertEqual(contract, before)
        self.assertGreater(len(plan["selected"]), 0)
        self.assertLess(len(plan["selected"]), 40)
        baseline = editor.baseline_for(a[0]["fields"], safe=True)
        for selected in plan["selected"]:
            self.assertIsNone(baseline[selected["value_path"]])
            self.assertEqual(baseline[selected["path"]], selected["null_flavor"])
            self.assertIn(selected["null_flavor"], ["UNK", "MSK"])
        zero, zero_plan = rich.mix_null_flavors(contract, pairs, allowed, 12, 0)
        self.assertEqual(zero, before)
        self.assertEqual(zero_plan["selected"], [])
        self.assertEqual(len(rich.mix_null_flavors(contract, pairs, allowed, 12, 1)[1]["selected"]), 40)
        self.assertEqual(editor.baseline_for(a[0]["fields"]), editor.baseline_for(fields))

    def test_rich_baseline_resolves_exclusive_fields_without_changing_mutations(self):
        fields = [{"code": code, "payloadPath": path, "roundTripValue": value,
                   "patch": {"owner": "patientInformation"}, "constraint": {"status": "verified"}}
                  for code, path, value in [("D.2.1", "patientBirthDate", "19900102"),
                                           ("D.7.3", "concomitantTherapies", True)]]
        mixed, _ = rich.mix_null_flavors([{"pageId": "DM", "fields": fields}], {}, {}, 1, 0)
        baseline = editor.baseline_for(mixed[0]["fields"], safe=True)
        self.assertIsNone(baseline["patientBirthDate"])
        self.assertIs(baseline["concomitantTherapies"], True)
        self.assertEqual(editor.baseline_for(mixed[0]["fields"]), editor.baseline_for(fields))

    def test_postmarket_has_no_empty_ind_rows_and_ct_required_values_are_kept(self):
        contract = json.loads(editor.DEFAULT_CONTRACT.read_text())
        mixed, plan = rich.mix_null_flavors(contract,
            editor.load_null_flavor_pairs(editor.DEFAULT_NULL_FLAVOR_PAIRS),
            editor.load_dictionary_null_flavors(rich.ROOT), 2026101306, 1)
        study = editor.baseline_for(editor.contract_rows(mixed, "SI"), safe=True)
        self.assertNotIn("fdaCrossReportedIndNumbers", study)
        drug = editor.baseline_for(editor.contract_rows(mixed, "DG"), safe=True)
        self.assertEqual(drug["drugReactionAssessments"][0]["methodOfAssessmentKr1"], "2")
        protected = {"C.2.r.4", "D.2.2a", "D.2.2b"}
        selected = {item["value_path"] for item in plan["selected"]}
        for page in mixed:
            for field in page["fields"]:
                if field["code"] in protected:
                    self.assertNotIn(field["payloadPath"], selected)

    def test_nested_value_is_cleared_in_same_row(self):
        field = {"payloadPath": "activeSubstances[].substanceName", "roundTripValue": "drug",
                 "_richNullFlavor": {"path": "activeSubstances[].substanceNameNullFlavor", "value": "UNK"}}
        self.assertEqual(editor.baseline_for([field], safe=True), {
            "activeSubstances": [{"substanceName": None, "substanceNameNullFlavor": "UNK"}]})

    def test_invalid_rates_are_rejected(self):
        for value in ["nan", "inf", "-0.01", "1.01"]:
            with self.assertRaises(argparse.ArgumentTypeError):
                rich.null_flavor_rate(value)
        self.assertEqual(rich.parser().parse_args([]).null_flavor_rate, 0)

    def test_authority_overwrites_and_unmapped_pairs_are_reported(self):
        contract = [{"pageId": "SI", "fields": [{
            "code": "C.3.3.1", "frontendPath": "senderGivenName",
            "payloadPath": "senderGivenName", "roundTripValue": "Name",
            "patch": {"kind": "row", "owner": "senderInformation"},
            "constraint": {"status": "verified"}}]}]
        pairs = {"SI": [
            {"value": "senderGivenName", "nullFlavor": "senderGivenNameNullFlavor"},
            {"value": "unmapped", "nullFlavor": "unmappedNullFlavor"}]}
        mixed, plan = rich.mix_null_flavors(contract, pairs, {"C.3.3.1": ["UNK"]}, 12, 1)
        self.assertEqual(mixed, contract)
        self.assertEqual(plan["selected"], [])
        self.assertEqual(plan["skipped"][0]["reason"], "authority setup overwrites field")
        self.assertEqual(plan["unsupported"], ["SI:unmapped"])


class RichSubmissionTests(unittest.TestCase):
    case_id = "00000000-0000-4000-8000-000000000001"
    submission_id = "00000000-0000-4000-8000-000000000002"

    def exercise(self, responses, authority="fda", clock=None):
        # Deterministic protocol fixtures only; no transport success is inferred.
        calls = []
        class FixtureClient:
            def request(self, method, path, payload=None, **kwargs):
                calls.append((method, path, payload, kwargs))
                status, data, transport = responses.pop(0)
                return status, json.dumps(data).encode(), transport
        args = rich.parser().parse_args(["--submit", "--authorities", authority,
            f"--{authority}-receiver", "configured receiver", "--password", "fixture-secret"])
        with tempfile.TemporaryDirectory() as directory:
            with patch.object(rich.time, "sleep"), patch.object(rich.time, "monotonic", side_effect=clock or [0, 1, 2, 3]):
                result = rich.submit_case(FixtureClient(), args, self.case_id, authority, Path(directory))[0]
            journal = next(Path(directory).glob("submission-*.jsonl")).read_text()
            self.assertNotIn("fixture-secret", journal)
            self.assertEqual(json.loads(journal.splitlines()[-1])["status"], result["status"])
        self.assertEqual(responses, [])
        return result, calls

    def record(self, **changes):
        return {"id": self.submission_id, "case_id": self.case_id,
            "gateway": "as2-submitter-http", "status": "dispatch_pending",
            "remote_submission_id": None, "ack1": None, **changes}

    def ready(self):
        return [(200, {"data": {}}, None), (200, {"data": {"ok": True}}, None)]

    def test_both_authorities_poll_without_resending(self):
        for authority in ("fda", "mfds"):
            with self.subTest(authority=authority):
                final = self.record(status="ack1_received", remote_submission_id="remote-id",
                    ack1={"level": 1, "success": True})
                result, calls = self.exercise(self.ready() + [
                    (202, {"data": self.record()}, None), (200, {"data": final}, None)], authority)
                self.assertEqual(result["status"], "PASS")
                self.assertEqual(result["regulatory_acceptance"], "not_verified")
                posts = [c for c in calls if c[0] == "POST" and "/submissions/" in c[1]]
                self.assertEqual(len(posts), 1)
                self.assertTrue(posts[0][1].endswith("/" + authority))
                self.assertEqual(posts[0][3]["extra_headers"]["X-Idempotency-Key"], result["idempotency_key"])

    def test_validation_or_receiver_failure_never_dispatches(self):
        for responses in [
            [(400, {}, None)],
            [(200, {}, None), (200, {"data": {"ok": False}}, None)],
        ]:
            result, calls = self.exercise(responses)
            self.assertEqual(result["status"], "FAIL")
            self.assertFalse(any("/submissions/" in c[1] for c in calls))

    def test_negative_ack_missing_ack_and_lost_response_fail(self):
        for response, clock in [
            ((201, {"data": self.record(status="rejected", ack1={"level": 1, "success": False})}, None), [0]),
            ((202, {"data": self.record()}, None), [0, 121]),
            ((None, {}, "TimeoutError"), [0]),
            ((201, {"data": self.record(case_id="wrong-case")}, None), [0]),
            ((201, {"data": self.record(gateway="fda-esg-nextgen-api")}, None), [0]),
        ]:
            result, calls = self.exercise(self.ready() + [response], clock=clock)
            self.assertEqual(result["status"], "FAIL")
            self.assertEqual(sum(c[0] == "POST" and "/submissions/" in c[1] for c in calls), 1)

    def test_http_errors_cannot_report_success(self):
        with tempfile.TemporaryDirectory() as directory:
            args = rich.parser().parse_args(["--artifact-dir", directory])
            for row in [{"kind": "login", "status": 401},
                        {"kind": "source_validation", "status": 200, "ok": None},
                        {"kind": "source_export", "status": None}]:
                self.assertEqual(rich.write_results(args, [row]), 1)

    def test_roundtrip_detects_owner_value_swaps_and_lost_units(self):
        source = b'<investigationEvent><patient><value>patient</value><date value="20260102"/><width value="1" unit="d"/></patient><parent><value>parent</value></parent></investigationEvent>'
        for changed in [source.replace(b'>patient<', b'>parent<').replace(b'>parent</value></parent>', b'>patient</value></parent>'),
                        source.replace(b'20260102', b'20230102'),
                        source.replace(b' unit="d"', b'')]:
            self.assertIn("preservation:fields", rich.compare_xml(source, changed))
        self.assertEqual(rich.compare_xml(source, source), [])

    def test_import_response_requires_a_real_case_id(self):
        row = {"caseId": self.case_id, "status": "success"}
        for response in [{"importedCases": [row]}, {"data": {"importedCases": [row]}}]:
            self.assertEqual(rich.imported_case_id(json.dumps(response).encode()), self.case_id)
        self.assertIsNone(rich.imported_case_id(json.dumps({"data": {
            "caseId": None, "importedCases": [{"status": "error", "caseId": None}]}}).encode()))

    def test_submission_requires_explicit_configuration(self):
        self.assertFalse(rich.parser().parse_args([]).submit)
        for options in [["--submit"], ["--submit", "--authorities", "fda"]]:
            with self.assertRaises(SystemExit):
                rich.run(rich.parser().parse_args(options))

    def test_submit_run_keeps_cases_and_calls_submission_only_after_roundtrip(self):
        for comparison in ([], ["preservation:field"]):
            with self.subTest(comparison=comparison), tempfile.TemporaryDirectory() as directory, ExitStack() as stack:
                args = rich.parser().parse_args(["--submit", "--authorities", "fda",
                    "--sender-presave-id", self.case_id, "--fda-receiver", "configured receiver",
                    "--artifact-dir", directory])
                client = stack.enter_context(patch.object(rich, "ApiClient")).return_value
                client.request.return_value = (200, b'{}', None)
                for name, value in {
                    "create_import_product": ("product", self.case_id, []),
                    "setup_rich_case": ("source-case", {"status": "PASS"}),
                    "link_case_sender": {"kind": "link_case_sender", "status": 200},
                    "configure_case_authority_types": {"kind": "configure_case_authority_types", "status": 200},
                    "configure_case_identification": {"kind": "configure_case_identification", "status": 200},
                    "validation": {"status": 200, "ok": True},
                    "export_xml": (200, b'<fixture/>', {}),
                    "unique_xml": b'<fixture/>',
                    "imported_case_id": self.case_id,
                    "compare_xml": comparison,
                }.items():
                    stack.enter_context(patch.object(rich, name, return_value=value))
                submit = stack.enter_context(patch.object(rich, "submit_case", return_value=[
                    {"kind": "submission", "status": "PASS"}]))
                cleanup = stack.enter_context(patch.object(rich, "delete_case"))
                self.assertEqual(rich.run(args), 1 if comparison else 0)
                self.assertEqual(submit.call_count, 0 if comparison else 1)
                cleanup.assert_not_called()

    def test_mfds_preparation_updates_existing_rows_and_allocates_free_sequence(self):
        for existing_study in [False, True]:
            calls = []
            class FixtureClient:
                def request(self, method, path, payload=None):
                    calls.append((method, path, payload))
                    if method == "GET":
                        rows = ({"studyRegistrationNumbers": [{"id": "registration", "deleted": False}]}
                                if path.endswith("SI") else {"patientIdentifiers": [{
                                    "id": "identifier", "deleted": False, "sequence_number": 5,
                                    "identifier_type_code": "4" if existing_study else "2"}]})
                        return 200, json.dumps({"rows": rows}).encode(), None
                    return 200, b"{}", None
            rich.configure_mfds_ct_case(FixtureClient(), self.case_id, 1)
            writes = [c[2]["rows"] for c in calls if c[0] == "PATCH"]
            self.assertEqual(writes[1]["studyRegistrationNumbers"][0]["id"], "registration")
            patient = writes[2]["patientIdentifiers"][0]
            if existing_study:
                self.assertEqual(patient["id"], "identifier")
                self.assertNotIn("sequenceNumber", patient)
            else:
                self.assertEqual(patient["sequenceNumber"], 6)
                self.assertNotIn("id", patient)

    def test_existing_sender_is_reused_without_creating_sender(self):
        with patch.object(rich, "ApiClient") as client_type:
            client = client_type.return_value
            client.request.return_value = (201, json.dumps({"data": {"id": self.submission_id}}).encode(), None)
            _, sender_id, _ = rich.create_import_product(client, "fixture", self.case_id)
            self.assertEqual(sender_id, self.case_id)
            self.assertEqual(client.request.call_count, 1)
            self.assertEqual(client.request.call_args.args[1], "/api/presaves/products")
            self.assertEqual(client.request.call_args.args[2]["data"]["rows"]["product"]["senderPresaveId"], self.case_id)


if __name__ == '__main__':
    unittest.main()
