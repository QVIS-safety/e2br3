import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import extract_presave_fields as extractor


ROOT = Path(__file__).resolve().parents[2]


class PresaveFieldExtractorTests(unittest.TestCase):
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

        self.assertIn("reporter.reporterNameNullFlavor", frontend)
        self.assertIn("reporter.reporterCountryNullFlavor", frontend)
        self.assertIn("ReporterPresave.reporter_name_null_flavor", backend)
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


if __name__ == "__main__":
    unittest.main()
