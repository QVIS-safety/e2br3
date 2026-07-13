import json
import unittest
from collections import Counter
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_dictionary


ICH_SAMPLE = """﻿Field Identification,,,,Business rules,,,,,,Q&A ,NullFlavor Applicable,,,,,,,,Field OIDs
SOURCE,HEADER ELEMENT,DATA ELEMENT NUMBER,DATA ELEMENT NAME,MAX LENGTH,DATA TYPE ,VALUE ALLOWED,CONFORMANCE,ICH BUSINESS RULE,REGIONAL BUSINESS RULE,,NI,MSK,UNK,NA,ASKU,NASK,NINF,PINF,Code system OID
ICH,N.1 ,-,ICH CSR Transmission Identification (batch wrapper),-,-,-,-,-,,,-,-,-,-,-,-,-,-,-
ICH,N.1 ,N.1.2,Batch Number,100,AN,Free Text,Mandatory ,Some rule text,,,No,No,No,No,No,No,No,No,2.16.840.1.113883.3.989.2.1.3.22
ICH,C.1,C.1.6.1,Are Additional Documents Available?,1,Boolean,true / false,Conditional-Mandatory ,Another rule,,,Yes,No,Yes,No,No,No,No,No,-
ICH,C.1.10.r,-,Linked Report Header,-,-,-,-,-,,,No,No,No,No,No,No,No,No,-
ICH,C.1.10.r,C.1.10.r,Linked Report Number,100,AN,Free Text,Optional,-,,,No,No,No,No,No,No,No,No,-
ICH,D,D.9,In Case of Death,-,-,-,-,-,,,-,-,-,-,-,-,-,-,-
"""


class ParseIchCsvTests(unittest.TestCase):
    def setUp(self):
        self.entries = {e["code"]: e for e in build_dictionary.parse_ich_csv(ICH_SAMPLE)}

    def test_extracts_element_entries_with_metadata(self):
        entry = self.entries["N.1.2"]
        self.assertEqual("Batch Number", entry["name"])
        self.assertEqual("N", entry["section"])
        self.assertEqual("element", entry["kind"])
        self.assertEqual("mandatory", entry["conformance"])
        self.assertEqual("AN", entry["data_type"])
        self.assertEqual("100", entry["max_length"])
        self.assertEqual("Free Text", entry["allowed_values"])
        self.assertEqual(
            {"kind": "descriptive"}, entry["allowed_value_constraint"]
        )
        self.assertEqual("2.16.840.1.113883.3.989.2.1.3.22", entry["oid"])
        self.assertNotIn("business_rule", entry)
        self.assertNotIn("null_flavors", entry)

    def test_rule_prose_is_extracted_separately(self):
        rules = build_dictionary.extract_ich_rules(ICH_SAMPLE)

        self.assertEqual("Some rule text", rules["N.1.2"])
        self.assertEqual("Another rule", rules["C.1.6.1"])
        self.assertNotIn("D.9", rules)

    def test_normalizes_conditional_mandatory_and_collects_null_flavors(self):
        entry = self.entries["C.1.6.1"]
        self.assertEqual("conditional_mandatory", entry["conformance"])
        self.assertEqual(["NI", "UNK"], entry["null_flavors"])
        self.assertNotIn("oid", entry)

    def test_groups_come_from_dash_conformance_rows(self):
        self.assertEqual("group", self.entries["D.9"]["kind"])
        self.assertNotIn("conformance", self.entries["D.9"])

    def test_element_row_replaces_same_code_header_row(self):
        entry = self.entries["C.1.10.r"]

        self.assertEqual("element", entry["kind"])
        self.assertEqual("optional", entry["conformance"])
        self.assertEqual("Free Text", entry["allowed_values"])

    def test_group_code_falls_back_to_header_element(self):
        entry = self.entries["N.1"]
        self.assertEqual("group", entry["kind"])
        self.assertEqual(
            "ICH CSR Transmission Identification (batch wrapper)", entry["name"]
        )

    def test_classifies_boolean_allowed_values(self):
        self.assertEqual(
            {"kind": "boolean"},
            self.entries["C.1.6.1"]["allowed_value_constraint"],
        )


class AllowedValueConstraintTests(unittest.TestCase):
    def test_extracts_explicit_code_set_in_source_order(self):
        self.assertEqual(
            {"kind": "code_set", "values": ["1", "2", "0", "9"]},
            build_dictionary.allowed_value_constraint(
                "1=Withdrawn 2=Reduced 0=Unknown 9=Not applicable"
            ),
        )

    def test_classifies_true_marker_separately_from_boolean(self):
        self.assertEqual(
            {"kind": "true_marker"},
            build_dictionary.allowed_value_constraint("true nullFlavor: NI"),
        )

    def test_classifies_non_enumerated_constraints(self):
        cases = {
            "Numeric nullFlavor: NINF, PINF": "numeric",
            "Constrained UCUM codes": "vocabulary",
            "See Appendix II for further information.": "format",
            "Free Text": "descriptive",
        }
        self.assertEqual(
            {source: kind for source, kind in cases.items()},
            {
                source: build_dictionary.allowed_value_constraint(source)["kind"]
                for source in cases
            },
        )

    def test_official_ich_allowed_values_are_fully_classified(self):
        source_path = build_dictionary.SOURCES_DIR / build_dictionary.ICH_SOURCE
        entries = build_dictionary.parse_ich_csv(
            source_path.read_text(encoding="utf-8")
        )
        classified = [
            entry["allowed_value_constraint"]
            for entry in entries
            if "allowed_values" in entry
        ]

        self.assertEqual(223, len(classified))
        self.assertEqual(
            {
                "boolean": 7,
                "code_set": 18,
                "descriptive": 90,
                "format": 25,
                "numeric": 41,
                "true_marker": 10,
                "vocabulary": 32,
            },
            dict(sorted(Counter(rule["kind"] for rule in classified).items())),
        )

    def test_committed_dictionary_matches_official_allowed_value_constraints(self):
        source_entries = build_dictionary.parse_ich_csv(
            (build_dictionary.SOURCES_DIR / build_dictionary.ICH_SOURCE).read_text(
                encoding="utf-8"
            )
        )
        dictionary_entries = json.loads(
            (build_dictionary.DICTIONARY_DIR / "ich-e2br3.json").read_text(
                encoding="utf-8"
            )
        )["entries"]
        expected = {
            entry["code"]: (
                entry["allowed_values"],
                entry["allowed_value_constraint"],
            )
            for entry in source_entries
            if "allowed_values" in entry
        }
        actual = {
            entry["code"]: (
                entry["allowed_values"],
                entry.get("allowed_value_constraint"),
            )
            for entry in dictionary_entries
            if "allowed_values" in entry
        }

        self.assertEqual(expected, actual)


class ParseMfdsTests(unittest.TestCase):
    SHEET1 = [
        ["연번", "Element ID", "항목명\n(영문)", "항목명\n(국문)", "필수\n여부", "OID", "항목검증룰", "관계\nElement ID", "최대길이", "데이터 \n유형", "허용치", "nullFlavor", "비고"],
        ["1", "N.1.1", "Types of Message in batch", "배치의 ICSR 유형", "필수", "2.16.840.1.113883.3.989.2.1.1.1", "rule", None, "2", "N", "1=ichicsr", None, None],
        ["2", "C.2.r.4.KR.1", "Other HCP Type", "기타 의료전문가 구분", "조건부필수", "2.16.840.1.113883.3.989.5.1.10.1.1", "kr rule", None, "1", "N", "1=간호사\n2=기타", None, None],
    ]
    SHEET2 = [
        ["일련번호", "Element ID", "항목명(국문)", "허용치", "OID"],
        ["1", "C.2.r.4.KR.1", "기타 의료전문가 구분", "1=간호사\n2=기타", "2.16.840.1.113883.3.989.5.1.10.1.1"],
    ]

    def test_builds_kr_extension_entries_only(self):
        entries = build_dictionary.parse_mfds_sheets(self.SHEET1, self.SHEET2)

        self.assertEqual(["C.2.r.4.KR.1"], [e["code"] for e in entries])
        entry = entries[0]
        self.assertEqual("Other HCP Type", entry["name"])
        self.assertEqual("기타 의료전문가 구분", entry["name_kr"])
        self.assertEqual("C", entry["section"])
        self.assertEqual("element", entry["kind"])
        self.assertEqual("conditional_mandatory", entry["conformance"])
        self.assertEqual("1", entry["max_length"])
        self.assertEqual("N", entry["data_type"])
        self.assertNotIn("business_rule", entry)
        self.assertEqual("2.16.840.1.113883.3.989.5.1.10.1.1", entry["oid"])

    def test_mfds_rules_cover_ich_and_kr_elements(self):
        rules = build_dictionary.extract_mfds_rules(self.SHEET1)

        self.assertEqual("rule", rules["N.1.1"])
        self.assertEqual("kr rule", rules["C.2.r.4.KR.1"])


class VocabularyUsageTests(unittest.TestCase):
    NULLFLAVOR_SHEET = [
        ["일련번호", "Element ID", "항목명(국문)", "허용치", "OID", "주의사항"],
        ["1", "C.1.7", "신속보고 여부", "NI", "oid", None],
        ["2", "C.2.r.1.1", "원보고자의 직위", "MSK, ASKU, NASK, UNK", "oid", None],
        ["3", "", "헤더 잡음", "ignored", None, None],
    ]
    MEDDRA_SHEET = [
        ["일련번호", "Element ID", "항목명(국문)", "허용치", "OID"],
        ["1", "D.7.1.r.1b", "과거 병력 MedDRA 코드", "MedDRA 코드 8자리", "2.16.840.1.113883.6.163"],
    ]

    def test_extract_nullflavor_usage_splits_comma_lists(self):
        mapping = build_dictionary.extract_nullflavor_usage(self.NULLFLAVOR_SHEET)

        self.assertEqual(["NI"], mapping["C.1.7"])
        self.assertEqual(["MSK", "ASKU", "NASK", "UNK"], mapping["C.2.r.1.1"])
        self.assertNotIn("", mapping)

    def test_extract_vocabulary_usage_labels_each_code(self):
        mapping = build_dictionary.extract_vocabulary_usage(self.MEDDRA_SHEET, "MedDRA")

        self.assertEqual({"D.7.1.r.1b": "MedDRA"}, mapping)

    def test_merge_nullflavors_unions_with_existing(self):
        entries = [
            {"code": "C.1.7", "name": "x", "section": "C", "kind": "element",
             "conformance": "optional", "null_flavors": ["NI"]},
            {"code": "C.2.r.1.1", "name": "y", "section": "C", "kind": "element", "conformance": "optional"},
            {"code": "C.9.9", "name": "z", "section": "C", "kind": "element", "conformance": "optional"},
        ]
        mapping = build_dictionary.extract_nullflavor_usage(self.NULLFLAVOR_SHEET)

        annotated = build_dictionary.merge_nullflavors(entries, mapping)

        self.assertEqual(2, annotated)
        self.assertEqual(["NI"], entries[0]["null_flavors"])
        self.assertEqual(["MSK", "ASKU", "NASK", "UNK"], entries[1]["null_flavors"])
        self.assertNotIn("null_flavors", entries[2])

    def test_merge_vocabulary_sets_field_on_matching_entries(self):
        entries = [
            {"code": "D.7.1.r.1b", "name": "x", "section": "D", "kind": "element", "conformance": "optional"},
            {"code": "C.1.2", "name": "y", "section": "C", "kind": "element", "conformance": "mandatory"},
        ]
        mapping = build_dictionary.extract_vocabulary_usage(self.MEDDRA_SHEET, "MedDRA")

        annotated = build_dictionary.merge_vocabulary(entries, mapping)

        self.assertEqual(1, annotated)
        self.assertEqual("MedDRA", entries[0]["vocabulary"])
        self.assertNotIn("vocabulary", entries[1])


class MergeMfdsProfilesTests(unittest.TestCase):
    SHEET1 = [
        ["연번", "Element ID", "항목명\n(영문)", "항목명\n(국문)", "필수\n여부", "OID", "항목검증룰"],
        ["1", "F.r.2.1", "Test Name", "검사항목명", "조건부필수", None, None],
        ["2", "C.2.r.4.KR.1", "Other HCP Type", "기타", "비필수", None, None],
        ["3", "N.1.1", "Types of Message in batch", "유형", "필수", None, None],
    ]

    def test_annotates_ich_entries_with_mfds_profile_conformance(self):
        entries = [
            {"code": "F.r.2.1", "name": "Test Name", "section": "F", "kind": "element", "conformance": "optional"},
            {"code": "N.1.1", "name": "Types of Message in batch", "section": "N", "kind": "element", "conformance": "mandatory"},
            {"code": "C.1.2", "name": "Date of Creation", "section": "C", "kind": "element", "conformance": "mandatory"},
        ]

        annotated = build_dictionary.merge_mfds_profiles(entries, self.SHEET1)

        self.assertEqual(2, annotated)
        self.assertEqual({"mfds": "conditional_mandatory"}, entries[0]["profiles"])
        self.assertEqual({"mfds": "mandatory"}, entries[1]["profiles"])
        self.assertNotIn("profiles", entries[2])


FDA_SAMPLE = """Field Identification,,,,Field Type,,,ICH,,Post-Market,,Pre-Market (IND and IND-exempt BA/BE),,VAERS,,,,,Type of Change,Q&A,Null Flag Applicable,,,,,,,,,Field OIDs,HL7 Data Type,,Xpath,
SOURCE,HEADER Element,DATA ELEMENT NUMBER,DATA ELEMENT NAME,MAX LENGTH,DATA TYPE ,VALUES ALLOWED,CONFORMANCE,ICH Business Rules,CONFORMANCE,Post-Market Business Rule,CONFORMANCE,Pre-Market Business Rule,MAX LENGTH,DATA TYPE,VALUES ALLOWED,CONFORMANCE,VAERS Business Rules,,,NI,MSK,UNK,NA,ASKU,NASK,NINF,PINF,OTH,Code system OID,HL7 Data Type,HL7 Data Type (sub component),Value,Null
ICH,N.1 ,N.1.1,Types of Message in batch,2,N,1=ichicsr,Required,ich rule,-,-,-,-,,,,,,,,No,No,No,No,No,No,No,No,No,2.16.840.1.113883.3.989.2.1.1.1,SC,code=ST,/MCCI_IN200100UV01/name,
FDA,C.1,FDA.C.1.12,Combination Product Report Indicator,1,Boolean,true,-,-,Required,pm rule,Required,pre rule,,,,Mandatory,vaers rule,,,Yes,No,No,No,No,No,No,No,No,2.16.840.1.113883.3.989.5.1,CE,,/some/xpath,
FDA,G.k.12.r,FDA.G.k.12.r.7,Device Manufacturer,-,-,-,-,-,-,-,-,-,,,,-,-,,,-,-,-,-,-,-,-,-,-,-,,,,
"""


class MergeFdaProfilesTests(unittest.TestCase):
    SAMPLE = FDA_SAMPLE.replace(
        "ICH,N.1 ,N.1.1,Types of Message in batch,2,N,1=ichicsr,Required,ich rule,-,-,-,-,",
        "ICH,N.1 ,N.1.1,Types of Message in batch,2,N,1=ichicsr,Required,ich rule,Required,pm rule,Conditional-Required,pre rule,",
    )

    def test_annotates_ich_entries_with_fda_profile_conformances(self):
        entries = [
            {"code": "N.1.1", "name": "Types of Message in batch", "section": "N", "kind": "element",
             "conformance": "mandatory", "profiles": {"mfds": "mandatory"}},
            {"code": "C.1.2", "name": "Date of Creation", "section": "C", "kind": "element", "conformance": "mandatory"},
        ]

        annotated = build_dictionary.merge_fda_profiles(entries, self.SAMPLE)

        self.assertEqual(1, annotated)
        self.assertEqual(
            {"mfds": "mandatory", "post_market": "mandatory", "pre_market": "conditional_mandatory"},
            entries[0]["profiles"],
        )
        self.assertNotIn("profiles", entries[1])

    def test_dash_conformances_do_not_create_profiles(self):
        entries = [
            {"code": "N.1.1", "name": "Types of Message in batch", "section": "N", "kind": "element", "conformance": "mandatory"},
        ]

        annotated = build_dictionary.merge_fda_profiles(entries, FDA_SAMPLE)

        self.assertEqual(0, annotated)
        self.assertNotIn("profiles", entries[0])


FDA_SEVERITY_SAMPLE = """Title row to skip,,,,,
,,,,,
DATA ELEMENT NUMBER,DATA ELEMENT NAME,BUSINESS RULE,"REJECTION, IF NOT MET","WARNING, IF NOT MET",ERROR ID,ERROR DESCRIPTION
C.1.3,Batch Receiver,rule,ü,,R0008,C.1.3 must be 2 when ...
FDA.C.1.12,Combination,rule,ü,,R0012,FDA.C.1.12 must be ...
C.1.10.r,Linked,rule,,ü,W0001,C.1.10.r should be provided
N.1.1,Type,rule,,,,
"""


class FdaSeverityTests(unittest.TestCase):
    def test_extract_severity_classifies_rejection_and_warning(self):
        mapping = build_dictionary.extract_fda_severity(FDA_SEVERITY_SAMPLE)

        self.assertEqual(
            {"severity": "rejection", "error_id": "R0008", "error_description": "C.1.3 must be 2 when ..."},
            mapping["C.1.3"],
        )
        self.assertEqual("warning", mapping["C.1.10.r"]["severity"])
        self.assertIn("FDA.C.1.12", mapping)

    def test_extract_severity_skips_rows_without_a_mark(self):
        mapping = build_dictionary.extract_fda_severity(FDA_SEVERITY_SAMPLE)

        self.assertNotIn("N.1.1", mapping)

    def test_merge_severity_annotates_ich_and_fda_entries(self):
        entries = [
            {"code": "C.1.3", "name": "x", "section": "C", "kind": "element", "conformance": "mandatory"},
            {"code": "C.9.9", "name": "y", "section": "C", "kind": "element", "conformance": "optional"},
        ]
        mapping = build_dictionary.extract_fda_severity(FDA_SEVERITY_SAMPLE)

        annotated = build_dictionary.merge_fda_severity(entries, mapping)

        self.assertEqual(1, annotated)
        self.assertEqual("rejection", entries[0]["fda_severity"])
        self.assertEqual("R0008", entries[0]["fda_error_id"])
        self.assertNotIn("fda_severity", entries[1])


class ParseFdaCsvTests(unittest.TestCase):
    def setUp(self):
        self.entries = {e["code"]: e for e in build_dictionary.parse_fda_csv(FDA_SAMPLE)}

    def test_only_fda_source_rows_become_entries(self):
        self.assertEqual({"FDA.C.1.12", "FDA.G.k.12.r.7"}, set(self.entries))

    def test_extracts_fda_element_with_profiles_and_xpath(self):
        entry = self.entries["FDA.C.1.12"]
        self.assertEqual("Combination Product Report Indicator", entry["name"])
        self.assertEqual("C", entry["section"])
        self.assertEqual("element", entry["kind"])
        self.assertEqual("mandatory", entry["conformance"])
        self.assertEqual(
            {"post_market": "mandatory", "pre_market": "mandatory", "vaers": "mandatory"},
            entry["profiles"],
        )
        self.assertEqual("/some/xpath", entry["xpath"])
        self.assertEqual("CE", entry["hl7_data_type"])
        self.assertEqual(["NI"], entry["null_flavors"])
        self.assertNotIn("business_rule", entry)

    def test_fda_rules_cover_ich_and_fda_elements(self):
        rules = build_dictionary.extract_fda_rules(MergeFdaProfilesTests.SAMPLE)

        self.assertIn("Post-Market: pm rule", rules["FDA.C.1.12"])
        self.assertIn("VAERS: vaers rule", rules["FDA.C.1.12"])
        self.assertIn("Post-Market: pm rule", rules["N.1.1"])
        self.assertNotIn("FDA.G.k.12.r.7", rules)

    def test_fda_rows_without_any_conformance_become_groups(self):
        entry = self.entries["FDA.G.k.12.r.7"]
        self.assertEqual("group", entry["kind"])
        self.assertNotIn("conformance", entry)


XPATH_SAMPLE = '''﻿"h: header
e: entity ","Element
number   ",Element name ,"ICH Data
Type ",HL7 Data Type ,"HL7 Data Type
（sub component) ",Category ,Xpath ,
h ,N.1,ICH CSR Transmission Identification , , , , ,,
e ,N.1.2,Batch Number ,100AN ,Instance Identifier  (II) ,extension=Character  String (ST) ,Value ,/MCCI_IN200100UV01/id,
 ,,ICH Code List Version row without code , , , , ,/MCCI_IN200100UV01/other,
e ,C.2.r.4.KR.1,Other HCP Type ,1N ,Coded With Equivalents (CE) , ,Value ,/MCCI_IN200100UV01/kr/path,
'''


class ParseXpathCsvTests(unittest.TestCase):
    def setUp(self):
        self.mapping = build_dictionary.parse_xpath_csv(XPATH_SAMPLE)

    def test_extracts_xpath_and_hl7_types_per_code(self):
        entry = self.mapping["N.1.2"]
        self.assertEqual("/MCCI_IN200100UV01/id", entry["xpath"])
        self.assertEqual("Instance Identifier  (II)", entry["hl7_data_type"])
        self.assertEqual("extension=Character  String (ST)", entry["hl7_component"])

    def test_skips_rows_without_an_element_code(self):
        self.assertNotIn("", self.mapping)
        self.assertEqual({"N.1", "N.1.2", "C.2.r.4.KR.1"}, set(self.mapping))

    def test_merge_xpath_annotates_matching_entries(self):
        entries = [
            {"code": "N.1.2", "name": "Batch Number", "section": "N", "kind": "element", "conformance": "mandatory"},
            {"code": "C.1.2", "name": "Date of Creation", "section": "C", "kind": "element", "conformance": "mandatory"},
        ]

        annotated = build_dictionary.merge_xpath(entries, self.mapping)

        self.assertEqual(1, annotated)
        self.assertEqual("/MCCI_IN200100UV01/id", entries[0]["xpath"])
        self.assertEqual("Instance Identifier  (II)", entries[0]["hl7_data_type"])
        self.assertNotIn("xpath", entries[1])


if __name__ == "__main__":
    unittest.main()
