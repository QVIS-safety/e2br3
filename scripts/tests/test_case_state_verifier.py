import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1] / "case-state-verifier"


class CaseStateVerifierLayoutTest(unittest.TestCase):
    def test_grammar_chain_and_portable_sources(self):
        previous = None
        for version in range(2, 6):
            path = ROOT / f"adaptive-case-edit-grammar-v{version}.json"
            grammar = json.loads(path.read_text())
            self.assertEqual(version, grammar["version"])
            if previous is not None:
                self.assertEqual(previous.name, grammar["inherits"])
            previous = path

        sources = list(ROOT.glob("*.py"))
        self.assertTrue(sources)
        for source in sources:
            text = source.read_text()
            self.assertNotIn("/home/opc/", text, source.name)
            self.assertNotIn("e2br3_ui_presave_import_20261104", text, source.name)


if __name__ == "__main__":
    unittest.main()
