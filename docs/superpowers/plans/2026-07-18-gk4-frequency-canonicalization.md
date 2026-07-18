# G.k.4.r.2/G.k.4.r.3 Frequency Canonicalization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the invented dosage `frequency_value` field, make `number_of_units` and `frequency_unit` the sole G.k.4.r.2/G.k.4.r.3 pair, validate it through shared rule tables, and render G.k.4.r.3 as a nine-option searchable autocomplete.

**Architecture:** The backend persists one numeric column and one unit column, maps them to `period/@value` and `period/@unit`, and uses the existing `NestedConstraintRule`/`eval_nested_constraints()` pipeline for terminology validation. The sibling frontend stores `numberOfUnits` and `frequencyUnit`, uses the existing `FormAutocomplete`, and keeps the static field-specific options separate from the general UCUM endpoint.

**Tech Stack:** Rust, SQLx/PostgreSQL bootstrap SQL, libxml, shared validator rule tables, Python registry validation, Next.js 15, React Hook Form, Zod 4, Jest/Testing Library.

## Global Constraints

- No database migration or compatibility alias; the environment will be reinitialized.
- No field-specific backend validator function, direct validation branch, or duplicate backend code set.
- Backend allowed-value validation uses the active `ICH-UCUM / frequency` terminology release and fails closed without it.
- G.k.4.r.3 exposes exactly `a`, `mo`, `wk`, `d`, `h`, `min`, `{cyclical}`, `{asnecessary}`, and `{total}`.
- Do not change G.k.4.r.1, G.k.4.r.6, unrelated DG layout, or the general UCUM service.
- Preserve unrelated untracked files in both repositories.

## Repository Boundaries

- Backend repository: `/Users/hyundonghoon/projects/rust/e2br3/e2br3`
- Frontend repository: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend`
- Commit backend and frontend changes independently because they are separate Git repositories.

---

### Task 1: Canonical XML value/unit mapping

**Files:**
- Modify: `crates/libs/lib-core/src/xml/import_sections/g_drug.rs`
- Modify: `crates/libs/lib-core/src/xml/import_runtime/helpers/g.rs`
- Modify: `crates/libs/lib-core/src/xml/import_runtime/g.rs`
- Modify: `crates/libs/lib-core/src/xml/export/sections/g.rs`
- Test: `crates/libs/lib-core/src/xml/export/sections/g.rs`
- Test: `crates/libs/lib-core/tests/import/g.rs`
- Modify: `crates/libs/lib-core/tests/import.rs`

**Interfaces:**
- Consumes: `GDrugPaths::DOSAGE_FREQUENCY_VALUE` and `GDrugPaths::DOSAGE_FREQUENCY_UNIT`.
- Produces: import structs with only `number_of_units: Option<i32>` and `frequency_unit: Option<String>`; export reads the same two fields.

- [ ] **Step 1: Add a failing export regression test**

In the existing `xml::export::sections::g` test module, extend `test_dosage()` and add:

```rust
#[test]
fn export_g_uses_number_of_units_for_period_value() {
    let case_id = Uuid::new_v4();
    let drug_id = Uuid::new_v4();
    let drug = test_drug(drug_id, case_id);
    let mut dosage = test_dosage(drug_id);
    dosage.number_of_units = Some(3);
    dosage.frequency_unit = Some("d".to_string());

    let xml = export_g_drugs_xml(&[drug], &[], &[dosage], &[], &[], &[], &[])
        .expect("export xml");

    assert!(xml.contains("<period value=\"3\" unit=\"d\"/>"), "{xml}");
}
```

- [ ] **Step 2: Run the export test and verify RED**

Run: `cargo test -p lib-core export_g_uses_number_of_units_for_period_value --lib -- --nocapture`

Expected: FAIL because current export emits `unit="d"` without `value="3"` when `frequency_value` is empty.

- [ ] **Step 3: Add a failing special-value import regression**

Enable `common` and `g` in `crates/libs/lib-core/tests/import.rs`. In
`crates/libs/lib-core/tests/import/g.rs`, keep the existing scenario assertion
but delete the duplicate `first_dosage.frequency_value` assertion. Add:

```rust
#[test]
fn import_g_preserves_special_frequency_unit_for_validation() {
    let xml = fixture("FAERS2022Scenario6.xml").replacen(
        "<period value=\"10\" unit=\"d\"/>",
        "<period value=\"10\" unit=\"{cyclical}\"/>",
        1,
    );
    let drugs = parse_g_drugs(&xml).expect("parse");
    let dosage = &drugs[0].dosages[0];

    assert_eq!(dosage.number_of_units, Some(10));
    assert_eq!(dosage.frequency_unit.as_deref(), Some("{cyclical}"));
}
```

- [ ] **Step 4: Run the import test and verify RED**

Run: `cargo test -p lib-core --test import import_g_section_all_fields_from_scenario6 -- --nocapture`

Then run: `cargo test -p lib-core --test import import_g_preserves_special_frequency_unit_for_validation -- --nocapture`

Expected: the special-value test FAILS because `normalize_code3()` currently drops `{cyclical}`.

- [ ] **Step 5: Implement the canonical mapping**

In import code, remove every `frequency_value` member and assignment. Keep only:

```rust
let number_of_units =
    first_attr(&mut xpath, &dose, GDrugPaths::DOSAGE_FREQUENCY_VALUE)
        .and_then(|v| v.parse::<i32>().ok());
let frequency_unit = normalize_frequency_unit(first_attr(
    &mut xpath,
    &dose,
    GDrugPaths::DOSAGE_FREQUENCY_UNIT,
));
```

Use a dedicated trim-only helper that preserves any non-empty value up to 50
characters; the shared validator, not the XML parser, decides vocabulary
membership:

```rust
fn normalize_frequency_unit(value: Option<String>) -> Option<String> {
    let value = value?.trim().to_string();
    (!value.is_empty() && value.chars().count() <= 50).then_some(value)
}
```

In export code, replace the current frequency block with:

```rust
if dose.number_of_units.is_some() || dose.frequency_unit.is_some() {
    out.push_str("<effectiveTime xsi:type=\"SXPR_TS\"><comp xsi:type=\"PIVL_TS\"><period");
    if let Some(v) = dose.number_of_units {
        out.push_str(" value=\"");
        out.push_str(&v.to_string());
        out.push_str("\"");
    }
    if let Some(u) = dose.frequency_unit.as_deref() {
        out.push_str(" unit=\"");
        out.push_str(&xml_escape(u));
        out.push_str("\"");
    }
    out.push_str("/></comp></effectiveTime>");
}
```

- [ ] **Step 6: Verify GREEN**

Run:

```bash
cargo test -p lib-core export_g_uses_number_of_units_for_period_value --lib
cargo test -p lib-core --test import import_g_section_all_fields_from_scenario6
cargo test -p lib-core --test import import_g_preserves_special_frequency_unit_for_validation
```

Expected: both targeted tests PASS.

- [ ] **Step 7: Commit backend XML mapping**

```bash
git add crates/libs/lib-core/src/xml crates/libs/lib-core/tests/import.rs crates/libs/lib-core/tests/import/g.rs
git commit -m "fix: canonicalize dosage interval xml mapping"
```

---

### Task 2: Remove the duplicate backend persistence field

**Files:**
- Modify: `db/bootstrap/07-drug-information.sql`
- Modify: `db/seed/001-demo-seed.sql`
- Modify: `db/seed/002-rich-demo-case.sql`
- Modify: `crates/libs/lib-core/src/model/drug.rs`
- Modify: `crates/services/web-server/tests/api/subresources_web.rs`
- Modify: `crates/services/web-server/tests/api/export_contract_web.rs`
- Modify: `crates/services/web-server/src/web/rest/cioms_export_rest/tests.rs`
- Modify: `crates/libs/lib-core/tests/xml/xml_export_g.rs`
- Modify: `crates/libs/validator/src/case/sections/g.rs`

**Interfaces:**
- Consumes: canonical `number_of_units` and `frequency_unit` established in Task 1.
- Produces: `DosageInformation`, `DosageInformationForCreate`, and `DosageInformationForUpdate` without `frequency_value`.

- [ ] **Step 1: Add API and companion-rule regressions before removing the field**

Change the dosage POST fixture in `subresources_web.rs` to send only:

```rust
"number_of_units": 1,
"frequency_unit": "d",
```

Add assertions:

```rust
assert_eq!(value["data"]["number_of_units"], 1);
assert_eq!(value["data"]["frequency_unit"], "d");
assert!(value["data"].get("frequency_value").is_none());
```

In the existing G validator test module, add:

```rust
#[test]
fn dosage_frequency_unit_is_required_from_number_of_units() {
    let mut ctx = empty_ctx();
    let mut parent = drug();
    parent.id = Uuid::from_u128(1);
    ctx.drugs.push(parent);
    let mut row = dosage();
    row.drug_id = Uuid::from_u128(1);
    row.number_of_units = Some(3);
    ctx.dosages.push(row);

    let mut issues = Vec::new();
    collect_ich_issues(&ctx, &mut issues);
    assert!(issues.iter().any(|issue| issue.code == "ICH.G.k.4.r.3.REQUIRED"));
}
```

- [ ] **Step 2: Run the API regression and verify RED**

Run:

```bash
cargo test -p web-server --test subresources_web dosage -- --nocapture
cargo test -p validator dosage_frequency_unit_is_required_from_number_of_units --lib -- --nocapture
```

Expected: API test FAILS because serialized `DosageInformation` still exposes
`frequency_value`; validator test FAILS because its companion trigger reads
`frequency_value` instead of `number_of_units`.

- [ ] **Step 3: Remove `frequency_value` from persistence and Rust models**

Delete the column from bootstrap SQL and remove the field from all three Rust dosage structs. Update seeds to use:

```sql
INSERT INTO dosage_information (..., number_of_units, frequency_unit, ...)
VALUES (..., 1, 'd', ...);
```

Use canonical UCUM code `d`, not the invalid seed label `day`. Remove the field from every affected struct literal in backend tests.

In `G_DOSAGE_COMPANION_RULES`, change only the trigger for
`ICH.G.k.4.r.3.REQUIRED`:

```rust
trigger: |dosage| dosage.number_of_units.is_some(),
```

- [ ] **Step 4: Run backend model and API tests**

Run:

```bash
cargo test -p lib-core --lib
cargo test -p validator dosage_frequency_unit_is_required_from_number_of_units --lib
cargo test -p web-server --test subresources_web dosage -- --nocapture
cargo check -p web-server
```

Expected: all commands exit 0.

- [ ] **Step 5: Prove the backend field is gone**

Run:

```bash
rg -n 'frequency_value' db crates --glob '!target/**'
```

Expected: no active backend/schema/test matches.

- [ ] **Step 6: Commit backend persistence cleanup**

```bash
git add db crates
git commit -m "refactor: remove duplicate dosage frequency value"
```

---

### Task 3: Activate G.k.4.r.3 through shared validation rule tables

**Files:**
- Modify: `crates/libs/validator/src/case/sections/g.rs`
- Modify: `crates/libs/validator/src/catalog.rs`
- Test: `crates/libs/validator/src/case/sections/g.rs`
- Test: `crates/libs/validator/src/case/sections/mod.rs`

**Interfaces:**
- Consumes: dictionary constraint `ICH.G.k.4.r.3.ALLOWED.VALUE` with `VocabularyScope::Frequency` and `VocabularyContext::for_active_codes()`.
- Produces: `G_DOSAGE_CONSTRAINT_RULES` evaluated only through `eval_nested_constraints()`.

- [ ] **Step 1: Write the failing shared vocabulary-rule validation test**

Add a test using the existing `drug()`, `dosage()`, and `empty_ctx()` helpers:

```rust
#[test]
fn dosage_frequency_unit_uses_frequency_vocabulary_scope() {
    const ALLOWED: [&str; 9] = [
        "a", "mo", "wk", "d", "h", "min",
        "{cyclical}", "{asnecessary}", "{total}",
    ];
    let active = ALLOWED.map(|code| {
        ("ICH-UCUM", crate::VocabularyScope::Frequency, code)
    });

    for unit in ALLOWED {
        let mut ctx = empty_ctx();
        ctx.vocabulary = crate::context::VocabularyContext::for_active_codes(&active);
        let mut parent = drug();
        parent.id = Uuid::from_u128(1);
        ctx.drugs.push(parent);
        let mut row = dosage();
        row.drug_id = Uuid::from_u128(1);
        row.frequency_unit = Some(unit.to_string());
        ctx.dosages.push(row);

        let mut issues = Vec::new();
        collect_ich_issues(&ctx, &mut issues);
        assert!(issues.iter().all(|issue| {
            issue.code != "ICH.G.k.4.r.3.ALLOWED.VALUE"
        }), "approved unit {unit} was rejected: {issues:#?}");
    }

    let mut ctx = empty_ctx();
    ctx.vocabulary = crate::context::VocabularyContext::for_active_codes(&active);
    let mut parent = drug();
    parent.id = Uuid::from_u128(1);
    ctx.drugs.push(parent);
    let mut row = dosage();
    row.drug_id = Uuid::from_u128(1);
    row.frequency_unit = Some("fortnight".to_string());
    ctx.dosages.push(row);

    let mut issues = Vec::new();
    collect_ich_issues(&ctx, &mut issues);
    assert!(issues.iter().any(|issue| issue.code == "ICH.G.k.4.r.3.ALLOWED.VALUE"));
}
```

- [ ] **Step 2: Run tests and verify RED**

Run:

```bash
cargo test -p validator dosage_frequency_unit_ --lib -- --nocapture
```

Expected: vocabulary test FAILS because no dosage constraint table is executed.

- [ ] **Step 3: Register the common dosage constraint table**

Add:

```rust
const G_DOSAGE_CONSTRAINT_RULES: &[NestedConstraintRule<DosageInformation>] = &[
    NestedConstraintRule {
        code: "ICH.G.k.4.r.3.ALLOWED.VALUE",
        path: |drug_idx, idx| format!(
            "drugs.{drug_idx}.dosages.{idx}.frequencyUnit"
        ),
        value: |dosage| ConstraintValue::Text(
            dosage.frequency_unit.as_deref().map(Cow::Borrowed),
        ),
    },
];
```

Call `eval_nested_constraints()` for dosages beside their length evaluation,
and add the table to `constraint_rule_codes()` and `table_rule_codes()`.

Classify `ICH.G.k.4.r.3.ALLOWED.VALUE` as `PHASES_CASE_VALIDATE` in the existing catalog phase table. Do not add any new evaluator or code-set function.

- [ ] **Step 4: Update exact-set expectations and verify GREEN**

Update only the expected counts/gated set changed by activating this existing rule. Run:

```bash
cargo test -p validator dosage_frequency_unit_ --lib
cargo test -p validator implemented_case_registry_matches_case_validate_catalog --lib
cargo test -p validator structured_allowed_value_target_has_only_official_vocabulary_gates_left --lib
```

Expected: all targeted tests PASS; G.k.4.r.3 is no longer in the gated set.

- [ ] **Step 5: Commit shared rule-table activation**

```bash
git add crates/libs/validator/src/case/sections/g.rs crates/libs/validator/src/case/sections/mod.rs crates/libs/validator/src/catalog.rs
git commit -m "fix: validate dosage frequency through rule tables"
```

---

### Task 4: Remove the duplicate registry entry

**Files:**
- Modify: `registry/sections/g-drug.json`
- Test: `registry/tools/test_validate.py`
- Test: `registry/tools/validate.py`

**Interfaces:**
- Consumes: backend fields `number_of_units`, `frequency_unit` and frontend fields `numberOfUnits`, `frequencyUnit`.
- Produces: official G.k.4.r.2 and G.k.4.r.3 registry rows with no local frequency-number row.

- [ ] **Step 1: Write a failing canonical-registry test**

Add to `RegistryValidatorTests`:

```python
def test_drug_registry_has_no_local_frequency_value(self):
    repo = Path(__file__).resolve().parents[2]
    rows = json.loads(
        (repo / "registry/sections/g-drug.json").read_text(encoding="utf-8")
    )
    ids = {row["id"] for row in rows}

    self.assertNotIn("G.k.local.dosage.frequencyValue", ids)
```

- [ ] **Step 2: Run the registry test and verify RED**

Run:

```bash
python3 -m unittest registry.tools.test_validate.RegistryValidatorTests.test_drug_registry_has_no_local_frequency_value
```

Expected: FAIL because the local registry row still exists.

- [ ] **Step 3: Correct official row evidence**

Keep exactly:

```json
{
  "id": "G.k.4.r.3",
  "e2br3_code": "G.k.4.r.3",
  "label": "Definition of the Time Interval Unit",
  "backend": { "field": "frequency_unit" },
  "frontend": { "field": "frequencyUnit" }
}
```

Preserve the surrounding complete metadata and point evidence to the current frontend `DrugDosageFields.tsx` path.

- [ ] **Step 4: Verify registry GREEN**

Run:

```bash
python3 registry/tools/validate.py
python3 -m unittest registry.tools.test_validate.RegistryValidatorTests.test_drug_registry_has_no_local_frequency_value
rg -n 'G\.k\.local\.dosage\.frequencyValue|frequency_value|frequencyValue' registry/sections/g-drug.json
```

Expected: validator exits 0 and `rg` returns no matches.

- [ ] **Step 5: Commit registry cleanup**

```bash
git add registry/sections/g-drug.json registry/tools/test_validate.py
git commit -m "refactor: remove local dosage frequency registry field"
```

---

### Task 5: Canonicalize frontend form and API contracts

**Files:**
- Modify: `lib/types/e2br3.ts`
- Modify: `lib/schemas/e2br3.ts`
- Modify: `lib/zod/sections/dg.ts`
- Modify: `lib/api/endpoints/cases/core/detail.drugs.ts`
- Modify: `lib/api/endpoints/cases/subresources/drug-dosage.ts`
- Modify: `app/(protected)/[authority]/case/[id]/detail/DG/model/sectionGModel.ts`
- Modify: `app/dg-preview/page.tsx`
- Test: `__tests__/api/dto-contracts.test.ts`
- Test: `__tests__/api/detail.drugs.test.ts`
- Test: `__tests__/field-error-banners/drugs.test.ts`

**Interfaces:**
- Consumes: backend payload keys `number_of_units` and `frequency_unit`.
- Produces: frontend form fields `numberOfUnits` and `frequencyUnit` only.

- [ ] **Step 1: Write failing DTO/detail tests**

Extend `dto-contracts.test.ts`:

```ts
it("sends only canonical dosage interval fields", async () => {
  await api.cases.upsertDosageInformation("case-1", "drug-1", [{
    numberOfUnits: 3,
    frequencyUnit: "d",
  }]);

  expect(lastJsonBody().data).toEqual(expect.objectContaining({
    number_of_units: 3,
    frequency_unit: "d",
  }));
  expect(lastJsonBody().data).not.toHaveProperty("frequency_value");
});
```

Extend `api/detail.drugs.test.ts` with a dosage row containing all three backend keys and assert the returned form row contains `numberOfUnits` and `frequencyUnit` but not `frequencyValue`.

- [ ] **Step 2: Run frontend contract tests and verify RED**

Run:

```bash
npm test -- __tests__/api/dto-contracts.test.ts __tests__/api/detail.drugs.test.ts --runInBand
```

Expected: FAIL because detail/save adapters still expose `frequencyValue`/`frequency_value`.

- [ ] **Step 3: Remove duplicate frontend fields**

Delete `frequencyValue`, `doseFrequencyValue`, and `doseFrequencyUnit` from types and schema. Remove `frequencyValue` from defaults, preview data, portable DG paths, detail transforms, save transforms, meaningful-row checks, and affected fixtures.

Replace the frontend companion refinement with:

```ts
if (hasNumber(value.numberOfUnits) && !hasText(value.frequencyUnit)) {
  addRequiredIssue(
    ctx,
    ["frequencyUnit"],
    "Time interval unit is required when number of units is provided.",
  );
}
```

- [ ] **Step 4: Verify frontend contracts GREEN**

Run:

```bash
npm test -- __tests__/api/dto-contracts.test.ts __tests__/api/detail.drugs.test.ts __tests__/field-error-banners/drugs.test.ts --runInBand
npx tsc --noEmit
```

Expected: tests and TypeScript check exit 0.

- [ ] **Step 5: Commit frontend contract cleanup**

```bash
git add lib app __tests__
git commit -m "refactor: canonicalize dosage interval form fields"
```

---

### Task 6: Replace G.k.4.r.3 select with the searchable autocomplete

**Files:**
- Modify: `app/(protected)/[authority]/case/[id]/detail/DG/hooks/useSectionGOptions.ts`
- Modify: `app/(protected)/[authority]/case/[id]/detail/DG/drug-entry/DrugDosageFields.tsx`
- Modify: `app/(protected)/[authority]/case/[id]/detail/DG/drug-entry/DrugDosageSection.tsx`
- Modify: `app/(protected)/[authority]/case/[id]/detail/DG/drug-entry/DrugEntry.tsx`
- Modify: `app/(protected)/[authority]/case/[id]/detail/DG/components/SectionG.tsx`
- Modify: `components/case-form/sections/sectionGOptions.ts`
- Modify: `components/forms/FormAutocomplete.tsx`
- Test: `__tests__/case-form/SectionG.dosage-frequency.test.tsx`
- Test: `__tests__/ui-binding/field-bindings.test.ts`

**Interfaces:**
- Consumes: existing `FormAutocomplete` and React Hook Form path `drugs.${drugIndex}.dosageInformation.${doseIndex}.frequencyUnit`.
- Produces: exported `DOSAGE_INTERVAL_UNIT_OPTIONS` from `components/case-form/sections/sectionGOptions.ts` containing nine `{ value, label }` entries.

- [ ] **Step 1: Write failing option and interaction tests**

Create a focused test that imports the options and asserts:

```ts
expect(DOSAGE_INTERVAL_UNIT_OPTIONS).toEqual([
  { value: "a", label: "a:year" },
  { value: "mo", label: "mo:month" },
  { value: "wk", label: "wk:week" },
  { value: "d", label: "d:day" },
  { value: "h", label: "h:hour" },
  { value: "min", label: "min:minute" },
  { value: "{cyclical}", label: "{cyclical}:cyclical" },
  { value: "{asnecessary}", label: "{asnecessary}:as necessary" },
  { value: "{total}", label: "{total}:total" },
]);
```

Render one dosage row, open the G.k.4.r.3 combobox, search `day` (a label
that is not contained in code `d`), select `d:day`, and assert
`frequencyUnit === "d"`. Also open the field-actions button and assert its
dialog identifies G.k.4.r.3 so audit/notation behavior is retained.

- [ ] **Step 2: Run UI test and verify RED**

Run: `npm test -- __tests__/case-form/SectionG.dosage-frequency.test.tsx --runInBand`

Expected: FAIL because the options constant and combobox do not exist.

- [ ] **Step 3: Implement static options and `FormAutocomplete`**

Export the exact nine-entry constant from
`components/case-form/sections/sectionGOptions.ts`. In the shared
`FormAutocomplete`, make static-option label text searchable without changing
the stored value:

```tsx
<CommandItem
  key={option.value}
  value={option.value}
  keywords={[option.label, option.secondary || ""]}
  onSelect={() => {
    field.onChange(option.value);
    onSelectOption?.(option);
    setOpen(false);
    setSearchQuery("");
  }}
>
```

Replace the `Select` block in `DrugDosageFields.tsx` with:

```tsx
<FormAutocomplete
  control={control}
  name={`drugs.${drugIndex}.dosageInformation.${doseIndex}.frequencyUnit`}
  label="Definition of the Time Interval Unit"
  fieldNumber="G.k.4.r.3"
  options={DOSAGE_INTERVAL_UNIT_OPTIONS}
  placeholder="Autocomplete"
  searchPlaceholder="Search code or unit"
  e2bLayout
  hideMoreButton
  trailingSlot={(
    <FieldActionsButton
      label="Definition of the Time Interval Unit"
      fieldNumber="G.k.4.r.3"
      auditTable="dosage_information"
      auditField="frequency_unit"
      auditRecordId={dosageRows[doseIndex]?.id}
    />
  )}
/>
```

Import `FieldActionsButton` from `E2BFormField`. Remove the old
`dosageIntervalUnitOptions` prop from `useSectionGOptions`, `SectionG`,
`DrugEntry`, `DrugDosageSection`, and `DrugDosageFields`; do not fetch the
general UCUM endpoint for this field.

- [ ] **Step 4: Verify UI GREEN**

Run:

```bash
npm test -- __tests__/case-form/SectionG.dosage-frequency.test.tsx __tests__/ui-binding/field-bindings.test.ts --runInBand
npx tsc --noEmit
```

Expected: tests pass, G.k.4.r.3 has `role="combobox"`, and selection stores only the canonical code.

- [ ] **Step 5: Commit frontend autocomplete**

```bash
git add app __tests__
git commit -m "feat: add dosage interval autocomplete"
```

---

### Task 7: Full cleanup and verification

**Files:**
- Modify if generated by the approved proof command: `artifacts/save-proof/*.json`
- Verify: both repositories

**Interfaces:**
- Consumes: all prior tasks.
- Produces: clean test runs and no active duplicate field references.

- [ ] **Step 1: Search for stale duplicate names**

Backend:

```bash
rg -n 'frequency_value|frequencyValue|doseFrequencyValue' db crates registry --glob '!target/**'
```

Frontend:

```bash
rg -n 'frequency_value|frequencyValue|doseFrequencyValue|doseFrequencyUnit' app lib __tests__ scripts --glob '!node_modules/**' --glob '!.next/**'
```

Expected: no active source, registry, adapter, schema, or test-fixture matches. Generated historical artifacts may be regenerated only through their existing proof command, not hand-edited.

- [ ] **Step 2: Run backend verification**

```bash
cargo fmt --check
cargo test -p lib-core --lib
cargo test -p validator --lib
cargo test -p web-server --test subresources_web
python3 registry/tools/validate.py
```

Expected: every command exits 0.

- [ ] **Step 3: Run frontend verification**

```bash
npx tsc --noEmit
npm test -- __tests__/api/dto-contracts.test.ts __tests__/api/detail.drugs.test.ts __tests__/case-form/SectionG.dosage-frequency.test.tsx __tests__/ui-binding/field-bindings.test.ts --runInBand
npm run proof:save-fields:all-fast
```

Expected: every command exits 0.

- [ ] **Step 4: Inspect repository diffs and preserve unrelated work**

```bash
git status --short
git diff --check
git -C /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend status --short
git -C /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend diff --check
```

Expected: only task files and pre-existing unrelated untracked files are present; no unrelated file is staged or modified.

- [ ] **Step 5: Commit any verified generated contract updates separately**

If and only if the proof command changed tracked generated artifacts:

```bash
git add artifacts/save-proof
git commit -m "test: refresh dosage interval save proof"
```

Do not create an empty commit when no artifacts changed.
