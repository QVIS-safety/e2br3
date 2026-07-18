# Presave Registry Reporter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the presave registry foundation and make reporter presave frontend, Rust persistence, nullFlavor constraints, and transfer to `PrimarySource` pass strict end-to-end validation.

**Architecture:** Keep case mappings in `registry/sections/` and add same-schema presave mappings under `registry/presaves/`. Join the two namespaces by `e2br3_code`; derive frontend, Rust, and transfer inventories from production source rather than committing a second matrix. This plan delivers the reporter slice only, leaving sender, receiver, product, study, and narrative population for a follow-on plan after the contract is proven.

**Tech Stack:** Python 3 standard library and `unittest`, JSON, Rust model source parsing, TypeScript/React Hook Form source parsing, Zod, Vitest/Jest-compatible frontend tests.

## Global Constraints

- Reuse `registry/schema.json` unchanged for every presave row.
- Preserve the existing `id`, `e2br3_code`, `backend`, `frontend`, `status`, and `local_only` semantics.
- Permit a code once in the case namespace and once in the presave namespace; reject duplicates within either namespace.
- Do not commit generated inventory JSON, reports, spreadsheets, or a hand-maintained transfer matrix.
- Prose evidence never proves a transfer; production source and executable contract tests do.
- Reporter is the only populated presave type in this plan.
- Do not enable all-six-type CI enforcement in this plan.

---

## File Structure

- Modify `registry/tools/extract_frontend_fields.py`: follow the current case-detail frontend layout and recognize field-path literals used through component configuration.
- Modify `registry/tools/test_extract_frontend_fields.py`: protect current-layout extraction and fail-closed glob behavior.
- Modify `registry/tools/test_validate.py`: restore the repository case frontend baseline and test presave CLI integration.
- Create `registry/tools/presave_registry.py`: load same-schema presave rows and expose deterministic namespace indexes.
- Create `registry/tools/test_presave_registry.py`: unit-test loading, duplicates, joins, and statuses.
- Create `registry/tools/extract_presave_fields.py`: derive reporter frontend, Rust backend, nullFlavor, and transfer inventories from production source.
- Create `registry/tools/test_extract_presave_fields.py`: unit-test every extractor and reporter production coverage.
- Modify `registry/tools/validate.py`: add strict presave registry and inventory modes without changing normal case validation behavior.
- Create `registry/presaves/index.json`: list the reporter section only during this phase.
- Create `registry/presaves/sections/c-reporter.json`: canonical reporter presave rows.
- Modify `registry/sections/c-safety-report.json`: add the two missing dedicated case nullFlavor companion rows needed by reporter joins.
- Modify `../frontend/E2BR3-frontend/lib/types/presave.ts`: add the missing reporter country nullFlavor field.
- Modify `../frontend/E2BR3-frontend/lib/schemas/presave.ts`: constrain reporter nullFlavor values explicitly.
- Modify `../frontend/E2BR3-frontend/components/presave/ReporterForm.tsx`: expose the dedicated reporter-country nullFlavor input.
- Modify `../frontend/E2BR3-frontend/lib/presave/canonicalMappers.ts`: read the country nullFlavor API field.
- Modify `../frontend/E2BR3-frontend/lib/presave/canonicalWriteMappers.ts`: write the country nullFlavor API field.
- Modify `../frontend/E2BR3-frontend/app/(protected)/[authority]/case/[id]/detail/RP/model/rpModel.ts`: transfer country nullFlavor to the case model.
- Modify `../frontend/E2BR3-frontend/__tests__/dashboard/presave-minimal-form-validation.test.ts`: execute reporter nullFlavor transfer behavior.
- Modify `registry/README.md` and `registry/SPEC.md`: document the presave namespace and strict commands.
- Modify `.github/workflows/ci.yml`: run reporter presave strict validation after both repositories are available.

---

### Task 1: Restore the Case Frontend Inventory Baseline

**Files:**
- Modify: `registry/tools/extract_frontend_fields.py`
- Modify: `registry/tools/test_extract_frontend_fields.py`
- Modify: `registry/tools/test_validate.py`

**Interfaces:**
- Consumes: existing `extract_frontend_fields(root, source_globs)` and `FrontendField`.
- Produces: `DEFAULT_SOURCE_GLOBS` matching current case-detail TSX files and `extract_literal_field_paths(source: str) -> set[str]`.

- [ ] **Step 1: Write failing tests for the current frontend layout and configured field literals**

Add these tests to `registry/tools/test_extract_frontend_fields.py`:

```python
def test_extracts_configured_and_set_value_field_paths(self):
    source = '''
const fields = [{ name: "patientInformation.gpMedicalRecordNumber" }];
setValue(`reactions.${index}.seriousness.criteriaResultsInDeath`, true);
<Input name={`drugs.${drugIndex}.cumulativeDoseValue`} />
'''
    self.assertEqual(
        [
            "drugs.cumulativeDoseValue",
            "patientInformation.gpMedicalRecordNumber",
            "reactions.seriousness.criteriaResultsInDeath",
        ],
        extractor.extract_field_paths_from_source(source),
    )

def test_default_glob_targets_current_case_detail_tree(self):
    self.assertEqual(
        ["../frontend/E2BR3-frontend/app/(protected)/*/case/*/detail/**/*.tsx"],
        extractor.DEFAULT_SOURCE_GLOBS,
    )
```

- [ ] **Step 2: Run the extractor tests and confirm failure**

Run:

```sh
python3 -m unittest registry/tools/test_extract_frontend_fields.py
```

Expected: FAIL because object-literal/setValue paths are not extracted and the default glob still points at removed `Section*.tsx` files.

- [ ] **Step 3: Implement literal field-path extraction and replace the stale glob**

In `registry/tools/extract_frontend_fields.py`, replace `DEFAULT_SOURCE_GLOBS` and extend raw extraction:

```python
DEFAULT_SOURCE_GLOBS = [
    "../frontend/E2BR3-frontend/app/(protected)/*/case/*/detail/**/*.tsx",
]

QUOTED_FIELD_PATH = re.compile(
    r'''["'`]((?:caseSummaryInformation|drugReactionAssessments|drugs|'''
    r'''literatureReferences|messageHeader|narrative|patientInformation|'''
    r'''primarySources|reactions|safetyReportIdentification|studyInformation|'''
    r'''testResults)\.[^"'`\s,)]+)["'`]'''
)

def extract_literal_field_paths(source: str) -> set[str]:
    return {match.group(1) for match in QUOTED_FIELD_PATH.finditer(source)}
```

At the end of `extract_raw_field_paths_from_source`, add:

```python
fields.update(extract_literal_field_paths(source))
```

- [ ] **Step 4: Run unit and repository frontend validation**

Run:

```sh
python3 -m unittest registry/tools/test_extract_frontend_fields.py
python3 registry/tools/validate.py --strict-frontend-inventory
python3 -m unittest registry/tools/test_validate.py
```

Expected: all commands PASS. If the repository comparison reports real field additions/removals, update the owning existing `registry/sections/*.json` row to the current production path; do not weaken extraction or add ignore entries.

- [ ] **Step 5: Commit the restored baseline**

```sh
git add registry/tools/extract_frontend_fields.py registry/tools/test_extract_frontend_fields.py registry/tools/test_validate.py registry/sections
git commit -m "fix: track current case frontend fields"
```

---

### Task 2: Add the Same-Schema Presave Namespace Loader

**Files:**
- Create: `registry/tools/presave_registry.py`
- Create: `registry/tools/test_presave_registry.py`
- Create: `registry/presaves/index.json`
- Create: `registry/presaves/sections/c-reporter.json`

**Interfaces:**
- Consumes: `validate.load_json`, `validate.validate_row`, and case row dictionaries.
- Produces: `PresaveRegistry(rows, by_code, backend_keys, frontend_keys)` and `load_presave_registry(root, result)`.

- [ ] **Step 1: Write failing loader and duplicate tests**

Create `registry/tools/test_presave_registry.py` with temporary fixtures asserting:

```python
def test_loads_same_code_in_case_and_presave_namespaces(self):
    loaded = presave_registry.load_presave_registry(self.root, self.result)
    self.assertEqual("ReporterPresave.reporter_given_name", loaded.backend_keys["C.2.r.1.2"])
    self.assertEqual([], self.result.errors)

def test_rejects_duplicate_code_inside_presave_namespace(self):
    presave_registry.load_presave_registry(self.root, self.result)
    self.assertIn("duplicate presave e2br3_code C.2.r.1.2", "\n".join(self.result.errors))
```

Use a complete valid row in the fixture with `backend.model = "ReporterPresave"`, `backend.field = "reporter_given_name"`, `frontend.section = "reporter"`, and `frontend.field = "reporterGivenName"`.

- [ ] **Step 2: Run the loader tests and confirm import failure**

```sh
python3 -m unittest registry/tools/test_presave_registry.py
```

Expected: FAIL with `ModuleNotFoundError: No module named 'presave_registry'`.

- [ ] **Step 3: Implement the loader**

Create `registry/tools/presave_registry.py` with this public shape:

```python
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import validate

@dataclass(frozen=True)
class PresaveRegistry:
    rows: tuple[dict[str, Any], ...]
    by_code: dict[str, dict[str, Any]]
    backend_keys: dict[str, str]
    frontend_keys: dict[str, str]

def load_presave_registry(root: Path, result: validate.ValidationResult) -> PresaveRegistry:
    index_path = root / "presaves/index.json"
    index = validate.load_json(index_path, result)
    rows: list[dict[str, Any]] = []
    by_code: dict[str, dict[str, Any]] = {}
    backend_keys: dict[str, str] = {}
    frontend_keys: dict[str, str] = {}
    if not isinstance(index, dict) or not isinstance(index.get("sections"), list):
        result.add(f"{index_path}: sections must be a non-empty list")
        return PresaveRegistry((), {}, {}, {})
    for relative in index["sections"]:
        source = root / "presaves" / relative
        payload = validate.load_json(source, result)
        if not isinstance(payload, list):
            result.add(f"{source}: section file must contain a JSON array")
            continue
        for row in payload:
            validate.validate_row(row, source, result)
            if not isinstance(row, dict) or not isinstance(row.get("e2br3_code"), str):
                continue
            code = row["e2br3_code"]
            if code in by_code:
                result.add(f"{row.get('id')}: duplicate presave e2br3_code {code}")
                continue
            by_code[code] = row
            rows.append(row)
            backend = row.get("backend", {})
            frontend = row.get("frontend", {})
            if backend.get("status") == "mapped":
                backend_keys[code] = f"{backend['model']}.{backend['field']}"
            if frontend.get("status") == "mapped":
                frontend_keys[code] = f"{frontend['section']}.{frontend['field']}"
    return PresaveRegistry(tuple(rows), by_code, backend_keys, frontend_keys)
```

- [ ] **Step 4: Seed an empty reporter section and run tests**

Create `registry/presaves/index.json`:

```json
{"sections":["sections/c-reporter.json"]}
```

Create `registry/presaves/sections/c-reporter.json` as `[]`. Run:

```sh
python3 -m unittest registry/tools/test_presave_registry.py
```

Expected: PASS.

- [ ] **Step 5: Commit the namespace loader**

```sh
git add registry/tools/presave_registry.py registry/tools/test_presave_registry.py registry/presaves
git commit -m "feat: add presave registry namespace"
```

---

### Task 3: Extract Reporter Frontend and Rust Inventories

**Files:**
- Create: `registry/tools/extract_presave_fields.py`
- Create: `registry/tools/test_extract_presave_fields.py`

**Interfaces:**
- Consumes: reporter form/type files and `ReporterPresave` Rust source.
- Produces: `extract_reporter_frontend(root) -> set[str]`, `extract_presave_backend(root, models) -> set[str]`, and `REPORTER_BACKEND_MODELS`.

- [ ] **Step 1: Write failing source-extraction tests**

Create tests that expect these normalized keys from minimal source fixtures:

```python
self.assertEqual(
    {"reporter.reporterGivenName", "reporter.reporterCountryNullFlavor"},
    extractor.extract_presave_frontend_source(source, "reporter"),
)
self.assertEqual(
    {"ReporterPresave.reporter_given_name", "ReporterPresave.country_code_null_flavor"},
    extractor.extract_rust_presave_source(rust, "ReporterPresave"),
)
```

The Rust fixture must also contain `id`, `organization_id`, `deleted`, and audit fields and assert they are excluded.

- [ ] **Step 2: Run tests and confirm module import failure**

```sh
python3 -m unittest registry/tools/test_extract_presave_fields.py
```

Expected: FAIL with `ModuleNotFoundError`.

- [ ] **Step 3: Implement focused reporter extraction**

Create `registry/tools/extract_presave_fields.py` with:

```python
REPORTER_FRONTEND_FILES = (
    "../frontend/E2BR3-frontend/components/presave/ReporterForm.tsx",
    "../frontend/E2BR3-frontend/lib/types/presave.ts",
)
REPORTER_BACKEND_MODELS = {
    "ReporterPresave": "crates/libs/lib-core/src/model/presave.rs",
}
TECHNICAL_FIELDS = {
    "id", "organization_id", "deleted", "created_at", "updated_at",
    "created_by", "updated_by",
}

def extract_rust_presave_source(source: str, model: str) -> set[str]:
    return {
        f"{model}.{field}"
        for field in validate.extract_rust_struct_fields(source, model)
        if field not in TECHNICAL_FIELDS
    }

def extract_presave_frontend_source(source: str, section: str) -> set[str]:
    names = set(re.findall(r"(?:register\(|name\s*=\s*)[^\n]*?[\"'`]([A-Za-z][A-Za-z0-9.]*)", source))
    names.update(re.findall(r"^\s{2}([A-Za-z][A-Za-z0-9]+)\??:\s", source, re.MULTILINE))
    return {f"{section}.{name}" for name in names if name not in {"id", "deleted"}}
```

Add repository wrapper functions that resolve paths from `root`, fail closed on missing files, and union deterministic sets.

- [ ] **Step 4: Add repository coverage assertions for fields that already exist and run tests**

Assert repository inventories contain:

```python
assert "reporter.reporterNameNullFlavor" in frontend
assert "ReporterPresave.reporter_name_null_flavor" in backend
assert "ReporterPresave.country_code_null_flavor" in backend
assert "ReporterPresave.qualification_null_flavor" in backend
```

Run:

```sh
python3 -m unittest registry/tools/test_extract_presave_fields.py
```

Expected: PASS. The backend country nullFlavor exists already; the missing
frontend country nullFlavor is added with its failing repository assertion in
Task 4.

- [ ] **Step 5: Commit extractor infrastructure**

```sh
git add registry/tools/extract_presave_fields.py registry/tools/test_extract_presave_fields.py
git commit -m "feat: extract presave field inventories"
```

---

### Task 4: Complete Reporter Country NullFlavor End to End

**Files:**
- Modify: `../frontend/E2BR3-frontend/lib/types/presave.ts`
- Modify: `../frontend/E2BR3-frontend/lib/schemas/presave.ts`
- Modify: `../frontend/E2BR3-frontend/components/presave/ReporterForm.tsx`
- Modify: `../frontend/E2BR3-frontend/lib/presave/canonicalMappers.ts`
- Modify: `../frontend/E2BR3-frontend/lib/presave/canonicalWriteMappers.ts`
- Modify: `../frontend/E2BR3-frontend/app/(protected)/[authority]/case/[id]/detail/RP/model/rpModel.ts`
- Modify: `../frontend/E2BR3-frontend/__tests__/dashboard/presave-minimal-form-validation.test.ts`
- Modify: `registry/tools/test_extract_presave_fields.py`

**Interfaces:**
- Consumes: backend `ReporterPresave.country_code_null_flavor` and case `PrimarySource.country_code_null_flavor`.
- Produces: `ReporterPresaveData.reporterCountryNullFlavor` and transfer target `PrimarySources.reporterCountryNullFlavor`.

- [ ] **Step 1: Add failing frontend behavior tests**

Add assertions that parsing accepts only `MSK`, `ASKU`, `NASK`, and `UNK`, canonical API input reads `country_code_null_flavor`, the write mapper emits `country_code_null_flavor`, and transfer returns `reporterCountryNullFlavor`:

```typescript
expect(reporterPresaveSchema.parse({
  reporterGivenName: "A",
  reporterOrganization: "Org",
  qualification: "1",
  reporterCountryNullFlavor: "ASKU",
}).reporterCountryNullFlavor).toBe("ASKU");

expect(reporterPresaveToPrimarySource({
  reporterGivenName: "A",
  reporterOrganization: "Org",
  qualification: "1",
  reporterCountryNullFlavor: "NASK",
}, false).reporterCountryNullFlavor).toBe("NASK");
```

Also add this repository assertion to
`registry/tools/test_extract_presave_fields.py`:

```python
self.assertIn("reporter.reporterCountryNullFlavor", frontend)
```

- [ ] **Step 2: Run focused frontend tests and confirm failure**

From `../frontend/E2BR3-frontend`, run:

```sh
npx vitest run __tests__/dashboard/presave-minimal-form-validation.test.ts __tests__/dashboard/canonical-presave-mappers.test.ts
```

Expected: FAIL because `reporterCountryNullFlavor` is absent.

- [ ] **Step 3: Implement the typed field, schema, form, mappers, and transfer**

Use one shared schema constant:

```typescript
const reporterCountryNullFlavorSchema = z.enum(["MSK", "ASKU", "NASK", "UNK"]);
```

Add `reporterCountryNullFlavor?: "MSK" | "ASKU" | "NASK" | "UNK"` to `ReporterPresaveData`; add the optional schema field; render the existing ReporterForm nullFlavor control beside reporter country; map aliases `reporterCountryNullFlavor` and `country_code_null_flavor`; write `country_code_null_flavor`; and add:

```typescript
reporterCountryNullFlavor: toNullFlavor(data.reporterCountryNullFlavor),
```

to `reporterPresaveToPrimarySource`.

- [ ] **Step 4: Run frontend and extractor tests**

```sh
cd ../frontend/E2BR3-frontend
npx vitest run __tests__/dashboard/presave-minimal-form-validation.test.ts __tests__/dashboard/canonical-presave-mappers.test.ts
cd ../../e2br3/e2br3
python3 -m unittest registry/tools/test_extract_presave_fields.py
```

Expected: PASS.

- [ ] **Step 5: Commit frontend and backend tests in their owning repositories**

```sh
git -C ../frontend/E2BR3-frontend add lib/types/presave.ts lib/schemas/presave.ts components/presave/ReporterForm.tsx lib/presave/canonicalMappers.ts lib/presave/canonicalWriteMappers.ts 'app/(protected)/[authority]/case/[id]/detail/RP/model/rpModel.ts' __tests__/dashboard/presave-minimal-form-validation.test.ts
git -C ../frontend/E2BR3-frontend commit -m "fix: preserve reporter country null flavor"
git add registry/tools/test_extract_presave_fields.py
git commit -m "test: require reporter country null flavor inventory"
```

---

### Task 5: Extract and Verify Reporter-to-Case Transfers

**Files:**
- Modify: `registry/tools/extract_presave_fields.py`
- Modify: `registry/tools/test_extract_presave_fields.py`

**Interfaces:**
- Consumes: production `reporterPresaveToPrimarySource` object construction.
- Produces: `extract_reporter_transfers(root) -> set[tuple[str, str]]` with `ReporterPresave.field -> PrimarySource.field` pairs.

- [ ] **Step 1: Write failing rename and nullFlavor transfer tests**

Use a TypeScript fixture containing the production object-construction form and assert:

```python
self.assertEqual(
    {
        ("ReporterPresave.reporter_given_name", "PrimarySource.reporter_given_name"),
        ("ReporterPresave.country_code_null_flavor", "PrimarySource.country_code_null_flavor"),
    },
    extractor.extract_reporter_transfer_source(source),
)
```

Also assert the repository transfer inventory includes every registry-mapped reporter field except explicitly non-transferable fields.

- [ ] **Step 2: Run tests and confirm missing function failure**

```sh
python3 -m unittest registry/tools/test_extract_presave_fields.py
```

Expected: FAIL because `extract_reporter_transfer_source` is absent.

- [ ] **Step 3: Implement explicit frontend-to-Rust name maps and object transfer parsing**

Add immutable name maps used only to normalize language naming, not to declare regulatory mappings:

```python
REPORTER_FRONTEND_TO_BACKEND = {
    "reporterGivenName": "reporter_given_name",
    "reporterCountry": "country_code",
    "reporterCountryNullFlavor": "country_code_null_flavor",
    "reporterNameNullFlavor": "reporter_name_null_flavor",
    "reporterAddressNullFlavor": "reporter_address_null_flavor",
    "qualificationNullFlavor": "qualification_null_flavor",
}
PRIMARY_SOURCE_FRONTEND_TO_BACKEND = {
    "reporterGivenName": "reporter_given_name",
    "reporterCountry": "country_code",
    "reporterCountryNullFlavor": "country_code_null_flavor",
    "reporterNameNullFlavor": "reporter_name_null_flavor",
    "reporterAddressNullFlavor": "reporter_address_null_flavor",
    "qualificationNullFlavor": "qualification_null_flavor",
}
```

Fill the remaining direct camelCase/snake_case reporter fields programmatically, parse `target: wrapper(data.source)` entries inside the returned object, and normalize them through these language maps.

- [ ] **Step 4: Run extractor tests**

```sh
python3 -m unittest registry/tools/test_extract_presave_fields.py
```

Expected: PASS, including repository production transfer coverage.

- [ ] **Step 5: Commit transfer extraction**

```sh
git add registry/tools/extract_presave_fields.py registry/tools/test_extract_presave_fields.py
git commit -m "feat: extract reporter presave transfers"
```

---

### Task 6: Integrate Presave Validation and CLI Modes

**Files:**
- Modify: `registry/tools/validate.py`
- Modify: `registry/tools/test_validate.py`
- Modify: `registry/tools/test_presave_registry.py`

**Interfaces:**
- Consumes: `load_presave_registry`, reporter inventories, case rows indexed by code.
- Produces: `validate_presave_registry(root, result, validate_inventory)` and CLI flags `--strict-presave-registry`, `--strict-presave-inventory`.

- [ ] **Step 1: Write failing join, missing inventory, wrong target, and CLI tests**

Cover these deterministic messages:

```python
"missing case registry join: C.2.r.1.2"
"missing presave frontend mapping: reporter.reporterGivenName"
"missing presave backend mapping: ReporterPresave.reporter_given_name"
"missing presave-to-case assignment: ReporterPresave.reporter_given_name -> PrimarySource.reporter_given_name"
"wrong presave-to-case target: ReporterPresave.country_code -> PrimarySource.organization; expected PrimarySource.country_code"
```

Assert `--strict-presave-inventory` implies structural presave validation.

- [ ] **Step 2: Run validator tests and confirm failure**

```sh
python3 -m unittest registry/tools/test_validate.py registry/tools/test_presave_registry.py
```

Expected: FAIL because the validator has no presave modes.

- [ ] **Step 3: Implement presave join and inventory comparison**

Add parameters to `validate_registry`:

```python
validate_presave_registry_rows: bool = False,
validate_presave_inventory: bool = False,
```

Load case rows into `case_rows_by_code`. When either presave flag is enabled, load the presave namespace. For each transferable row, require a joined case row with mapped backend. When inventory is enabled, compare exact frontend/backend sets and normalized transfer pairs. Skip case joins only for rows whose row status is `not_applicable` and `local_only is True`.

Add CLI parsing:

```python
strict_presave_registry = "--strict-presave-registry" in sys.argv[1:]
strict_presave_inventory = "--strict-presave-inventory" in sys.argv[1:]
```

Pass `validate_presave_registry_rows=strict_presave_registry or strict_presave_inventory` and `validate_presave_inventory=strict_presave_inventory`.

- [ ] **Step 4: Run validator suites**

```sh
python3 -m unittest registry/tools/test_validate.py registry/tools/test_presave_registry.py registry/tools/test_extract_presave_fields.py
```

Expected: PASS for fixtures; repository strict presave inventory remains red until rows are populated in Task 7.

- [ ] **Step 5: Commit validator integration**

```sh
git add registry/tools/validate.py registry/tools/test_validate.py registry/tools/test_presave_registry.py
git commit -m "feat: validate presave registry coverage"
```

---

### Task 7: Populate Reporter Rows and Missing Case NullFlavor Joins

**Files:**
- Modify: `registry/presaves/sections/c-reporter.json`
- Modify: `registry/sections/c-safety-report.json`
- Modify: `registry/tools/test_validate.py`

**Interfaces:**
- Consumes: reporter frontend/backend/transfer inventories and current case row schema.
- Produces: complete reporter strict registry and inventory coverage.

- [ ] **Step 1: Add repository strict reporter test**

Add:

```python
def test_repository_reporter_presave_inventory_is_complete(self):
    result = validate.validate_registry(
        validate_backend_inventory=False,
        validate_presave_registry_rows=True,
        validate_presave_inventory=True,
    )
    self.assertEqual([], result.errors)
```

- [ ] **Step 2: Run the test and capture missing rows**

```sh
python3 -m unittest registry.tools.test_validate.RegistryValidatorTests.test_repository_reporter_presave_inventory_is_complete
```

Expected: FAIL with deterministic missing presave mappings and missing joins for reporter name/qualification nullFlavor.

- [ ] **Step 3: Add the complete reporter mapping set**

Populate `c-reporter.json` with one complete row for each pair below, copying label, section, authority, and code semantics from the joined case row:

```text
C.2.r.1.1  ReporterPresave.reporter_title                 reporter.reporterTitle
C.2.r.1.2  ReporterPresave.reporter_given_name            reporter.reporterGivenName
C.2.r.1.3  ReporterPresave.reporter_middle_name           reporter.reporterMiddleName
C.2.r.1.4  ReporterPresave.reporter_family_name           reporter.reporterFamilyName
C.2.r.2.1  ReporterPresave.organization                   reporter.reporterOrganization
C.2.r.2.2  ReporterPresave.department                     reporter.reporterDepartment
C.2.r.2.3  ReporterPresave.street                         reporter.reporterStreet
C.2.r.2.4  ReporterPresave.city                           reporter.reporterCity
C.2.r.2.5  ReporterPresave.state                          reporter.reporterState
C.2.r.2.6  ReporterPresave.postcode                       reporter.reporterPostcode
C.2.r.2.7  ReporterPresave.telephone                      reporter.reporterTelephone
C.2.r.3    ReporterPresave.country_code                   reporter.reporterCountry
C.2.r.4    ReporterPresave.qualification                  reporter.qualification
C.2.r.4.KR.1 ReporterPresave.qualification_kr1            reporter.qualificationKr1
C.2.r.5    ReporterPresave.primary_source_regulatory      reporter.primarySourceForRegulatoryPurposes
C.2.r.local.reporterNameNullFlavor ReporterPresave.reporter_name_null_flavor reporter.reporterNameNullFlavor
C.2.r.local.reporterAddressNullFlavor ReporterPresave.reporter_address_null_flavor reporter.reporterAddressNullFlavor
C.2.r.local.reporterCountryNullFlavor ReporterPresave.country_code_null_flavor reporter.reporterCountryNullFlavor
C.2.r.local.qualificationNullFlavor ReporterPresave.qualification_null_flavor reporter.qualificationNullFlavor
```

Every row has `status: "complete"`, mapped backend/frontend evidence pointing at production files, and `local_only: true` only for the four nullFlavor companion rows.

Add matching complete local-only case rows for `C.2.r.local.reporterNameNullFlavor -> PrimarySource.reporter_name_null_flavor -> primarySources.reporterNameNullFlavor` and `C.2.r.local.qualificationNullFlavor -> PrimarySource.qualification_null_flavor -> primarySources.qualificationNullFlavor`.

- [ ] **Step 4: Run strict validation**

```sh
python3 registry/tools/validate.py --strict-presave-registry
python3 registry/tools/validate.py --strict-presave-inventory
python3 registry/tools/validate.py --strict-backend-inventory
python3 registry/tools/validate.py --strict-frontend-inventory
```

Expected: all four commands PASS.

- [ ] **Step 5: Commit canonical reporter rows**

```sh
git add registry/presaves/sections/c-reporter.json registry/sections/c-safety-report.json registry/tools/test_validate.py
git commit -m "feat: register reporter presave fields"
```

---

### Task 8: Document and Enforce the Reporter Milestone

**Files:**
- Modify: `registry/README.md`
- Modify: `registry/SPEC.md`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: both strict presave CLI flags.
- Produces: documented commands and CI enforcement for the configured reporter slice.

- [ ] **Step 1: Add a failing CI/documentation contract test**

In `registry/tools/test_validate.py`, add:

```python
def test_ci_runs_strict_presave_inventory(self):
    workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
    self.assertIn("python3 registry/tools/validate.py --strict-presave-inventory", workflow)
```

- [ ] **Step 2: Run the test and confirm failure**

```sh
python3 -m unittest registry.tools.test_validate.RegistryValidatorTests.test_ci_runs_strict_presave_inventory
```

Expected: FAIL because CI does not invoke the new mode.

- [ ] **Step 3: Update documentation and CI**

Document `registry/presaves/`, same-schema code joins, reporter-only configured scope, both commands, and the distinction between dedicated and in-band nullFlavor fields. Add this command to the registry CI job after frontend checkout/setup:

```sh
python3 registry/tools/validate.py --strict-presave-inventory
```

Do not claim all six presave types are covered; name reporter as the current strict scope and list the five follow-on types.

- [ ] **Step 4: Run the full verification suite**

```sh
python3 -m unittest registry/tools/test_extract_frontend_fields.py registry/tools/test_extract_presave_fields.py registry/tools/test_presave_registry.py registry/tools/test_validate.py
python3 registry/tools/validate.py
python3 registry/tools/validate.py --strict-backend-inventory
python3 registry/tools/validate.py --strict-frontend-inventory
python3 registry/tools/validate.py --strict-dictionary
python3 registry/tools/validate.py --strict-presave-registry
python3 registry/tools/validate.py --strict-presave-inventory
git diff --check
```

Expected: every command PASS and `git diff --check` prints nothing.

- [ ] **Step 5: Commit reporter CI enforcement**

```sh
git add registry/README.md registry/SPEC.md .github/workflows/ci.yml registry/tools/test_validate.py
git commit -m "ci: enforce reporter presave registry"
```

---

## Follow-on Plan Boundary

After this plan is green, write a separate expansion plan for sender, receiver,
product, study, and narrative. That plan reuses the loader, extractors, join
rules, transfer contract, diagnostics, and CI mode created here. It must add one
presave section at a time and keep strict validation green after each type.
