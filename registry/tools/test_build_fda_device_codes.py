from build_fda_device_codes import derive_terms, parse_source, sql_str

SAMPLE_TSV = (
    "NCIt Subset Code\tCDRH Subset Name\tNCIt Concept Code\tNCIt Preferred Term\t"
    "NCIt Definition\tCDRH Preferred Term\tCDRH Source Code\tCDRH Definition\tIMDRF Code\n"
    "C54451\tMedical Device Problem\tC1\tTop\tdef\tPatient Device Interaction Problem\t4001\tdef\tIMDRF:A01\n"
    "C54451\tMedical Device Problem\tC2\tMid\tdef\tPatient-Device Incompatibility\t2682\tdef\tIMDRF:A0101\n"
    "C54451\tMedical Device Problem\tC3\tLeaf\tdef\tBiocompatibility\t2886\tdef\tIMDRF:A010101\n"
)


def test_parse_source_splits_rows_by_tab():
    rows = parse_source(SAMPLE_TSV)
    assert len(rows) == 3
    assert rows[0]["NCIt Concept Code"] == "C1"
    assert rows[2]["IMDRF Code"] == "IMDRF:A010101"


def test_derive_terms_builds_level_hierarchy_from_imdrf_prefix():
    rows = parse_source(SAMPLE_TSV)
    derived = derive_terms(rows)

    level1_row, level2_row, level3_row = derived

    assert level1_row["level1_term"] == "Patient Device Interaction Problem"
    assert level1_row["level2_term"] is None
    assert level1_row["level3_term"] is None

    assert level2_row["level1_term"] == "Patient Device Interaction Problem"
    assert level2_row["level2_term"] == "Patient-Device Incompatibility"
    assert level2_row["level3_term"] is None

    assert level3_row["level1_term"] == "Patient Device Interaction Problem"
    assert level3_row["level2_term"] == "Patient-Device Incompatibility"
    assert level3_row["level3_term"] == "Biocompatibility"
    assert level3_row["fda_code"] == "2886"
    assert level3_row["imdrf_code"] == "IMDRF:A010101"


def test_sql_str_escapes_single_quotes_and_handles_none():
    assert sql_str("O'Brien's Device") == "'O''Brien''s Device'"
    assert sql_str(None) == "NULL"
