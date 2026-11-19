"""Pin input semantics, not test implementations. Rebaselines require owner review."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path

import editor_dictionary


REPO_ROOT = Path(__file__).resolve().parents[2]


def semantics(root: Path) -> dict:
    dictionary = {
        code: {"source": source, **{key: entry[key]
                                  for key in editor_dictionary.INPUT_PROPERTIES if key in entry}}
        for code, (source, entry) in editor_dictionary.dictionary_entries(root).items()
    }
    fields = {}
    exclusions = {}
    for path in sorted((root / "editor-contracts").glob("*.json")):
        if path.name == "schema.json":
            continue
        document = json.loads(path.read_text())
        page = document["pageId"]
        exclusions[page] = sorted(item["code"] for item in document.get("dictionaryExclusions", []))
        for field in document["fields"]:
            bindings = {}
            for binding in field.get("dictionaryInputs", []):
                bindings[binding["code"]] = {
                    "source": binding["source"],
                    "checks": {
                        prop: {key: value for key, value in spec.items()
                               if key not in ("valid", "invalid", "reason")}
                        for prop, spec in binding["checks"].items()
                    },
                }
            fields[f'{page}/{field["code"]}'] = {
                "authority": field["authority"],
                "dictionaryInputs": bindings,
                "constraint": {key: value for key, value in field.get("constraint", {}).items()
                               if key not in ("invalidValue", "reason")},
                "persistence": {key: value for key, value in field.get("persistence", {}).items()
                                if key != "audit"},
            }
    vocabularies = {path.name: json.loads(path.read_text())
                    for path in sorted((root / "vocabularies").glob("*.json"))}
    return {"dictionary": dictionary, "fields": fields,
            "exclusions": exclusions, "vocabularies": vocabularies}


def fingerprint(root: Path) -> str:
    encoded = json.dumps(semantics(root), sort_keys=True, ensure_ascii=False,
                         separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def check(repo: Path) -> None:
    editor_dictionary.check(repo / "registry")
    actual = fingerprint(repo / "registry")
    expected = (repo / ".github/input-contract.sha256").read_text().strip()
    if actual != expected:
        raise ValueError(
            "Input contract semantics changed (including ownership/unresolved status). "
            "Do not auto-update the lock: official evidence and owner approval are required. "
            f"Candidate SHA-256: {actual}"
        )


if __name__ == "__main__":
    check(REPO_ROOT)
    print("Input contract semantics match the reviewed baseline.")
