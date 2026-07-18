import sys
import tempfile
import unittest
from unittest.mock import patch
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import extract_presave_fields as extractor


ROOT = Path(__file__).resolve().parents[2]


class PresaveFieldExtractorTests(unittest.TestCase):
    def test_resolves_explicit_frontend_worktree_root(self):
        with tempfile.TemporaryDirectory() as tmp:
            frontend_root = Path(tmp)
            source = frontend_root / "components" / "presave" / "ReporterForm.tsx"
            source.parent.mkdir(parents=True)
            source.write_text("reporterGivenName?: string;", encoding="utf-8")

            with patch.dict(
                "os.environ", {"E2BR3_FRONTEND_ROOT": str(frontend_root)}
            ):
                resolved = extractor.resolve_frontend_path(
                    ROOT, "components/presave/ReporterForm.tsx"
                )

        self.assertEqual(source, resolved)

    def test_configured_sections_include_all_presave_domains(self):
        self.assertEqual(
            {"sender", "receiver", "product", "reporter", "study", "narrative"},
            set(extractor.PRESAVE_SECTIONS),
        )

    def test_extracts_frontend_fields_from_form_and_type_sources(self):
        source = '''
  reporterGivenName?: string;
  reporterCountryNullFlavor?: string;
  id?: string;
  <Input {...register("reporterGivenName")} />
  <Controller name="reporterCountryNullFlavor" />
'''
        self.assertEqual(
            {"reporter.reporterGivenName", "reporter.reporterCountryNullFlavor"},
            extractor.extract_presave_frontend_source(source, "reporter"),
        )

    def test_extracts_nontechnical_rust_fields(self):
        source = '''
pub struct ReporterPresave {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub reporter_given_name: Option<String>,
    pub country_code_null_flavor: Option<String>,
    pub deleted: bool,
    pub created_at: DateTime,
}
'''
        self.assertEqual(
            {
                "ReporterPresave.reporter_given_name",
                "ReporterPresave.country_code_null_flavor",
            },
            extractor.extract_rust_presave_source(source, "ReporterPresave"),
        )

    def test_repository_reporter_inventories_include_existing_null_flavors(self):
        frontend = extractor.extract_reporter_frontend(ROOT)
        backend = extractor.extract_presave_backend(
            ROOT, extractor.REPORTER_BACKEND_MODELS
        )

        self.assertIn("reporter.reporterTitleNullFlavor", frontend)
        self.assertIn("reporter.reporterGivenNameNullFlavor", frontend)
        self.assertIn("reporter.reporterOrganizationNullFlavor", frontend)
        self.assertIn("reporter.reporterTelephoneNullFlavor", frontend)
        self.assertIn("reporter.reporterCountryNullFlavor", frontend)
        self.assertIn("ReporterPresave.reporter_title_null_flavor", backend)
        self.assertIn("ReporterPresave.reporter_given_name_null_flavor", backend)
        self.assertIn("ReporterPresave.organization_null_flavor", backend)
        self.assertIn("ReporterPresave.telephone_null_flavor", backend)
        self.assertIn("ReporterPresave.country_code_null_flavor", backend)
        self.assertIn("ReporterPresave.qualification_null_flavor", backend)

    def test_extracts_reporter_to_primary_source_transfers(self):
        source = '''
return {
  reporterGivenName: data.reporterGivenName || "",
  reporterCountryNullFlavor: toNullFlavor(data.reporterCountryNullFlavor),
};
'''
        self.assertEqual(
            {
                (
                    "ReporterPresave.reporter_given_name",
                    "PrimarySource.reporter_given_name",
                ),
                (
                    "ReporterPresave.country_code_null_flavor",
                    "PrimarySource.country_code_null_flavor",
                ),
            },
            extractor.extract_reporter_transfer_source(source),
        )

    def test_repository_transfer_includes_country_null_flavor(self):
        self.assertIn(
            (
                "ReporterPresave.country_code_null_flavor",
                "PrimarySource.country_code_null_flavor",
            ),
            extractor.extract_reporter_transfers(ROOT),
        )
        transfers = extractor.extract_reporter_transfers(ROOT)
        self.assertIn(
            ("ReporterPresave.reporter_given_name_null_flavor", "PrimarySource.reporter_given_name_null_flavor"),
            transfers,
        )
        self.assertIn(
            ("ReporterPresave.city_null_flavor", "PrimarySource.city_null_flavor"),
            transfers,
        )

    def test_transfer_pattern_rejects_cross_wired_receiver_assignments(self):
        batch_spec = extractor.TRANSFER_SPECS["receiver"][2]
        source = '''
const batchReceiverIdentifier = route.batchReceiverIdentifier;
const messageReceiverIdentifier = route.messageReceiverIdentifier;
importValue("messageHeader.batchReceiverIdentifier", messageReceiverIdentifier);
importValue("messageHeader.messageReceiverIdentifier", batchReceiverIdentifier);
'''

        self.assertFalse(extractor.transfer_spec_matches(source, batch_spec))

    def test_transfer_pattern_ignores_commented_assignments(self):
        narrative_spec = extractor.TRANSFER_SPECS["narrative"][0]
        source = '''
// setValue("narrative.caseNarrative", d.caseNarrative);
/* setValue("narrative.caseNarrative", d.caseNarrative); */
'''

        self.assertFalse(extractor.transfer_spec_matches(source, narrative_spec))


if __name__ == "__main__":
    unittest.main()
