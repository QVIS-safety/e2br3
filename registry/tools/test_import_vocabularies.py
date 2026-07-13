import hashlib
import json
import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import import_vocabularies


FIXTURES = Path(__file__).resolve().parent / "fixtures"


class VocabularyImporterTests(unittest.TestCase):
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
