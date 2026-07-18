# G.k.4.r.8 Dosage Text Validation Consistency Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enforce the official repeated dosage-text 2,000-character boundary in the frontend create gate and resolve backend dosage-row issue paths to rendered frontend fields.

**Architecture:** Keep the generated Catalog constraint as the source of the boundary and message, but invoke only `ICH.G.k.4.r.8.LENGTH.MAX` from the existing nested drug syntax collector. Normalize backend `drugs.N.dosages.M.*` paths in the existing alias layer without changing backend validation, persistence, XML, or other portable rules.

**Tech Stack:** TypeScript, Zod, Jest, Rust validator regression suite

## Global Constraints

- `G.k.4.r.8` remains optional and belongs only to repeated `DosageInformation` rows.
- A present dosage-text value may contain at most 2,000 characters.
- The removed drug-level `DrugInformation.dosage_text` / `drugDosageText` field must not return.
- Do not activate every portable Catalog rule in `collectSyntaxIssues` as part of this change.
- Do not add database migrations or change XML/CIOMS behavior.

---

### Task 1: Connect G.k.4.r.8 to the frontend create gate

**Files:**
- Modify: `lib/validation/syntax.ts:452-505`
- Test: `__tests__/validation.syntax.test.ts`

**Interfaces:**
- Consumes: `validatePortableCatalogValue(ruleCode: string, value: unknown, valueType: "string" | "boolean" | "number"): boolean` and the module-local `catalogConstraintsByCode` map.
- Produces: `collectSyntaxIssues()` issue at `drugs.N.dosageInformation.M.dosageText` using the generated Catalog message.

- [ ] **Step 1: Write the failing 2,000/2,001 boundary test**

Add a test that calls `collectSyntaxIssues` twice with nested dosage rows. Assert no dosage-text issue for `"X".repeat(2000)` and an issue at `drugs.1.dosageInformation.1.dosageText` for `"X".repeat(2001)`.

```ts
it("enforces the repeated dosage text Catalog boundary", () => {
  const issuesAtLimit = collectSyntaxIssues({
    drugs: [{ dosageInformation: [{ dosageText: "X".repeat(2000) }] }],
  });
  expect(
    issuesAtLimit.some(
      (issue) => issue.path === "drugs.0.dosageInformation.0.dosageText",
    ),
  ).toBe(false);

  const issuesOverLimit = collectSyntaxIssues({
    drugs: [{}, { dosageInformation: [{}, { dosageText: "X".repeat(2001) }] }],
  });
  expect(
    issuesOverLimit.some(
      (issue) => issue.path === "drugs.1.dosageInformation.1.dosageText",
    ),
  ).toBe(true);
});
```

- [ ] **Step 2: Run the test and verify RED**

Run: `npx jest __tests__/validation.syntax.test.ts --runInBand`

Expected: FAIL because the 2,001-character repeated dosage text produces no issue.

- [ ] **Step 3: Implement the narrow Catalog-backed check**

In `collectDrugSyntaxIssues`, read `dose.dosageText`. When present, validate it with `ICH.G.k.4.r.8.LENGTH.MAX`. If invalid, fetch the same constraint from `catalogConstraintsByCode` and add its generated message at the concrete dosage-information path.

```ts
const dosageTextRuleCode = "ICH.G.k.4.r.8.LENGTH.MAX";
const dosageText = typeof dose.dosageText === "string" ? dose.dosageText : undefined;
if (
  dosageText !== undefined &&
  !validatePortableCatalogValue(dosageTextRuleCode, dosageText, "string")
) {
  const constraint = catalogConstraintsByCode.get(dosageTextRuleCode);
  if (!constraint) throw new Error(`missing generated Catalog rule: ${dosageTextRuleCode}`);
  issues.push({
    path: `drugs.${drugIndex}.dosageInformation.${doseIndex}.dosageText`,
    section: "drugs",
    message: constraint.message,
  });
}
```

- [ ] **Step 4: Run the syntax and generated-Catalog tests and verify GREEN**

Run: `npx jest __tests__/validation.syntax.test.ts __tests__/validation.catalog-rule-exhaustive.test.ts --runInBand`

Expected: PASS with the boundary regression and all generated associations green.

- [ ] **Step 5: Commit the frontend create-gate change**

```bash
git add lib/validation/syntax.ts __tests__/validation.syntax.test.ts
git commit -m "fix: validate repeated dosage text length"
```

### Task 2: Normalize backend dosage-row issue paths

**Files:**
- Modify: `lib/validation/backendPathAliases.ts:1-45`
- Test: `__tests__/validation/backendFieldBanners.test.ts`

**Interfaces:**
- Consumes: `applyBackendPathAliases(path: string): string` through `resolveBackendIssueFieldPath`.
- Produces: canonical `drugs.N.dosageInformation.M.*` paths for backend `drugs.N.dosages.M.*` issues.

- [ ] **Step 1: Write the failing alias test**

Add a focused resolver assertion:

```ts
expect(
  resolveBackendIssueFieldPath({
    code: "ICH.G.k.4.r.8.LENGTH.MAX",
    path: "drugs.2.dosages.3.dosageText",
  }),
).toBe("drugs.2.dosageInformation.3.dosageText");
```

- [ ] **Step 2: Run the test and verify RED**

Run: `npx jest __tests__/validation/backendFieldBanners.test.ts --runInBand`

Expected: FAIL because the resolver returns the unchanged `drugs.2.dosages.3.dosageText` path.

- [ ] **Step 3: Add the structural dosage alias**

Add this replacement to `applyBackendPathAliases`:

```ts
.replace(
  /^drugs\.(\d+)\.dosages\.(\d+)\./u,
  "drugs.$1.dosageInformation.$2.",
)
```

- [ ] **Step 4: Run resolver and rendered-banner tests and verify GREEN**

Run: `npx jest __tests__/validation/backendFieldBanners.test.ts __tests__/field-error-banners/drugs.test.ts --runInBand`

Expected: PASS; existing backend field banners remain green.

- [ ] **Step 5: Commit the frontend path-normalization change**

```bash
git add lib/validation/backendPathAliases.ts __tests__/validation/backendFieldBanners.test.ts
git commit -m "fix: resolve backend dosage validation paths"
```

### Task 3: Verify the cross-repository contract

**Files:**
- Verify: `crates/libs/validator/src/case/sections/g.rs`
- Verify: `crates/libs/validator/src/portable_bindings/g.rs`
- Verify: `lib/validation/syntax.ts`
- Verify: `lib/validation/backendPathAliases.ts`

**Interfaces:**
- Consumes: backend rule code `ICH.G.k.4.r.8.LENGTH.MAX` and frontend generated binding `drugs[].dosageInformation[].dosageText`.
- Produces: evidence that the backend, generated frontend rule, create gate, and banner resolver share one repeated-field contract.

- [ ] **Step 1: Run focused frontend verification**

Run: `npx jest __tests__/validation.syntax.test.ts __tests__/validation.catalog-rule-exhaustive.test.ts __tests__/validation/backendFieldBanners.test.ts __tests__/field-error-banners/drugs.test.ts --runInBand`

Expected: all suites pass with zero failures.

- [ ] **Step 2: Run frontend type and diff checks**

Run: `npx tsc --noEmit && git diff --check`

Expected: exit code 0.

- [ ] **Step 3: Re-run the backend nested dosage rule test**

Run: `cargo test -p validator max_length_rules_cover_g_nested_drug_collections`

Expected: the focused validator test passes.

- [ ] **Step 4: Confirm deleted drug-level validation wiring remains absent**

Run: `rg -n "drugDosageText|G.k.local.supplemental.dosageText" lib app registry crates`

Expected: no production matches for the deleted field; frontend regression-test fixtures may intentionally mention `drugDosageText` as forbidden input.

- [ ] **Step 5: Review clean worktrees and commit any plan-only updates**

Run in both worktrees: `git status --short && git log -3 --oneline`

Expected: only intentional commits and no unstaged production changes.
