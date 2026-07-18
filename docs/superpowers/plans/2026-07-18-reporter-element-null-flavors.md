# Reporter Element NullFlavor Hard-Cutover Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the two reporter group nullFlavor fields and replace them end to end with eleven E2B element-level nullFlavor fields.

**Architecture:** Add parallel value/nullFlavor columns to `primary_sources` and `reporter_presaves`, carry them through Rust and frontend contracts, and import/export each XML element independently. Preserve the existing registry row format with eleven local persistence-companion rows joined between reporter presave and case namespaces.

**Tech Stack:** PostgreSQL, Rust/sqlx, libxml2 XPath, TypeScript, React Hook Form, Zod, Jest, Python registry validation.

## Global Constraints

- Delete `reporterNameNullFlavor`, `reporterAddressNullFlavor`, `reporter_name_null_flavor`, and `reporter_address_null_flavor` without migration, alias, fallback, or dual write.
- Add exactly eleven individual fields from the approved design.
- C.2.r.1.1 Reporter Title accepts `MSK`, `UNK`, `ASKU`, and `NASK`; the other ten individual fields accept `MSK`, `ASKU`, and `NASK`.
- Reuse the existing shared frontend nullFlavor control.
- A nullFlavor clears and disables only its matching value field.
- Keep country and qualification nullFlavor behavior unchanged.
- Do not hand-edit generated catalog bindings without running their generator.

---

### Task 1: Replace database and Rust persistence contracts

**Files:**
- Create: `db/migrations/20260718_reporter_element_null_flavors.sql`
- Modify: `db/bootstrap/01-safetydb-schema.sql`
- Modify: `db/bootstrap/03-safety-report-identification.sql`
- Modify: `crates/libs/lib-core/src/model/safety_report.rs`
- Modify: `crates/libs/lib-core/src/model/presave.rs`
- Modify: `crates/services/web-server/src/web/rest/case_editor_rest/direct.rs`
- Modify: `crates/services/web-server/src/web/rest/case_editor_rest/portable_save.rs`
- Modify: `crates/services/web-server/src/openapi.rs`
- Modify: `crates/libs/validator/src/portable_bindings/c.rs`
- Modify: `crates/libs/validator/src/catalog.rs`
- Modify: `crates/libs/validator/src/case/sections/c.rs`
- Modify: `crates/libs/validator/src/c_reporter_policy.rs`
- Test: `crates/services/web-server/tests/api/subresources_web.rs`
- Test: `crates/libs/lib-core/tests/section_presave.rs`

**Interfaces:**
- Consumes: existing `PrimarySource` and `ReporterPresave` CRUD contracts.
- Produces: eleven snake_case `Option<String>` properties on both models and REST payloads.

- [ ] **Step 1: Write failing REST and presave persistence tests**

Create/update a reporter with representative independent fields and assert they round-trip without either removed key:

```rust
"reporter_title_null_flavor": "MSK",
"reporter_given_name_null_flavor": "ASKU",
"organization_null_flavor": "NASK",
"telephone_null_flavor": "MSK"
```

Assert the returned JSON has these exact keys and does not contain
`reporter_name_null_flavor` or `reporter_address_null_flavor`. Add the same four
assertions to reporter-presave CRUD tests.

- [ ] **Step 2: Verify RED**

```sh
cargo test -p web-server --test subresources_web reporter -- --nocapture
cargo test -p lib-core --test section_presave reporter_null_flavor -- --nocapture
```

Expected: tests fail because the new fields are absent.

- [ ] **Step 3: Add the destructive migration and bootstrap schema**

The migration performs no data copy:

```sql
ALTER TABLE primary_sources
  DROP COLUMN IF EXISTS reporter_name_null_flavor,
  DROP COLUMN IF EXISTS reporter_address_null_flavor;
ALTER TABLE reporter_presaves
  DROP COLUMN IF EXISTS reporter_name_null_flavor,
  DROP COLUMN IF EXISTS reporter_address_null_flavor;
```

For both tables add:

```sql
reporter_title_null_flavor VARCHAR(4) CHECK (reporter_title_null_flavor IN ('MSK','UNK','ASKU','NASK')),
reporter_given_name_null_flavor VARCHAR(4) CHECK (reporter_given_name_null_flavor IN ('MSK','ASKU','NASK')),
reporter_middle_name_null_flavor VARCHAR(4) CHECK (reporter_middle_name_null_flavor IN ('MSK','ASKU','NASK')),
reporter_family_name_null_flavor VARCHAR(4) CHECK (reporter_family_name_null_flavor IN ('MSK','ASKU','NASK')),
organization_null_flavor VARCHAR(4) CHECK (organization_null_flavor IN ('MSK','ASKU','NASK')),
department_null_flavor VARCHAR(4) CHECK (department_null_flavor IN ('MSK','ASKU','NASK')),
street_null_flavor VARCHAR(4) CHECK (street_null_flavor IN ('MSK','ASKU','NASK')),
city_null_flavor VARCHAR(4) CHECK (city_null_flavor IN ('MSK','ASKU','NASK')),
state_null_flavor VARCHAR(4) CHECK (state_null_flavor IN ('MSK','ASKU','NASK')),
postcode_null_flavor VARCHAR(4) CHECK (postcode_null_flavor IN ('MSK','ASKU','NASK')),
telephone_null_flavor VARCHAR(4) CHECK (telephone_null_flavor IN ('MSK','ASKU','NASK'))
```

- [ ] **Step 4: Replace Rust models, SQL columns/binds, REST parsing, and OpenAPI properties**

Add the eleven `Option<String>` fields to `PrimarySource`, its create/update
types, `ReporterPresave`, its create/update/insert types, and their BMC bind
lists. Replace both removed direct/portable-save aliases with exact camelCase and
snake_case pairs for the eleven new fields. Update existing struct literals in
validator, CIOMS, XML, intake, and API tests with `None` for the new fields and
remove the two old fields.

- [ ] **Step 5: Rebind validation without changing the shared rule engine**

In `portable_bindings/c.rs`, replace the two group nullFlavor bindings with
eleven bindings and point every reporter value binding at its matching
nullFlavor path. Keep the existing `ICH.C.2.r.*.NULLFLAVOR.ALLOWED` codes.
In `catalog.rs`, change the `ICH.C.2.r.2.1.REQUIRED` value policy from
`NonEmpty` to `NonEmptyOrNullFlavor`. In `case/sections/c.rs`, feed the presence
evaluator the actual `organization` and `organization_null_flavor` values; do
not manufacture a `"present"` marker. In `c_reporter_policy.rs`, count every new
nullFlavor as reporter content. Do not modify `case/sections/rule_table.rs`.

Add validator regressions proving C.2.r.1.1 accepts `UNK`, the other ten fields
reject `UNK`, every value binding points only to its own companion, and a study
report with an organization nullFlavor satisfies C.2.r.2.1 REQUIRED.

- [ ] **Step 6: Verify GREEN and commit**

```sh
cargo test -p lib-core --test section_presave
cargo test -p web-server --test subresources_web reporter -- --nocapture
cargo test -p validator portable_bindings::c
cargo test -p validator c_reporter_policy
cargo check -p lib-core -p web-server
git add db crates
git commit -m "refactor: split reporter null flavor persistence"
```

---

### Task 2: Import and export element-level XML nullFlavor

**Files:**
- Modify: `crates/libs/lib-core/src/xml/export/sections/c.rs`
- Modify: `crates/libs/lib-core/src/xml/import_runtime/helpers/c.rs`
- Modify: `crates/libs/lib-core/src/xml/import_runtime/c.rs`
- Test: `crates/libs/lib-core/tests/xml/xml_export_c.rs`
- Test: `crates/libs/lib-core/tests/xml/xml_import_c.rs`

**Interfaces:**
- Consumes: the eleven `PrimarySource` nullFlavor fields from Task 1.
- Produces: independent XML `nullFlavor` attributes for C.2.r.1.1–1.4 and C.2.r.2.1–2.7.

- [ ] **Step 1: Write failing XML isolation tests**

Export a reporter with `reporter_given_name_null_flavor = ASKU` and
`city_null_flavor = NASK`. Assert only the given-name and city nodes contain
those attributes. Import the XML and assert the same two Rust fields are set
while all nine siblings remain `None`.

- [ ] **Step 2: Verify RED**

```sh
cargo test -p lib-core --test xml xml_export_c -- --nocapture
cargo test -p lib-core --test xml xml_import_c -- --nocapture
```

Expected: the new XML attributes are not persisted.

- [ ] **Step 3: Implement per-node value-or-nullFlavor handling**

For prefix, given[1], given[2], family, organization, department, street,
city, state, postalCode, and telephone, write the value when present; otherwise
write only that field's `nullFlavor`. Read each node's `nullFlavor` into its
matching import property. Do not fan values between organization and department
or between any name nodes.

- [ ] **Step 4: Verify and commit**

```sh
cargo test -p lib-core --test xml xml_export_c -- --nocapture
cargo test -p lib-core --test xml xml_import_c -- --nocapture
git add crates/libs/lib-core/src/xml crates/libs/lib-core/tests/xml
git commit -m "fix: preserve reporter element null flavors in XML"
```

---

### Task 3: Replace frontend group fields in Presave and Case Edit

**Files:**
- Modify: `../frontend/E2BR3-frontend/lib/types/e2br3.ts`
- Modify: `../frontend/E2BR3-frontend/lib/types/presave.ts`
- Modify: `../frontend/E2BR3-frontend/lib/schemas/presave.ts`
- Modify: `../frontend/E2BR3-frontend/components/presave/ReporterForm.tsx`
- Modify: `../frontend/E2BR3-frontend/lib/presave/canonicalMappers.ts`
- Modify: `../frontend/E2BR3-frontend/lib/presave/canonicalWriteMappers.ts`
- Modify: `../frontend/E2BR3-frontend/app/(protected)/[authority]/case/[id]/detail/RP/model/rpModel.ts`
- Modify: `../frontend/E2BR3-frontend/app/(protected)/[authority]/case/[id]/detail/RP/components/ReporterEditorPanel.tsx`
- Modify: `../frontend/E2BR3-frontend/lib/case-save/pages/RP/save.ts`
- Modify: `../frontend/E2BR3-frontend/lib/case-save/pages/direct-page-patch.ts`
- Modify: `../frontend/E2BR3-frontend/lib/api/endpoints/cases/core/detail.reporter.ts`
- Modify: `../frontend/E2BR3-frontend/lib/api/endpoints/cases/subresources/sender.ts`
- Regenerate: `../frontend/E2BR3-frontend/lib/zod/generated/catalogBindings.ts`
- Regenerate: `../frontend/E2BR3-frontend/lib/zod/generated/catalogConstraints.ts`
- Test: `../frontend/E2BR3-frontend/__tests__/case-form/ReporterSection.reporter-null-flavors.test.tsx`
- Test: `../frontend/E2BR3-frontend/__tests__/dashboard/presave-minimal-form-validation.test.ts`
- Test: `../frontend/E2BR3-frontend/__tests__/dashboard/canonical-presave-mappers.test.ts`

**Interfaces:**
- Consumes: exact camelCase/snake_case API fields from Task 1.
- Produces: eleven independent frontend fields in Case Edit and Reporter Presave.

- [ ] **Step 1: Write failing UI, schema, mapper, and transfer tests**

Assert Reporter Title renders an existing nullFlavor select with
`["", "MSK", "UNK", "ASKU", "NASK"]` and each of the other ten fields renders
`["", "MSK", "ASKU", "NASK"]`. Select `reporterCityNullFlavor = NASK` and
assert only `reporterCity` clears. Parse a presave with an empty given name plus
`reporterGivenNameNullFlavor = ASKU`, and an empty organization plus
`reporterOrganizationNullFlavor = NASK`. Assert canonical read/write and
presave-to-case transfer preserve all eleven exact fields and omit both removed
group names.

- [ ] **Step 2: Verify RED**

```sh
npx jest --runInBand __tests__/case-form/ReporterSection.reporter-null-flavors.test.tsx __tests__/dashboard/presave-minimal-form-validation.test.ts __tests__/dashboard/canonical-presave-mappers.test.ts
```

- [ ] **Step 3: Replace types, validation, mappers, and transfer**

Define `ReporterElementNullFlavor = "MSK" | "ASKU" | "NASK"` and
`ReporterTitleNullFlavor = ReporterElementNullFlavor | "UNK"`. Add the eleven
optional camelCase fields to both case and presave types, using the title-specific
type only for C.2.r.1.1. Delete both group fields. Change required validation to:

```ts
if (!data.reporterGivenNameNullFlavor && !data.reporterGivenName?.trim()) {
  ctx.addIssue({
    code: z.ZodIssueCode.custom,
    path: ["reporterGivenName"],
    message: "Reporter's given name is required",
  });
}
if (!data.reporterOrganizationNullFlavor && !data.reporterOrganization?.trim()) {
  ctx.addIssue({
    code: z.ZodIssueCode.custom,
    path: ["reporterOrganization"],
    message: "Reporter's organisation is required",
  });
}
```

Read/write each exact backend key and copy it directly in
`reporterPresaveToPrimarySource`.

- [ ] **Step 4: Render the shared per-field controls**

Attach one `NullFlavorSelect` to every Case Edit value field and reuse the
existing presave `NullFlavorButton`/helper per individual field. Each
`clearFieldsOnSet` array contains only its matching value path, and each input's
disabled state watches only its matching nullFlavor.

- [ ] **Step 5: Regenerate validation bindings, verify, and commit**

```sh
BACKEND_REPO=/Users/hyundonghoon/projects/rust/e2br3/e2br3/.worktrees/reporter-element-null-flavors node scripts/validation/sync-catalog-constraints.mjs --write
npx jest --runInBand __tests__/case-form/ReporterSection.reporter-null-flavors.test.tsx __tests__/dashboard/presave-minimal-form-validation.test.ts __tests__/dashboard/canonical-presave-mappers.test.ts
git add app components lib __tests__
git commit -m "refactor: split reporter null flavor fields"
```

---

### Task 4: Replace group registry rows with eleven companion rows

**Files:**
- Modify: `registry/sections/c-safety-report.json`
- Modify: `registry/presaves/sections/c-reporter.json`
- Modify: `registry/tools/extract_presave_fields.py`
- Modify: `registry/tools/test_extract_presave_fields.py`

**Interfaces:**
- Consumes: eleven backend/frontend field pairs from Tasks 1 and 3.
- Produces: exact case/presave joins and transfer inventory coverage.

- [ ] **Step 1: Write failing registry assertions**

Assert both namespaces omit the two group codes and contain all eleven codes
from the design. Assert transfer inventory includes, for example:

```python
("ReporterPresave.reporter_given_name_null_flavor",
 "PrimarySource.reporter_given_name_null_flavor")
("ReporterPresave.city_null_flavor", "PrimarySource.city_null_flavor")
```

- [ ] **Step 2: Verify RED**

```sh
python3 -m unittest registry/tools/test_extract_presave_fields.py registry/tools/test_validate.py
```

- [ ] **Step 3: Replace rows and normalization maps**

Delete `C.2.r.local.reporterNameNullFlavor` and
`C.2.r.local.reporterAddressNullFlavor` in both namespaces. Add the eleven
approved `C.2.r.local.<Field>NullFlavor` rows with `local_only: true`, mapped
backend/frontend evidence, and notes naming the official E2B element. Replace
the two group entries in both transfer name maps with eleven exact entries.

- [ ] **Step 4: Verify and commit**

```sh
python3 registry/tools/validate.py --strict-backend-inventory
python3 registry/tools/validate.py --strict-frontend-inventory
python3 registry/tools/validate.py --strict-presave-inventory
python3 -m unittest registry/tools/test_extract_presave_fields.py registry/tools/test_validate.py
git add registry
git commit -m "refactor: register reporter element null flavors"
```

---

### Task 5: Prove legacy removal and full integration

**Files:**
- Verify: backend and frontend files modified in Tasks 1–4; this task introduces no new production files.

**Interfaces:**
- Consumes: completed backend, XML, frontend, and registry changes.
- Produces: a clean hard-cutover verification result.

- [ ] **Step 1: Prove legacy names are absent**

```sh
rg -n 'reporterNameNullFlavor|reporterAddressNullFlavor|reporter_name_null_flavor|reporter_address_null_flavor' crates db registry ../frontend/E2BR3-frontend/app ../frontend/E2BR3-frontend/components ../frontend/E2BR3-frontend/lib ../frontend/E2BR3-frontend/__tests__
```

Expected: no matches.

- [ ] **Step 2: Run backend and registry verification**

```sh
cargo test -p lib-core
cargo test -p validator
cargo test -p web-server --test subresources_web
python3 -m unittest registry/tools/test_extract_frontend_fields.py registry/tools/test_extract_presave_fields.py registry/tools/test_presave_registry.py registry/tools/test_validate.py
python3 registry/tools/validate.py --strict-frontend-inventory
python3 registry/tools/validate.py --strict-presave-inventory
```

- [ ] **Step 3: Run frontend verification**

```sh
npx jest --runInBand __tests__/case-form/ReporterSection.reporter-null-flavors.test.tsx __tests__/dashboard/presave-minimal-form-validation.test.ts __tests__/dashboard/canonical-presave-mappers.test.ts __tests__/case-save/reporter.coordinator.test.ts
npx tsc --noEmit
git diff --check
```
