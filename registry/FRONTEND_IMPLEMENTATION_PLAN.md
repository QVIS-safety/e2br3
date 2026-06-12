# Frontend Input Inventory Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `registry/tools/extract_frontend_fields.py` and strict frontend inventory validation so the registry proves actual frontend input fields are mapped exactly once.

**Architecture:** Keep `registry/sections/*.json` as the only canonical mapping data. The frontend extractor derives normalized input names from React Hook Form source files at runtime, returns deterministic inventory rows, and `validate.py --strict-frontend-inventory` compares those rows against mapped registry frontend blocks. No generated inventory files are committed.

**Tech Stack:** Python standard library `unittest`, `json`, `pathlib`, `re`, `glob`; frontend React/TypeScript source under `../frontend/E2BR3-frontend/components/case-form/sections/`.

---

## File Structure

- Create: `registry/tools/extract_frontend_fields.py`
  - Extract editable field names from frontend source.
  - Normalize repeatable indexes.
  - Print deterministic JSON inventory to stdout.
- Create: `registry/tools/test_extract_frontend_fields.py`
  - Unit tests for normalization, extraction patterns, CLI-safe inventory shape, and fail-closed missing files/globs.
- Modify: `registry/tools/validate.py`
  - Add `--strict-frontend-inventory`.
  - Compare extracted frontend keys to registry mapped frontend keys.
  - Keep normal validation CI-safe.
- Modify: `registry/tools/test_validate.py`
  - Add strict frontend comparison tests using temp frontend files.
  - Add one scoped repository test only after a section is intentionally aligned.
- Modify: `registry/sections/*.json`
  - Only after extractor and strict mode exist, align registry frontend keys section-by-section.

## Implementation Rules

- Extract actual editable input field names, not DTO/type-only fields.
- Do not scrape labels, comments, markdown, artifacts, tests, or broad API response mappers.
- Do not commit extractor output.
- Do not create `registry/generated/`.
- Normal validation remains:

```sh
python3 registry/tools/validate.py
```

- Strict frontend validation is explicit:

```sh
python3 registry/tools/validate.py --strict-frontend-inventory
```

- A registry frontend key is always:

```text
frontend.section + "." + frontend.field
```

Example registry block:

```json
{
  "frontend": {
    "status": "mapped",
    "section": "reactions",
    "field": "reactionCountry"
  }
}
```

maps to extractor key:

```text
reactions.reactionCountry
```

For nested sections already present in registry, concatenate exactly:

```json
{
  "frontend": {
    "section": "narrative.senderDiagnoses",
    "field": "diagnosisMeddraVersion"
  }
}
```

maps to:

```text
narrative.senderDiagnoses.diagnosisMeddraVersion
```

## Task 1: Build Frontend Field Normalization

**Files:**
- Create: `registry/tools/test_extract_frontend_fields.py`
- Create: `registry/tools/extract_frontend_fields.py`

- [ ] **Step 1: Write failing normalization tests**

Create `registry/tools/test_extract_frontend_fields.py`:

```python
import unittest

import extract_frontend_fields as extractor


class FrontendFieldExtractorTests(unittest.TestCase):
    def test_normalizes_template_repeatable_indexes(self):
        self.assertEqual(
            "reactions.reactionCountry",
            extractor.normalize_field_path("reactions.${activeIndex}.reactionCountry"),
        )
        self.assertEqual(
            "patientInformation.medicalHistoryEpisodes.comments",
            extractor.normalize_field_path(
                "patientInformation.medicalHistoryEpisodes.${index}.comments"
            ),
        )

    def test_normalizes_numeric_repeatable_indexes(self):
        self.assertEqual(
            "testResults.comments",
            extractor.normalize_field_path("testResults.0.comments"),
        )

    def test_preserves_business_field_names(self):
        self.assertEqual(
            "reactions.reactionCountry",
            extractor.normalize_field_path("reactions.${activeIndex}.reactionCountry"),
        )
```

- [ ] **Step 2: Run tests and verify they fail**

Run:

```sh
python3 -m unittest registry/tools/test_extract_frontend_fields.py
```

Expected: FAIL because `extract_frontend_fields.py` does not exist.

- [ ] **Step 3: Implement minimal normalization**

Create `registry/tools/extract_frontend_fields.py`:

```python
#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


REPO_ROOT = Path(__file__).resolve().parents[2]
FRONTEND_ROOT = REPO_ROOT.parent / "frontend" / "E2BR3-frontend"


@dataclass(frozen=True)
class FrontendField:
    key: str
    section: str
    field: str
    file: str
    raw: str


def normalize_field_path(raw: str) -> str:
    value = raw.strip()
    value = re.sub(r"\.\$\{[^}]+\}", "", value)
    value = re.sub(r"\.\d+(?=\.|$)", "", value)
    value = value.replace("`", "").replace('"', "").replace("'", "")
    value = re.sub(r"\.+", ".", value)
    return value.strip(".")


def split_key(key: str) -> tuple[str, str]:
    if "." not in key:
        return key, ""
    section, field = key.split(".", 1)
    return section, field
```

- [ ] **Step 4: Run tests and verify they pass**

Run:

```sh
python3 -m unittest registry/tools/test_extract_frontend_fields.py
```

Expected: PASS.

## Task 2: Extract `name` And `register` Field Declarations

**Files:**
- Modify: `registry/tools/test_extract_frontend_fields.py`
- Modify: `registry/tools/extract_frontend_fields.py`

- [ ] **Step 1: Add failing extraction tests**

Append to `FrontendFieldExtractorTests`:

```python
    def test_extracts_literal_and_template_name_props(self):
        source = '''
<Controller name="patientInformation.patientAge.value" control={control} />
<Controller
  name={`reactions.${activeIndex}.reactionCountry`}
  control={control}
/>
'''

        fields = extractor.extract_field_paths_from_source(source)

        self.assertEqual(
            [
                "patientInformation.patientAge.value",
                "reactions.reactionCountry",
            ],
            fields,
        )

    def test_extracts_register_calls(self):
        source = '''
<Input {...register("safetyReportIdentification.receiverEmail")} />
<Input {...register(`testResults.${index}.comments`)} />
'''

        fields = extractor.extract_field_paths_from_source(source)

        self.assertEqual(
            [
                "safetyReportIdentification.receiverEmail",
                "testResults.comments",
            ],
            fields,
        )
```

- [ ] **Step 2: Run tests and verify they fail**

Run:

```sh
python3 -m unittest registry/tools/test_extract_frontend_fields.py
```

Expected: FAIL because `extract_field_paths_from_source` does not exist.

- [ ] **Step 3: Implement source extraction**

Add to `extract_frontend_fields.py`:

```python
FIELD_PATTERNS = [
    re.compile(r"name\s*=\s*[\"']([^\"']+)[\"']"),
    re.compile(r"name\s*=\s*\{\s*`([^`]+)`\s*\}"),
    re.compile(r"register\(\s*[\"']([^\"']+)[\"']"),
    re.compile(r"register\(\s*`([^`]+)`"),
]


def extract_field_paths_from_source(source: str) -> list[str]:
    fields: set[str] = set()
    for pattern in FIELD_PATTERNS:
        for match in pattern.finditer(source):
            raw = match.group(1)
            if not raw:
                continue
            key = normalize_field_path(raw)
            if key:
                fields.add(key)
    return sorted(fields)
```

- [ ] **Step 4: Run tests and verify they pass**

Run:

```sh
python3 -m unittest registry/tools/test_extract_frontend_fields.py
```

Expected: PASS.

## Task 3: Build File Inventory And CLI JSON Output

**Files:**
- Modify: `registry/tools/test_extract_frontend_fields.py`
- Modify: `registry/tools/extract_frontend_fields.py`

- [ ] **Step 1: Add failing file inventory tests**

Add imports:

```python
import json
import tempfile
from pathlib import Path
```

Append tests:

```python
    def test_extracts_inventory_from_configured_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            frontend = root / "frontend"
            frontend.mkdir()
            section = frontend / "SectionE.tsx"
            section.write_text(
                '<Controller name={`reactions.${activeIndex}.reactionCountry`} />',
                encoding="utf-8",
            )

            fields = extractor.extract_frontend_fields(
                root=root,
                source_globs=["frontend/SectionE.tsx"],
            )

        self.assertEqual(1, len(fields))
        self.assertEqual("reactions.reactionCountry", fields[0].key)
        self.assertEqual("reactions", fields[0].section)
        self.assertEqual("reactionCountry", fields[0].field)
        self.assertEqual("reactions.${activeIndex}.reactionCountry", fields[0].raw)

    def test_missing_glob_fails_closed(self):
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(extractor.FrontendInventoryError):
                extractor.extract_frontend_fields(
                    root=Path(tmp),
                    source_globs=["frontend/Missing.tsx"],
                )

    def test_json_output_is_deterministic(self):
        field = extractor.FrontendField(
            key="reactions.reactionCountry",
            section="reactions",
            field="reactionCountry",
            file="frontend/SectionE.tsx",
            raw="reactions.${activeIndex}.reactionCountry",
        )

        payload = extractor.fields_to_json([field])

        self.assertEqual(
            [
                {
                    "key": "reactions.reactionCountry",
                    "section": "reactions",
                    "field": "reactionCountry",
                    "file": "frontend/SectionE.tsx",
                    "raw": "reactions.${activeIndex}.reactionCountry",
                }
            ],
            json.loads(payload),
        )
```

- [ ] **Step 2: Run tests and verify they fail**

Run:

```sh
python3 -m unittest registry/tools/test_extract_frontend_fields.py
```

Expected: FAIL because file inventory functions do not exist.

- [ ] **Step 3: Implement file inventory and JSON output**

Add to `extract_frontend_fields.py`:

```python
class FrontendInventoryError(Exception):
    pass


DEFAULT_SOURCE_GLOBS = [
    "../frontend/E2BR3-frontend/components/case-form/sections/SectionC*.tsx",
    "../frontend/E2BR3-frontend/components/case-form/sections/SectionD.tsx",
    "../frontend/E2BR3-frontend/components/case-form/sections/SectionDH.tsx",
    "../frontend/E2BR3-frontend/components/case-form/sections/SectionE.tsx",
    "../frontend/E2BR3-frontend/components/case-form/sections/SectionF.tsx",
    "../frontend/E2BR3-frontend/components/case-form/sections/SectionG.tsx",
    "../frontend/E2BR3-frontend/components/case-form/sections/SectionH.tsx",
]


def extract_frontend_fields(
    root: Path = REPO_ROOT,
    source_globs: Iterable[str] = DEFAULT_SOURCE_GLOBS,
) -> list[FrontendField]:
    inventory: dict[tuple[str, str, str], FrontendField] = {}
    for source_glob in source_globs:
        matches = sorted(root.glob(source_glob))
        if not matches:
            raise FrontendInventoryError(f"frontend source glob matched no files: {source_glob}")
        for source_path in matches:
            try:
                source = source_path.read_text(encoding="utf-8")
            except OSError as exc:
                raise FrontendInventoryError(f"could not read frontend source {source_path}: {exc}") from exc
            for raw_path in extract_raw_field_paths_from_source(source):
                key = normalize_field_path(raw_path)
                section, field = split_key(key)
                relative_file = str(source_path.relative_to(root))
                inventory[(key, relative_file, raw_path)] = FrontendField(
                    key=key,
                    section=section,
                    field=field,
                    file=relative_file,
                    raw=raw_path,
                )
    return sorted(inventory.values(), key=lambda item: (item.key, item.file, item.raw))


def fields_to_json(fields: Iterable[FrontendField]) -> str:
    payload = [
        {
            "key": field.key,
            "section": field.section,
            "field": field.field,
            "file": field.file,
            "raw": field.raw,
        }
        for field in sorted(fields, key=lambda item: (item.key, item.file, item.raw))
    ]
    return json.dumps(payload, indent=2) + "\n"
```

Rename the earlier function:

```python
def extract_raw_field_paths_from_source(source: str) -> list[str]:
    fields: set[str] = set()
    for pattern in FIELD_PATTERNS:
        for match in pattern.finditer(source):
            raw = match.group(1)
            if raw:
                fields.add(raw.strip())
    return sorted(fields)


def extract_field_paths_from_source(source: str) -> list[str]:
    return sorted(
        normalize_field_path(raw)
        for raw in extract_raw_field_paths_from_source(source)
        if normalize_field_path(raw)
    )
```

Add CLI:

```python
def main() -> int:
    try:
        fields = extract_frontend_fields()
    except FrontendInventoryError as exc:
        print(f"frontend inventory extraction failed: {exc}", file=sys.stderr)
        return 1
    print(fields_to_json(fields), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 4: Run tests and CLI**

Run:

```sh
python3 -m unittest registry/tools/test_extract_frontend_fields.py
python3 registry/tools/extract_frontend_fields.py >/tmp/e2br3-frontend-fields.json
python3 -m json.tool /tmp/e2br3-frontend-fields.json >/dev/null
```

Expected: tests PASS, CLI exits 0, JSON parses.

## Task 4: Add Strict Frontend Inventory Comparison To Validator

**Files:**
- Modify: `registry/tools/test_validate.py`
- Modify: `registry/tools/validate.py`

- [ ] **Step 1: Add failing validator tests**

Add to `registry/tools/test_validate.py`:

```python
    def test_rejects_frontend_field_present_in_source_but_missing_from_registry(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source_dir = root / "frontend"
            source_dir.mkdir()
            (source_dir / "SectionE.tsx").write_text(
                '<Controller name={`reactions.${activeIndex}.reactionCountry`} />',
                encoding="utf-8",
            )
            self.write_registry(root, "[]")

            result = validate.validate_registry(
                root,
                validate_backend_inventory=False,
                validate_frontend_inventory=True,
                frontend_source_globs=["frontend/SectionE.tsx"],
            )

        self.assertIn(
            "missing frontend mapping: reactions.reactionCountry",
            "\n".join(result.errors),
        )

    def test_rejects_registry_frontend_mapping_absent_from_source(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source_dir = root / "frontend"
            source_dir.mkdir()
            (source_dir / "SectionE.tsx").write_text(
                '<Controller name={`reactions.${activeIndex}.reactionCountry`} />',
                encoding="utf-8",
            )
            row = self.valid_row().replace('"section": "sender"', '"section": "reactions"')
            row = row.replace('"field": "organizationName"', '"field": "reactionOutcome"')
            self.write_registry(root, row)

            result = validate.validate_registry(
                root,
                validate_backend_inventory=False,
                validate_frontend_inventory=True,
                frontend_source_globs=["frontend/SectionE.tsx"],
            )

        self.assertIn(
            "unknown frontend mapping: reactions.reactionOutcome",
            "\n".join(result.errors),
        )
```

- [ ] **Step 2: Run tests and verify they fail**

Run:

```sh
python3 -m unittest registry/tools/test_validate.py
```

Expected: FAIL because `validate_frontend_inventory` and `frontend_source_globs` are not supported.

- [ ] **Step 3: Import extractor and compare frontend sets**

Modify `registry/tools/validate.py`:

```python
import extract_frontend_fields
```

Change the `validate_registry` signature:

```python
def validate_registry(
    root: Path = ROOT,
    backend_models: dict[str, str] | None = None,
    validate_backend_inventory: bool = True,
    validate_frontend_inventory: bool = False,
    frontend_source_globs: list[str] | None = None,
) -> ValidationResult:
```

After backend inventory comparison, add:

```python
    if validate_frontend_inventory:
        try:
            source_frontend = {
                field.key
                for field in extract_frontend_fields.extract_frontend_fields(
                    root=root,
                    source_globs=frontend_source_globs
                    or extract_frontend_fields.DEFAULT_SOURCE_GLOBS,
                )
            }
        except extract_frontend_fields.FrontendInventoryError as exc:
            result.add(str(exc))
            return result

        registry_frontend = set(seen_frontend)
        for key in sorted(source_frontend - registry_frontend):
            result.add(f"missing frontend mapping: {key}")
        for key in sorted(registry_frontend - source_frontend):
            result.add(f"unknown frontend mapping: {key}")
```

Change CLI:

```python
strict_frontend_inventory = "--strict-frontend-inventory" in sys.argv[1:]
result = validate_registry(
    validate_backend_inventory=strict_backend_inventory,
    validate_frontend_inventory=strict_frontend_inventory,
)
```

- [ ] **Step 4: Run tests and verify they pass**

Run:

```sh
python3 -m unittest registry/tools/test_validate.py
```

Expected: PASS.

## Task 5: Add Section-Scoped Frontend Alignment Tests

**Files:**
- Modify: `registry/tools/test_validate.py`
- Modify: `registry/sections/*.json`

- [ ] **Step 1: Start with one narrow section file**

Add a scoped repository test for SectionF because it has direct `testResults.*`
inputs:

```python
    def test_repository_f_frontend_inputs_have_registry_rows(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            validate_backend_inventory=False,
            validate_frontend_inventory=True,
            frontend_source_globs=[
                "../frontend/E2BR3-frontend/components/case-form/sections/SectionF.tsx"
            ],
        )

        self.assertEqual([], result.errors)
```

- [ ] **Step 2: Run the scoped test and inspect failures**

Run:

```sh
python3 -m unittest registry.tools.test_validate.RegistryValidatorTests.test_repository_f_frontend_inputs_have_registry_rows
```

Expected initially: FAIL with missing or unknown frontend mappings.

- [ ] **Step 3: Align SectionF registry rows**

Edit `registry/sections/f-test.json` only.

Use normalized input paths from extractor output. Examples:

```json
{
  "frontend": {
    "status": "mapped",
    "section": "testResults",
    "field": "comments",
    "file": "../frontend/E2BR3-frontend/components/case-form/sections/SectionF.tsx",
    "evidence": "SectionF binds name testResults.${activeIndex}.comments."
  }
}
```

If the registry currently points at aliases like `testUnit` or `testResult`, keep
the frontend field as the actual input name only when the input is present in
SectionF. Do not use backend DTO field names as frontend keys.

- [ ] **Step 4: Run scoped test until green**

Run:

```sh
python3 -m unittest registry.tools.test_validate.RegistryValidatorTests.test_repository_f_frontend_inputs_have_registry_rows
python3 registry/tools/validate.py
python3 registry/tools/validate.py --strict-backend-inventory
```

Expected: scoped frontend test PASS, normal validation PASS, strict backend PASS.

- [ ] **Step 5: Repeat section-by-section**

Add one scoped repository test at a time in this order:

```python
def test_repository_e_frontend_inputs_have_registry_rows(...)
def test_repository_d_frontend_inputs_have_registry_rows(...)
def test_repository_g_frontend_inputs_have_registry_rows(...)
def test_repository_h_frontend_inputs_have_registry_rows(...)
def test_repository_c_frontend_inputs_have_registry_rows(...)
```

For each section:

```sh
python3 -m unittest registry.tools.test_validate.RegistryValidatorTests.<test_name>
```

Expected before alignment: deterministic missing/unknown frontend mapping errors.

Fix only that section JSON. Then run:

```sh
python3 -m unittest registry/tools/test_validate.py
python3 registry/tools/validate.py
python3 registry/tools/validate.py --strict-backend-inventory
```

Expected after each section: all existing tests PASS.

## Task 6: Enable Whole-Registry Strict Frontend Validation

**Files:**
- Modify: `registry/tools/test_validate.py`
- Modify: `registry/README.md`

- [ ] **Step 1: Add final repository frontend test**

Add:

```python
    def test_repository_frontend_inputs_have_registry_rows(self):
        registry_root = Path(__file__).resolve().parents[1]

        result = validate.validate_registry(
            registry_root,
            validate_backend_inventory=False,
            validate_frontend_inventory=True,
        )

        self.assertEqual([], result.errors)
```

- [ ] **Step 2: Run final strict frontend CLI**

Run:

```sh
python3 registry/tools/validate.py --strict-frontend-inventory
```

Expected: PASS only after all configured frontend section files are aligned.

- [ ] **Step 3: Update README from planned to active**

Change `registry/README.md`:

```markdown
- `tools/extract_frontend_fields.py`: frontend input-field extractor.
```

Change:

```markdown
Planned frontend input-field inventory command:
```

to:

```markdown
Extract frontend input-field inventory:
```

Change:

```markdown
Planned strict frontend validation command:
```

to:

```markdown
Validate registry rows against extracted frontend input fields:
```

- [ ] **Step 4: Run final verification**

Run:

```sh
python3 -m unittest registry/tools/test_extract_frontend_fields.py
python3 -m unittest registry/tools/test_validate.py
python3 registry/tools/validate.py
python3 registry/tools/validate.py --strict-backend-inventory
python3 registry/tools/validate.py --strict-frontend-inventory
python3 registry/tools/extract_frontend_fields.py >/tmp/e2br3-frontend-fields.json
python3 -m json.tool /tmp/e2br3-frontend-fields.json >/dev/null
for f in registry/index.json registry/schema.json registry/sections/*.json; do python3 -m json.tool "$f" >/dev/null || exit 1; done
rm -rf registry/tools/__pycache__
```

Expected:

- extractor tests PASS
- validator tests PASS
- normal validation PASS
- strict backend validation PASS
- strict frontend validation PASS
- extractor JSON parses
- all registry JSON parses
- no `registry/tools/__pycache__` remains

## Self-Review Checklist

- Spec coverage:
  - `extract_frontend_fields.py` CLI is planned in Task 3.
  - Actual input field extraction is planned in Task 2.
  - Repeatable normalization is planned in Task 1.
  - Strict frontend validation is planned in Task 4.
  - Section-by-section alignment is planned in Task 5.
  - Whole-registry strict frontend validation is planned in Task 6.
- No committed generated output is introduced.
- No second canonical matrix is introduced.
- Normal validation remains CI-safe until strict frontend mode is explicitly run.
