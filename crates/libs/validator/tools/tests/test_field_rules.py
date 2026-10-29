import json
from pathlib import Path
import unittest


class FieldRuleMappingTests(unittest.TestCase):
    def test_lr_editor_contract_tracks_regional_business_validation_gap(self):
        repo = Path(__file__).resolve().parents[5]
        contract = json.loads(
            (repo / "crates/libs/validator/field-rules/lr.json").read_text(encoding="utf-8")
        )
        registry = json.loads(
            (repo / "registry/sections/c-safety-report.json").read_text(encoding="utf-8")
        )

        self.assertEqual(
            {
                field["code"]: field["businessValidation"]["status"]
                for field in contract["editorMappings"]
            },
            {
                "C.4.r.1": "not_applicable",
                "C.4.r.local.referenceTextNullFlavor": "not_applicable",
                "C.4.r.2": "not_applicable",
            },
        )
        literature_attachment = next(
            row for row in registry if row["id"] == "C.4.r.2"
        )
        self.assertEqual("incomplete", literature_attachment["status"])
        self.assertIn("FDA", literature_attachment["action"])
        self.assertIn("file name", literature_attachment["action"])
        self.assertIn("mediaType", literature_attachment["action"])

    def test_dm_age_business_validation_uses_backend_issue_paths(self):
        repo = Path(__file__).resolve().parents[5]
        contract = json.loads(
            (repo / "crates/libs/validator/field-rules/dm.json").read_text(encoding="utf-8")
        )
        actual = {
            field["code"]: (
                field.get("businessValidation", {}).get("ruleCode"),
                field.get("businessValidation", {}).get("issuePath"),
            )
            for field in contract["editorMappings"]
            if field["code"] in {"D.2.2a", "D.2.2b", "D.2.2.1a", "D.2.2.1b"}
        }
        self.assertEqual(
            {
                "D.2.2a": (
                    "ICH.D.2.2a.REQUIRED",
                    "patientInformation.ageAtTimeOfOnset",
                ),
                "D.2.2b": (
                    "ICH.D.2.2b.REQUIRED",
                    "patientInformation.ageUnit",
                ),
                "D.2.2.1a": (
                    "ICH.D.2.2.1a.REQUIRED",
                    "patientInformation.gestationPeriod",
                ),
                "D.2.2.1b": (
                    "ICH.D.2.2.1b.REQUIRED",
                    "patientInformation.gestationPeriodUnit",
                ),
            },
            actual,
        )

    def test_si_business_validation_uses_backend_rules_and_issue_paths(self):
        repo = Path(__file__).resolve().parents[5]
        contract = json.loads(
            (repo / "crates/libs/validator/field-rules/si.json").read_text(encoding="utf-8")
        )
        actual = {
            field["code"]: (
                field.get("businessValidation", {}).get("ruleCode"),
                field.get("businessValidation", {}).get("issuePath"),
            )
            for field in contract["editorMappings"]
            if field["code"] in {
                "C.5.4",
                "FDA.C.5.5a",
                "FDA.C.5.5b",
                "FDA.C.5.6.r",
            }
        }
        self.assertEqual(
            {
                "C.5.4": (
                    "ICH.C.5.4.REQUIRED",
                    "studyInformation.0.studyTypeReaction",
                ),
                "FDA.C.5.5a": (
                    "FDA.R0024",
                    "studyInformation.0.fdaIndNumberOccurred",
                ),
                "FDA.C.5.5b": (
                    "FDA.R0025",
                    "studyInformation.0.fdaPreAndaNumberOccurred",
                ),
                "FDA.C.5.6.r": (
                    "FDA.R0026",
                    "studyInformation.0.fdaCrossReportedIndNumbers.0.indNumber",
                ),
            },
            actual,
        )
