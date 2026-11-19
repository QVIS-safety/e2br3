import json
from pathlib import Path
import shutil
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_input_contract_lock as guard


class InputContractLockTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "registry"
        for directory in ("dictionary", "editor-contracts", "vocabularies"):
            shutil.copytree(guard.REPO_ROOT / "registry" / directory, self.root / directory)
        self.before = guard.fingerprint(self.root)

    def edit(self, relative, mutate):
        path = self.root / relative
        document = json.loads(path.read_text())
        mutate(document)
        path.write_text(json.dumps(document))

    def test_current_baseline(self):
        guard.check(guard.REPO_ROOT)

    def test_joint_dictionary_and_contract_weakening_changes_fingerprint(self):
        self.edit("dictionary/ich-e2br3.json", lambda d: next(
            e for e in d["entries"] if e["code"] == "C.1.1").update(max_length="101"))
        self.edit("editor-contracts/ci.json", lambda d:
                  d["fields"][0]["dictionaryInputs"][0]["checks"]["max_length"].update(dictionaryValue="101"))
        self.assertNotEqual(self.before, guard.fingerprint(self.root))

    def test_owner_downgrade_and_validation_stage_move_change_fingerprint(self):
        path = self.root / "editor-contracts/ci.json"
        original = path.read_text()
        for owner in ("unresolved", "validator"):
            with self.subTest(owner=owner):
                path.write_text(original)
                self.edit("editor-contracts/ci.json", lambda d:
                          d["fields"][0]["dictionaryInputs"][0]["checks"]["max_length"].update(owner=owner))
                self.assertNotEqual(self.before, guard.fingerprint(self.root))

    def test_field_deletion_and_vocab_widening_change_fingerprint(self):
        self.edit("editor-contracts/ci.json", lambda d: d["fields"].pop())
        self.assertNotEqual(self.before, guard.fingerprint(self.root))
        before_vocab = guard.fingerprint(self.root)
        self.edit("vocabularies/ich-time.json", lambda d: d["entries"].append({"code": "new-unit"}))
        self.assertNotEqual(before_vocab, guard.fingerprint(self.root))

    def test_test_fixtures_and_implementation_names_are_not_locked(self):
        def mutate(document):
            field = document["fields"][0]
            field["roundTripValue"] = "different fixture"
            field["constraint"]["invalidValue"] = "different fixture"
            binding = field["dictionaryInputs"][0]
            binding["function"] = "refactored_function"
            binding["checks"]["max_length"]["valid"].append({"value": "other", "nullFlavor": None})
        self.edit("editor-contracts/ci.json", mutate)
        self.assertEqual(self.before, guard.fingerprint(self.root))

    def test_missing_lock_fails_closed(self):
        catalog = self.root.parent / "crates/libs/validator/rules.json"
        catalog.parent.mkdir(parents=True)
        shutil.copyfile(guard.REPO_ROOT / "crates/libs/validator/rules.json", catalog)
        with self.assertRaises(FileNotFoundError):
            guard.check(self.root.parent)

    def test_documented_downgrade_still_fails_the_lock(self):
        catalog = self.root.parent / "crates/libs/validator/rules.json"
        catalog.parent.mkdir(parents=True)
        shutil.copyfile(guard.REPO_ROOT / "crates/libs/validator/rules.json", catalog)
        lock = self.root.parent / ".github/input-contract.sha256"
        lock.parent.mkdir()
        lock.write_text(self.before)
        self.edit("editor-contracts/ci.json", lambda d:
                  d["fields"][0]["dictionaryInputs"][0]["checks"]["max_length"].update(
                      owner="unresolved", reason="A reason does not authorize removing enforcement."))
        guard.editor_dictionary.check(self.root)
        with self.assertRaisesRegex(ValueError, "Input contract semantics changed"):
            guard.check(self.root.parent)


if __name__ == "__main__":
    unittest.main()
