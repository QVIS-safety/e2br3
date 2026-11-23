import tempfile
import copy
import json
import shutil
import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import generate_editor_contract_web_tests as generator
import editor_dictionary


class EditorContractWebTestGeneratorTests(unittest.TestCase):
    def test_dictionary_constraints_generate_executable_tests(self):
        root = generator.REPO_ROOT / "registry"
        checks = editor_dictionary.check(root)
        self.assertEqual(388, len(checks))
        generated = editor_dictionary.render_tests(root)
        self.assertEqual(len(checks), generated.count("dictionary_test!("))
        self.assertIn('"DM", "D.3", "D.3", "max_length", input_contracts::generated::d::d_3', generated)
        self.assertIn('"DM", "D.1", "FDA.D.1", "null_flavors", input_contracts::generated::d::fda_d_1', generated)
        self.assertEqual(generated, (generator.REPO_ROOT / "crates/libs/input-contracts/tests/editor_dictionary/generated.rs").read_text())

    def test_dictionary_gate_rejects_missing_stale_and_unexecutable_bindings(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "registry"
            for directory in ("dictionary", "editor-contracts"):
                shutil.copytree(generator.REPO_ROOT / "registry" / directory, root / directory)
            catalog = root.parent / "crates/libs/validator/rules.json"
            catalog.parent.mkdir(parents=True)
            shutil.copyfile(generator.REPO_ROOT / "crates/libs/validator/rules.json", catalog)
            path = root / "editor-contracts/dm.json"
            original = path.read_text()
            mutations = {
                "missing.*dictionary inputs": lambda b: b.clear(),
                "property coverage": lambda b: b[0]["checks"].pop("max_length"),
                "stale dictionary value": lambda b: b[0]["checks"]["max_length"].update(dictionaryValue="7"),
                "stale dictionary guidance": lambda b: b[0].update(guidance="unsupported guidance"),
                "wrong input function": lambda b: b[0].update(function="d::d_4"),
                "positive and negative": lambda b: b[0]["checks"]["max_length"].update(invalid=[]),
                "length boundary": lambda b: b[0]["checks"]["max_length"]["valid"].pop(),
                "unknown validator rules": lambda b: b[0]["checks"]["max_length"].update(owner="validator", reason="test", ruleCodes=["NONEXISTENT"]),
                "unresolved requirement": lambda b: b[0]["checks"]["max_length"].update(owner="unresolved", reason=""),
            }
            for error, mutate in mutations.items():
                with self.subTest(error=error):
                    document = json.loads(original)
                    field = next(f for f in document["fields"] if f["code"] == "D.3")
                    mutate(field["dictionaryInputs"])
                    path.write_text(json.dumps(document))
                    with self.assertRaisesRegex(ValueError, error):
                        editor_dictionary.check(root)
            path.write_text(original)
            dictionary_path = root / "dictionary/ich-e2br3.json"
            dictionary = json.loads(dictionary_path.read_text())
            dictionary["entries"].append({"code": "D.NEW", "kind": "element"})
            dictionary_path.write_text(json.dumps(dictionary))
            with self.assertRaisesRegex(ValueError, "lack editor disposition"):
                editor_dictionary.check(root)

    def test_dictionary_length_uses_r3_and_rejects_unknown_formats(self):
        self.assertEqual(6, editor_dictionary.length_limit("6"))
        self.assertEqual(10, editor_dictionary.length_limit("4* E2B(R2)\n10* E2B(R3)"))
        with self.assertRaisesRegex(ValueError, "unsupported dictionary length"):
            editor_dictionary.length_limit("unknown")

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
                'field_contract_test!(g_k_9_i_1, "G.k.9.i.1");',
                outputs["DG"],
            )
            self.assertIn('field_contract_test!(g_k_local_sourceproductpresaveid, "G.k.local.sourceProductPresaveId");', outputs["DG"])
            self.assertIn('field_contract_test!(n_1_5, "N.1.5");', outputs["N"])
            self.assertEqual(8, outputs["N"].count("field_contract_test!("))

    def test_rejects_blocked_field_without_reason(self):
        fields = [{"code": "G.k.9.i.1", "execution": {"status": "blocked"}}]

        with self.assertRaisesRegex(ValueError, "blocked execution requires a reason"):
            generator.render_page("DG", fields)

    def test_runtime_references_require_a_known_source_and_no_literal(self):
        root = generator.REPO_ROOT / "registry"
        contract = generator.load_contract(root, "DG")
        generator.validate_behavior_bindings(contract, root)
        for mutation in [
            {"roundTripReference": "unknown"},
            {"roundTripValue": "00000000-0000-0000-0000-000000000001"},
            {"execution": {"status": "blocked", "reason": "contradictory fixture"}},
        ]:
            with self.subTest(mutation=mutation):
                changed = copy.deepcopy(contract)
                field = next(field for field in changed["fields"] if field["code"] == "G.k.9.i.1")
                field.update(mutation)
                with self.assertRaisesRegex(ValueError, "invalid runtime roundtrip reference"):
                    generator.validate_behavior_bindings(changed, root)

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
