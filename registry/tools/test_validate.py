import json
import tempfile
import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import validate


class RegistryValidatorTests(unittest.TestCase):
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

    def test_unmapped_null_flavor_columns_are_not_reported_as_missing(self):
        # The in-band pattern derives the flavor from the base field at the API layer,
        # so an unmapped support column must stay opt-in rather than demand a row.
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

        self.assertNotIn("patient_initial_null_flavor", "\n".join(result.errors))

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

    def test_repository_reporter_presave_inventory_is_complete(self):
        result = validate.validate_registry(
            validate_backend_inventory=False,
            validate_presave_registry_rows=True,
            validate_presave_inventory=True,
        )

        self.assertEqual([], result.errors)

    def test_ci_runs_strict_presave_inventory(self):
        workflow = (validate.ROOT.parent / ".github/workflows/ci.yml").read_text(
            encoding="utf-8"
        )

        self.assertIn(
            "python3 registry/tools/validate.py --strict-presave-inventory",
            workflow,
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
