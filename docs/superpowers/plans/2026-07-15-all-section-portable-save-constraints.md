# All-Section Portable Save Constraints Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply Catalog-backed storage-representation validation to every editable Case Editor section, showing dirty-field errors below controls, disabling Save actions, and rejecting direct API bypasses before database mutation.

**Architecture:** The backend validation crate owns portable constraints and an explicit section/path binding manifest. The existing exporter generates deterministic TypeScript constraints and bindings, while the frontend reuses its current Zod, React Hook Form, and save-disabled flow. Backend direct and repeatable handlers call one shared manifest-driven pre-mutation gate; no authority selector, runtime Catalog API, new frontend store, or new error protocol is introduced.

**Tech Stack:** Rust, serde/serde_json, Axum, TypeScript, Zod, React Hook Form, Jest, Cargo tests.

## Global Constraints

- The Catalog remains the source of truth for rule values, limits, formats, nullFlavor sets, and messages.
- `PortableFieldBinding` is the only source of truth for Case Editor field-to-rule connections.
- Include maximum length, primitive type, E2B datetime/base64, complete inline allowed values, and allowed nullFlavor values.
- Exclude required/mandatory/conditional rules, business conditions, vocabulary/terminology, semantic date rules, submission rules, and XML structural rules.
- Do not branch portable frontend validation by ICH, FDA, or MFDS authority.
- Validate only supplied values on create and dirty/changed values on update.
- Declare `PortableValueType` from the actual form/API JSON representation, not from the Catalog constraint kind; an E2B numeric shape stored as text remains a string binding.
- Preserve concrete repeated indexes; do not use owner, alias, canonical-path, or sequence fallback.
- Keep the existing backend `400 Bad Request` protocol; include rule code and concrete path in its message.
- Do not add a runtime Catalog endpoint, cache, ETag, manual copy step, structured validation response, or frontend validation store.
- Work in isolated backend and frontend worktrees created from each repository's local `dev` branch.
- Use backend worktree `/Users/hyundonghoon/projects/rust/e2br3/e2br3/.worktrees/all-section-portable-save-constraints`.
- Use frontend worktree `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/.worktrees/all-section-portable-save-constraints`.
- Use branch `codex/all-section-portable-save-constraints` in both repositories.

## File Structure

Backend responsibilities:

- `crates/libs/validator/src/portable_constraints.rs`: authority-independent portable constraint projection and strict value evaluation.
- `crates/libs/validator/src/portable_bindings/mod.rs`: binding/exclusion types, aggregate APIs, and global coverage checks.
- `crates/libs/validator/src/portable_bindings/c.rs`: CI, RP, SD, LR, and SI bindings.
- `crates/libs/validator/src/portable_bindings/d.rs`: DM and DH bindings.
- `crates/libs/validator/src/portable_bindings/e.rs`: AE bindings.
- `crates/libs/validator/src/portable_bindings/f.rs`: LB bindings.
- `crates/libs/validator/src/portable_bindings/g.rs`: DG bindings, including nested repeated paths.
- `crates/libs/validator/src/portable_bindings/h.rs`: NR bindings.
- `crates/libs/validator/src/portable_bindings/n.rs`: message-header bindings rendered on SD.
- `crates/libs/validator/src/bin/export_zod_constraints.rs`: deterministic constraint and binding TypeScript generation.
- `crates/services/web-server/src/web/rest/case_editor_rest/portable_save.rs`: raw request traversal and shared pre-mutation validation.
- `crates/services/web-server/src/web/rest/case_editor_rest/direct.rs`: direct-page gate calls.
- `crates/services/web-server/src/web/rest/case_editor_rest/common.rs`: repeatable create/PATCH macro gate calls.

Frontend responsibilities:

- `lib/zod/generated/catalogConstraints.ts`: generated constraints; never edited manually.
- `lib/zod/generated/catalogBindings.ts`: generated section/path bindings; never edited manually.
- `lib/validation/syntax.ts`: generic object/repeated/nested traversal and Catalog issue evaluation.
- `lib/case-editor/validation-state.ts`: dirty-path issue selection.
- `components/case-form/hooks/useCaseEditorValidationState.ts`: apply only visible portable issues to React Hook Form.
- `components/case-form/E2BFormField.tsx`: render existing errors below controls.
- `lib/zod/sections/{ci,rp,sd,lr,si,dm,dh,ae,lb,dg,nr}.ts`: remove handwritten representation constraints replaced by generated bindings; retain only nonportable UI behavior that does not enter the portable save gate.

---

### Task 1: Generalize Portable Constraints To All Catalog Authorities

**Files:**
- Modify: `crates/libs/validator/src/portable_constraints.rs`
- Modify: `crates/libs/validator/src/lib.rs`
- Test: `crates/libs/validator/src/lib.rs`

**Interfaces:**
- Produces: `pub fn portable_constraints() -> Vec<PortableConstraint>`.
- Produces: `pub fn validate_portable_value(rule_code: &str, value: PortableInputValue<'_>, null_flavor: Option<&str>) -> Result<(), PortableConstraintViolation>`.
- Preserves temporarily: `portable_ich_constraints()` as a deprecated compatibility wrapper until Task 3 migrates the exporter.

- [ ] **Step 1: Write failing projection and strict-type tests**

Add tests proving that the projection contains representative ICH, FDA, and MFDS max-length/inline/nullFlavor rules, contains no vocabulary or required rule, and rejects a JSON boolean supplied to a string binding before value normalization.

```rust
#[test]
fn portable_projection_spans_catalog_authorities_without_business_rules() {
    let codes = portable_constraints()
        .into_iter()
        .map(|rule| rule.code)
        .collect::<BTreeSet<_>>();
    assert!(codes.contains("ICH.C.1.1.LENGTH.MAX"));
    assert!(codes.iter().any(|code| code.starts_with("FDA.") && code.ends_with(".LENGTH.MAX")));
    assert!(codes.iter().any(|code| code.starts_with("MFDS.") && code.ends_with(".LENGTH.MAX")));
    assert!(codes.iter().all(|code| !code.ends_with(".REQUIRED")));
    assert!(codes.iter().all(|code| !code.ends_with(".VOCABULARY")));
}

#[test]
fn portable_string_binding_rejects_boolean_input() {
    let result = validate_portable_value(
        "ICH.C.1.3.ALLOWED.VALUE",
        PortableInputValue::Boolean(true),
        None,
    );
    assert!(result.is_err());
}
```

- [ ] **Step 2: Run the tests and verify RED**

Run:

```bash
cargo test -p validator portable_projection_spans_catalog_authorities_without_business_rules
cargo test -p validator portable_string_binding_rejects_boolean_input
```

Expected: compilation fails because `portable_constraints` and `PortableInputValue` do not exist.

- [ ] **Step 3: Implement authority-independent projection and typed input**

Replace the ICH filters with projection over all regulatory authorities and add:

```diff
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PortableInputValue<'a> {
    Missing,
    String(&'a str),
    Boolean(bool),
    Number(&'a serde_json::Number),
    InvalidType,
}

-pub fn portable_ich_constraints() -> Vec<PortableConstraint> {
+pub fn portable_constraints() -> Vec<PortableConstraint> {
-    for rule in MAX_LENGTH_RULES.iter().filter(|rule| rule.authority == RegulatoryAuthority::Ich) {
+    for rule in MAX_LENGTH_RULES.iter() {
-    for rule in ALLOWED_VALUE_RULES.iter().filter(|rule| rule.authority == RegulatoryAuthority::Ich) {
+    for rule in ALLOWED_VALUE_RULES.iter() {
-    for rule in NULL_FLAVOR_RULES.iter().filter(|rule| rule.authority == RegulatoryAuthority::Ich) {
+    for rule in NULL_FLAVOR_RULES.iter() {
 }

+pub fn portable_ich_constraints() -> Vec<PortableConstraint> {
+    portable_constraints()
+        .into_iter()
+        .filter(|rule| rule.code.starts_with("ICH."))
+        .collect()
+}
```

The diff applies identically to the three existing Catalog loops; no new rule
classification is added in this task.

Make primitive mismatch return a `PortableConstraintViolation`; keep absent/blank optional values valid.

- [ ] **Step 4: Run validator tests and verify GREEN**

Run: `cargo test -p validator --lib`

Expected: all validator library tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/libs/validator/src/portable_constraints.rs crates/libs/validator/src/lib.rs
git commit -m "refactor: generalize portable catalog constraints"
```

### Task 2: Add Explicit Portable Field Binding Modules

**Files:**
- Create: `crates/libs/validator/src/portable_bindings/mod.rs`
- Create: `crates/libs/validator/src/portable_bindings/c.rs`
- Create: `crates/libs/validator/src/portable_bindings/d.rs`
- Create: `crates/libs/validator/src/portable_bindings/e.rs`
- Create: `crates/libs/validator/src/portable_bindings/f.rs`
- Create: `crates/libs/validator/src/portable_bindings/g.rs`
- Create: `crates/libs/validator/src/portable_bindings/h.rs`
- Create: `crates/libs/validator/src/portable_bindings/n.rs`
- Modify: `crates/libs/validator/src/lib.rs`
- Test: `crates/libs/validator/src/portable_bindings/mod.rs`

**Interfaces:**
- Produces: `PortableValueType::{String, Boolean, Number}`.
- Produces: `PortableFieldBinding { section, frontend_path, request_path, value_type, rule_codes, null_flavor_path }`.
- Produces: `PortableBindingExclusion { rule_code, reason }`.
- Produces: `portable_field_bindings() -> Vec<&'static PortableFieldBinding>`.
- Produces: `portable_binding_exclusions() -> Vec<&'static PortableBindingExclusion>`.
- Produces: `bindings_for_section(section: &str) -> impl Iterator<Item = &'static PortableFieldBinding>`.

- [ ] **Step 1: Write failing manifest integrity tests**

```rust
#[test]
fn every_binding_references_a_portable_catalog_rule() {
    let portable = portable_constraints()
        .into_iter()
        .map(|rule| rule.code)
        .collect::<BTreeSet<_>>();
    for binding in portable_field_bindings() {
        for code in binding.rule_codes {
            assert!(portable.contains(*code), "unknown portable rule {code}");
        }
    }
}

#[test]
fn binding_paths_are_explicit_and_fallback_free() {
    for binding in portable_field_bindings() {
        assert!(!binding.frontend_path.contains(".*"));
        assert!(!binding.request_path.contains(".*"));
        assert!(!binding.frontend_path.contains(".."));
    }
}
```

Also test uniqueness of `(section, frontend_path, rule_code)` and reject duplicate exclusions.

- [ ] **Step 2: Run the tests and verify RED**

Run: `cargo test -p validator portable_bindings`

Expected: compilation fails because the binding module does not exist.

- [ ] **Step 3: Implement manifest types and section modules**

Use static slices per E2B section:

```rust
pub struct PortableFieldBinding {
    pub section: &'static str,
    pub frontend_path: &'static str,
    pub request_path: &'static str,
    pub value_type: PortableValueType,
    pub rule_codes: &'static [&'static str],
    pub null_flavor_path: Option<&'static str>,
}
```

Seed every module with the listed representative editable path so Task 3 can
prove every traversal shape before the complete migrations in Tasks 7-9:

- `c.rs`: CI `safetyReportIdentification.reportType`, RP
  `primarySources[].reporterTitle`, SD
  `safetyReportIdentification.senderOrganization`, LR
  `literatureReferences[].literatureReference`, and SI
  `studyInformation.studyName`.
- `d.rs`: DM `patientInformation.patientInitials` and DH
  `patientInformation.pastDrugHistory[].drugName`.
- `e.rs`: AE `reactions[].primarySourceReaction`.
- `f.rs`: LB `testResults[].testName` and its numeric result value.
- `g.rs`: DG `drugs[].dosageInformation[].doseValue`.
- `h.rs`: NR `narrative.caseNarrative`.
- `n.rs`: SD `messageHeader.messageSenderIdentifier`.

Tasks 7-9 fill the remaining bindings. A known portable rule with no editable
Case Editor field may receive an explicit entry whose reason is one of
`not_in_case_editor_model`, `export_only`, or
`authority_dependent_business_value`. Do not add a generic catch-all exclusion.

- [ ] **Step 4: Run manifest integrity tests and verify GREEN**

Run: `cargo test -p validator portable_bindings`

Expected: all binding integrity tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/libs/validator/src/portable_bindings crates/libs/validator/src/lib.rs
git commit -m "feat: map portable catalog rules to editor fields"
```

### Task 3: Generate Frontend Constraints And Bindings Together

**Files:**
- Modify: `crates/libs/validator/src/bin/export_zod_constraints.rs`
- Test: `crates/libs/validator/src/bin/export_zod_constraints.rs`
- Frontend modify: `scripts/validation/sync-catalog-constraints.mjs`
- Frontend modify: `package.json`
- Frontend generate: `lib/zod/generated/catalogConstraints.ts`
- Frontend generate: `lib/zod/generated/catalogBindings.ts`
- Frontend test: `__tests__/validation.catalog-generated.test.ts`

**Interfaces:**
- `export-zod-constraints --constraints-output CONSTRAINTS_PATH --bindings-output BINDINGS_PATH` writes both artifacts.
- `--check-constraints CONSTRAINTS_PATH --check-bindings BINDINGS_PATH` exits nonzero on either drift.
- Generated binding type:

```ts
export type GeneratedCatalogBinding = {
  section: string;
  frontendPath: string;
  valueType: "string" | "boolean" | "number";
  ruleCodes: readonly string[];
  nullFlavorPath: string | null;
};
```

- [ ] **Step 1: Write failing deterministic-output tests**

Assert that generated output contains representative CI, DM, AE, LB, DG, NR,
and SD bindings, contains FDA/MFDS portable constraints, and is byte-identical
when rendered twice.

```rust
#[test]
fn renders_all_section_bindings_deterministically() {
    let first = render_bindings_typescript(&portable_field_bindings());
    let second = render_bindings_typescript(&portable_field_bindings());
    assert_eq!(first, second);
    for section in ["CI", "RP", "SD", "LR", "SI", "DM", "DH", "AE", "LB", "DG", "NR"] {
        assert!(first.contains(&format!("\"section\": \"{section}\"")));
    }
}
```

- [ ] **Step 2: Run generator tests and verify RED**

Run: `cargo test -p validator --bin export-zod-constraints`

Expected: compilation fails because binding rendering is absent.

- [ ] **Step 3: Implement two-file deterministic generation**

Sort constraints by rule code and bindings by `(section, frontend_path,
rule_codes)`. Serialize data with `serde_json::to_string_pretty`; do not build
JSON with string concatenation. Update the Node sync script to invoke one Cargo
command that writes/checks both files. Migrate the exporter to
`portable_constraints()` and remove the temporary `portable_ich_constraints()`
wrapper once `rg "portable_ich_constraints"` finds no other caller.

- [ ] **Step 4: Generate artifacts and run drift tests**

Run from the frontend worktree:

```bash
BACKEND_REPO=/Users/hyundonghoon/projects/rust/e2br3/e2br3/.worktrees/all-section-portable-save-constraints npm run sync:validation-catalog
BACKEND_REPO=/Users/hyundonghoon/projects/rust/e2br3/e2br3/.worktrees/all-section-portable-save-constraints npm run check:validation-catalog
npx jest __tests__/validation.catalog-generated.test.ts --runInBand
```

Expected: generation check and Jest test pass.

- [ ] **Step 5: Commit both repositories**

Backend:

```bash
git add crates/libs/validator/src/bin/export_zod_constraints.rs
git commit -m "feat: export portable editor bindings"
```

Frontend:

```bash
git add package.json scripts/validation/sync-catalog-constraints.mjs lib/zod/generated __tests__/validation.catalog-generated.test.ts
git commit -m "build: generate portable editor bindings"
```

### Task 4: Evaluate Generated Object, Repeated, And Nested Bindings In Zod

**Files:**
- Frontend modify: `lib/validation/syntax.ts`
- Frontend modify: `lib/zod/types.ts`
- Frontend test: `__tests__/validation.syntax.test.ts`

**Interfaces:**
- Produces: `collectPortableCatalogIssues(payload: unknown): SyntaxIssue[]`.
- Consumes: generated `catalogConstraints` and `catalogBindings`.
- Path grammar: dot-separated object segments plus `[]` repeated segments.

- [ ] **Step 1: Write failing traversal parity tests**

Cover these exact shapes:

```ts
it.each([
  ["CI object", { safetyReportIdentification: { reportType: "9" } }, "safetyReportIdentification.reportType"],
  ["AE row", { reactions: [{ primarySourceReaction: "x".repeat(251) }] }, "reactions.0.primarySourceReaction"],
  ["DG nested row", { drugs: [{ dosageInformation: [{ doseValue: "not-a-number" }] }] }, "drugs.0.dosageInformation.0.doseValue"],
])("emits a concrete path for %s", (_name, payload, path) => {
  expect(collectPortableCatalogIssues(payload)).toEqual(
    expect.arrayContaining([expect.objectContaining({ path })]),
  );
});
```

Also test wrong primitive type, blank optional value, explicit allowed
nullFlavor, and one failing vector for every portable constraint kind.

- [ ] **Step 2: Run tests and verify RED**

Run: `npx jest __tests__/validation.syntax.test.ts --runInBand`

Expected: nested/generated binding tests fail because only section-local rules are traversed.

- [ ] **Step 3: Implement generic generated-binding traversal**

Parse each path segment structurally. `[]` expands only arrays found at that
exact segment and appends each concrete numeric index. Do not search descendant
objects or retry a missing segment under another root. Resolve nullFlavor only
from the binding's explicit companion path at the same repeated-index context.

Reuse the existing max-length, numeric, E2B datetime, base64, inline-value, and
nullFlavor evaluators. Return all issues sorted by concrete path and rule code.

- [ ] **Step 4: Run syntax tests and typecheck**

```bash
npx jest __tests__/validation.syntax.test.ts --runInBand
npx tsc --noEmit
```

Expected: tests and typecheck pass.

- [ ] **Step 5: Commit**

```bash
git add lib/validation/syntax.ts lib/zod/types.ts __tests__/validation.syntax.test.ts
git commit -m "feat: evaluate generated editor constraints"
```

### Task 5: Show Only Dirty Portable Errors And Disable Save Actions

**Files:**
- Frontend modify: `lib/case-editor/validation-state.ts`
- Frontend modify: `components/case-form/hooks/useCaseEditorValidationState.ts`
- Frontend modify: `components/case-form/E2BFormField.tsx`
- Frontend test: `__tests__/case-form/CaseEditor.validation-errors.integration.test.ts`
- Frontend test: `__tests__/case-form/E2BFormField.actions.test.ts`

**Interfaces:**
- Produces: `visiblePortableIssues(issues, dirtyFields, hasPersistedBaseline)`.
- Consumes: `collectPortableCatalogIssues`; legacy or semantic issue collectors are not passed into this save-disabled calculation.
- Existing `hasBlockingSyntaxErrors` becomes true exactly when the visible portable issue list is nonempty.
- Existing `saveDisabled` wiring remains `hasBlockingSyntaxErrors || !hasUnsavedChanges`.

- [ ] **Step 1: Write failing dirty/display/button tests**

Add integration tests proving:

```ts
expect(latestLayoutProps().saveDisabled).toBe(false); // unrelated legacy issue
expect(methods.getFieldState("messageHeader.messageSenderIdentifier").error).toBeUndefined();

// After editing the invalid field:
expect(latestLayoutProps().saveDisabled).toBe(true);
expect(methods.getFieldState(path).error?.type).toBe("catalog");

// After correction:
expect(latestLayoutProps().saveDisabled).toBe(false);
expect(methods.getFieldState(path).error).toBeUndefined();
```

Render `E2BFormField` and assert the error element follows the control container
in DOM order rather than preceding it.

- [ ] **Step 2: Run tests and verify RED**

```bash
npx jest __tests__/case-form/CaseEditor.validation-errors.integration.test.ts __tests__/case-form/E2BFormField.actions.test.ts --runInBand
```

Expected: persisted untouched issues are still injected and error DOM order is above the control.

- [ ] **Step 3: Implement minimal filtering and error placement**

```ts
export function visiblePortableIssues(
  issues: SyntaxIssue[],
  dirty: unknown,
  hasPersistedBaseline: boolean,
): SyntaxIssue[] {
  if (!hasPersistedBaseline) return issues;
  return issues.filter((issue) => hasDirtyFieldAtPath(dirty, issue.path));
}
```

Replace the hook's save-gate call to the combined legacy collector with
`collectPortableCatalogIssues`. Use the dirty-filtered result for `setError`,
section error flags, and `hasBlockingSyntaxErrors`. Set errors with
`{ type: "catalog", message }`. Semantic backend validation remains on its
existing independent path.
Move the existing `error` block in `E2BFormField` and `E2BRadioField` below the
control row; do not introduce a new component or store.

- [ ] **Step 4: Run integration tests and typecheck**

```bash
npx jest __tests__/case-form/CaseEditor.validation-errors.integration.test.ts __tests__/case-form/E2BFormField.actions.test.ts --runInBand
npx tsc --noEmit
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add lib/case-editor/validation-state.ts components/case-form/hooks/useCaseEditorValidationState.ts components/case-form/E2BFormField.tsx __tests__/case-form/CaseEditor.validation-errors.integration.test.ts __tests__/case-form/E2BFormField.actions.test.ts
git commit -m "feat: block saves on edited catalog violations"
```

### Task 6: Add The Shared Backend Pre-Mutation Gate

**Files:**
- Create: `crates/services/web-server/src/web/rest/case_editor_rest/portable_save.rs`
- Modify: `crates/services/web-server/src/web/rest/case_editor_rest/mod.rs`
- Modify: `crates/services/web-server/src/web/rest/case_editor_rest/common.rs`
- Modify: `crates/services/web-server/src/web/rest/case_editor_rest/direct.rs`
- Test: `crates/services/web-server/src/web/rest/case_editor_rest/portable_save.rs`
- Test: `crates/services/web-server/tests/api/case_editor_contract_web.rs`
- Frontend test: `__tests__/case-form/case-editor-route-loading.test.tsx`

**Interfaces:**
- Produces: `validate_direct_changes(section: &str, changes: &BTreeMap<String, CaseEditorFieldPatch>) -> Result<()>`.
- Produces: `validate_row_payload(section: &str, row_key: &str, row: &Map<String, Value>, changed_paths: Option<&BTreeSet<String>>) -> Result<()>`.
- Produces errors through existing `Error::BadRequest { message }`.

- [ ] **Step 1: Write failing no-write and concrete-path tests**

Add unit tests for direct, row, and nested row payload traversal. Add API
contract tests that write a valid baseline, submit an invalid changed value,
expect `400`, reload the row, and assert the baseline value is unchanged.

Add a frontend route test proving that all eleven route-owned data-entry pages
provide `sectionScopedEditor` and therefore call `patchEditorPageProjection` or
`patchEditorPageRow` instead of their legacy page coordinator mutation tasks.
This task gates the current route-owned Case Editor APIs; unrelated legacy CRUD
screens are not silently treated as covered.

Required concrete vectors:

- CI direct invalid inline value;
- SD direct overlength string;
- AE row overlength string;
- LB row invalid numeric shape;
- DG nested dosage invalid numeric shape with `drugs.0.dosageInformation.1` path;
- nullFlavor not in its declared set.

- [ ] **Step 2: Run tests and verify RED**

```bash
cargo test -p web-server portable_save
cargo test -p web-server --test api case_editor_contract_web::portable_
npx jest __tests__/case-form/case-editor-route-loading.test.tsx --runInBand
```

Expected: invalid non-CI values currently reach model parsing or persistence.

- [ ] **Step 3: Implement exact path traversal and common gate calls**

Convert raw `serde_json::Value` to `PortableInputValue` according to each
binding's declared `PortableValueType`. Reject type mismatch before string
normalization. Expand only explicit `[]` segments and preserve indexes in the
error path.

Call `validate_direct_changes` before each direct handler constructs or writes
its model. Call `validate_row_payload` in both forms of
`repeatable_page_row_create_handler!` and every form of
`repeatable_page_row_patch_handler!`, before `$bmc::create`/`$bmc::update` and
before validation-cache mutation.

Replace `validate_ci_save_constraints` with the shared direct gate.

- [ ] **Step 4: Run backend gate tests**

```bash
cargo test -p web-server --lib
cargo test -p web-server --test api case_editor_contract_web::
```

Expected: web-server unit tests and all Case Editor API contract tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/services/web-server/src/web/rest/case_editor_rest crates/services/web-server/tests/api/case_editor_contract_web.rs
git commit -m "feat: enforce portable constraints before editor writes"
```

Frontend:

```bash
git add __tests__/case-form/case-editor-route-loading.test.tsx
git commit -m "test: keep editor saves on constrained routes"
```

### Task 7: Migrate C And N Case Editor Pages

**Files:**
- Frontend modify: `lib/zod/sections/ci.ts`
- Frontend modify: `lib/zod/sections/rp.ts`
- Frontend modify: `lib/zod/sections/sd.ts`
- Frontend modify: `lib/zod/sections/lr.ts`
- Frontend modify: `lib/zod/sections/si.ts`
- Frontend test: `__tests__/validation.syntax.test.ts`
- Test: `crates/libs/validator/src/portable_bindings/c.rs`
- Test: `crates/libs/validator/src/portable_bindings/n.rs`

**Interfaces:**
- Consumes generated bindings for page IDs CI, RP, SD, LR, and SI.
- Removes handwritten max/regex/email rules only when the manifest supplies the same portable constraint.

- [ ] **Step 1: Add failing page coverage tests**

For every frontend `fieldCoverage` path in CI/RP/SD/LR/SI that has a portable
Catalog rule, assert exactly one generated binding. Add representative dirty
issue vectors for each page and assert no required rule appears.

- [ ] **Step 2: Run C/N coverage tests and verify RED**

```bash
cargo test -p validator portable_bindings::c
cargo test -p validator portable_bindings::n
npx jest __tests__/validation.syntax.test.ts --runInBand
```

Expected: uncovered C/N fields and duplicate handwritten/generated issues are reported.

- [ ] **Step 3: Complete C/N bindings and remove duplicate handwritten rules**

Bind all portable `C.*` and `N.*` rules to their explicit page/form/request
paths. Retain only rules that are outside the portable categories and ensure
those retained rules do not enter the portable save-disabled result.

- [ ] **Step 4: Regenerate and verify C/N pages**

```bash
BACKEND_REPO=/Users/hyundonghoon/projects/rust/e2br3/e2br3/.worktrees/all-section-portable-save-constraints npm run sync:validation-catalog
BACKEND_REPO=/Users/hyundonghoon/projects/rust/e2br3/e2br3/.worktrees/all-section-portable-save-constraints npm run check:validation-catalog
npx jest __tests__/validation.syntax.test.ts --runInBand
```

Expected: C/N coverage and frontend tests pass without duplicate issues.

- [ ] **Step 5: Commit backend and frontend slices**

Backend: `git commit -am "feat: bind C and N portable editor fields"`

Frontend:

```bash
git add lib/zod/sections/ci.ts lib/zod/sections/rp.ts lib/zod/sections/sd.ts lib/zod/sections/lr.ts lib/zod/sections/si.ts lib/zod/generated __tests__/validation.syntax.test.ts
git commit -m "refactor: use catalog constraints across C and N pages"
```

### Task 8: Migrate D, E, And F Case Editor Pages

**Files:**
- Frontend modify: `lib/zod/sections/dm.ts`
- Frontend modify: `lib/zod/sections/dh.ts`
- Frontend modify: `lib/zod/sections/ae.ts`
- Frontend modify: `lib/zod/sections/lb.ts`
- Frontend test: `__tests__/validation.syntax.test.ts`
- Test: `crates/libs/validator/src/portable_bindings/{d,e,f}.rs`

**Interfaces:**
- Consumes generated bindings for DM, DH, AE, and LB.
- Must preserve concrete indexes for patient history, parent history, reactions, and test results.

- [ ] **Step 1: Add failing D/E/F coverage and nested path tests**

Assert one binding for every portable fieldCoverage path and add vectors for:

- `patientInformation.medicalHistoryEpisodes.1.comments`;
- `patientInformation.parentInformation.pastDrugHistory.1.drugName`;
- `reactions.1.primarySourceReaction`;
- `testResults.1.testName` and numeric result fields.

- [ ] **Step 2: Run D/E/F tests and verify RED**

```bash
cargo test -p validator portable_bindings::d
cargo test -p validator portable_bindings::e
cargo test -p validator portable_bindings::f
npx jest __tests__/validation.syntax.test.ts --runInBand
```

Expected: uncovered paths or duplicate legacy representation issues fail.

- [ ] **Step 3: Complete D/E/F bindings and remove duplicate rules**

Map every portable D/E/F Catalog code explicitly. Declare nullFlavor companions
at the same repeated context. Do not fabricate nullFlavor paths for model fields
that do not have one.

- [ ] **Step 4: Regenerate and verify D/E/F pages**

```bash
BACKEND_REPO=/Users/hyundonghoon/projects/rust/e2br3/e2br3/.worktrees/all-section-portable-save-constraints npm run sync:validation-catalog
npx jest __tests__/validation.syntax.test.ts --runInBand
cargo test -p validator portable_bindings
```

Expected: all commands pass.

- [ ] **Step 5: Commit backend and frontend slices**

Backend: `git commit -am "feat: bind D E and F portable editor fields"`

Frontend:

```bash
git add lib/zod/sections/dm.ts lib/zod/sections/dh.ts lib/zod/sections/ae.ts lib/zod/sections/lb.ts lib/zod/generated __tests__/validation.syntax.test.ts
git commit -m "refactor: use catalog constraints across D E and F pages"
```

### Task 9: Migrate G And H Case Editor Pages

**Files:**
- Frontend modify: `lib/zod/sections/dg.ts`
- Frontend modify: `lib/zod/sections/nr.ts`
- Frontend test: `__tests__/validation.syntax.test.ts`
- Test: `crates/libs/validator/src/portable_bindings/{g,h}.rs`

**Interfaces:**
- Consumes generated bindings for DG and NR.
- Must support nested paths under indications, assessments, active substances, dosage information, device information, and narrative text.

- [ ] **Step 1: Add failing G/H coverage and multi-index tests**

Add vectors with nonzero indexes for:

```text
drugs.1.activeSubstances.2.substanceName
drugs.1.dosageInformation.2.doseValue
drugs.1.drugReactionAssessments.2.sourceOfAssessment
drugs.1.fdaDeviceInfo.deviceProblemCodes.2.valueCode
narrative.caseNarrative
```

Assert returned paths preserve every index and no owner/sequence fallback path
is emitted.

- [ ] **Step 2: Run G/H tests and verify RED**

```bash
cargo test -p validator portable_bindings::g
cargo test -p validator portable_bindings::h
npx jest __tests__/validation.syntax.test.ts --runInBand
```

Expected: nested uncovered paths fail.

- [ ] **Step 3: Complete G/H bindings and remove duplicate rules**

Map every portable G/H rule explicitly. A request path for a nested child must
name the complete child chain; do not reduce it to the owning drug row.

- [ ] **Step 4: Regenerate and verify G/H pages**

```bash
BACKEND_REPO=/Users/hyundonghoon/projects/rust/e2br3/e2br3/.worktrees/all-section-portable-save-constraints npm run sync:validation-catalog
npx jest __tests__/validation.syntax.test.ts --runInBand
cargo test -p validator portable_bindings
```

Expected: all commands pass.

- [ ] **Step 5: Commit backend and frontend slices**

Backend: `git commit -am "feat: bind G and H portable editor fields"`

Frontend:

```bash
git add lib/zod/sections/dg.ts lib/zod/sections/nr.ts lib/zod/generated __tests__/validation.syntax.test.ts
git commit -m "refactor: use catalog constraints across G and H pages"
```

### Task 10: Enforce Complete Coverage And Run Final Verification

**Files:**
- Modify: `crates/libs/validator/src/portable_bindings/mod.rs`
- Frontend modify: `__tests__/validation.catalog-generated.test.ts`
- Frontend modify: `__tests__/case-form/CaseEditor.validation-errors.integration.test.ts`
- Modify: `docs/superpowers/specs/2026-07-15-all-section-portable-save-constraints-design.md` only if implementation discoveries require a factual clarification.

**Interfaces:**
- Produces a hard coverage gate where every portable rule is bound or explicitly excluded exactly once.
- Produces final backend/frontend parity evidence.

- [ ] **Step 1: Write the final failing global coverage test**

```rust
#[test]
fn every_portable_rule_is_bound_or_explicitly_excluded_once() {
    let expected = portable_constraints().into_iter().map(|rule| rule.code).collect::<BTreeSet<_>>();
    let bound = portable_field_bindings().into_iter()
        .flat_map(|binding| binding.rule_codes.iter().copied())
        .collect::<BTreeSet<_>>();
    let excluded = portable_binding_exclusions().into_iter()
        .map(|entry| entry.rule_code)
        .collect::<BTreeSet<_>>();
    assert!(bound.is_disjoint(&excluded));
    assert_eq!(expected, bound.union(&excluded).copied().map(str::to_owned).collect());
}
```

Add a generated frontend assertion that emitted rule-code/path pairs exactly
equal backend bindings and contain all eleven Case Editor page IDs.

- [ ] **Step 2: Run global coverage tests and verify RED if anything remains**

```bash
cargo test -p validator every_portable_rule_is_bound_or_explicitly_excluded_once
BACKEND_REPO=/Users/hyundonghoon/projects/rust/e2br3/e2br3/.worktrees/all-section-portable-save-constraints npm run check:validation-catalog
npx jest __tests__/validation.catalog-generated.test.ts --runInBand
```

Expected: any remaining omission is named by rule code; no count-only assertion is accepted.

- [ ] **Step 3: Resolve every named omission explicitly**

For each failure, add the concrete binding or one specific exclusion with an
allowed reason. Reject authority-conflicting constraints at generation time;
never apply an implicit allowed-value intersection.

- [ ] **Step 4: Run full fresh verification**

Backend:

```bash
cargo fmt --all -- --check
cargo test -p validator --lib
cargo test -p validator --bin export-zod-constraints
cargo test -p web-server --lib
cargo test -p web-server --test api case_editor_contract_web::
```

Frontend:

```bash
BACKEND_REPO=/Users/hyundonghoon/projects/rust/e2br3/e2br3/.worktrees/all-section-portable-save-constraints npm run check:validation-catalog
npx jest __tests__/validation.catalog-generated.test.ts __tests__/validation.syntax.test.ts __tests__/case-form/CaseEditor.validation-errors.integration.test.ts __tests__/case-form/E2BFormField.actions.test.ts --runInBand
npx tsc --noEmit
```

Expected: all commands exit zero. Existing `with-rpc` cfg warnings may remain,
but no new warning or generated drift is accepted.

- [ ] **Step 5: Commit final coverage gates**

Backend:

```bash
git add crates/libs/validator/src/portable_bindings
git commit -m "test: enforce portable editor constraint coverage"
```

Frontend:

```bash
git add lib/zod/generated __tests__/validation.catalog-generated.test.ts __tests__/validation.syntax.test.ts __tests__/case-form/CaseEditor.validation-errors.integration.test.ts __tests__/case-form/E2BFormField.actions.test.ts
git commit -m "test: cover portable constraints across editor pages"
```
