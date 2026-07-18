# Remove Drug-Level Dosage Text Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Delete app-local `DrugInformation.dosage_text` and use repeated `DosageInformation.dosage_text` rows exclusively in case editing, E2B(R3) XML, and CIOMS.

**Architecture:** The repeated dosage table remains the canonical E2B(R3) representation. CIOMS performs an explicit lossy-shape conversion by joining every populated dosage-row text in `sequence_number` order, while XML preserves one text per repeated dosage node. Backend and frontend remove the legacy drug-level field without a database migration.

**Tech Stack:** Rust/Axum/SQLx/PostgreSQL backend, Next.js/TypeScript frontend, Python registry validator, cargo test, Jest, TypeScript.

## Global Constraints

- Both backend and frontend branches start from `origin/dev`.
- Do not add a database migration; update bootstrap/init SQL only.
- Preserve `DosageInformation.dosage_text` and frontend dosage-row `dosageText`.
- Preserve duplicate non-blank dosage texts in CIOMS because equal text may describe distinct regimens.
- CIOMS joins trimmed dosage texts with `"\n"` in ascending `sequence_number`.
- Frontend must stop sending the legacy drug-level field before or together with backend deployment.
- Follow red-green TDD for every behavior change.

---

### Task 1: Make CIOMS preserve all repeated dosage texts

**Files:**
- Modify: `crates/services/web-server/src/web/rest/cioms_export_rest/types.rs`
- Test: `crates/services/web-server/src/web/rest/cioms_export_rest/tests.rs`

**Interfaces:**
- Consumes: `CiomsCaseData.dosages: Vec<DosageInformation>` and the selected suspect drug id.
- Produces: `CiomsFormData.suspect_drug_dose: String` containing all non-blank dosage texts in sequence order.

- [ ] **Step 1: Write the failing CIOMS aggregation test**

Add a focused test using one suspect drug and dosage rows deliberately supplied out of order:

```rust
#[test]
fn cioms_joins_all_suspect_dosage_texts_in_sequence_order() {
	let drug_id = test_uuid();
	let mut first = dosage_with_route(drug_id, "PO");
	first.sequence_number = 1;
	first.dosage_text = Some("  first regimen  ".to_string());
	let mut blank = dosage_with_route(drug_id, "IV");
	blank.sequence_number = 2;
	blank.dosage_text = Some("   ".to_string());
	let mut third = dosage_with_route(drug_id, "IM");
	third.sequence_number = 3;
	third.dosage_text = Some("third regimen".to_string());

	let data = CiomsCaseData {
		case_number: "SR-DOSAGE-TEXTS".to_string(),
		report: None,
		patient: None,
		reactions: Vec::new(),
		drugs: vec![suspect_drug(drug_id)],
		dosages: vec![third, blank, first],
		indications: Vec::new(),
		primary_sources: Vec::new(),
		senders: Vec::new(),
		narrative: None,
	};

	let form = CiomsFormData::from_case_data(&data, &default_settings());

	assert_eq!(form.suspect_drug_dose, "first regimen\nthird regimen");
}
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```bash
cargo test -p web-server cioms_joins_all_suspect_dosage_texts_in_sequence_order
```

Expected: FAIL because current code selects a single dosage row.

- [ ] **Step 3: Implement the minimal sequence-ordered aggregation**

Add a private helper in `types.rs`:

```rust
fn dosage_texts_for_drug(data: &CiomsCaseData, drug_id: Uuid) -> String {
	let mut rows: Vec<_> = data
		.dosages
		.iter()
		.filter(|dosage| dosage.drug_id == drug_id)
		.collect();
	rows.sort_by_key(|dosage| dosage.sequence_number);
	rows.into_iter()
		.filter_map(|dosage| dosage.dosage_text.as_deref())
		.map(str::trim)
		.filter(|text| !text.is_empty())
		.collect::<Vec<_>>()
		.join("\n")
}
```

Set `suspect_drug_dose` from this helper. Keep the existing single
`suspect_dosage` selection for route, dates, and duration.

- [ ] **Step 4: Run CIOMS tests and verify GREEN**

Run:

```bash
cargo test -p web-server cioms_export_rest
```

Expected: all CIOMS tests pass. Update the existing latest-child PDF assertion
so both dosage texts are expected while route and indication retain their
current selection behavior.

- [ ] **Step 5: Commit the CIOMS behavior**

```bash
git add crates/services/web-server/src/web/rest/cioms_export_rest/types.rs \
  crates/services/web-server/src/web/rest/cioms_export_rest/tests.rs
git commit -m "fix: preserve repeated dosage text in CIOMS"
```

---

### Task 2: Remove drug-level dosage text from backend, XML, SQL, and registry

**Files:**
- Modify: `crates/libs/lib-core/src/model/drug.rs`
- Modify: `crates/libs/lib-core/src/xml/import_sections/g_drug.rs`
- Modify: `crates/libs/lib-core/src/xml/import_runtime/g.rs`
- Modify: `crates/libs/lib-core/src/xml/import_runtime/helpers/g.rs`
- Modify: `crates/libs/lib-core/src/xml/export/sections/g.rs`
- Modify: `crates/services/web-server/src/openapi.rs`
- Modify: backend Rust tests and fixtures that construct `DrugInformation`
- Modify: `db/bootstrap/07-drug-information.sql`
- Modify: `db/seed/002-rich-demo-case.sql`
- Modify: `registry/sections/g-drug.json`
- Modify: `registry/tools/test_validate.py`
- Test: `crates/libs/lib-core/tests/xml/xml_export_g.rs`
- Test: `registry/tools/test_validate.py`

**Interfaces:**
- Removes: `DrugInformation.dosage_text`, corresponding create/update DTO members, SQL bindings, OpenAPI members, and `G.k.local.supplemental.dosageText`.
- Preserves: `DosageInformation.dosage_text` and registry row `G.k.4.r.8`.

- [ ] **Step 1: Write the failing XML behavior test**

In `xml_export_g.rs`, keep the dosage-row fixture text `"Dose text"` and
set the legacy drug-level fixture to `"legacy drug text"`. Add:

```rust
assert!(xml.contains("<text>Dose text</text>"));
assert!(!xml.contains("<text>legacy drug text</text>"));
```

- [ ] **Step 2: Write the failing registry/model inventory test**

Extend the removal regression test in `registry/tools/test_validate.py`:

```python
self.assertNotIn(
    "G.k.local.supplemental.dosageText",
    registry_ids,
)
drug_source = (repo / "crates/libs/lib-core/src/model/drug.rs").read_text()
drug_information_source = drug_source.split("// -- DosageInformation", 1)[0]
self.assertNotIn("pub dosage_text: Option<String>", drug_information_source)
bootstrap = (repo / "db/bootstrap/07-drug-information.sql").read_text()
drug_table = bootstrap.split("CREATE TABLE dosage_information", 1)[0]
self.assertNotIn("dosage_text", drug_table)
```

- [ ] **Step 3: Run both tests and verify RED**

Run:

```bash
cargo test -p lib-core --test xml xml_export_g
python3 -m unittest discover -s registry/tools -p 'test_validate.py'
```

Expected: XML test finds the legacy drug text; registry/model test finds the
local row and drug-level field.

- [ ] **Step 4: Remove the backend field at every drug-level boundary**

Apply these targeted removals:

- Delete `dosage_text` only from `DrugInformation`,
  `DrugInformationForCreate`, and `DrugInformationForUpdate`.
- Renumber SQL placeholders and remove the corresponding binds in create and
  update statements.
- Delete drug-level import parsing and runtime transfer; retain dosage-row
  parsing and transfer.
- Delete the drug-level `<text>` export block before `<consumable>`; retain
  the dosage-loop text block.
- Remove drug-level OpenAPI DTO members.
- Remove only the first `drug_information.dosage_text` bootstrap column;
  retain the `dosage_information.dosage_text` column.
- Remove the drug-level seed insert column/value while retaining dosage-row
  seed values.
- Remove registry row `G.k.local.supplemental.dosageText`; retain
  `G.k.4.r.8`.
- Update Rust fixtures and API contract tests so `DrugInformation`
  constructors compile and drug endpoints no longer accept or return the
  legacy field.

- [ ] **Step 5: Run focused backend and registry tests and verify GREEN**

Run:

```bash
cargo test -p lib-core --test xml
cargo test -p lib-core --test import
cargo test -p web-server cioms_export_rest
python3 registry/tools/validate.py
python3 registry/tools/validate.py --strict-backend-inventory
python3 registry/tools/validate.py --strict-dictionary
python3 -m unittest discover -s registry/tools -p 'test_*.py'
```

Expected: all commands pass and repeated dosage text remains covered.

- [ ] **Step 6: Commit the backend removal**

```bash
git add crates db/bootstrap/07-drug-information.sql \
  db/seed/002-rich-demo-case.sql registry
git commit -m "refactor: remove drug-level dosage text"
```

---

### Task 3: Remove the hidden drug-level field from frontend

**Files:**
- Modify: `lib/api/endpoints/cases/core/detail.drugs.ts`
- Modify: `lib/api/endpoints/cases/subresources/drug.ts`
- Modify: `lib/types/e2br3.ts`
- Modify: `lib/zod/sections/dg.ts`
- Modify: `app/(protected)/[authority]/case/[id]/detail/DG/hooks/useSectionGDrugs.ts`
- Modify: `app/dg-preview/page.tsx`
- Modify: frontend tests and fixtures containing `drugDosageText`
- Test: `__tests__/api/dto-contracts.test.ts`

**Interfaces:**
- Removes: drug object property `drugDosageText` and drug REST
  `dosage_text`.
- Preserves: `drugs[].dosageInformation[].dosageText` and dosage REST
  `dosage_text`.

- [ ] **Step 1: Create a frontend branch from current dev**

```bash
git fetch origin dev
git switch -c codex/remove-drug-level-dosage-text origin/dev
```

- [ ] **Step 2: Write the failing drug DTO test**

Add to `dto-contracts.test.ts`:

```typescript
it("does not send legacy drug-level dosage text", async () => {
  await api.cases.upsertDrugs("case-1", [
    {
      drugCharacterization: "1",
      medicinalProduct: "Product",
      drugDosageText: "legacy drug text",
    },
  ]);

  expect(lastJsonBody().data).not.toHaveProperty("dosage_text");
});
```

Also extend the existing dosage DTO test input with
`dosageText: "500 mg twice daily"` and assert:

```typescript
expect(lastJsonBody().data.dosage_text).toBe("500 mg twice daily");
```

- [ ] **Step 3: Run the DTO test and verify RED**

Run:

```bash
npx jest __tests__/api/dto-contracts.test.ts --runInBand
```

Expected: the legacy drug DTO test fails because `dosage_text` is still sent;
the repeated dosage assertion passes.

- [ ] **Step 4: Remove only the drug-level frontend path**

- Delete both `drugDosageText` load mappings from `detail.drugs.ts`.
- Delete legacy text selection, cumulative-unit concatenation, and
  `dosage_text` from the drug payload in `drug.ts`.
- Delete the drug-level `dosageText`/`drugDosageText` members from the
  `DrugInformation` frontend type only.
- Delete the drug-level zod rule/path.
- Delete hidden defaults and preview fixtures.
- Retain dosage editor binding, dosage detail mapping, dosage subresource
  payload, dosage types, and generated `G.k.4.r.8` binding.

- [ ] **Step 5: Run frontend tests and verify GREEN**

Run:

```bash
npx jest __tests__/api/dto-contracts.test.ts \
  __tests__/case-save/drugs.coordinator.test.ts --runInBand
npx tsc --noEmit
```

Expected: all tests and TypeScript compilation pass.

- [ ] **Step 6: Commit the frontend removal**

```bash
git add app lib __tests__
git commit -m "refactor: remove drug-level dosage text"
```

---

### Task 4: Cross-repository verification and publication

**Files:**
- Verify only; no additional files expected.

**Interfaces:**
- Backend and frontend must both be descendants of `origin/dev`.
- Backend must contain no application reference to
  `DrugInformation.dosage_text`; frontend must contain no
  `drugDosageText`; repeated dosage text must remain.

- [ ] **Step 1: Run final backend verification**

```bash
git diff --check
cargo check --all-targets
cargo test -p lib-core -p validator --lib
cargo test -p web-server cioms_export_rest
python3 registry/tools/validate.py
python3 registry/tools/validate.py --strict-backend-inventory
python3 registry/tools/validate.py --strict-dictionary
python3 registry/tools/validate.py --strict-frontend-inventory
python3 -m unittest discover -s registry/tools -p 'test_*.py'
```

- [ ] **Step 2: Run final frontend verification**

```bash
git diff --check
npx tsc --noEmit
npx jest __tests__/api/dto-contracts.test.ts \
  __tests__/case-save/drugs.coordinator.test.ts --runInBand
```

- [ ] **Step 3: Audit the exact removals**

```bash
rg -n 'drugDosageText|G\.k\.local\.supplemental\.dosageText' app lib __tests__
rg -n 'dosage_text' crates/libs/lib-core/src/model/drug.rs \
  crates/libs/lib-core/src/xml db/bootstrap/07-drug-information.sql \
  registry/sections/g-drug.json
```

Expected: the frontend command has no hits. Backend hits are limited to
`DosageInformation`, repeated dosage XML, and `G.k.4.r.8`.

- [ ] **Step 4: Push frontend before backend and open dev-targeted PRs**

Push the frontend branch and create a PR targeting `dev`. After it is
mergeable, push the backend branch and create a second PR targeting `dev`.
Merge frontend first, then backend. Do not delete worktrees until both merges
and `origin/dev` ancestry checks succeed.
