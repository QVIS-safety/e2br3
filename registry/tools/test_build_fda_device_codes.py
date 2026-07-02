import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_fda_device_codes


SAMPLE_TSV = (
    "NCIt Subset Code\tCDRH Subset Name\tNCIt Concept Code\tNCIt Preferred Term\t"
    "NCIt Definition\tCDRH Preferred Term\tCDRH Source Code\tCDRH Definition\tIMDRF Code\n"
    "C54451\tMedical Device Problem\tC1\tTop\tdef\tPatient Device Interaction Problem\t4001\tdef\tIMDRF:A01\n"
    "C54451\tMedical Device Problem\tC2\tMid\tdef\tPatient-Device Incompatibility\t2682\tdef\tIMDRF:A0101\n"
    "C54451\tMedical Device Problem\tC3\tLeaf\tdef\tBiocompatibility\t2886\tdef\tIMDRF:A010101\n"
)

SAMPLE_TSV_WITH_OTHER_SUBSET = SAMPLE_TSV + (
    "C99999\tSome Other Subset\tCX\tOther\tdef\tUnrelated Term\t9999\tdef\tIMDRF:Z99\n"
)


class ParseSourceTests(unittest.TestCase):
    def test_parse_source_splits_rows_by_tab(self):
        rows = build_fda_device_codes.parse_source(SAMPLE_TSV)
        self.assertEqual(3, len(rows))
        self.assertEqual("C1", rows[0]["NCIt Concept Code"])
        self.assertEqual("IMDRF:A010101", rows[2]["IMDRF Code"])


class DeriveTermsTests(unittest.TestCase):
    def test_derive_terms_builds_level_hierarchy_from_imdrf_prefix(self):
        rows = build_fda_device_codes.parse_source(SAMPLE_TSV)
        derived = build_fda_device_codes.derive_terms(rows)

        level1_row, level2_row, level3_row = derived

        self.assertEqual("Patient Device Interaction Problem", level1_row["level1_term"])
        self.assertIsNone(level1_row["level2_term"])
        self.assertIsNone(level1_row["level3_term"])

        self.assertEqual("Patient Device Interaction Problem", level2_row["level1_term"])
        self.assertEqual("Patient-Device Incompatibility", level2_row["level2_term"])
        self.assertIsNone(level2_row["level3_term"])

        self.assertEqual("Patient Device Interaction Problem", level3_row["level1_term"])
        self.assertEqual("Patient-Device Incompatibility", level3_row["level2_term"])
        self.assertEqual("Biocompatibility", level3_row["level3_term"])
        self.assertEqual("2886", level3_row["fda_code"])
        self.assertEqual("IMDRF:A010101", level3_row["imdrf_code"])


class SqlStrTests(unittest.TestCase):
    def test_sql_str_escapes_single_quotes_and_handles_none(self):
        self.assertEqual("'O''Brien''s Device'", build_fda_device_codes.sql_str("O'Brien's Device"))
        self.assertEqual("NULL", build_fda_device_codes.sql_str(None))


class BuildSqlTests(unittest.TestCase):
    def test_build_sql_excludes_rows_from_a_different_subset(self):
        sql = build_fda_device_codes.build_sql(SAMPLE_TSV_WITH_OTHER_SUBSET)

        self.assertIn("'C1'", sql)
        self.assertIn("'C2'", sql)
        self.assertIn("'C3'", sql)
        self.assertNotIn("'CX'", sql)


if __name__ == "__main__":
    unittest.main()
