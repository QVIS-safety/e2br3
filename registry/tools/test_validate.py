import contextlib
import io
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
import sys
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import validate
import editor_contract


class RegistryValidatorTests(unittest.TestCase):
    def test_cli_rejects_unknown_options(self):
        script = Path(__file__).with_name("validate.py")
        for option in ("--strict-frontend", "--editor-page"):
            completed = subprocess.run(
                [sys.executable, str(script), option], capture_output=True, text=True
            )
            self.assertEqual(2, completed.returncode)
            self.assertIn(f"unknown option: {option}", completed.stderr)

    def test_active_editor_contract_fields_are_explicit_rows_payloads(self):
        repo = Path(__file__).resolve().parents[2]
        active_pages = {"CI", "RP", "SD", "LR", "SI", "DM", "NR", "AE", "LB", "DG", "DH"}

        violations = []
        for page_id in sorted(active_pages):
            contract = editor_contract.load_editor_contract(
                repo / "registry", page_id
            )
            for field in contract["fields"]:
                patch = field.get("patch", {})
                if not field.get("payloadPath"):
                    violations.append(f"{page_id}/{field.get('code')}: missing payloadPath")
                if patch.get("kind") not in {"row", "rows"}:
                    violations.append(
                        f"{page_id}/{field.get('code')}: invalid patch kind {patch.get('kind')}"
                    )
                if not patch.get("owner"):
                    violations.append(f"{page_id}/{field.get('code')}: missing patch owner")

        self.assertEqual([], violations)

    def test_verified_constraints_declare_explicit_invalid_values(self):
        repo = Path(__file__).resolve().parents[2]
        violations = []
        for path in sorted((repo / "registry/editor-contracts").glob("*.json")):
            if path.name == "schema.json":
                continue
            contract = json.loads(path.read_text(encoding="utf-8"))
            for field in contract["fields"]:
                constraint = field.get("constraint", {})
                if constraint.get("status") != "verified":
                    continue
                if "invalidValue" not in constraint:
                    violations.append(
                        f"{contract['pageId']}/{field['code']}: missing constraint.invalidValue"
                    )

        self.assertEqual([], violations)

    def test_input_type_editor_contracts_declare_primitive_boundary_evidence(self):
        repo = Path(__file__).resolve().parents[2]
        violations = []
        for path in sorted((repo / "registry/editor-contracts").glob("*.json")):
            if path.name == "schema.json":
                continue
            contract = json.loads(path.read_text(encoding="utf-8"))
            for field in contract["fields"]:
                constraint = field.get("constraint", {})
                if constraint.get("ruleCode") != "INPUT.TYPE":
                    continue
                expected_type = type(field.get("roundTripValue"))
                invalid_type = type(constraint.get("invalidValue"))
                if expected_type not in {bool, int, float}:
                    violations.append(f"{field['code']}: unsupported roundtrip type")
                if invalid_type is expected_type:
                    violations.append(f"{field['code']}: invalid value has expected type")
                if constraint.get("issuePath") != field.get("frontendPath"):
                    violations.append(f"{field['code']}: issuePath does not match frontendPath")

        self.assertEqual([], violations)

    def test_separate_null_flavor_fields_declare_symmetric_partners(self):
        repo = Path(__file__).resolve().parents[2]
        violations = []
        for path in sorted((repo / "registry/editor-contracts").glob("*.json")):
            if path.name == "schema.json":
                continue
            contract = json.loads(path.read_text(encoding="utf-8"))
            fields = {field["code"]: field for field in contract["fields"]}
            for field in fields.values():
                if not field["frontendPath"].endswith("NullFlavor"):
                    continue
                partner_code = field.get("nullFlavorPartnerCode")
                partner = fields.get(partner_code)
                if partner is None:
                    violations.append(f"{field['code']}: missing NullFlavor partner")
                elif partner.get("nullFlavorPartnerCode") != field["code"]:
                    violations.append(f"{field['code']}: partner is not symmetric")

        self.assertEqual([], violations)

    def test_rule_source_coverage_report_cli_outputs_json(self):
        stdout = io.StringIO()
        with (
            mock.patch.object(
                sys,
                "argv",
                [
                    "validate.py",
                    "--report-rule-source-coverage",
                    "LR",
                ],
            ),
            contextlib.redirect_stdout(stdout),
        ):
            exit_code = validate.main()

        report = json.loads(stdout.getvalue())
        self.assertEqual(0, exit_code)
        self.assertTrue(report)
        self.assertEqual("LR", report[0]["page"])
        self.assertEqual(
            {
                "page",
                "fieldId",
                "element",
                "authority",
                "coverage",
                "disposition",
            },
            set(report[0]),
        )

    def test_sd_editor_contract_excludes_export_owned_message_header(self):
        repo = Path(__file__).resolve().parents[2]
        contract = json.loads(
            (repo / "registry/editor-contracts/sd.json").read_text(encoding="utf-8")
        )
        registry = json.loads(
            (repo / "registry/sections/n-message-header.json").read_text(
                encoding="utf-8"
            )
        )
        export_codes = {"N.1.5", "N.2.r.1", "N.2.r.2", "N.2.r.3"}

        self.assertTrue(
            {field["code"] for field in contract["fields"]}.isdisjoint(
                export_codes
            )
        )
        for row in registry:
            if row["id"] not in export_codes:
                continue
            self.assertNotIn("editor_page", row)
            self.assertIn(
                "app/(protected)/submission/message-header.ts",
                row["frontend"]["file"],
            )



    def test_dm_parent_age_unit_radix_options_match_frontend_source(self):
        repo = Path(__file__).resolve().parents[2]
        contract = json.loads(
            (repo / "registry/editor-contracts/dm.json").read_text(encoding="utf-8")
        )
        field = next(row for row in contract["fields"] if row["code"] == "D.10.2.2b")
        self.assertEqual(
            field["control"],
            {
                "kind": "radix_select",
                "options": [
                    {"value": "10.a", "label": "10.a = Decade"},
                    {"value": "a", "label": "a = Year"},
                ],
            },
        )
        source = (validate.extract_frontend_fields.resolve_frontend_root(repo) / "lib/e2b/ucum.ts").read_text(
            encoding="utf-8"
        )
        self.assertIn('option.value === "10.a" || option.value === "a"', source)


    def test_certified_editor_sections_track_regional_business_rule_gaps(self):
        repo = Path(__file__).resolve().parents[2]
        registry_rows = []
        for path in (repo / "registry/sections").glob("*.json"):
            registry_rows.extend(json.loads(path.read_text(encoding="utf-8")))
        status_by_code = {
            row["id"]: row["status"]
            for row in registry_rows
            if row.get("editor_page") in {"RP", "SD", "LR"}
        }

        regional_gaps = {
            # RP: FDA VAERS primary-reporter requirements/MSK restrictions and
            # MFDS study/therapeutic-use conditional requirements.
            "C.2.r.1.2",
            "C.2.r.1.4",
            "C.2.r.2.1",
            "C.2.r.2.3",
            "C.2.r.2.4",
            "C.2.r.2.5",
            "C.2.r.2.6",
            "C.2.r.2.7",
            "C.2.r.3",
            "FDA.C.2.r.2.8",
            "C.2.r.4",
            "C.2.r.4.KR.1",
            "C.2.r.5",
            # SD: FDA sender/identifier rules and MFDS conditional/format rules.
            "C.3.1.KR.1",
            "C.3.3.1",
            "C.3.3.2",
            "C.3.3.3",
            "C.3.3.5",
            "C.3.4.1",
            "C.3.4.2",
            "C.3.4.3",
            "C.3.4.4",
            "C.3.4.5",
            "C.3.4.6",
            "C.3.4.7",
            "C.3.4.8",
            # LR: FDA attachment file-name/mediaType consistency.
            "C.4.r.2",
        }

        self.assertEqual(
            {},
            {
                code: status_by_code.get(code)
                for code in sorted(regional_gaps)
                if status_by_code.get(code) != "incomplete"
            },
        )

    def test_drug_registry_has_no_local_frequency_value(self):
        repo = Path(__file__).resolve().parents[2]
        rows = json.loads(
            (repo / "registry/sections/g-drug.json").read_text(encoding="utf-8")
        )
        ids = {row["id"] for row in rows}

        self.assertNotIn("G.k.local.dosage.frequencyValue", ids)

    def write_registry(self, root: Path, row: str) -> None:
        (root / "index.json").write_text(
            '{"sections":["sections/c-safety-report.json"]}',
            encoding="utf-8",
        )
        sections = root / "sections"
        sections.mkdir()
        (sections / "c-safety-report.json").write_text(row, encoding="utf-8")

    def editor_row(self, *, code: str = "C.1.3", status: str = "complete") -> dict:
        return {
            "id": code,
            "e2br3_code": code,
            "label": "Type of Report",
            "section": "C",
            "authority": "ICH",
            "status": status,
            "editor_page": "CI",
            "backend": {
                "status": "mapped",
                "model": "SafetyReportIdentification",
                "field": "report_type",
                "evidence": "model field",
            },
            "frontend": {
                "status": "mapped",
                "section": "safetyReportIdentification",
                "field": "reportType",
                "evidence": "mounted input",
            },
        }

    def editor_field(self, *, code: str = "C.1.3") -> dict:
        return {
            "code": code,
            "authority": "ICH",
            "frontendPath": "safetyReportIdentification.reportType",
            "payloadPath": "reportType",
            "projectionPath": "safetyReportIdentification.reportType",
            "patch": {"kind": "row", "owner": "safetyReportIdentification"},
            "roundTripValue": "2",
            "constraint": {
                "status": "verified",
                "ruleCode": "ICH.C.1.3.ALLOWED.VALUE",
                "invalidValue": "__INVALID__",
            },
        }

    def test_complete_editor_field_does_not_require_business_validation_stage(self):
        field = self.editor_field()
        result = validate.ValidationResult()

        editor_contract.validate_editor_contract(
            [self.editor_row()], {"pageId": "CI", "fields": [field]}, result
        )

        self.assertEqual([], result.errors)

    def test_complete_editor_field_requires_manifest_entry(self):
        result = validate.ValidationResult()

        editor_contract.validate_editor_contract(
            [self.editor_row()], {"pageId": "CI", "fields": []}, result
        )

        self.assertIn("C.1.3 complete but missing from CI editor contract", result.errors)

    def test_editor_completion_is_independent_from_business_rule_status(self):
        row = self.editor_row(status="incomplete")
        row["editor_status"] = "complete"
        result = validate.ValidationResult()

        editor_contract.validate_editor_contract(
            [row], {"pageId": "CI", "fields": []}, result
        )

        self.assertIn("C.1.3 complete but missing from CI editor contract", result.errors)

    def test_registry_accepts_complete_editor_with_incomplete_business_status(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            row = self.editor_row(status="incomplete")
            row["editor_status"] = "complete"
            self.write_registry(root, json.dumps([row]))

            result = validate.validate_registry(
                root,
                validate_backend_inventory=False,
            )

        self.assertEqual([], result.errors)

    def test_complete_rp_row_requires_field_contract(self) -> None:
        row = self.editor_row(code="C.2.r.1.1")
        row["editor_page"] = "RP"
        row["frontend"]["section"] = "primarySources"
        row["frontend"]["field"] = "reporterTitle"
        result = validate.ValidationResult()

        editor_contract.validate_editor_contract(
            [row], {"pageId": "RP", "fields": []}, result
        )

        self.assertIn(
            "C.2.r.1.1 complete but missing from RP editor contract",
            result.errors,
        )

    def test_editor_contract_requires_registry_frontend_path_match(self):
        field = self.editor_field()
        field["frontendPath"] = "case.reportType"
        result = validate.ValidationResult()

        editor_contract.validate_editor_contract(
            [self.editor_row()], {"pageId": "CI", "fields": [field]}, result
        )

        self.assertIn(
            "C.1.3 frontend path case.reportType does not match registry ['safetyReportIdentification.reportType']",
            result.errors,
        )

    def test_editor_contract_accepts_one_of_multiple_registry_frontend_paths(self):
        row = self.editor_row()
        row["frontend"]["field"] = "reportType/reportTypeNullFlavor"
        result = validate.ValidationResult()

        editor_contract.validate_editor_contract(
            [row], {"pageId": "CI", "fields": [self.editor_field()]}, result
        )

        self.assertEqual([], result.errors)

    def test_editor_contract_rejects_business_validation_ownership(self):
        field = self.editor_field()
        field["businessValidation"] = {
            "status": "not_applicable",
            "reason": "No business validation rule exists for this local field.",
        }
        result = validate.ValidationResult()

        editor_contract.validate_editor_contract(
            [self.editor_row()], {"pageId": "CI", "fields": [field]}, result
        )

        self.assertEqual(["C.1.3 businessValidation belongs in the validator crate"], result.errors)

    def test_editor_contract_blocked_stage_requires_reason(self):
        result = validate.ValidationResult()
        editor_contract._validate_stage(
            "C.1.local.fixture", "constraint", {"status": "blocked"}, result
        )
        self.assertEqual(
            ["C.1.local.fixture constraint blocked requires reason"], result.errors
        )

    def test_radix_select_control_rejects_duplicate_values_and_empty_labels(self):
        result = validate.ValidationResult()
        editor_contract._validate_control(
            "D.2.2b",
            {
                "kind": "radix_select",
                "options": [
                    {"value": "a", "label": "a = Year"},
                    {"value": "a", "label": ""},
                ],
            },
            result,
        )
        self.assertEqual(
            [
                "D.2.2b radix_select option value 'a' is duplicated",
                "D.2.2b radix_select option label is required",
            ],
            result.errors,
        )

    def test_radix_select_control_rejects_stale_round_trip_value(self):
        result = validate.ValidationResult()
        editor_contract._validate_control(
            "FDA.G.k.10.1",
            {
                "kind": "radix_select",
                "options": [
                    {"value": "C102834", "label": "C102834 = Type 1"},
                ],
            },
            result,
            "1",
        )
        self.assertEqual(
            [
                "FDA.G.k.10.1 roundTripValue '1' is not declared by its radix_select control"
            ],
            result.errors,
        )

    def test_radix_select_control_accepts_exact_dynamic_option_source(self):
        result = validate.ValidationResult()
        editor_contract._validate_control(
            "G.k.9.i.1",
            {
                "kind": "radix_select",
                "dynamicOptions": {
                    "source": "case.reactions",
                    "valuePath": "id",
                    "labelPath": "primarySourceReaction",
                },
            },
            result,
        )
        self.assertEqual([], result.errors)

    def test_radix_select_control_rejects_invalid_dynamic_option_source(self):
        result = validate.ValidationResult()
        editor_contract._validate_control(
            "G.k.9.i.1",
            {
                "kind": "radix_select",
                "dynamicOptions": {
                    "source": "",
                    "valuePath": "id",
                },
            },
            result,
        )
        self.assertEqual(
            [
                "G.k.9.i.1 radix_select dynamicOptions requires source",
                "G.k.9.i.1 radix_select dynamicOptions requires labelPath",
            ],
            result.errors,
        )

    def test_radix_select_control_rejects_ambiguous_option_sources(self):
        dynamic = {
            "source": "case.reactions",
            "valuePath": "id",
            "labelPath": "primarySourceReaction",
        }
        for control in (
            {"kind": "radix_select"},
            {
                "kind": "radix_select",
                "options": [{"value": "one", "label": "One"}],
                "dynamicOptions": dynamic,
            },
        ):
            with self.subTest(control=control):
                result = validate.ValidationResult()
                editor_contract._validate_control(
                    "G.k.9.i.1", control, result
                )
                self.assertEqual(
                    [
                        "G.k.9.i.1 radix_select control requires exactly one of options or dynamicOptions"
                    ],
                    result.errors,
                )

    def test_async_autocomplete_control_requires_query_and_option_values(self):
        result = validate.ValidationResult()
        editor_contract._validate_control(
            "C.2.r.3",
            {"kind": "async_autocomplete", "queryValue": "", "optionValue": ""},
            result,
        )
        self.assertEqual(
            [
                "C.2.r.3 async_autocomplete control requires queryValue",
                "C.2.r.3 async_autocomplete control requires optionValue",
                "C.2.r.3 async_autocomplete control requires optionDataValue",
                "C.2.r.3 async_autocomplete control requires optionLabel",
                "C.2.r.3 async_autocomplete control requires selectedLabel",
            ],
            result.errors,
        )

    def test_si_country_declares_the_reviewed_country_autocomplete_binding(self):
        repo = Path(__file__).resolve().parents[2]
        contract = json.loads(
            (repo / "registry/editor-contracts/si.json").read_text(encoding="utf-8")
        )
        country = next(field for field in contract["fields"] if field["code"] == "C.5.1.r.2")
        self.assertEqual(
            {
                "kind": "async_autocomplete",
                "queryValue": "KR",
                "optionValue": "KR",
                "optionDataValue": "KR",
                "optionLabel": "KR",
                "selectedLabel": "KR",
            },
            country["control"],
        )

    def test_native_select_control_requires_exact_options(self):
        result = validate.ValidationResult()
        editor_contract._validate_control(
            "D.8.r.6a", {"kind": "native_select", "options": []}, result
        )
        self.assertEqual(
            ["D.8.r.6a native_select control requires options"], result.errors
        )

    def test_radio_group_control_requires_exact_options(self):
        result = validate.ValidationResult()
        editor_contract._validate_control(
            "G.k.1", {"kind": "radio_group", "options": []}, result
        )
        self.assertEqual(
            ["G.k.1 radio_group control requires options"], result.errors
        )

    def test_async_autocomplete_selection_effect_requires_exact_pair(self):
        result = validate.ValidationResult()
        editor_contract._validate_control(
            "D.8.r.6b",
            {
                "kind": "async_autocomplete",
                "queryValue": "Alarm clock headache",
                "optionValue": "10088978",
                "optionDataValue": "10088978",
                "optionLabel": "Alarm clock headache (10088978)",
                "selectedLabel": "10088978",
                "selectionEffects": [],
            },
            result,
        )
        self.assertEqual(
            ["D.8.r.6b async_autocomplete selectionEffects requires unique frontendPath/value pairs"],
            result.errors,
        )

    def test_async_autocomplete_selection_effects_accept_distinct_pairs(self):
        result = validate.ValidationResult()
        editor_contract._validate_control(
            "G.k.2.1.KR.1b",
            {
                "kind": "async_autocomplete",
                "queryValue": "acet",
                "optionValue": "1086735",
                "optionDataValue": "1086735",
                "optionLabel": "1-naphthaleneacetic acid (1086735)",
                "selectedLabel": "1086735",
                "selectionEffects": [
                    {"frontendPath": "drugs[].mfdsMpidVersion", "value": "GLOBALC3Mar26"},
                    {"frontendPath": "drugs[].medicinalProduct", "value": "1-naphthaleneacetic acid"},
                ],
            },
            result,
        )
        self.assertEqual([], result.errors)

    def test_dh_terminology_controls_match_reviewed_live_options(self):
        repo = Path(__file__).resolve().parents[2]
        contract = json.loads(
            (repo / "registry/editor-contracts/dh.json").read_text(encoding="utf-8")
        )
        fields = {field["code"]: field for field in contract["fields"]}
        self.assertEqual(
            {
                "kind": "async_autocomplete",
                "queryValue": "Advanced Aspirin",
                "optionValue": "Advanced Aspirin",
                "optionDataValue": "Advanced Aspirin",
                "optionLabel": "Advanced Aspirin (000027-01-917)",
                "selectedLabel": "Advanced Aspirin",
                "selectionRule": "exact_query_equal_first_source_order",
                "sourceCode": "000027-01-917",
                "sourceVersion": "2026-03",
                "sourceLanguage": "en",
            },
            fields["D.8.r.1"]["control"],
        )
        for code in ("D.8.r.6a", "D.8.r.7a"):
            self.assertEqual(
                {"kind": "native_select", "options": [{"value": "28.1", "label": "28.1"}]},
                fields[code]["control"],
            )
        for code in ("D.8.r.6b", "D.8.r.7b"):
            self.assertEqual("10088978", fields[code]["roundTripValue"])
            self.assertEqual("10088978", fields[code]["control"]["optionDataValue"])
            self.assertEqual("28.1", fields[code]["control"]["sourceVersion"])
            self.assertEqual("en", fields[code]["control"]["sourceLanguage"])
        self.assertEqual(
            [{"frontendPath": fields["D.8.r.6a"]["frontendPath"], "value": "28.1"}],
            fields["D.8.r.6b"]["control"]["selectionEffects"],
        )
        self.assertEqual(
            [{"frontendPath": fields["D.8.r.7a"]["frontendPath"], "value": "28.1"}],
            fields["D.8.r.7b"]["control"]["selectionEffects"],
        )

    def test_dg_form_autocomplete_controls_match_production_options(self):
        repo = Path(__file__).resolve().parents[2]
        contract = json.loads(
            (repo / "registry/editor-contracts/dg.json").read_text(encoding="utf-8")
        )
        fields = {field["code"]: field for field in contract["fields"]}
        expected = {
            "FDA.G.k.10a": [
                ("1", "Test"),
                ("2", "Reference"),
                ("3", "Bulk ingredient"),
                ("4", "Bulk Ingredient For Human Prescription Compounding"),
                (
                    "5",
                    "Unapproved Drug Product Manufactured Exclusively for Private Label Distributer",
                ),
            ],
            "FDA.G.k.10.1": [
                ("C102834", "Type 1: Convenience Kit of Co-Package"),
                (
                    "C102835",
                    "Type 2: Prefilled Drug Delivery Device/System (syringe, patch, etc.)",
                ),
                (
                    "C102836",
                    "Type 3: Prefilled Biologic Delivery Device/System (syringe, patch, etc.)",
                ),
                (
                    "C102837",
                    "Type 4: Device Coated/Impregnated/Otherwise Combined with Drug",
                ),
                (
                    "C102838",
                    "Type 5: Device Coated or Otherwise Combined with Biologic",
                ),
                ("C102839", "Type 6: Drug/Biologic Combination"),
                ("C102840", "Type 7: Separate Products Requiring Cross Labeling"),
                (
                    "C102841",
                    "Type 8: Possible Combination Based on Cross Labeling of Separate Products (Temporary Type)",
                ),
                (
                    "C102842",
                    "Type 9: Other Type of Part 3 Combination Product (e.g., Drug/Device/Biological Product)",
                ),
            ],
            "FDA.G.k.12.r.2.r": [
                ("1", "Correction"),
                ("2", "Additional Information"),
                ("3", "Response to FDA Request"),
                ("4", "Device Evaluation"),
            ],
            "FDA.G.k.12.r.11.r": [
                ("1", "Recall"),
                ("2", "Repair"),
                ("3", "Replacement"),
                ("4", "Relabeling"),
                ("5", "Notification"),
                ("6", "Inspection"),
                ("7", "Patient Monitoring"),
                ("8", "Modification or Adjustment"),
                ("9", "Other"),
            ],
        }
        for code, pairs in expected.items():
            field = fields[code]
            self.assertEqual("radix_select", field["control"]["kind"])
            self.assertEqual(
                [
                    {"value": value, "label": f"{value} = {display_name}"}
                    for value, display_name in pairs
                ],
                field["control"]["options"],
            )
            self.assertIn(
                field["roundTripValue"],
                [option["value"] for option in field["control"]["options"]],
            )
        self.assertEqual("C102834", fields["FDA.G.k.10.1"]["roundTripValue"])
        self.assertNotEqual("1", fields["FDA.G.k.10.1"]["roundTripValue"])
        self.assertEqual(
            {
                "kind": "async_autocomplete",
                "queryValue": "US",
                "optionValue": "US",
                "optionDataValue": "US",
                "optionLabel": "US",
                "selectedLabel": "US",
            },
            fields["FDA.G.k.12.r.7.1e"]["control"],
        )
        self.assertEqual(
            [
                {"value": "1", "label": "1 = WHO-UMC"},
                {"value": "2", "label": "2 = KRCT"},
            ],
            fields["G.k.9.i.2.r.2.KR.1"]["control"]["options"],
        )
        self.assertEqual(
            [
                {"value": "1", "label": "1 = 확실함 (Certain)"},
                {"value": "2", "label": "2 = 상당히 확실함 (Probable)"},
                {"value": "3", "label": "3 = 가능함 (Possible)"},
                {"value": "4", "label": "4 = 가능성 적음 (Unlikely)"},
                {"value": "5", "label": "5 = 평가곤란 (Conditional/unclassified)"},
                {"value": "6", "label": "6 = 평가불가 (Unassessable/unclassifiable)"},
            ],
            fields["G.k.9.i.2.r.3.KR.1"]["control"]["options"],
        )
        self.assertEqual(
            [
                {"value": "1", "label": "1 = 관련성 있음"},
                {"value": "2", "label": "2 = 관련성 없음"},
            ],
            fields["G.k.9.i.2.r.3.KR.2"]["control"]["options"],
        )

    def test_dg_pending_controls_match_production_sources(self):
        repo = Path(__file__).resolve().parents[2]
        contract = json.loads(
            (repo / "registry/editor-contracts/dg.json").read_text(encoding="utf-8")
        )
        fields = {field["code"]: field for field in contract["fields"]}

        self.assertEqual(
            {
                "kind": "radio_group",
                "options": [
                    {"value": "1", "label": "1 = Suspect"},
                    {"value": "2", "label": "2 = Concomitant"},
                    {"value": "3", "label": "3 = Interacting"},
                    {"value": "4", "label": "4 = Drug Not Administered"},
                ],
            },
            fields["G.k.1"]["control"],
        )
        terminology_source = (repo / "db/bootstrap/09-terminology.sql").read_text(
            encoding="utf-8"
        )
        self.assertIn(
            "('drug_characterization', '4', 'Drug Not Administered', 4)",
            terminology_source,
        )

        expected_radix_options = {
            "G.k.4.r.1b": [
                ("mg", "mg = milligram"),
                ("g", "g = gram"),
                ("ug", "ug = microgram"),
                ("mL", "mL = milliliter"),
                ("L", "L = liter"),
                ("[IU]", "[IU] = international unit"),
            ],
            "G.k.4.r.3": [
                ("a", "a:year"),
                ("mo", "mo:month"),
                ("wk", "wk:week"),
                ("d", "d:day"),
                ("h", "h:hour"),
                ("min", "min:minute"),
                ("{cyclical}", "{cyclical}:cyclical"),
                ("{asnecessary}", "{asnecessary}:as necessary"),
                ("{total}", "{total}:total"),
            ],
            "G.k.4.r.6b": [
                ("s", "s = second"),
                ("min", "min = minute"),
                ("h", "h = hour"),
                ("d", "d = day"),
                ("wk", "wk = week"),
                ("mo", "mo = month"),
                ("a", "a = year"),
            ],
        }
        for code, options in expected_radix_options.items():
            self.assertEqual(
                {
                    "kind": "radix_select",
                    "options": [
                        {"value": value, "label": label}
                        for value, label in options
                    ],
                },
                fields[code]["control"],
            )

        self.assertEqual(
            {
                "kind": "native_select",
                "options": [{"value": "28.1", "label": "28.1"}],
            },
            fields["G.k.7.r.2a"]["control"],
        )
        self.assertEqual("10004087", fields["G.k.7.r.2b"]["control"]["optionValue"])
        self.assertEqual("28.1", fields["G.k.7.r.2b"]["control"]["sourceVersion"])
        self.assertEqual(
            [{"frontendPath": fields["G.k.7.r.2a"]["frontendPath"], "value": "28.1"}],
            fields["G.k.7.r.2b"]["control"]["selectionEffects"],
        )
        self.assertEqual("1086735", fields["G.k.2.1.KR.1b"]["control"]["optionValue"])
        self.assertEqual("GLOBALC3Mar26", fields["G.k.2.1.KR.1b"]["control"]["sourceVersion"])
        self.assertEqual(
            {
                "mfds-g-product-required": {
                    "kind": "mfds_product_direct_input",
                    "productionReference": "mfdsProduct.item_seq",
                }
            },
            fields["G.k.2.1.KR.1b"]["scenarioControls"],
        )
        self.assertEqual(
            [
                {"frontendPath": fields["G.k.2.1.KR.1a"]["frontendPath"], "value": "GLOBALC3Mar26"},
                {"frontendPath": fields["G.k.2.2"]["frontendPath"], "value": "1-naphthaleneacetic acid"},
            ],
            fields["G.k.2.1.KR.1b"]["control"]["selectionEffects"],
        )
        self.assertEqual("M040353", fields["G.k.2.3.r.1.KR.1b"]["control"]["optionValue"])
        self.assertEqual("2026-10-01", fields["G.k.2.3.r.1.KR.1b"]["control"]["sourceVersion"])
        self.assertEqual(
            [
                {"frontendPath": fields["G.k.2.3.r.1.KR.1a"]["frontendPath"], "value": "2026-10-01"},
                {"frontendPath": fields["G.k.2.3.r.1"]["frontendPath"], "value": "아세트아미노펜"},
            ],
            fields["G.k.2.3.r.1.KR.1b"]["control"]["selectionEffects"],
        )
        self.assertEqual(
            {
                "mfds-g-substance-version-required": {
                    "kind": "mfds_foreign_substance_direct_input",
                    "productionReference": "whoSubstance.code",
                }
            },
            fields["G.k.2.3.r.1.KR.1b"]["scenarioControls"],
        )
        self.assertEqual(
            [{
                "code": "G.k.2.4",
                "frontendPath": "drugs[].obtainDrugCountry",
                "value": "KR",
            }],
            fields["G.k.2.3.r.1.KR.1b"]["renderPrerequisites"],
        )

        for code in (
            "FDA.G.k.12.r.2.r",
            "FDA.G.k.12.r.3.r",
            "FDA.G.k.12.r.11.r",
        ):
            self.assertEqual("FDA Device Code Value", fields[code]["auditItem"])

    def test_lb_terminology_controls_match_reviewed_live_options(self):
        repo = Path(__file__).resolve().parents[2]
        contract = json.loads(
            (repo / "registry/editor-contracts/lb.json").read_text(encoding="utf-8")
        )
        fields = {field["code"]: field for field in contract["fields"]}
        version = fields["F.r.2.2a"]
        code = fields["F.r.2.2b"]
        self.assertEqual("28.1", version["roundTripValue"])
        self.assertEqual(
            {"kind": "native_select", "options": [{"value": "28.1", "label": "28.1"}]},
            version["control"],
        )
        self.assertEqual("10088978", code["roundTripValue"])
        self.assertEqual("Alarm clock headache", code["control"]["queryValue"])
        self.assertEqual("Alarm clock headache (10088978)", code["control"]["optionLabel"])
        self.assertEqual("10088978", code["control"]["selectedLabel"])
        self.assertEqual("28.1", code["control"]["sourceVersion"])
        self.assertEqual("en", code["control"]["sourceLanguage"])
        self.assertEqual(
            [{"frontendPath": version["frontendPath"], "value": "28.1"}],
            code["control"]["selectionEffects"],
        )
        self.assertEqual(
            {
                "kind": "async_autocomplete",
                "queryValue": "mg/dL",
                "optionValue": "mg/dL",
                "optionDataValue": "mg/dL",
                "optionLabel": "mg/dL = milligram per deciliter",
                "selectedLabel": "mg/dL = milligram per deciliter",
            },
            fields["F.r.3.3"]["control"],
        )
        self.assertEqual(
            "testResults[].testResultUnit",
            next(field for field in json.loads(
                (repo / "crates/libs/validator/field-rules/lb.json").read_text()
            )["editorMappings"] if field["code"] == "F.r.3.3")["businessValidation"]["issuePath"],
        )

    def test_string_checkbox_control_requires_explicit_string_values(self):
        result = validate.ValidationResult()
        editor_contract._validate_control(
            "C.2.r.5",
            {"kind": "string_checkbox", "checkedValue": "", "uncheckedValue": False},
            result,
        )
        self.assertEqual(
            [
                "C.2.r.5 string_checkbox control requires checkedValue",
                "C.2.r.5 string_checkbox control requires uncheckedValue",
            ],
            result.errors,
        )

    def test_incomplete_is_a_valid_registry_status(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(
                root,
                json.dumps([self.editor_row(status="incomplete")]),
            )

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertEqual([], result.errors)

    def test_default_validation_enforces_audited_source_coverage(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(
                root,
                json.dumps([self.editor_row(status="incomplete")]),
            )
            rules = root / "dictionary" / "rules"
            rules.mkdir(parents=True)
            (rules / "ich.json").write_text(
                json.dumps(
                    {
                        "authority": "ICH",
                        "source": "fixture",
                        "rules": {"C.1.3": "report type requirement"},
                    }
                ),
                encoding="utf-8",
            )
            contracts = root / "editor-contracts"
            contracts.mkdir()
            (contracts / "ci.json").write_text(
                json.dumps(
                    {
                        "pageId": "CI",
                        "registryFile": "sections/c-safety-report.json",
                        "fields": [
                            {"code": "C.1.3", "authority": "ICH"}
                        ],
                    }
                ),
                encoding="utf-8",
            )
            (root / "rule-source-coverage.json").write_text(
                json.dumps(
                    {
                        "version": 1,
                        "auditedPages": ["CI"],
                        "sources": [],
                    }
                ),
                encoding="utf-8",
            )

            result = validate.validate_registry(
                root,
                validate_backend_inventory=False,
            )

        self.assertIn(
            "missing ICH/C.1.3 source coverage",
            result.errors,
        )

    def valid_row(self, overrides: dict[str, str] | None = None) -> str:
        values = {
            "authority": "ICH",
            "status": "complete",
            "backend_status": "mapped",
            "frontend_status": "mapped",
            "extra_fields": "",
        }
        if overrides:
            values.update(overrides)
        return """[
  {{
    "id": "C.3.2",
    "e2br3_code": "C.3.2",
    "label": "Sender's Organisation",
    "section": "C",
    "authority": "{authority}",
    "status": "{status}",
    "backend": {{"status": "{backend_status}", "model": "SenderInformation", "field": "organization_name", "evidence": "example"}},
    "frontend": {{"status": "{frontend_status}", "section": "sender", "field": "organizationName", "evidence": "example"}}{extra_fields}
  }}
]""".format(**values)

    def test_accepts_valid_single_authority_row(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.valid_row())

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertEqual([], result.errors)

    def test_rejects_combined_authority_values(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.valid_row({"authority": "ICH+FDA"}))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn("invalid authority", "\n".join(result.errors))

    def test_complete_rows_require_every_side_to_be_mapped(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.valid_row({"backend_status": "missing"}))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn("complete rows require backend.status to be mapped", "\n".join(result.errors))

    def test_backend_missing_status_requires_backend_mapping_missing(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.valid_row({"status": "backend_missing"}))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn("backend_missing rows require backend.status to be missing", "\n".join(result.errors))

    def test_frontend_missing_status_requires_frontend_mapping_missing(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.valid_row({"status": "frontend_missing"}))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn("frontend_missing rows require frontend.status to be missing", "\n".join(result.errors))

    def test_conflict_status_requires_a_conflicting_side(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.valid_row({"status": "conflict"}))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn("conflict rows require backend.status or frontend.status to be conflict", "\n".join(result.errors))

    def test_rejects_exporter_and_validator_blocks_until_ready(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(
                root,
                self.valid_row(
                    {
                        "extra_fields": ',\n    "xml": {"status": "mapped", "evidence": "example"},\n    "validation": {"status": "mapped", "evidence": "example"}'
                    }
                ),
            )

            result = validate.validate_registry(root, validate_backend_inventory=False)

        errors = "\n".join(result.errors)
        self.assertIn("unsupported field xml", errors)
        self.assertIn("unsupported field validation", errors)

    def test_mapped_backend_requires_model_and_field(self):
        row = self.valid_row().replace('"model": "SenderInformation", ', "")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, row)

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn("backend.model is required when status is mapped", "\n".join(result.errors))

    def test_mapped_frontend_requires_section_and_field(self):
        row = self.valid_row().replace('"section": "sender", ', "")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, row)

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn("frontend.section is required when status is mapped", "\n".join(result.errors))

    def test_rejects_duplicate_backend_model_field(self):
        two_rows = self.valid_row().replace(
            "\n]",
            """,
  {
    "id": "C.3.3.3",
    "e2br3_code": "C.3.3.3",
    "label": "Sender Given Name",
    "section": "C",
    "authority": "ICH",
    "status": "complete",
    "backend": {"status": "mapped", "model": "SenderInformation", "field": "organization_name", "evidence": "example"},
    "frontend": {"status": "mapped", "section": "sender", "field": "senderGivenName", "evidence": "example"}
  }
]""",
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, two_rows)

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn("duplicate backend mapping SenderInformation.organization_name", "\n".join(result.errors))

    def test_rejects_duplicate_frontend_section_field(self):
        two_rows = self.valid_row().replace(
            "\n]",
            """,
  {
    "id": "C.3.3.3",
    "e2br3_code": "C.3.3.3",
    "label": "Sender Given Name",
    "section": "C",
    "authority": "ICH",
    "status": "complete",
    "backend": {"status": "mapped", "model": "SenderInformation", "field": "person_given_name", "evidence": "example"},
    "frontend": {"status": "mapped", "section": "sender", "field": "organizationName", "evidence": "example"}
  }
]""",
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, two_rows)

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn("duplicate frontend mapping sender.organizationName", "\n".join(result.errors))

    def test_extracts_public_fields_from_rust_struct(self):
        source = """
#[derive(Debug, Clone)]
pub struct PatientInformation {
    pub id: Uuid,
    pub case_id: Uuid,
    pub patient_initial: Option<String>,
    pub patient_age_group: Option<String>,
}
"""

        fields = validate.extract_rust_struct_fields(source, "PatientInformation")

        self.assertEqual(["id", "case_id", "patient_initial", "patient_age_group"], fields)

    def test_struct_extraction_fails_when_struct_is_missing(self):
        source = "pub struct OtherModel { pub id: Uuid }"

        with self.assertRaises(validate.InventoryError):
            validate.extract_rust_struct_fields(source, "PatientInformation")

    def test_rejects_backend_field_present_in_source_but_missing_from_registry(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source_dir = root / "crates/libs/lib-core/src/model"
            source_dir.mkdir(parents=True)
            (source_dir / "patient.rs").write_text(
                """
pub struct PatientInformation {
    pub id: Uuid,
    pub case_id: Uuid,
    pub patient_initial: Option<String>,
}
""",
                encoding="utf-8",
            )
            self.write_registry(root, "[]")

            result = validate.validate_registry(
                root,
                backend_models={"PatientInformation": "crates/libs/lib-core/src/model/patient.rs"},
            )

        self.assertIn(
            "missing backend mapping: PatientInformation.patient_initial",
            "\n".join(result.errors),
        )

    def test_rejects_registry_backend_mapping_absent_from_source(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source_dir = root / "crates/libs/lib-core/src/model"
            source_dir.mkdir(parents=True)
            (source_dir / "patient.rs").write_text(
                """
pub struct PatientInformation {
    pub id: Uuid,
    pub case_id: Uuid,
    pub patient_initial: Option<String>,
}
""",
                encoding="utf-8",
            )
            row = self.valid_row().replace('"model": "SenderInformation"', '"model": "PatientInformation"')
            row = row.replace('"field": "organization_name"', '"field": "patient_sex"')
            self.write_registry(root, row)

            result = validate.validate_registry(
                root,
                backend_models={"PatientInformation": "crates/libs/lib-core/src/model/patient.rs"},
            )

        self.assertIn(
            "unknown backend mapping: PatientInformation.patient_sex",
            "\n".join(result.errors),
        )

    def null_flavor_source(self) -> str:
        return """
pub struct PatientInformation {
    pub id: Uuid,
    pub case_id: Uuid,
    pub patient_initial: Option<String>,
    pub patient_initial_null_flavor: Option<String>,
}
"""

    def null_flavor_row(self, field: str) -> str:
        row = self.valid_row().replace('"model": "SenderInformation"', '"model": "PatientInformation"')
        row = row.replace('"field": "organization_name"', f'"field": "{field}"')
        return row.replace('"field": "organizationName"', '"field": "patientInitialNullFlavor"')

    def test_accepts_backend_mapping_to_an_existing_null_flavor_column(self):
        # A dedicated nullFlavor field (its own frontend input and its own column)
        # must be mappable, so end-to-end joins resolve to a real column.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source_dir = root / "crates/libs/lib-core/src/model"
            source_dir.mkdir(parents=True)
            (source_dir / "patient.rs").write_text(self.null_flavor_source(), encoding="utf-8")
            self.write_registry(root, self.null_flavor_row("patient_initial_null_flavor"))

            result = validate.validate_registry(
                root,
                backend_models={"PatientInformation": "crates/libs/lib-core/src/model/patient.rs"},
            )

        # patient_initial stays unmapped-but-ignored; only the null_flavor mapping matters here.
        self.assertNotIn("unknown backend mapping", "\n".join(result.errors))

    def test_rejects_backend_mapping_to_a_null_flavor_column_that_does_not_exist(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source_dir = root / "crates/libs/lib-core/src/model"
            source_dir.mkdir(parents=True)
            (source_dir / "patient.rs").write_text(self.null_flavor_source(), encoding="utf-8")
            self.write_registry(root, self.null_flavor_row("patient_sex_null_flavor"))

            result = validate.validate_registry(
                root,
                backend_models={"PatientInformation": "crates/libs/lib-core/src/model/patient.rs"},
            )

        self.assertIn(
            "unknown backend mapping: PatientInformation.patient_sex_null_flavor",
            "\n".join(result.errors),
        )

    def test_null_flavor_column_requires_its_base_element_mapping(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source_dir = root / "crates/libs/lib-core/src/model"
            source_dir.mkdir(parents=True)
            (source_dir / "patient.rs").write_text(self.null_flavor_source(), encoding="utf-8")
            self.write_registry(root, "[]")

            result = validate.validate_registry(
                root,
                backend_models={"PatientInformation": "crates/libs/lib-core/src/model/patient.rs"},
            )

        self.assertIn(
            "missing nullFlavor companion mapping: PatientInformation.patient_initial_null_flavor",
            "\n".join(result.errors),
        )

    def test_rejects_frontend_field_present_in_source_but_missing_from_registry(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source_dir = root / "frontend"
            source_dir.mkdir()
            (source_dir / "SectionE.tsx").write_text(
                '<Controller name={`reactions.${activeIndex}.reactionCountry`} />',
                encoding="utf-8",
            )
            self.write_registry(root, "[]")

            result = validate.validate_registry(
                root,
                validate_backend_inventory=False,
                validate_frontend_inventory=True,
                frontend_source_globs=["frontend/SectionE.tsx"],
            )

        self.assertIn(
            "missing frontend mapping: reactions.reactionCountry",
            "\n".join(result.errors),
        )

    def test_rejects_registry_frontend_mapping_absent_from_source(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source_dir = root / "frontend"
            source_dir.mkdir()
            (source_dir / "SectionE.tsx").write_text(
                '<Controller name={`reactions.${activeIndex}.reactionCountry`} />',
                encoding="utf-8",
            )
            row = self.valid_row().replace('"section": "sender"', '"section": "reactions"')
            row = row.replace('"field": "organizationName"', '"field": "reactionOutcome"')
            self.write_registry(root, row)

            result = validate.validate_registry(
                root,
                validate_backend_inventory=False,
                validate_frontend_inventory=True,
                frontend_source_globs=["frontend/SectionE.tsx"],
            )

        self.assertIn(
            "unknown frontend mapping: reactions.reactionOutcome",
            "\n".join(result.errors),
        )

    def test_backend_inventory_paths_are_resolved_from_repo_root_when_registry_root_is_nested(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo_root = Path(tmp)
            registry_root = repo_root / "registry"
            registry_root.mkdir()
            source_dir = repo_root / "crates/libs/lib-core/src/model"
            source_dir.mkdir(parents=True)
            (source_dir / "patient.rs").write_text(
                """
pub struct PatientInformation {
    pub id: Uuid,
    pub case_id: Uuid,
    pub patient_initial: Option<String>,
}
""",
                encoding="utf-8",
            )
            self.write_registry(registry_root, "[]")

            result = validate.validate_registry(
                registry_root,
                backend_models={"PatientInformation": "crates/libs/lib-core/src/model/patient.rs"},
            )

        self.assertIn(
            "missing backend mapping: PatientInformation.patient_initial",
            "\n".join(result.errors),
        )

    def test_backend_inventory_ignores_explicit_technical_foreign_keys(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source_dir = root / "crates/libs/lib-core/src/model"
            source_dir.mkdir(parents=True)
            (source_dir / "patient.rs").write_text(
                """
pub struct MedicalHistoryEpisode {
    pub id: Uuid,
    pub case_id: Uuid,
    pub patient_id: Uuid,
    pub source_patient_presave_id: Option<Uuid>,
    pub meddra_code: Option<String>,
}
""",
                encoding="utf-8",
            )

            keys = validate.extract_backend_inventory(
                root,
                {"MedicalHistoryEpisode": "crates/libs/lib-core/src/model/patient.rs"},
            )

        self.assertEqual({"MedicalHistoryEpisode.meddra_code"}, keys)

    def test_backend_inventory_ignores_null_flavor_support_fields(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source_dir = root / "crates/libs/lib-core/src/model"
            source_dir.mkdir(parents=True)
            (source_dir / "safety_report.rs").write_text(
                """
pub struct SafetyReportIdentification {
    pub id: Uuid,
    pub case_id: Uuid,
    pub transmission_date: Option<Date>,
    pub transmission_date_null_flavor: Option<String>,
}
""",
                encoding="utf-8",
            )

            keys = validate.extract_backend_inventory(
                root,
                {"SafetyReportIdentification": "crates/libs/lib-core/src/model/safety_report.rs"},
            )

        self.assertEqual({"SafetyReportIdentification.transmission_date"}, keys)

    def test_backend_inventory_ignores_plumbing_only_on_the_owning_model(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source_dir = root / "crates/libs/lib-core/src/model"
            source_dir.mkdir(parents=True)
            (source_dir / "case.rs").write_text(
                """
pub struct Case {
    pub id: Uuid,
    pub report_year: Option<String>,
    pub workflow_status: String,
}

pub struct Reaction {
    pub id: Uuid,
    pub workflow_status: Option<String>,
}
""",
                encoding="utf-8",
            )

            keys = validate.extract_backend_inventory(
                root,
                {
                    "Case": "crates/libs/lib-core/src/model/case.rs",
                    "Reaction": "crates/libs/lib-core/src/model/case.rs",
                },
            )

        # Case.workflow_status is app plumbing, but the same name on another
        # model must stay tracked -- the ignore is scoped, not global.
        self.assertEqual({"Case.report_year", "Reaction.workflow_status"}, keys)

    def test_repository_section_n_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"MessageHeader": "crates/libs/lib-core/src/model/message_header.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_c1_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={
                "SafetyReportIdentification": "crates/libs/lib-core/src/model/safety_report.rs"
            },
        )

        self.assertEqual([], result.errors)

    def test_repository_c2_primary_source_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"PrimarySource": "crates/libs/lib-core/src/model/safety_report.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_c3_sender_information_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"SenderInformation": "crates/libs/lib-core/src/model/safety_report.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_receiver_information_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"ReceiverInformation": "crates/libs/lib-core/src/model/receiver.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_c5_study_information_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={
                "StudyInformation": "crates/libs/lib-core/src/model/safety_report.rs",
                "StudyRegistrationNumber": "crates/libs/lib-core/src/model/safety_report.rs",
                "StudyFdaCrossReportedInd": "crates/libs/lib-core/src/model/safety_report.rs",
            },
        )

        self.assertEqual([], result.errors)

    def test_repository_c1_case_identifier_children_have_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={
                "OtherCaseIdentifier": "crates/libs/lib-core/src/model/case_identifiers.rs",
                "LinkedReportNumber": "crates/libs/lib-core/src/model/case_identifiers.rs",
            },
        )

        self.assertEqual([], result.errors)

    def test_repository_c1_c4_document_inventory_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={
                "DocumentsHeldBySender": "crates/libs/lib-core/src/model/safety_report.rs",
                "LiteratureReference": "crates/libs/lib-core/src/model/safety_report.rs",
            },
        )

        self.assertEqual([], result.errors)

    def test_repository_d9_autopsy_cause_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"AutopsyCauseOfDeath": "crates/libs/lib-core/src/model/patient.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_d9_reported_cause_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"ReportedCauseOfDeath": "crates/libs/lib-core/src/model/patient.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_d7_medical_history_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"MedicalHistoryEpisode": "crates/libs/lib-core/src/model/patient.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_d8_past_drug_history_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"PastDrugHistory": "crates/libs/lib-core/src/model/patient.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_d1_patient_identifier_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"PatientIdentifier": "crates/libs/lib-core/src/model/patient.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_d_patient_information_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"PatientInformation": "crates/libs/lib-core/src/model/patient.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_d9_patient_death_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"PatientDeathInformation": "crates/libs/lib-core/src/model/patient.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_d10_parent_information_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"ParentInformation": "crates/libs/lib-core/src/model/patient.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_d10_parent_medical_history_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"ParentMedicalHistory": "crates/libs/lib-core/src/model/parent_history.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_d10_parent_past_drug_history_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"ParentPastDrugHistory": "crates/libs/lib-core/src/model/parent_history.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_h5_case_summary_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"CaseSummaryInformation": "crates/libs/lib-core/src/model/narrative.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_h_narrative_information_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"NarrativeInformation": "crates/libs/lib-core/src/model/narrative.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_h_sender_diagnosis_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"SenderDiagnosis": "crates/libs/lib-core/src/model/narrative.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_e_reaction_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"Reaction": "crates/libs/lib-core/src/model/reaction.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_f_test_result_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"TestResult": "crates/libs/lib-core/src/model/test_result.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_gk23_active_substance_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"DrugActiveSubstance": "crates/libs/lib-core/src/model/drug.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_gk4_dosage_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"DosageInformation": "crates/libs/lib-core/src/model/drug.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_gk7_indication_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"DrugIndication": "crates/libs/lib-core/src/model/drug.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_fda_device_characteristic_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"DrugDeviceCharacteristic": "crates/libs/lib-core/src/model/drug.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_gk_drug_information_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={"DrugInformation": "crates/libs/lib-core/src/model/drug.rs"},
        )

        self.assertEqual([], result.errors)

    def test_repository_gk8_gk9_recurrence_assessment_has_complete_backend_inventory(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            backend_models={
                "DrugReactionAssessment": "crates/libs/lib-core/src/model/drug_reaction_assessment.rs",
                "RelatednessAssessment": "crates/libs/lib-core/src/model/drug_reaction_assessment.rs",
            },
        )

        self.assertEqual([], result.errors)

    def test_repository_frontend_inputs_have_registry_rows(self):
        registry_root = Path(__file__).resolve().parents[1]
        frontend_root = validate.extract_frontend_fields.resolve_frontend_root(
            registry_root.parent
        )
        if not frontend_root.exists():
            self.skipTest("sibling frontend checkout is not available")

        result = validate.validate_registry(
            registry_root,
            validate_backend_inventory=False,
            validate_frontend_inventory=True,
        )

        self.assertEqual([], result.errors)


class DictionaryValidatorTests(unittest.TestCase):
    def write_registry(self, root: Path, row: str) -> None:
        (root / "index.json").write_text(
            '{"sections":["sections/c-safety-report.json"]}',
            encoding="utf-8",
        )
        sections = root / "sections"
        sections.mkdir()
        (sections / "c-safety-report.json").write_text(row, encoding="utf-8")

    def write_dictionary(self, root: Path, name: str, payload: str) -> None:
        dictionary_dir = root / "dictionary"
        dictionary_dir.mkdir(exist_ok=True)
        (dictionary_dir / name).write_text(payload, encoding="utf-8")

    def ich_dictionary(self, entries: str) -> str:
        return '{"authority": "ICH", "source": "test", "entries": [%s]}' % entries

    def mfds_dictionary(self, entries: str) -> str:
        return '{"authority": "MFDS", "source": "test", "entries": [%s]}' % entries

    def sender_row(self, authority: str = "ICH", code: str = "C.3.2") -> str:
        return """[
  {
    "id": "%s",
    "e2br3_code": "%s",
    "label": "Sender's Organisation",
    "section": "C",
    "authority": "%s",
    "status": "complete",
    "backend": {"status": "mapped", "model": "SenderInformation", "field": "organization_name", "evidence": "example"},
    "frontend": {"status": "mapped", "section": "sender", "field": "organizationName", "evidence": "example"}
  }
]""" % (code, code, authority)

    def validate_allowed_constraint(self, constraint: dict[str, object]):
        entry = {
            "code": "C.3.1",
            "name": "Sender Type",
            "section": "C",
            "kind": "element",
            "conformance": "mandatory",
            "allowed_values": "Numeric",
            "allowed_value_constraint": constraint,
        }
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row(code="C.3.1"))
            self.write_dictionary(
                root,
                "ich-e2br3.json",
                self.ich_dictionary(json.dumps(entry)),
            )
            return validate.validate_registry(root, validate_backend_inventory=False)

    SENDER_ENTRY = '{"code": "C.3.2", "name": "Sender\'s Organisation", "section": "C", "kind": "element", "conformance": "mandatory"}'

    def test_dictionary_numeric_constraint_requires_shape(self):
        result = self.validate_allowed_constraint(
            {"kind": "numeric", "enforcement": "case_validate"}
        )

        self.assertIn(
            "numeric allowed_value_constraint requires numeric_shape",
            "\n".join(result.errors),
        )

    def test_dictionary_identifier_rejects_vocabulary_scope(self):
        result = self.validate_allowed_constraint(
            {
                "kind": "vocabulary",
                "identifier_profile": "mpid",
                "vocabulary_scope": "all",
                "enforcement": "case_validate",
            }
        )

        self.assertIn(
            "identifier_profile cannot be combined with vocabulary_scope",
            "\n".join(result.errors),
        )

    def test_dictionary_element_entries_require_conformance(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row())
            self.write_dictionary(
                root,
                "ich-e2br3.json",
                self.ich_dictionary('{"code": "C.3.2", "name": "Sender", "section": "C", "kind": "element"}'),
            )

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn("conformance is required for element entries", "\n".join(result.errors))

    def test_rejects_duplicate_codes_across_dictionaries(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row())
            self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(self.SENDER_ENTRY))
            self.write_dictionary(
                root,
                "mfds-regional.json",
                '{"authority": "MFDS", "source": "test", "entries": [%s]}' % self.SENDER_ENTRY,
            )

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn("duplicate dictionary code C.3.2", "\n".join(result.errors))

    def test_strict_dictionary_flags_ich_code_missing_from_dictionary(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row())
            self.write_dictionary(
                root,
                "ich-e2br3.json",
                self.ich_dictionary('{"code": "C.1.2", "name": "Date of Creation", "section": "C", "kind": "element", "conformance": "mandatory"}'),
            )

            result = validate.validate_registry(
                root,
                validate_backend_inventory=False,
                validate_dictionary_membership=True,
            )

        self.assertIn(
            "e2br3_code C.3.2 is not defined in the ICH dictionary",
            "\n".join(result.errors),
        )

    def test_strict_dictionary_accepts_code_defined_in_dictionary(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row())
            self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(self.SENDER_ENTRY))

            result = validate.validate_registry(
                root,
                validate_backend_inventory=False,
                validate_dictionary_membership=True,
            )

        self.assertEqual([], result.errors)

    def local_only_row(self, code: str) -> str:
        return self.sender_row(code=code).replace(
            '"status": "complete",',
            '"status": "complete",\n    "local_only": true,',
        )

    def test_local_only_rows_are_exempt_from_dictionary_membership(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.local_only_row("C.3.local.receiverName"))
            self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(self.SENDER_ENTRY))

            result = validate.validate_registry(
                root,
                validate_backend_inventory=False,
                validate_dictionary_membership=True,
            )

        self.assertNotIn("is not defined in the", "\n".join(result.errors))

    def test_local_only_rows_must_not_use_dictionary_codes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.local_only_row("C.3.2"))
            self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(self.SENDER_ENTRY))

            result = validate.validate_registry(
                root,
                validate_backend_inventory=False,
                validate_dictionary_membership=True,
            )

        self.assertIn(
            "local_only row uses e2br3_code C.3.2 which is defined in the ICH dictionary",
            "\n".join(result.errors),
        )

    def test_local_only_must_be_a_boolean(self):
        row = self.sender_row().replace(
            '"status": "complete",',
            '"status": "complete",\n    "local_only": "yes",',
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, row)

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn("local_only must be a boolean", "\n".join(result.errors))

    def test_strict_dictionary_flags_synthetic_codes(self):
        row = self.sender_row(code="C.3@localOnlyControl")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, row)
            self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(self.SENDER_ENTRY))

            result = validate.validate_registry(
                root,
                validate_backend_inventory=False,
                validate_dictionary_membership=True,
            )

        self.assertIn(
            "e2br3_code C.3@localOnlyControl is not defined in the ICH dictionary",
            "\n".join(result.errors),
        )

    def test_strict_dictionary_skips_authorities_without_a_dictionary(self):
        row = self.sender_row(authority="FDA", code="FDA.C.1.x")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, row)
            self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(self.SENDER_ENTRY))

            result = validate.validate_registry(
                root,
                validate_backend_inventory=False,
                validate_dictionary_membership=True,
            )

        self.assertNotIn("is not defined in the", "\n".join(result.errors))

    def test_strict_dictionary_flags_mfds_code_missing_from_official_extensions(self):
        row = self.sender_row(authority="MFDS", code="C.1.KR.customField")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, row)
            self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(self.SENDER_ENTRY))
            self.write_dictionary(
                root,
                "mfds-regional.json",
                '{"authority": "MFDS", "source": "test", "entries": ['
                '{"code": "C.2.r.4.KR.1", "name": "Other HCP Type", "section": "C", "kind": "element", "conformance": "optional"}'
                "]}",
            )

            result = validate.validate_registry(
                root,
                validate_backend_inventory=False,
                validate_dictionary_membership=True,
            )

        self.assertIn(
            "e2br3_code C.1.KR.customField is not defined in the MFDS dictionary",
            "\n".join(result.errors),
        )

    def test_rules_files_must_reference_dictionary_codes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row())
            self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(self.SENDER_ENTRY))
            rules_dir = root / "dictionary" / "rules"
            rules_dir.mkdir(parents=True)
            (rules_dir / "ich.json").write_text(
                '{"authority": "ICH", "source": "test", "rules": {"C.3.2": "real rule", "C.9.9": "ghost rule"}}',
                encoding="utf-8",
            )

            result = validate.validate_registry(root, validate_backend_inventory=False)

        errors = "\n".join(result.errors)
        self.assertIn("rule for unknown code C.9.9", errors)
        self.assertNotIn("C.3.2", errors)

    def test_dictionary_entries_accept_vocabulary(self):
        entry = (
            '{"code": "C.3.2", "name": "Sender", "section": "C", "kind": "element",'
            ' "conformance": "mandatory", "vocabulary": "MedDRA"}'
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row())
            self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(entry))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertEqual([], result.errors)

    def test_dictionary_entries_accept_receiver_specific_vocabularies(self):
        entry = (
            '{"code": "D.8.r.1.KR.1b", "name": "Medicinal Product ID",'
            ' "section": "D", "kind": "element", "conformance": "conditional_mandatory",'
            ' "vocabulary_variants": ['
            ' {"receiver": "KR", "vocabulary": "MFDS_PRODUCT", "vocabulary_scope": "item_seq"},'
            ' {"receiver": "FR", "vocabulary": "WHODrug", "vocabulary_scope": "all"}]}'
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row(authority="MFDS", code="D.8.r.1.KR.1b"))
            self.write_dictionary(root, "mfds-regional.json", self.mfds_dictionary(entry))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertEqual([], result.errors)

    def test_dictionary_receiver_specific_vocabularies_reject_duplicate_receiver(self):
        entry = (
            '{"code": "D.8.r.1.KR.1b", "name": "Medicinal Product ID",'
            ' "section": "D", "kind": "element", "conformance": "conditional_mandatory",'
            ' "vocabulary_variants": ['
            ' {"receiver": "KR", "vocabulary": "MFDS_PRODUCT", "vocabulary_scope": "item_seq"},'
            ' {"receiver": "KR", "vocabulary": "WHODrug", "vocabulary_scope": "all"}]}'
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row(authority="MFDS", code="D.8.r.1.KR.1b"))
            self.write_dictionary(root, "mfds-regional.json", self.mfds_dictionary(entry))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn("duplicate vocabulary receiver 'KR'", "\n".join(result.errors))

    def test_dictionary_receiver_specific_vocabularies_reject_unconditional_vocabulary(self):
        entry = (
            '{"code": "D.8.r.1.KR.1b", "name": "Medicinal Product ID",'
            ' "section": "D", "kind": "element", "conformance": "conditional_mandatory",'
            ' "vocabulary": "WHODrug", "vocabulary_variants": ['
            ' {"receiver": "KR", "vocabulary": "MFDS_PRODUCT", "vocabulary_scope": "item_seq"}]}'
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row(authority="MFDS", code="D.8.r.1.KR.1b"))
            self.write_dictionary(root, "mfds-regional.json", self.mfds_dictionary(entry))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn("cannot combine vocabulary with vocabulary_variants", "\n".join(result.errors))

    def test_dictionary_receiver_specific_vocabularies_reject_unknown_metadata(self):
        entry = (
            '{"code": "D.8.r.1.KR.1b", "name": "Medicinal Product ID",'
            ' "section": "D", "kind": "element", "conformance": "conditional_mandatory",'
            ' "vocabulary_variants": ['
            ' {"receiver": "US", "vocabulary": "MagicCodes", "vocabulary_scope": "products"}]}'
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row(authority="MFDS", code="D.8.r.1.KR.1b"))
            self.write_dictionary(root, "mfds-regional.json", self.mfds_dictionary(entry))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        errors = "\n".join(result.errors)
        self.assertIn("invalid vocabulary receiver 'US'", errors)
        self.assertIn("invalid vocabulary 'MagicCodes'", errors)
        self.assertIn("invalid vocabulary_scope 'products'", errors)

    def test_dictionary_entries_accept_allowed_value_code_set(self):
        entry = (
            '{"code": "C.3.1", "name": "Sender Type", "section": "C",'
            ' "kind": "element", "conformance": "mandatory",'
            ' "allowed_values": "1=Company 2=Authority",'
            ' "allowed_value_constraint":'
            ' {"kind": "code_set", "values": ["1", "2"],'
            ' "enforcement": "case_validate"}}'
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row(code="C.3.1"))
            self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(entry))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertEqual([], result.errors)

    def test_dictionary_code_set_constraint_requires_values(self):
        entry = (
            '{"code": "C.3.1", "name": "Sender Type", "section": "C",'
            ' "kind": "element", "conformance": "mandatory",'
            ' "allowed_values": "1=Company 2=Authority",'
            ' "allowed_value_constraint": {"kind": "code_set"}}'
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row(code="C.3.1"))
            self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(entry))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn(
            "code_set allowed_value_constraint requires values",
            "\n".join(result.errors),
        )

    def test_dictionary_allowed_value_constraint_requires_source_text(self):
        entry = (
            '{"code": "C.3.1", "name": "Sender Type", "section": "C",'
            ' "kind": "element", "conformance": "mandatory",'
            ' "allowed_value_constraint":'
            ' {"kind": "code_set", "values": ["1", "2"]}}'
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row(code="C.3.1"))
            self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(entry))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn(
            "allowed_value_constraint requires allowed_values source text",
            "\n".join(result.errors),
        )

    def test_dictionary_allowed_value_constraint_kind_must_be_known(self):
        entry = (
            '{"code": "C.3.1", "name": "Sender Type", "section": "C",'
            ' "kind": "element", "conformance": "mandatory",'
            ' "allowed_values": "1=Company 2=Authority",'
            ' "allowed_value_constraint": {"kind": "enum", "values": ["1"]}}'
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row(code="C.3.1"))
            self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(entry))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn(
            "invalid allowed_value_constraint kind 'enum'",
            "\n".join(result.errors),
        )

    def test_dictionary_vocabulary_must_be_a_known_value(self):
        entry = (
            '{"code": "C.3.2", "name": "Sender", "section": "C", "kind": "element",'
            ' "conformance": "mandatory", "vocabulary": "MagicCodes"}'
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row())
            self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(entry))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn("invalid vocabulary 'MagicCodes'", "\n".join(result.errors))

    def test_dictionary_entries_accept_fda_severity(self):
        entry = (
            '{"code": "C.3.2", "name": "Sender", "section": "C", "kind": "element",'
            ' "conformance": "mandatory", "fda_severity": "rejection", "fda_error_id": "R0008"}'
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row())
            self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(entry))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertEqual([], result.errors)

    def test_dictionary_entries_accept_condition_text(self):
        entry = (
            '{"code": "C.5.4", "name": "Study Type", "section": "C", "kind": "element",'
            ' "conformance": "conditional_mandatory",'
            ' "condition_text": "Optional, but required if C.1.3=2 (Report from study)."}'
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row())
            self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(entry))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertEqual([], result.errors)

    def test_ich_pdf_optional_conformance_corrections_are_applied(self):
        dictionary_path = (
            Path(__file__).resolve().parents[1]
            / "dictionary"
            / "ich-e2br3.json"
        )
        entries = {
            entry["code"]: entry
            for entry in json.loads(dictionary_path.read_text(encoding="utf-8"))[
                "entries"
            ]
        }
        optional_in_ich_pdf = {
            "C.2.r.2.5",
            "D.8.r.2a",
            "D.8.r.2b",
            "D.8.r.3a",
            "D.8.r.3b",
            "G.k.2.1.1a",
            "G.k.2.1.1b",
            "G.k.2.1.2a",
            "G.k.2.1.2b",
            "G.k.4.r.2",
        }

        self.assertEqual(
            {},
            {
                code: entries[code]["conformance"]
                for code in sorted(optional_in_ich_pdf)
                if entries[code]["conformance"] != "optional"
            },
        )

    def test_dictionary_fda_severity_must_be_valid(self):
        entry = (
            '{"code": "C.3.2", "name": "Sender", "section": "C", "kind": "element",'
            ' "conformance": "mandatory", "fda_severity": "fatal"}'
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row())
            self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(entry))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn("invalid fda_severity 'fatal'", "\n".join(result.errors))

    def test_dictionary_entries_no_longer_carry_rule_prose(self):
        entry = (
            '{"code": "C.3.2", "name": "Sender", "section": "C", "kind": "element",'
            ' "conformance": "mandatory", "business_rule": "prose"}'
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row())
            self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(entry))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn("unsupported field business_rule", "\n".join(result.errors))

    def test_dictionary_entries_accept_profiles_and_xpath(self):
        entry = (
            '{"code": "C.3.2", "name": "Sender", "section": "C", "kind": "element",'
            ' "conformance": "mandatory",'
            ' "profiles": {"post_market": "mandatory", "vaers": "conditional_mandatory"},'
            ' "hl7_data_type": "Instance Identifier (II)",'
            ' "hl7_component": "extension=Character String (ST)",'
            ' "xpath": "/MCCI_IN200100UV01/PORR_IN049016UV/sender"}'
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row())
            self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(entry))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertEqual([], result.errors)

    def test_dictionary_profiles_values_must_be_valid_conformances(self):
        entry = (
            '{"code": "C.3.2", "name": "Sender", "section": "C", "kind": "element",'
            ' "conformance": "mandatory", "profiles": {"post_market": "sometimes"}}'
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row())
            self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(entry))

            result = validate.validate_registry(root, validate_backend_inventory=False)

        self.assertIn("invalid profile conformance 'sometimes'", "\n".join(result.errors))

    def test_strict_dictionary_reports_elements_without_registry_rows(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row())
            self.write_dictionary(
                root,
                "ich-e2br3.json",
                self.ich_dictionary(
                    self.SENDER_ENTRY
                    + ', {"code": "C.1.1", "name": "Safety Report Unique Identifier", "section": "C", "kind": "element", "conformance": "mandatory"}'
                ),
            )

            result = validate.validate_registry(
                root,
                validate_backend_inventory=False,
                validate_dictionary_membership=True,
            )

        self.assertIn(
            "missing registry row for ICH element C.1.1",
            "\n".join(result.errors),
        )

    def test_strict_dictionary_requires_optional_elements_but_not_groups(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_registry(root, self.sender_row())
            self.write_dictionary(
                root,
                "ich-e2br3.json",
                self.ich_dictionary(
                    self.SENDER_ENTRY
                    + ', {"code": "C.1", "name": "Case Safety Report", "section": "C", "kind": "group"}'
                    + ', {"code": "C.1.5", "name": "Date of Most Recent Information", "section": "C", "kind": "element", "conformance": "optional"}'
                ),
            )

            result = validate.validate_registry(
                root,
                validate_backend_inventory=False,
                validate_dictionary_membership=True,
            )

        self.assertEqual(
            ["missing registry row for ICH element C.1.5"], result.errors
        )

    def test_repository_all_presave_inventories_are_complete(self):
        result = validate.validate_registry(
            validate_backend_inventory=False,
            validate_presave_registry_rows=True,
            validate_presave_inventory=True,
        )

        self.assertEqual([], result.errors)

    def test_repository_presave_index_covers_every_supported_section(self):
        index_path = validate.ROOT / "presaves/index.json"
        index = json.loads(index_path.read_text(encoding="utf-8"))

        self.assertEqual(
            [
                "sections/c-sender.json",
                "sections/c-receiver.json",
                "sections/g-product.json",
                "sections/c-reporter.json",
                "sections/c-study.json",
                "sections/h-narrative.json",
            ],
            index["sections"],
        )

    def test_ci_runs_strict_presave_inventory(self):
        workflow = (validate.ROOT.parent / ".github/workflows/ci.yml").read_text(
            encoding="utf-8"
        )

        self.assertIn(
            "python3 registry/tools/validate.py --strict-presave-inventory",
            workflow,
        )

    def test_ci_checks_out_frontend_branch_matching_backend_release_line(self):
        workflow = (validate.ROOT.parent / ".github/workflows/ci.yml").read_text(
            encoding="utf-8"
        )

        self.assertIn(
            "ref: ${{ (github.base_ref == 'dev' || github.ref_name == 'dev') && 'dev' || 'main' }}",
            workflow,
        )

    def test_ci_restores_xml_validation_assets_before_workspace_tests(self):
        workflow = (validate.ROOT.parent / ".github/workflows/ci.yml").read_text(
            encoding="utf-8"
        )

        self.assertIn("name: Checkout XML validation assets", workflow)
        self.assertIn("ref: 698f907f3abe6a555594931ace82e9fbcb9c204d", workflow)
        self.assertIn("deploy/ec2/schemas", workflow)
        self.assertIn("docs/refs/instances", workflow)
        self.assertIn("name: Prepare XML validation assets", workflow)
        self.assertLess(
            workflow.index("name: Prepare XML validation assets"),
            workflow.index("name: Run remaining workspace tests"),
        )

    def test_ci_serializes_workspace_tests_that_share_the_database(self):
        workflow = (validate.ROOT.parent / ".github/workflows/ci.yml").read_text(
            encoding="utf-8"
        )

        self.assertIn(
            "scripts/test-isolated-db.sh --workspace --exclude lib-core --exclude validator -- --test-threads=1",
            workflow,
        )

    def test_ci_requires_api_tests_before_publishing_dev_image(self):
        workflow = (validate.ROOT.parent / ".github/workflows/ci.yml").read_text(
            encoding="utf-8"
        )
        deploy_job = workflow.split("\n  deploy-dev:\n", 1)[1]

        self.assertIn(
            "needs: [registry, fmt, clippy, validator, test]",
            deploy_job,
        )



class RemovedOrphanLocalFieldsTests(unittest.TestCase):
    def test_removed_orphan_rows_and_backend_storage_are_absent(self):
        repo = Path(__file__).resolve().parents[2]
        removed_rows = {
            "E.local.includedInEmaImeList",
            "D.local.patientGivenName",
            "D.local.patientFamilyName",
            "G.k.local.supplemental.brandName",
            "G.k.local.supplemental.genericName",
            "G.k.local.supplemental.dosageText",
            "G.k.local.parentDosageText",
            "G.k.local.dosage.firstAdministrationTime",
            "G.k.local.dosage.lastAdministrationTime",
            "G.k.local.recurrence.reactionRecurred",
            "G.k.local.recurrence.rechallengeAction",
            "G.k.local.rechallenge",
            "G.k.local.recurrence.meddraVersion",
            "G.k.local.recurrence.meddraCode",
            "G.k.local.assessmentRecurrence.meddraVersion",
            "G.k.local.assessmentRecurrence.meddraCode",
        }
        registry_ids = set()
        for section in (repo / "registry" / "sections").glob("*.json"):
            registry_ids.update(row["id"] for row in json.loads(section.read_text()))
        self.assertTrue(removed_rows.isdisjoint(registry_ids))

        model_sources = "\n".join(
            path.read_text()
            for path in [
                repo / "crates/libs/lib-core/src/model/patient.rs",
                repo / "crates/libs/lib-core/src/model/drug.rs",
                repo / "crates/libs/lib-core/src/model/drug_reaction_assessment.rs",
                repo / "crates/libs/lib-core/src/model/reaction.rs",
                repo / "db/bootstrap/04-patient-information.sql",
                repo / "db/bootstrap/05-reactions.sql",
                repo / "db/bootstrap/07-drug-information.sql",
            ]
        )
        for field in (
            "patient_given_name",
            "patient_family_name",
            "drug_generic_name",
            "parent_dosage_text",
            "first_administration_time",
            "last_administration_time",
            "recurrence_meddra_version",
            "recurrence_meddra_code",
            "included_in_ema_ime_list",
        ):
            self.assertNotIn(field, model_sources)
        self.assertNotIn("pub brand_name: Option<String>", model_sources)
        self.assertNotIn("\n    brand_name VARCHAR", model_sources)
        self.assertFalse((repo / "crates/libs/lib-core/src/model/drug_recurrence.rs").exists())
        drug_source = (repo / "crates/libs/lib-core/src/model/drug.rs").read_text()
        drug_information_source = drug_source.split("// -- DosageInformation", 1)[0]
        self.assertNotIn("pub dosage_text: Option<String>", drug_information_source)
        bootstrap = (repo / "db/bootstrap/07-drug-information.sql").read_text()
        drug_table = bootstrap.split("CREATE TABLE dosage_information", 1)[0]
        self.assertNotIn("dosage_text", drug_table)


if __name__ == "__main__":
    unittest.main()
