# ICH CI Catalog Zod Save Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove the Catalog-driven architecture end to end for the ICH CI page: generate frontend constraints from the Rust Catalog, use them in the existing Zod syntax flow, and reject the same invalid direct API patch before persistence.

**Architecture:** The Rust `validator` crate exposes a portable, vocabulary-free constraint DTO and evaluator. A small binary in the same crate writes one generated TypeScript map directly into the frontend repository. Existing CI field definitions retain only field placement and Catalog rule codes; the Case Editor CI PATCH handler applies those same rule codes before calling the BMC update.

**Tech Stack:** Rust 2021, serde/serde_json, existing `validator` Catalog and rule evaluators, Axum Case Editor REST, TypeScript, Zod 4, Jest.

## Global Constraints

- This phase covers ICH CI (`C.1`) only; FDA/MFDS and other editor pages are later migrations.
- Include only `max_length`, numeric shape, portable format, `inline_allowed_values`, and `null_flavor`.
- Exclude every vocabulary/terminology-backed constraint, including MedDRA, EDQM, MFDS products, WHODrug, and `ich_identifier` because its country check uses terminology data.
- Required, conditional, companion, future-date, exporter, and submission rules must not enter the save gate.
- The frontend generated file is never edited manually.
- No runtime Catalog API, cache, ETag, polling, or terminology lookup.
- ICH constraints always apply; request-provided authorities cannot disable them.
- Work in the existing backend worktree and create a clean frontend worktree before frontend edits because the frontend checkout has user-owned changes.

## File Structure

Backend:

- Create `crates/libs/validator/src/portable_constraints.rs`: serializable portable constraint DTO, Catalog projection, nullFlavor values, and single-value evaluator.
- Create `crates/libs/validator/src/bin/export_zod_constraints.rs`: deterministic TypeScript generator with `--output` and `--check`.
- Modify `crates/libs/validator/src/lib.rs`: export the portable constraint API.
- Modify `crates/libs/validator/src/catalog.rs`: expose the embedded ICH nullFlavor values required by the portable projection.
- Modify `crates/services/web-server/src/web/rest/case_editor_rest/direct.rs`: bind CI request fields to ICH rule codes and run the gate before `SafetyReportIdentificationBmc::update_by_case`.
- Modify `crates/services/web-server/tests/api/case_editor_contract_web.rs`: prove invalid CI direct API input is rejected and not persisted.

Frontend worktree:

- Create `lib/zod/generated/catalogConstraints.ts`: generated constraint map.
- Create `scripts/validation/sync-catalog-constraints.mjs`: run the backend generator against the frontend output path.
- Modify `package.json`: add `sync:validation-catalog` and `check:validation-catalog`.
- Modify `lib/zod/types.ts`: add a Catalog-backed field rule variant while retaining legacy rules for unmigrated pages.
- Modify `lib/zod/sections/ci.ts`: replace migrated handwritten CI values with `ruleCode` bindings and remove CI required save rules.
- Modify `lib/validation/syntax.ts`: resolve Catalog-backed rules into Zod schemas.
- Modify `__tests__/validation.syntax.test.ts`: test generated max length, format, inline values, nullFlavor, and non-blocking requiredness.
- Create `__tests__/validation.catalog-generated.test.ts`: reject unsupported or stale generated content.

---

### Task 1: Portable Rust Constraint Projection

**Files:**
- Create: `crates/libs/validator/src/portable_constraints.rs`
- Modify: `crates/libs/validator/src/catalog.rs`
- Modify: `crates/libs/validator/src/lib.rs`
- Test: `crates/libs/validator/src/portable_constraints.rs`

**Interfaces:**
- Consumes: `max_length_for_rule`, `allowed_value_constraint_for_rule`, `AllowedValueConstraintKind`, `NumericShape`, `FormatName`, and ICH dictionary nullFlavor metadata.
- Produces: `PortableConstraint`, `PortableConstraintKind`, `portable_ich_constraints()`, and `validate_portable_value()`.

- [ ] **Step 1: Add failing projection tests**

Add tests covering one representative of every included kind and every excluded kind:

```rust
#[test]
fn projects_only_portable_ich_constraints() {
    let rules = portable_ich_constraints();
    let by_code = rules.iter().map(|rule| (rule.code.as_str(), rule)).collect::<HashMap<_, _>>();

    assert_eq!(by_code["ICH.C.1.1.LENGTH.MAX"].kind, PortableConstraintKind::MaxLength);
    assert_eq!(by_code["ICH.C.1.3.ALLOWED.VALUE"].kind, PortableConstraintKind::InlineAllowedValues);
    assert_eq!(by_code["ICH.C.1.2.ALLOWED.VALUE"].kind, PortableConstraintKind::Format);
    assert!(by_code.contains_key("ICH.C.1.7.NULLFLAVOR.ALLOWED"));
    assert!(!by_code.contains_key("ICH.C.1.8.1.ALLOWED.VALUE")); // ich_identifier uses terminology
    assert!(!rules.iter().any(|rule| rule.code.ends_with(".VOCABULARY")));
    assert!(!rules.iter().any(|rule| rule.code.ends_with(".REQUIRED")));
}

#[test]
fn portable_evaluator_matches_ci_catalog_values() {
    assert!(validate_portable_value("ICH.C.1.3.ALLOWED.VALUE", Some("1"), None).is_ok());
    assert!(validate_portable_value("ICH.C.1.3.ALLOWED.VALUE", Some("9"), None).is_err());
    assert!(validate_portable_value("ICH.C.1.1.LENGTH.MAX", Some(&"X".repeat(100)), None).is_ok());
    assert!(validate_portable_value("ICH.C.1.1.LENGTH.MAX", Some(&"X".repeat(101)), None).is_err());
}
```

- [ ] **Step 2: Run the tests and verify failure**

Run:

```bash
cargo test -p validator --lib portable_constraints -- --nocapture
```

Expected: FAIL because `portable_constraints` and its public functions do not exist.

- [ ] **Step 3: Expose ICH nullFlavor values from the embedded dictionary**

Extend `EmbeddedDictionaryEntry` in `catalog.rs` to retain `null_flavors` and add:

```rust
pub fn null_flavors_for_rule(code: &str) -> Option<Vec<String>> {
    let element_code = code
        .strip_prefix("ICH.")?
        .strip_suffix(".NULLFLAVOR.ALLOWED")?;
    embedded_ich_dictionary()
        .entries
        .iter()
        .find(|entry| entry.code == element_code)
        .map(|entry| entry.null_flavors.clone())
}
```

Refactor the current `OnceLock` dictionary parsing so allowed-value and nullFlavor accessors share one parsed `EmbeddedDictionary`; do not parse the JSON twice.

- [ ] **Step 4: Implement the portable DTO and projection**

Create these public types in `portable_constraints.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PortableConstraintKind {
    MaxLength,
    Numeric,
    Format,
    InlineAllowedValues,
    NullFlavor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableConstraint {
    pub code: String,
    pub kind: PortableConstraintKind,
    pub max_length: Option<usize>,
    pub values: Vec<String>,
    pub numeric_shape: Option<NumericShape>,
    pub format_name: Option<FormatName>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortableConstraintViolation {
    pub code: String,
    pub message: String,
}
```

`portable_ich_constraints()` must:

- project every `MAX_LENGTH_RULES` entry whose authority is ICH;
- project ICH allowed-value entries when kind is `CodeSet`, `Boolean`, `TrueMarker`, `Numeric`, or portable `Format`; do not filter on the existing `enforcement` marker because C.1.2 format and C.1.3 code-set metadata are currently marked `case_validate` but this save-gate scope explicitly classifies malformed present representations as save-blocking;
- map `CodeSet`, `Boolean`, and `TrueMarker` to `InlineAllowedValues` using complete Catalog values (`Boolean` becomes `false,true`; `TrueMarker` becomes `true` plus its nullFlavor handling);
- include `E2bDatetime` and `Base64`, but exclude `IchIdentifier`;
- project ICH `NULL_FLAVOR_RULES` using `null_flavors_for_rule()`;
- sort by `code` and reject duplicate codes in a test.

`validate_portable_value(rule_code, value, null_flavor)` returns `Ok(())` for absent optional values and returns `PortableConstraintViolation` only when a present representation violates the selected rule. Reuse `validate_allowed_value_constraint` for numeric/format/inline values instead of reimplementing its parsing.

- [ ] **Step 5: Export the module and run tests**

Add to `lib.rs`:

```rust
mod portable_constraints;
pub use portable_constraints::*;
```

Run:

```bash
cargo test -p validator --lib portable_constraints -- --nocapture
cargo test -p validator --lib
```

Expected: portable tests pass; full validator baseline remains 153 or increases only by the new tests.

- [ ] **Step 6: Commit**

```bash
git add crates/libs/validator/src/catalog.rs crates/libs/validator/src/lib.rs crates/libs/validator/src/portable_constraints.rs
git commit -m "feat: expose portable catalog constraints"
```

### Task 2: Deterministic TypeScript Generator

**Files:**
- Create: `crates/libs/validator/src/bin/export_zod_constraints.rs`
- Modify: `crates/libs/validator/Cargo.toml`
- Test: `crates/libs/validator/src/bin/export_zod_constraints.rs`

**Interfaces:**
- Consumes: `validator::portable_ich_constraints()`.
- Produces: CLI `export-zod-constraints --output <path>` and `export-zod-constraints --check <path>`.

- [ ] **Step 1: Write failing formatter tests**

Factor output into `fn render_typescript(rules: &[PortableConstraint]) -> String` and test:

```rust
#[test]
fn output_is_sorted_and_marks_file_generated() {
    let output = render_typescript(&portable_ich_constraints());
    assert!(output.starts_with("// Generated from the backend validation Catalog. Do not edit.\n"));
    assert!(output.contains("export const catalogConstraints ="));
    assert!(output.find("ICH.C.1.1.LENGTH.MAX") < output.find("ICH.C.1.2.ALLOWED.VALUE"));
}
```

- [ ] **Step 2: Verify the test fails**

Run:

```bash
cargo test -p validator --bin export-zod-constraints -- --nocapture
```

Expected: FAIL because the binary does not exist.

- [ ] **Step 3: Implement the binary without a new crate**

Use `std::env::args`, `std::fs`, and `serde_json`; do not add Clap. Render JSON as a TypeScript literal:

```rust
fn render_typescript(rules: &[PortableConstraint]) -> String {
    let json = serde_json::to_string_pretty(rules).expect("portable constraints serialize");
    format!(
        "// Generated from the backend validation Catalog. Do not edit.\n\
         export const catalogConstraints = {json} as const;\n"
    )
}
```

`--output` creates parent directories and writes only when contents differ. `--check` compares generated contents and exits with code 1 plus `catalog constraints are stale: <path>` when missing or different.

- [ ] **Step 4: Run binary tests and deterministic smoke check**

Run:

```bash
cargo test -p validator --bin export-zod-constraints
tmp_file="$(mktemp)"
cargo run -p validator --bin export-zod-constraints -- --output "$tmp_file"
cargo run -p validator --bin export-zod-constraints -- --check "$tmp_file"
```

Expected: all commands exit 0 and the second invocation produces no file diff.

- [ ] **Step 5: Commit**

```bash
git add crates/libs/validator/Cargo.toml crates/libs/validator/src/bin/export_zod_constraints.rs
git commit -m "feat: generate frontend catalog constraints"
```

### Task 3: Frontend Sync Command And Generated Artifact

**Files:**
- Create: `scripts/validation/sync-catalog-constraints.mjs`
- Create: `lib/zod/generated/catalogConstraints.ts`
- Modify: `package.json`
- Create frontend worktree before edits.

**Interfaces:**
- Consumes: backend `export-zod-constraints` binary.
- Produces: `npm run sync:validation-catalog` and `npm run check:validation-catalog`.

- [ ] **Step 1: Create a clean frontend worktree**

From the frontend repository:

```bash
git worktree add .worktrees/catalog-zod-save-constraints -b codex/catalog-zod-save-constraints dev
```

`dev` resolves to `89a9cdff1b7a101c8b40447f4285056c4bf71ba6` at plan-writing time. Do not include the dirty checkout's local changes.

- [ ] **Step 2: Add the sync script**

The script resolves:

```js
const frontendRoot = path.resolve(import.meta.dirname, "../..");
const backendRoot = process.env.BACKEND_REPO
  ? path.resolve(process.env.BACKEND_REPO)
  : path.resolve(frontendRoot, "../../e2br3");
const output = path.join(frontendRoot, "lib/zod/generated/catalogConstraints.ts");
```

Accept only `--write` or `--check`, then run:

```js
spawnSync("cargo", [
  "run", "--quiet",
  "--manifest-path", path.join(backendRoot, "Cargo.toml"),
  "-p", "validator", "--bin", "export-zod-constraints", "--",
  mode === "--check" ? "--check" : "--output",
  output,
], { stdio: "inherit" });
```

Propagate the child exit status and fail when the backend checkout does not exist.

- [ ] **Step 3: Add package scripts and generate the file**

Add:

```json
"sync:validation-catalog": "node scripts/validation/sync-catalog-constraints.mjs --write",
"check:validation-catalog": "node scripts/validation/sync-catalog-constraints.mjs --check"
```

Run with the backend worktree explicitly:

```bash
BACKEND_REPO=/Users/hyundonghoon/projects/rust/e2br3/e2br3/.worktrees/catalog-zod-save-constraints npm run sync:validation-catalog
BACKEND_REPO=/Users/hyundonghoon/projects/rust/e2br3/e2br3/.worktrees/catalog-zod-save-constraints npm run check:validation-catalog
```

Expected: the generated TS file is created, then check exits 0.

- [ ] **Step 4: Commit in the frontend worktree**

```bash
git add package.json scripts/validation/sync-catalog-constraints.mjs lib/zod/generated/catalogConstraints.ts
git commit -m "build: generate catalog constraints"
```

### Task 4: Catalog-Backed CI Zod Rules

**Files:**
- Modify: `lib/zod/types.ts`
- Modify: `lib/zod/sections/ci.ts`
- Modify: `lib/validation/syntax.ts`
- Modify: `__tests__/validation.syntax.test.ts`
- Create: `__tests__/validation.catalog-generated.test.ts`

**Interfaces:**
- Consumes: `catalogConstraints` generated in Task 3.
- Produces: `FrontendCatalogFieldRule` and Catalog-backed `schemaForRule()` behavior.

- [ ] **Step 1: Add failing generated-Catalog tests**

Add tests proving:

```ts
it("uses generated CI constraints", () => {
  expect(collectSyntaxIssues({ safetyReportIdentification: { reportType: "9" } }))
    .toEqual(expect.arrayContaining([
      expect.objectContaining({ path: "safetyReportIdentification.reportType" }),
    ]));
});

it("does not block a draft for missing required CI values", () => {
  expect(collectSyntaxIssues({ safetyReportIdentification: {} }))
    .toEqual([]);
});
```

In `validation.catalog-generated.test.ts`, assert that all CI `ruleCode` values exist in `catalogConstraints` and that no generated entry has vocabulary/descriptive kind.

- [ ] **Step 2: Verify failures**

Run:

```bash
npx jest __tests__/validation.syntax.test.ts __tests__/validation.catalog-generated.test.ts --runInBand
```

Expected: FAIL because Catalog field rules and the generated-map evaluator do not exist; the missing-required test also fails under current required collection.

- [ ] **Step 3: Add a backward-compatible Catalog rule variant**

In `types.ts` define:

```ts
export interface FrontendCatalogFieldRule {
  field: string;
  kind: "catalog";
  ruleCode: string;
  valueType: "string" | "boolean" | "number";
}

export type FrontendSyntaxFieldRule = FrontendLegacyFieldRule | FrontendCatalogFieldRule;
```

Build `catalogConstraintsByCode` as a `Map<string, GeneratedCatalogConstraint>` in `syntax.ts`. `valueType` describes only the Case Editor wire representation; it does not duplicate Catalog semantics. Keep legacy `max`, `regex`, and `email` variants for unmigrated pages. During module initialization, throw if a CI binding references a missing generated rule. Remove `required` from issue collection so requiredness no longer blocks draft saves.

- [ ] **Step 4: Resolve generated constraints into Zod**

For `kind: "catalog"`, first reject a present value whose JavaScript primitive type differs from `valueType`; do not run it through the existing `coerceToString`. Normalize a correctly typed boolean to `"true"` or `"false"` only for comparison with generated values. Then support:

```ts
switch (constraint.kind) {
  case "max_length":
    return strictCatalogPrimitive(rule.valueType).superRefine((value, ctx) => {
      if (value !== "" && String(value).length > constraint.maxLength!) {
        ctx.addIssue({ code: "custom", message: constraint.message });
      }
    });
  case "inline_allowed_values":
  case "null_flavor":
    return strictCatalogPrimitive(rule.valueType).refine(
      (value) => value === "" || constraint.values.includes(String(value)),
      constraint.message,
    );
  case "numeric":
    return numericSchema(constraint.numericShape, constraint.message);
  case "format":
    return formatSchema(constraint.formatName, constraint.message);
}
```

Implement `e2b_datetime` and `base64` with the existing frontend format helpers. Do not implement `ich_identifier` or any vocabulary branch; throw during module initialization if generated content contains an unsupported kind.

- [ ] **Step 5: Replace CI handwritten semantics with rule-code bindings**

For the CI fields currently supported by Catalog and the editor payload, replace repeated `max`, allowed-value, datetime, and nullFlavor values with entries such as:

```ts
{ field: "reportType", kind: "catalog", valueType: "string", ruleCode: "ICH.C.1.3.ALLOWED.VALUE" },
{ field: "reportType", kind: "catalog", valueType: "string", ruleCode: "ICH.C.1.3.LENGTH.MAX" },
{ field: "transmissionDate", kind: "catalog", valueType: "string", ruleCode: "ICH.C.1.2.ALLOWED.VALUE" },
{ field: "nullificationReason", kind: "catalog", valueType: "string", ruleCode: "ICH.C.1.11.2.LENGTH.MAX" },
```

Delete CI `kind: "required"` entries. Leave FDA/MFDS entries and unmigrated ICH rules as legacy entries in this phase.

- [ ] **Step 6: Run frontend tests and typecheck**

Run:

```bash
npx jest __tests__/validation.syntax.test.ts __tests__/validation.catalog-generated.test.ts __tests__/case-editor/validation-state.test.ts --runInBand
npx tsc --noEmit
```

Expected: all selected tests pass and TypeScript exits 0.

- [ ] **Step 7: Commit in the frontend worktree**

```bash
git add lib/zod/types.ts lib/zod/sections/ci.ts lib/validation/syntax.ts __tests__/validation.syntax.test.ts __tests__/validation.catalog-generated.test.ts
git commit -m "refactor: drive CI zod rules from catalog"
```

### Task 5: Backend CI Direct-API Save Gate

**Files:**
- Modify: `crates/services/web-server/src/web/rest/case_editor_rest/direct.rs`
- Modify: `crates/services/web-server/tests/api/case_editor_contract_web.rs`

**Interfaces:**
- Consumes: `validator::validate_portable_value()` and CI `CaseEditorFieldPatch` values.
- Produces: `validate_ci_save_constraints(changes) -> Result<()>`, called before BMC update.

- [ ] **Step 1: Add a failing API persistence test**

Following the existing authenticated Case Editor test setup:

1. Create a case with CI `reportType = "1"`.
2. PATCH `/api/cases/{case_id}/editor/pages/CI` with `reportType = "9"`.
3. Assert a 400-series response containing `ICH.C.1.3.ALLOWED.VALUE`.
4. GET the CI page and assert `reportType` remains `"1"`.

Name the test `ci_patch_rejects_catalog_constraint_before_write`.

- [ ] **Step 2: Run the API test and verify failure**

Run:

```bash
cargo test -p web-server --test api ci_patch_rejects_catalog_constraint_before_write -- --nocapture
```

Expected: FAIL because the PATCH currently persists `"9"` and only refreshes semantic validation afterward.

- [ ] **Step 3: Implement the narrow CI binding and gate**

Add a local helper in `direct.rs`:

```rust
fn validate_ci_save_constraints(
    changes: &BTreeMap<String, CaseEditorFieldPatch>,
) -> Result<()> {
    const BINDINGS: &[(&str, &[&str])] = &[
        ("reportType", &["ICH.C.1.3.LENGTH.MAX", "ICH.C.1.3.ALLOWED.VALUE"]),
        ("fulfilExpeditedCriteriaNullFlavor", &["ICH.C.1.7.NULLFLAVOR.ALLOWED"]),
        ("otherCaseIdentifiersExist", &["ICH.C.1.9.1.ALLOWED.VALUE"]),
        ("otherCaseIdentifiersExistNullFlavor", &["ICH.C.1.9.1.NULLFLAVOR.ALLOWED"]),
    ];
    // Extract only present primitive patch values, evaluate every bound rule,
    // and convert a violation to the existing Error::BadRequest response.
}
```

Call it immediately after request context/permission validation and before building or applying `SafetyReportIdentificationForUpdate`. Apply ICH bindings unconditionally. Preserve the concrete field name in the message:

```text
ICH.C.1.3.ALLOWED.VALUE at safetyReportIdentification.reportType: <catalog message>
```

Do not run required, future-date, vocabulary, FDA, or MFDS rules.

- [ ] **Step 4: Add focused unit tests for boolean and nullFlavor extraction**

Add tests beside `direct.rs` proving:

- string `reportType = 9` fails;
- missing `reportType` is ignored;
- valid `fulfilExpeditedCriteriaNullFlavor` passes;
- invalid nullFlavor fails;
- an object value is rejected by the existing primitive patch parser before persistence.

- [ ] **Step 5: Run backend verification**

Run:

```bash
cargo test -p validator --lib
cargo test -p web-server --test api case_editor_contract_web -- --nocapture
cargo test -p web-server --lib
```

Expected: all tests pass; the new API test confirms no DB change.

- [ ] **Step 6: Commit**

```bash
git add crates/services/web-server/src/web/rest/case_editor_rest/direct.rs crates/services/web-server/tests/api/case_editor_contract_web.rs
git commit -m "feat: reject invalid CI patches before write"
```

### Task 6: End-To-End Parity And Documentation

**Files:**
- Modify: `docs/superpowers/specs/2026-07-14-catalog-zod-save-constraints-design.md`
- Modify frontend: `__tests__/validation.catalog-generated.test.ts`

**Interfaces:**
- Consumes: Rust generated artifact and frontend evaluator.
- Produces: a reproducible CI vertical slice and an explicit expansion inventory.

- [ ] **Step 1: Add parity vectors to the generated artifact test**

For CI, run the following vectors through `collectSyntaxIssues` and assert the expected rule/path behavior:

```ts
[
  ["reportType", "1", true],
  ["reportType", "9", false],
  ["transmissionDate", "20260715120000+0900", true],
  ["transmissionDate", "2026-07-15", false],
  ["nullificationReason", "X".repeat(2000), true],
  ["nullificationReason", "X".repeat(2001), false],
]
```

- [ ] **Step 2: Run final frontend verification**

Run in the frontend worktree:

```bash
BACKEND_REPO=/Users/hyundonghoon/projects/rust/e2br3/e2br3/.worktrees/catalog-zod-save-constraints npm run check:validation-catalog
npx jest __tests__/validation.syntax.test.ts __tests__/validation.catalog-generated.test.ts __tests__/case-editor/validation-state.test.ts --runInBand
npx tsc --noEmit
```

Expected: generated check, tests, and typecheck all pass.

- [ ] **Step 3: Run final backend verification**

Run:

```bash
cargo test -p validator --lib
cargo test -p web-server --test api case_editor_contract_web -- --nocapture
cargo test -p web-server --lib
```

Expected: all commands pass with no warnings introduced by this change.

- [ ] **Step 4: Record the completed slice and remaining pages**

Update the design document migration section to state that CI is implemented and list remaining pages exactly: `RP`, `SD`, `LR`, `SI`, `DM`, `DH`, `AE`, `LB`, `DG`, and `NR`. Do not claim those pages are protected by the new gate.

- [ ] **Step 5: Commit documentation and parity tests**

Backend:

```bash
git add docs/superpowers/specs/2026-07-14-catalog-zod-save-constraints-design.md
git commit -m "docs: record CI catalog constraint slice"
```

Frontend:

```bash
git add __tests__/validation.catalog-generated.test.ts
git commit -m "test: prove CI catalog constraint parity"
```
