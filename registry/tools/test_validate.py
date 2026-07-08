import tempfile
import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import validate


class RegistryValidatorTests(unittest.TestCase):
    def write_registry(self, root: Path, row: str) -> None:
        (root / "index.json").write_text(
            '{"sections":["sections/c-safety-report.json"]}',
            encoding="utf-8",
        )
        sections = root / "sections"
        sections.mkdir()
        (sections / "c-safety-report.json").write_text(row, encoding="utf-8")

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
                "DrugRecurrenceInformation": "crates/libs/lib-core/src/model/drug_recurrence.rs",
                "DrugReactionAssessment": "crates/libs/lib-core/src/model/drug_reaction_assessment.rs",
                "RelatednessAssessment": "crates/libs/lib-core/src/model/drug_reaction_assessment.rs",
            },
        )

        self.assertEqual([], result.errors)

    def test_repository_frontend_inputs_have_registry_rows(self):
        registry_root = Path(__file__).resolve().parents[1]
        frontend_root = (
            registry_root.parent / "../frontend/E2BR3-frontend/components/case-form/sections"
        ).resolve()
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

    SENDER_ENTRY = '{"code": "C.3.2", "name": "Sender\'s Organisation", "section": "C", "kind": "element", "conformance": "mandatory"}'

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

    def test_strict_dictionary_reports_mandatory_elements_without_registry_rows(self):
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
            "missing registry row for mandatory ICH element C.1.1",
            "\n".join(result.errors),
        )

    def test_strict_dictionary_does_not_require_rows_for_groups_or_optional_elements(self):
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

        self.assertEqual([], result.errors)


if __name__ == "__main__":
    unittest.main()
