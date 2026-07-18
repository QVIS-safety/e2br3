import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import presave_registry
import validate


class PresaveRegistryTests(unittest.TestCase):
    def valid_row(self, code: str = "C.2.r.1.2") -> dict:
        return {
            "id": code,
            "e2br3_code": code,
            "label": "Reporter Given Name",
            "section": "C",
            "authority": "ICH",
            "status": "complete",
            "backend": {
                "status": "mapped",
                "model": "ReporterPresave",
                "field": "reporter_given_name",
                "evidence": "ReporterPresave reporter_given_name",
            },
            "frontend": {
                "status": "mapped",
                "section": "reporter",
                "field": "reporterGivenName",
                "evidence": "Reporter form reporterGivenName",
            },
        }

    def write_presaves(self, root: Path, rows: list[dict]) -> None:
        sections = root / "presaves" / "sections"
        sections.mkdir(parents=True)
        (root / "presaves" / "index.json").write_text(
            json.dumps({"sections": ["sections/c-reporter.json"]}),
            encoding="utf-8",
        )
        (sections / "c-reporter.json").write_text(
            json.dumps(rows), encoding="utf-8"
        )

    def test_loads_same_code_in_case_and_presave_namespaces(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_presaves(root, [self.valid_row()])
            result = validate.ValidationResult()

            loaded = presave_registry.load_presave_registry(root, result)

        self.assertEqual(
            "ReporterPresave.reporter_given_name",
            loaded.backend_keys["C.2.r.1.2"],
        )
        self.assertEqual("reporter.reporterGivenName", loaded.frontend_keys["C.2.r.1.2"])
        self.assertEqual("reporter", loaded.section_by_code["C.2.r.1.2"])
        self.assertEqual(("C.2.r.1.2",), loaded.codes_by_section["reporter"])
        self.assertEqual([], result.errors)

    def test_rejects_duplicate_code_inside_presave_namespace(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_presaves(root, [self.valid_row(), self.valid_row()])
            result = validate.ValidationResult()

            presave_registry.load_presave_registry(root, result)

        self.assertIn(
            "duplicate presave e2br3_code C.2.r.1.2", "\n".join(result.errors)
        )


if __name__ == "__main__":
    unittest.main()
