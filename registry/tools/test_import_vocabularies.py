import hashlib
import json
import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import import_vocabularies


FIXTURES = Path(__file__).resolve().parent / "fixtures"


class VocabularyImporterTests(unittest.TestCase):
    def test_ich_dose_uses_current_dose_flag_not_strength_or_ucum_syntax(self):
        raw = b'''<CodeList><Identification><ShortName>ich-dose-strength-unit</ShortName><Version>test</Version></Identification><SimpleCodeList>
        <Row><Value ColumnRef="code"><SimpleValue>{DF}</SimpleValue></Value><Value ColumnRef="cf"><SimpleValue>y</SimpleValue></Value><Value ColumnRef="dosef"><SimpleValue>y</SimpleValue></Value><Value ColumnRef="strgth"><SimpleValue>n</SimpleValue></Value></Row>
        <Row><Value ColumnRef="code"><SimpleValue>mg/mL</SimpleValue></Value><Value ColumnRef="cf"><SimpleValue>y</SimpleValue></Value><Value ColumnRef="dosef"><SimpleValue>n</SimpleValue></Value><Value ColumnRef="strgth"><SimpleValue>y</SimpleValue></Value></Row>
        <Row><Value ColumnRef="code"><SimpleValue>obsolete</SimpleValue></Value><Value ColumnRef="cf"><SimpleValue>n</SimpleValue></Value><Value ColumnRef="dosef"><SimpleValue>y</SimpleValue></Value></Row>
        </SimpleCodeList></CodeList>'''
        result = import_vocabularies.normalize_ich_units(raw)
        self.assertEqual(result["entries"], [{"code": "{DF}", "scopes": ["dose"]}])
        self.assertEqual(result["source_sha256"], hashlib.sha256(raw).hexdigest())
        with self.assertRaisesRegex(ValueError, "dose applicability"):
            import_vocabularies.normalize_ich_units(raw.replace(b'ColumnRef="dosef"', b'ColumnRef="other"'))
        with self.assertRaisesRegex(ValueError, "CL25 or CL26"):
            import_vocabularies.normalize_ich_units((FIXTURES / "ucum-essence-minimal.xml").read_bytes())

    def test_official_ich_snapshots_pin_source_and_field_subsets(self):
        dose = json.loads((import_vocabularies.VOCABULARY_DIR / "ich-dose.json").read_text())
        duration = json.loads((import_vocabularies.VOCABULARY_DIR / "ich-time.json").read_text())
        for snapshot in (dose, duration):
            import_vocabularies.validate_snapshot(snapshot)
        self.assertEqual(dose["version"], "2.3")
        self.assertEqual(len(dose["entries"]), 107)
        self.assertEqual(dose["source_sha256"], "f7d9d579fdc774dfa23d67b295e662818a7343212a6b64479d5d302eb96cbcd8")
        self.assertEqual(duration["version"], "2.2")
        self.assertEqual(duration["source_sha256"], "d831184fedf12e5ca194e079413a7ba697d705b70adfa563903404830bbd08bf")
        self.assertEqual({row["code"] for row in duration["entries"]}, {"s", "min", "h", "d", "wk", "mo", "a", "10.a"})
        self.assertTrue(all(row["scopes"] == ["time"] for row in duration["entries"]))
        self.assertTrue(all(row["scopes"] == ["dose"] for row in dose["entries"]))

    def test_snapshot_validation_rejects_duplicate_codes(self):
        snapshot = import_vocabularies.normalize_iso639(
            (FIXTURES / "iso-639-2-minimal.txt").read_bytes()
        )
        snapshot["entries"].append(snapshot["entries"][0])

        with self.assertRaisesRegex(ValueError, "sorted unique codes"):
            import_vocabularies.validate_snapshot(snapshot)

    def test_ucum_normalization_is_deterministic_and_preserves_grammar_symbols(self):
        raw = (FIXTURES / "ucum-essence-minimal.xml").read_bytes()

        first = import_vocabularies.normalize_ucum(raw)
        second = import_vocabularies.normalize_ucum(raw)

        self.assertEqual(first, second)
        self.assertEqual(hashlib.sha256(raw).hexdigest(), first["source_sha256"])
        self.assertEqual("2.2", first["version"])
        self.assertIn({"code": "m", "scopes": ["prefix"]}, first["entries"])
        self.assertIn({"code": "g", "scopes": ["unit"]}, first["entries"])
        self.assertIn({"code": "d", "scopes": ["frequency"]}, first["entries"])
        self.assertIn(
            {"code": "{cyclical}", "scopes": ["frequency"]}, first["entries"]
        )
        self.assertNotIn("mg", [entry["code"] for entry in first["entries"]])

    def test_iso639_uses_both_set_two_alpha_three_codes(self):
        result = import_vocabularies.normalize_iso639(
            (FIXTURES / "iso-639-2-minimal.txt").read_bytes()
        )

        self.assertEqual(
            ["eng", "fra", "fre", "kor"],
            [entry["code"] for entry in result["entries"]],
        )
        self.assertTrue(all(entry["scopes"] == ["all"] for entry in result["entries"]))

    def test_edqm_keeps_only_current_dose_form_and_route_terms(self):
        result = import_vocabularies.normalize_edqm(
            (FIXTURES / "edqm-minimal.json").read_bytes()
        )

        self.assertEqual(
            [
                {"code": "100000073664", "scopes": ["dose_form"]},
                {"code": "20053000", "scopes": ["route"]},
            ],
            result["entries"],
        )

    def test_write_snapshot_is_byte_deterministic(self):
        snapshot = import_vocabularies.normalize_iso639(
            (FIXTURES / "iso-639-2-minimal.txt").read_bytes()
        )
        expected = (
            json.dumps(snapshot, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
        ).encode()

        self.assertEqual(expected, import_vocabularies.snapshot_bytes(snapshot))


if __name__ == "__main__":
    unittest.main()
