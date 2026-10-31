import tempfile
import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import generate_editor_contract_web_tests as generator


class EditorContractWebTestGeneratorTests(unittest.TestCase):
    def test_generates_one_named_macro_per_active_registry_field(self):
        repo = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as tmp:
            outputs = generator.generate(repo / "registry", Path(tmp))

            for page_id in generator.ACTIVE_PAGES:
                contract = generator.load_contract(repo / "registry", page_id)
                generated = outputs[page_id]
                self.assertEqual(
                    len([field for field in contract["fields"] if field.get("execution", {}).get("status") != "blocked"]),
                    generated.count("field_contract_test!("),
                )
                self.assertNotIn("payloadPath", generated)
                self.assertNotIn("projectionPath", generated)
                self.assertNotIn("DATABASE_URL", generated)
                self.assertNotIn("#[serial]", generated)

            self.assertIn(
                'field_contract_test!(c_3_2, "C.3.2");',
                outputs["SD"],
            )
            self.assertIn(
                'field_contract_test!(d_10_6, "D.10.6");',
                outputs["DM"],
            )
            self.assertIn(
                "// BLOCKED editor contract G.k.9.i.1:",
                outputs["DG"],
            )
            self.assertIn('field_contract_test!(n_1_5, "N.1.5");', outputs["N"])
            self.assertEqual(8, outputs["N"].count("field_contract_test!("))

    def test_rejects_blocked_field_without_reason(self):
        fields = [{"code": "G.k.9.i.1", "execution": {"status": "blocked"}}]

        with self.assertRaisesRegex(ValueError, "blocked execution requires a reason"):
            generator.render_page("DG", fields)

    def test_rejects_duplicate_normalized_test_names(self):
        fields = [{"code": "C.1-a"}, {"code": "C.1.a"}]

        with self.assertRaisesRegex(ValueError, "duplicate generated test name c_1_a"):
            generator.render_page("CI", fields)

    def test_root_is_module_only_and_generated_count_matches_registry(self):
        repo = Path(__file__).resolve().parents[2]
        root = (
            repo
            / "crates/services/web-server/tests/api_editor/mod.rs"
        ).read_text(encoding="utf-8")
        self.assertNotIn("async fn", root)
        self.assertNotIn("#[tokio::test]", root)
        for page_id in generator.ACTIVE_PAGES:
            self.assertIn(f'mod {page_id.lower()};', root)

        registry_count = sum(
            len([field for field in generator.load_contract(repo / "registry", page_id)["fields"] if field.get("execution", {}).get("status") != "blocked"])
            for page_id in generator.ACTIVE_PAGES
        )
        generated_count = sum(
            (
                repo
                / "crates/services/web-server/tests/api_editor/pages"
                / f"{page_id.lower()}.generated.rs"
            )
            .read_text(encoding="utf-8")
            .count("field_contract_test!(")
            for page_id in generator.ACTIVE_PAGES
        )
        self.assertEqual(registry_count, generated_count)


if __name__ == "__main__":
    unittest.main()
