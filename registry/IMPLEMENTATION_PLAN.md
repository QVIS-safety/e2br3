# E2BR3 Registry Source Coverage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `registry/tools/validate.py` prove that the existing registry section JSONs cover every configured backend BMC field and frontend field without adding generated inventory files or a second matrix.

**Architecture:** Keep `registry/sections/*.json` as the only canonical mapping data. The validator extracts backend and frontend inventories from source code in memory, compares those sets against mapped registry rows, and reports deterministic missing or unknown mapping errors. Backend coverage ships first because Rust model extraction is stable; frontend coverage is added after the backend rules are proven.

**Tech Stack:** Python standard library `unittest`, `json`, `pathlib`, `re`; Rust source under `crates/libs/lib-core/src/model/`; sibling frontend source under `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend`.

---

## File Structure

- Modify: `registry/tools/validate.py`
  - Add source inventory extraction.
  - Add backend model scope configuration.
  - Add frontend source scope configuration after backend coverage is stable.
  - Keep all extracted inventories in memory only.
- Modify: `registry/tools/test_validate.py`
  - Add TDD tests for Rust struct extraction, backend set comparison, unknown registry mappings, fail-closed parse behavior, and later frontend extraction.
- Modify: `registry/sections/*.json`
  - Add missing rows to the existing section JSON files only.
  - Use `frontend_missing` or `backend_missing` rows when one side is real but the opposite side is not verified yet.
- Modify: `registry/SPEC.md`
  - Update only if implementation discovers a needed rule that changes the contract.
- Do not create: `registry/generated/`, inventory JSON, markdown reports, spreadsheets, or a second canonical matrix.

## Implementation Rules

- The validator input is registry JSON plus implementation source files.
- The validator must not read committed inventory data.
- Missing backend fields are tracked by rows in the existing section JSON files.
- A backend field present in source but absent from registry is an error.
- A backend mapping declared in registry but absent from source is an error.
- Duplicate mapped backend keys are errors.
- Duplicate mapped frontend keys are errors.
- Fail closed if a configured source file or configured struct cannot be read or parsed.
- Use deterministic sorted error output.

## Backend Model Scope

Start with case-domain backend models that have BMC ownership and E2BR3 section meaning:

```python
BACKEND_MODELS = {
    "SafetyReportIdentification": "crates/libs/lib-core/src/model/safety_report.rs",
    "SenderInformation": "crates/libs/lib-core/src/model/safety_report.rs",
    "PrimarySource": "crates/libs/lib-core/src/model/safety_report.rs",
    "LiteratureReference": "crates/libs/lib-core/src/model/safety_report.rs",
    "DocumentsHeldBySender": "crates/libs/lib-core/src/model/safety_report.rs",
    "StudyInformation": "crates/libs/lib-core/src/model/safety_report.rs",
    "StudyRegistrationNumber": "crates/libs/lib-core/src/model/safety_report.rs",
    "StudyFdaCrossReportedInd": "crates/libs/lib-core/src/model/safety_report.rs",
    "ReceiverInformation": "crates/libs/lib-core/src/model/receiver.rs",
    "PatientInformation": "crates/libs/lib-core/src/model/patient.rs",
    "PatientIdentifier": "crates/libs/lib-core/src/model/patient.rs",
    "MedicalHistoryEpisode": "crates/libs/lib-core/src/model/patient.rs",
    "PastDrugHistory": "crates/libs/lib-core/src/model/patient.rs",
    "PatientDeathInformation": "crates/libs/lib-core/src/model/patient.rs",
    "ReportedCauseOfDeath": "crates/libs/lib-core/src/model/patient.rs",
    "AutopsyCauseOfDeath": "crates/libs/lib-core/src/model/patient.rs",
    "ParentInformation": "crates/libs/lib-core/src/model/patient.rs",
    "ParentMedicalHistory": "crates/libs/lib-core/src/model/parent_history.rs",
    "ParentPastDrugHistory": "crates/libs/lib-core/src/model/parent_history.rs",
    "Reaction": "crates/libs/lib-core/src/model/reaction.rs",
    "TestResult": "crates/libs/lib-core/src/model/test_result.rs",
    "DrugInformation": "crates/libs/lib-core/src/model/drug.rs",
    "DrugActiveSubstance": "crates/libs/lib-core/src/model/drug.rs",
    "DosageInformation": "crates/libs/lib-core/src/model/drug.rs",
    "DrugIndication": "crates/libs/lib-core/src/model/drug.rs",
    "DrugDeviceCharacteristic": "crates/libs/lib-core/src/model/drug.rs",
    "DrugRecurrenceInformation": "crates/libs/lib-core/src/model/drug_recurrence.rs",
    "DrugReactionAssessment": "crates/libs/lib-core/src/model/drug_reaction_assessment.rs",
    "RelatednessAssessment": "crates/libs/lib-core/src/model/drug_reaction_assessment.rs",
    "NarrativeInformation": "crates/libs/lib-core/src/model/narrative.rs",
    "SenderDiagnosis": "crates/libs/lib-core/src/model/narrative.rs",
    "CaseSummaryInformation": "crates/libs/lib-core/src/model/narrative.rs",
    "MessageHeader": "crates/libs/lib-core/src/model/message_header.rs",
}
```

Ignore backend technical fields by exact field name:

```python
IGNORED_BACKEND_FIELDS = {
    "id",
    "case_id",
    "drug_id",
    "reaction_id",
    "created_at",
    "updated_at",
    "created_by",
    "updated_by",
    "sequence_number",
}
```

If a field appears technical but is not listed here, do not silently skip it. Let validation fail and decide explicitly whether it should be mapped or added to `IGNORED_BACKEND_FIELDS`.

## Task 1: Add Backend Struct Field Extraction Tests

**Files:**
- Modify: `registry/tools/test_validate.py`

- [ ] **Step 1: Write tests for extracting Rust struct fields**

Add these tests to `RegistryValidatorTests`:

```python
    def test_extracts_public_fields_from_rust_struct(self):
        source = """
#[derive(Debug, Clone)]
pub struct PatientInformation {
    pub id: Uuid,
    pub case_id: Uuid,
    pub patient_initial: Option<String>,
    pub patient_age_group: Option<String>,
}
"""

        fields = validate.extract_rust_struct_fields(source, "PatientInformation")

        self.assertEqual(["id", "case_id", "patient_initial", "patient_age_group"], fields)

    def test_struct_extraction_fails_when_struct_is_missing(self):
        source = "pub struct OtherModel { pub id: Uuid }"

        with self.assertRaises(validate.InventoryError):
            validate.extract_rust_struct_fields(source, "PatientInformation")
```

- [ ] **Step 2: Run tests and verify they fail**

Run:

```sh
python3 -m unittest registry/tools/test_validate.py
```

Expected: FAIL because `extract_rust_struct_fields` and `InventoryError` do not exist yet.

## Task 2: Implement Minimal Backend Struct Field Extraction

**Files:**
- Modify: `registry/tools/validate.py`

- [ ] **Step 1: Add `InventoryError` and `extract_rust_struct_fields`**

Add near the dataclass definitions:

```python
class InventoryError(Exception):
    pass
```

Add this function:

```python
def extract_rust_struct_fields(source: str, struct_name: str) -> list[str]:
    marker = f"pub struct {struct_name}"
    start = source.find(marker)
    if start == -1:
        raise InventoryError(f"could not find Rust struct {struct_name}")

    brace_start = source.find("{", start)
    if brace_start == -1:
        raise InventoryError(f"could not find body for Rust struct {struct_name}")

    depth = 0
    end = None
    for index in range(brace_start, len(source)):
        char = source[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                end = index
                break

    if end is None:
        raise InventoryError(f"could not parse body for Rust struct {struct_name}")

    body = source[brace_start + 1 : end]
    fields: list[str] = []
    for line in body.splitlines():
        stripped = line.strip()
        if not stripped.startswith("pub "):
            continue
        name = stripped.removeprefix("pub ").split(":", 1)[0].strip()
        if name:
            fields.append(name)
    return fields
```

- [ ] **Step 2: Run extraction tests**

Run:

```sh
python3 -m unittest registry/tools/test_validate.py
```

Expected: PASS for the new extraction tests and existing tests.

## Task 3: Add Backend Inventory Set Comparison Tests

**Files:**
- Modify: `registry/tools/test_validate.py`

- [ ] **Step 1: Write tests for missing and unknown backend mappings**

Add these tests:

```python
    def test_rejects_backend_field_present_in_source_but_missing_from_registry(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source_dir = root / "crates/libs/lib-core/src/model"
            source_dir.mkdir(parents=True)
            (source_dir / "patient.rs").write_text(
                """
pub struct PatientInformation {
    pub id: Uuid,
    pub case_id: Uuid,
    pub patient_initial: Option<String>,
}
""",
                encoding="utf-8",
            )
            self.write_registry(root, "[]")

            result = validate.validate_registry(
                root,
                backend_models={"PatientInformation": "crates/libs/lib-core/src/model/patient.rs"},
            )

        self.assertIn(
            "missing backend mapping: PatientInformation.patient_initial",
            "\n".join(result.errors),
        )

    def test_rejects_registry_backend_mapping_absent_from_source(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source_dir = root / "crates/libs/lib-core/src/model"
            source_dir.mkdir(parents=True)
            (source_dir / "patient.rs").write_text(
                """
pub struct PatientInformation {
    pub id: Uuid,
    pub case_id: Uuid,
    pub patient_initial: Option<String>,
}
""",
                encoding="utf-8",
            )
            row = self.valid_row().replace('"model": "SenderInformation"', '"model": "PatientInformation"')
            row = row.replace('"field": "organization_name"', '"field": "patient_sex"')
            self.write_registry(root, row)

            result = validate.validate_registry(
                root,
                backend_models={"PatientInformation": "crates/libs/lib-core/src/model/patient.rs"},
            )

        self.assertIn(
            "unknown backend mapping: PatientInformation.patient_sex",
            "\n".join(result.errors),
        )
```

- [ ] **Step 2: Run tests and verify they fail**

Run:

```sh
python3 -m unittest registry/tools/test_validate.py
```

Expected: FAIL because `validate_registry` does not accept `backend_models` and does not compare extracted source fields.

## Task 4: Implement Backend Inventory Validation

**Files:**
- Modify: `registry/tools/validate.py`

- [ ] **Step 1: Add backend scope constants**

Add the `BACKEND_MODELS` and `IGNORED_BACKEND_FIELDS` constants from the "Backend Model Scope" section above.

- [ ] **Step 2: Add extraction helpers**

Add:

```python
def backend_key(model: str, field_name: str) -> str:
    return f"{model}.{field_name}"


def extract_backend_inventory(root: Path, backend_models: dict[str, str]) -> set[str]:
    keys: set[str] = set()
    for model_name, relative_path in sorted(backend_models.items()):
        source_path = root / relative_path
        try:
            source = source_path.read_text(encoding="utf-8")
        except FileNotFoundError as exc:
            raise InventoryError(f"{source_path}: configured backend source file does not exist") from exc

        fields = extract_rust_struct_fields(source, model_name)
        for field_name in fields:
            if field_name in IGNORED_BACKEND_FIELDS:
                continue
            keys.add(backend_key(model_name, field_name))
    return keys
```

- [ ] **Step 3: Change `validate_registry` signature**

Change:

```python
def validate_registry(root: Path = ROOT) -> ValidationResult:
```

to:

```python
def validate_registry(
    root: Path = ROOT,
    backend_models: dict[str, str] | None = None,
    validate_backend_inventory: bool = True,
) -> ValidationResult:
```

Inside the function, set:

```python
    if backend_models is None:
        backend_models = BACKEND_MODELS
```

- [ ] **Step 4: Compare extracted backend keys against registry keys**

After all rows have been read and `seen_backend` has been populated, add:

```python
    if validate_backend_inventory:
        try:
            source_backend = extract_backend_inventory(root, backend_models)
        except InventoryError as exc:
            result.add(str(exc))
            return result

        registry_backend = set(seen_backend)
        for key in sorted(source_backend - registry_backend):
            result.add(f"missing backend mapping: {key}")
        for key in sorted(registry_backend - source_backend):
            result.add(f"unknown backend mapping: {key}")
```

- [ ] **Step 5: Keep existing tests stable**

Existing temp-directory tests that do not create backend source should call:

```python
validate.validate_registry(root, validate_backend_inventory=False)
```

Use this only in tests for pure registry row validation. Do not disable backend inventory validation in the real CLI path.

- [ ] **Step 6: Run tests**

Run:

```sh
python3 -m unittest registry/tools/test_validate.py
```

Expected: PASS.

## Task 5: Decide Strictness Migration Before Enabling Real Backend Scope in CI

**Files:**
- Modify: `registry/tools/validate.py`
- Modify: `registry/sections/*.json`

- [ ] **Step 1: Run real backend validation and capture missing keys from stderr**

Run:

```sh
python3 registry/tools/validate.py --strict-backend-inventory
```

Expected: FAIL because the current seeded registry is not exhaustive.

- [ ] **Step 2: Classify each missing backend key**

For every `missing backend mapping: Model.field`:

- Add a row to the correct existing section JSON when the field is an E2BR3 business field.
- Add the exact field to `IGNORED_BACKEND_FIELDS` only when it is truly technical.
- Use `status: "frontend_missing"` when backend exists but frontend is not verified.
- Use `authority: "ICH"`, `"FDA"`, or `"MFDS"` only.
- Use an actual E2BR3 code if known.
- If the E2BR3 code is not known after source review, do not invent one. Use `status: "conflict"` with concrete evidence and an `action` saying the E2BR3 code must be assigned.

- [ ] **Step 3: Add rows section-by-section**

Recommended order:

1. `registry/sections/n-message-header.json`
2. `registry/sections/c-safety-report.json`
3. `registry/sections/d-patient.json`
4. `registry/sections/e-reaction.json`
5. `registry/sections/f-test.json`
6. `registry/sections/g-drug.json`
7. `registry/sections/h-narrative.json`

After each section, run:

```sh
python3 -m unittest registry/tools/test_validate.py
python3 registry/tools/validate.py --strict-backend-inventory
```

Expected during migration: tests pass, validator may still fail for remaining sections.

- [ ] **Step 4: Finish backend strict mode**

When all configured backend source keys are either mapped or explicitly ignored, run:

```sh
python3 -m unittest registry/tools/test_validate.py
python3 registry/tools/validate.py --strict-backend-inventory
```

Expected: PASS.

## Task 6: Add Frontend Inventory Extraction Tests

**Files:**
- Modify: `registry/tools/test_validate.py`

- [ ] **Step 1: Write minimal frontend extraction tests**

Add tests using temporary frontend source files with field declarations such as:

```tsx
register("patient.patientAgeGroup")
setValue("patient.patientInitial", value)
<input name="patient.patientSex" />
```

Expected extracted keys:

```python
{
    "patient.patientAgeGroup",
    "patient.patientInitial",
    "patient.patientSex",
}
```

- [ ] **Step 2: Run tests and verify they fail**

Run:

```sh
python3 -m unittest registry/tools/test_validate.py
```

Expected: FAIL because frontend extraction is not implemented yet.

## Task 7: Implement Frontend Inventory Validation

**Files:**
- Modify: `registry/tools/validate.py`

- [ ] **Step 1: Add frontend source configuration**

Add:

```python
FRONTEND_ROOT = Path("/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend")
FRONTEND_SOURCE_GLOBS = [
    "src/**/*.ts",
    "src/**/*.tsx",
]
```

- [ ] **Step 2: Add frontend extraction helpers**

Extract string literal field names from reliable constructs only:

```python
register("...")
setValue("...", ...)
name="..."
name={'...'}
```

Only keep keys that contain one dot and belong to a configured case section prefix. Do not scrape labels.

- [ ] **Step 3: Add frontend comparison**

Compare extracted frontend keys with mapped registry frontend keys:

```text
missing frontend mapping: section.fieldPath
unknown frontend mapping: section.fieldPath
```

- [ ] **Step 4: Run tests**

Run:

```sh
python3 -m unittest registry/tools/test_validate.py
```

Expected: PASS.

## Task 8: Migrate Frontend Rows Section-By-Section

**Files:**
- Modify: `registry/sections/*.json`

- [ ] **Step 1: Run real frontend validation**

Run:

```sh
python3 registry/tools/validate.py
```

Expected during migration: FAIL with missing or unknown frontend mappings.

- [ ] **Step 2: Fix registry rows only**

For every frontend missing/unknown error:

- Add or correct the row in the existing section JSON.
- Keep backend mapping unchanged when already verified.
- Use `backend_missing` only when the frontend exists but backend is genuinely unverified.
- Do not create frontend inventory files.

- [ ] **Step 3: Verify strict 1-1-1 registry**

Run:

```sh
python3 -m unittest registry/tools/test_validate.py
python3 registry/tools/validate.py
```

Expected: PASS.

## Task 9: Final Verification

**Files:**
- Inspect: `registry/`
- Inspect: `.github/workflows/ci.yml`

- [ ] **Step 1: Confirm no generated registry artifacts exist**

Run:

```sh
find registry -path '*/generated/*' -o -name '*inventory*.json' -o -name '*report*.md'
```

Expected: no output.

- [ ] **Step 2: Run registry verification**

Run:

```sh
python3 -m unittest registry/tools/test_validate.py
python3 registry/tools/validate.py
python3 registry/tools/validate.py --strict-backend-inventory
python3 -m json.tool registry/schema.json >/dev/null
python3 -m json.tool registry/index.json >/dev/null
for f in registry/sections/*.json; do python3 -m json.tool "$f" >/dev/null || exit 1; done
```

Expected: all commands pass.

- [ ] **Step 3: Check git diff for accidental unrelated edits**

Run:

```sh
git status --short
git diff -- registry .github/workflows/ci.yml
```

Expected: registry changes only, plus the intended CI registry job if already present.

## Self-Review Checklist

- [ ] The plan keeps the section JSON files as the only canonical matrix.
- [ ] The plan does not introduce generated committed outputs.
- [ ] Missing BMC fields are tracked in existing section JSON files, not hidden in a separate inventory.
- [ ] Unknown registry mappings fail validation.
- [ ] Backend extraction is implemented and migrated before frontend extraction.
- [ ] Frontend extraction is constrained to source-level field declarations.
- [ ] CI can run `python3 registry/tools/validate.py` after migration is complete.
