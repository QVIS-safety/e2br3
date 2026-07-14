# Case Rule Table Consolidation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move all 63 direct-only case-validation rules into two shared declarative evaluators while preserving all catalog semantics, issue codes, paths, and no-fallback behavior.

**Architecture:** Sections prepare values, concrete paths, and `RuleFacts`; `CatalogValueRule<T>` delegates required and conditional decisions to the catalog, while `ViolationRule<T>` handles only algorithmic invalidity such as ordering and exclusion. Complex FDA/MFDS data is normalized into small section-local owned views before table evaluation, so `rule_table.rs` never performs database access or invents index fallbacks.

**Tech Stack:** Rust 2021, Cargo, existing validator catalog engine, `RuleFacts`, `ValidationContext`, SQLx-backed section orchestration, built-in Rust test framework.

## Global Constraints

- The catalog remains the source of truth for required and conditional applicability.
- Rule tables must not contain catalog-condition `trigger` closures.
- Preserve current issue codes, paths, severities, nullFlavor behavior, and authority behavior.
- Do not add owner, canonical-path, index, terminology, or condition fallback.
- Keep database access and cross-record assembly in section modules.
- Do not change dictionary, catalog, or regulatory semantics during this refactor.
- Do not modify or stage unrelated dirty-worktree files.

---

## File Map

- `crates/libs/validator/src/case/sections/rule_table.rs`: owns the two new generic rule shapes and evaluators.
- `crates/libs/validator/src/case/sections/mod.rs`: owns exact catalog/table/direct coverage accounting.
- `crates/libs/validator/src/case/sections/c.rs`: C scalar, aggregate, FDA study, and MFDS conditioned bindings.
- `crates/libs/validator/src/case/sections/d.rs`: D patient relations and MFDS past-drug bindings.
- `crates/libs/validator/src/case/sections/e.rs`: E seriousness predicates and FDA conditioned binding.
- `crates/libs/validator/src/case/sections/g.rs`: MFDS prepared views and FDA device prepared views.
- `crates/libs/validator/src/case/sections/h.rs`: H narrative presence binding.
- `crates/libs/validator/src/case/sections/n.rs`: N message-header presence binding.
- `registry/catalog-implementation-inventory.md`: generated-by-test coverage totals and command.

### Task 1: Make Coverage Accounting Truthful

**Files:**
- Modify: `crates/libs/validator/src/case/sections/mod.rs`
- Modify: `crates/libs/validator/src/case/sections/c.rs`
- Modify: `crates/libs/validator/src/case/sections/d.rs`
- Modify: `crates/libs/validator/src/case/sections/e.rs`
- Modify: `crates/libs/validator/src/case/sections/f.rs`
- Modify: `crates/libs/validator/src/case/sections/g.rs`
- Modify: `crates/libs/validator/src/case/sections/h.rs`
- Modify: `crates/libs/validator/src/case/sections/n.rs`
- Modify: `registry/catalog-implementation-inventory.md`

**Interfaces:**
- Produces: `implemented_table_rule_codes() -> BTreeSet<&'static str>`
- Produces: `implemented_direct_rule_codes() -> BTreeSet<&'static str>`
- Produces: `implemented_case_rule_codes() -> BTreeSet<&'static str>` as their union.

- [ ] **Step 1: Write the failing inventory-accounting test**

Add a test that computes table-only, direct-only, overlap, and union sets rather
than relying on documented arithmetic:

```rust
#[test]
fn case_rule_inventory_baseline_is_exact() {
    let table = implemented_table_rule_codes();
    let direct = implemented_direct_rule_codes();
    let overlap = table.intersection(&direct).copied().collect::<BTreeSet<_>>();
    let direct_only = direct.difference(&table).copied().collect::<BTreeSet<_>>();

    assert_eq!(table.len(), 398);
    assert_eq!(direct.len(), 64);
    assert_eq!(overlap, [
        "ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN",
    ].into_iter().collect());
    assert_eq!(direct_only.len(), 63);
    assert_eq!(table.union(&direct).count(), 461);
}
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```bash
cargo test -p validator case_rule_inventory_baseline_is_exact --lib -- --nocapture
```

Expected: compilation fails because the separate table/direct APIs do not yet
exist.

- [ ] **Step 3: Separate section inventory APIs**

In each section, split the current merged function into:

```rust
#[cfg(test)]
pub(super) fn table_rule_codes() -> Vec<&'static str> { /* rule-table codes */ }

#[cfg(test)]
pub(super) fn direct_rule_codes() -> &'static [&'static str] { /* direct list */ }
```

Aggregate those functions in `mod.rs`. Ensure G's dosage future-date rule is in
both source sets until its redundant direct entry is removed by the migration.

- [ ] **Step 4: Correct the inventory document**

Record `398 table`, `64 direct`, `1 overlap`, `63 direct-only`, and `461 union`.
State that these values are enforced by `case_rule_inventory_baseline_is_exact`.

- [ ] **Step 5: Verify GREEN and commit**

```bash
cargo test -p validator case_rule_inventory_baseline_is_exact --lib
cargo test -p validator implemented_case_registry_matches_case_validate_catalog --lib
git diff --check
git add crates/libs/validator/src/case/sections registry/catalog-implementation-inventory.md
git commit -m "test: separate case rule coverage inventories"
```

Expected: both focused tests pass; only the listed files are committed.

### Task 2: Make Conditioned Issue Codes Catalog-Self-Contained

**Files:**
- Modify: `crates/libs/validator/src/catalog.rs`
- Test: `crates/libs/validator/src/catalog.rs`

**Interfaces:**
- Produces: every issue code passed to `CatalogValueRule` resolves its own
  condition and value policy without borrowing metadata from another code.

- [ ] **Step 1: Add the failing alias-parity test**

```rust
#[test]
fn conditioned_issue_codes_own_their_value_policy() {
    for value in [Some("true"), Some("false"), Some("1"), None] {
        assert_eq!(
            is_rule_value_valid(
                "FDA.C.1.12.RECOMMENDED",
                value,
                None,
                RuleFacts::default(),
            ),
            is_rule_value_valid(
                "FDA.C.1.12.REQUIRED",
                value,
                None,
                RuleFacts::default(),
            ),
        );
    }
}
```

- [ ] **Step 2: Run the test and verify RED**

```bash
cargo test -p validator conditioned_issue_codes_own_their_value_policy --lib
```

Expected: fail because `FDA.C.1.12.RECOMMENDED` currently defaults to a
different value policy.

- [ ] **Step 3: Bind the issue code to its own catalog policy**

Add this binding beside the required code:

```rust
ValuePolicyBinding {
    code: "FDA.C.1.12.RECOMMENDED",
    policy: ValuePolicy::FdaBooleanStringOrNullFlavor,
},
```

Do not add alias resolution or fallback logic.

- [ ] **Step 4: Audit all conditioned helper triples**

Run the following source audit and inspect every nonidentical triple before the
helper is removed:

```bash
rg -n -A 4 'push_issue_if_conditioned_value_invalid\(' \
  crates/libs/validator/src/case/sections/{c,d,e,g}.rs
```

The only expected nonidentical triple is the recommended C.1.12 call. Record
that expectation in the test name and remove the triple-code helper only after
all callers have migrated.

- [ ] **Step 5: Verify and commit**

```bash
cargo test -p validator conditioned_issue_codes_own_their_value_policy --lib
cargo test -p validator catalog::tests --lib
git add crates/libs/validator/src/catalog.rs
git commit -m "fix: make conditioned issue policy self-contained"
```

### Task 3: Add the Two Shared Evaluators

**Files:**
- Modify: `crates/libs/validator/src/case/sections/rule_table.rs`

**Interfaces:**
- Produces: `CatalogValueRule<T>` and `eval_catalog_values<T>`.
- Produces: `ViolationRule<T>` and `eval_violations<T>`.
- Consumes: existing `RuleValue`, `RuleFacts`, and catalog issue helpers.

- [ ] **Step 1: Add failing `CatalogValueRule` tests**

Under `rule_table.rs`, add tests with an owned prepared view:

```rust
struct PreparedValue {
    path: String,
    value: Option<String>,
    facts: RuleFacts,
}

const RULES: &[CatalogValueRule<PreparedValue>] = &[CatalogValueRule {
    code: "MFDS.C.5.4.KR.1.REQUIRED",
    path: |item| item.path.clone(),
    value: |item| RuleValue::borrowed(item.value.as_deref(), None),
    facts: |item| item.facts,
}];
```

Test three cases: condition false plus missing value emits nothing; condition
true plus missing value emits once at `studyInformation.2.studyTypeReactionKr1`;
condition true plus present value emits nothing.

- [ ] **Step 2: Add failing `ViolationRule` tests**

```rust
struct PreparedViolation {
    path: String,
    violated: bool,
}

const RULES: &[ViolationRule<PreparedViolation>] = &[ViolationRule {
    code: "ICH.D.8.MPID_PHPID.EXCLUSIVE",
    path: |item| item.path.clone(),
    violated: |item| item.violated,
}];
```

Assert one issue for `violated: true`, no issue for `false`, and exact concrete
path preservation.

- [ ] **Step 3: Run both tests and verify RED**

```bash
cargo test -p validator catalog_value_rule_tests --lib
cargo test -p validator violation_rule_tests --lib
```

Expected: compilation fails because both rule types are absent.

- [ ] **Step 4: Implement the minimal generic shapes**

```rust
pub(crate) struct CatalogValueRule<T> {
    pub code: &'static str,
    pub path: fn(&T) -> String,
    pub value: for<'a> fn(&'a T) -> RuleValue<'a>,
    pub facts: fn(&T) -> RuleFacts,
}

pub(crate) fn eval_catalog_values<T>(
    issues: &mut Vec<ValidationIssue>,
    items: &[T],
    rules: &[CatalogValueRule<T>],
) {
    for item in items {
        for rule in rules {
            let RuleValue::Text { value, null_flavor } = (rule.value)(item);
            let _ = push_issue_if_rule_invalid(
                issues,
                rule.code,
                (rule.path)(item),
                value.as_deref(),
                null_flavor,
                (rule.facts)(item),
            );
        }
    }
}

pub(crate) struct ViolationRule<T> {
    pub code: &'static str,
    pub path: fn(&T) -> String,
    pub violated: fn(&T) -> bool,
}

pub(crate) fn eval_violations<T>(
    issues: &mut Vec<ValidationIssue>,
    items: &[T],
    rules: &[ViolationRule<T>],
) {
    for item in items {
        for rule in rules {
            if (rule.violated)(item) {
                push_issue_by_code(issues, rule.code, (rule.path)(item));
            }
        }
    }
}
```

Add both types to the test-only rule-code inventory trait.

- [ ] **Step 5: Verify and commit**

```bash
cargo test -p validator catalog_value_rule_tests --lib
cargo test -p validator violation_rule_tests --lib
cargo test -p validator --lib
git add crates/libs/validator/src/case/sections/rule_table.rs
git commit -m "refactor: add catalog value and violation evaluators"
```

Expected: all validator lib tests pass.

### Task 4: Migrate Existing Conditioned Calls in C, D, and E

**Files:**
- Modify: `crates/libs/validator/src/case/sections/c.rs`
- Modify: `crates/libs/validator/src/case/sections/d.rs`
- Modify: `crates/libs/validator/src/case/sections/e.rs`

**Interfaces:**
- Consumes: `CatalogValueRule<T>` and `eval_catalog_values<T>`.
- Produces: section-local prepared rows carrying owned paths, values, and facts.

- [ ] **Step 1: Add characterization tests for the 13 conditioned rules**

Extend existing section test modules so each group checks an emitting and silent
case for these exact codes:

```text
FDA.C.1.7.1.REQUIRED
FDA.C.1.12.REQUIRED
FDA.C.1.12.RECOMMENDED
MFDS.C.3.1.KR.1.REQUIRED
MFDS.C.2.r.4.KR.1.REQUIRED
MFDS.C.5.4.KR.1.REQUIRED
FDA.D.11.REQUIRED
FDA.D.12.REQUIRED
MFDS.D.8.r.1.KR.1a.REQUIRED
MFDS.D.8.r.1.KR.1b.REQUIRED
MFDS.D.10.8.r.1.KR.1a.REQUIRED
MFDS.D.10.8.r.1.KR.1b.REQUIRED
FDA.E.i.3.2h.REQUIRED
```

For indexed and nested rules, assert the exact nonzero index path.

- [ ] **Step 2: Run the new tests before refactoring**

```bash
cargo test -p validator case::sections::c --lib
cargo test -p validator case::sections::d --lib
cargo test -p validator case::sections::e --lib
```

Expected: tests pass against current behavior; temporarily change one expected
code to prove the new test fails, then restore it before implementation.

- [ ] **Step 3: Introduce small prepared value structs**

Use one owned view type per homogeneous table so receiver/profile facts are
calculated once and rules are never cross-applied to unrelated rows. The exact
view set is:

```text
CReportRegionalRuleView
MfdsSenderRuleView
MfdsPrimarySourceRuleView
MfdsStudyRuleView
DPatientRegionalRuleView
MfdsPastDrugRuleView
MfdsParentPastDrugRuleView
FdaReactionRuleView
```

Each type follows this shape, with fields specific to its table:

```rust
struct MfdsPastDrugRuleView {
    path: String,
    medicinal_product_id: Option<String>,
    medicinal_product_version: Option<String>,
    facts: RuleFacts,
}
```

Define section-specific static tables with no `trigger` field. After Task 2,
each table row uses its own issue code for both catalog condition and value
policy evaluation; no alias code appears in the table.

- [ ] **Step 4: Replace direct helper loops with `eval_catalog_values`**

Build prepared vectors in the same collection order as today. Parent D.10 paths
must use the resolved parent ID and actual sequence index; skip unresolved owners
rather than substituting zero.

Remove the migrated codes from each `direct_rule_codes()` list and add their
tables to `table_rule_codes()`.

- [ ] **Step 5: Verify the direct-only reduction and commit**

Update the inventory baseline from 63 to 50 direct-only rules and assert catalog
union remains 461.

```bash
cargo test -p validator case_rule_inventory_baseline_is_exact --lib
cargo test -p validator case::sections::c --lib
cargo test -p validator case::sections::d --lib
cargo test -p validator case::sections::e --lib
cargo test -p validator --lib
git add crates/libs/validator/src/case/sections/{c.rs,d.rs,e.rs,mod.rs}
git commit -m "refactor: tableize conditioned case values"
```

### Task 5: Migrate the Eleven MFDS G Conditioned Rules

**Files:**
- Modify: `crates/libs/validator/src/case/sections/g.rs`

**Interfaces:**
- Consumes: `CatalogValueRule<T>` and `eval_catalog_values<T>`.
- Produces: `MfdsDrugRuleView`, `MfdsSubstanceRuleView`, and `MfdsRelatednessRuleView`.

- [ ] **Step 1: Add receiver/profile characterization tests**

Cover KR, FR, CT/CU, and unrelated receiver behavior for these codes:

```text
MFDS.G.k.2.1.KR.1a.REQUIRED
MFDS.G.k.2.1.KR.1b.REQUIRED
MFDS.G.k.2.3.r.1.KR.1a.REQUIRED
MFDS.G.k.2.3.r.1.KR.1b.REQUIRED
MFDS.G.k.9.i.2.r.1.REQUIRED
MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED
MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED
MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED
MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED
MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED
MFDS.KR.FOREIGN.WHOMPID.REQUIRED
```

Assert that an unresolved drug/substance relationship does not emit a path under
`drugs.0`.

- [ ] **Step 2: Run characterization tests and prove sensitivity**

```bash
cargo test -p validator case::sections::g --lib -- --nocapture
```

Expected: pass; temporarily invert one receiver expectation to observe failure,
then restore it.

- [ ] **Step 3: Build owned MFDS prepared views**

Each view stores concrete indexes/paths, owned values, and complete `RuleFacts`.
Drop rows whose parent ownership cannot be resolved. Do not use `"drugs"` or
index zero as an error path.

- [ ] **Step 4: Evaluate presence through catalog tables**

Replace all eleven conditioned helper calls with `eval_catalog_values`. Preserve
the two existing algorithmic profile/allowed-code checks for Task 7, rather than
putting those checks in a catalog-value trigger.

Move the eleven codes from the direct inventory to the table inventory.

- [ ] **Step 5: Verify and commit**

Update the inventory assertion from 50 to 39 direct-only rules.

```bash
cargo test -p validator case_rule_inventory_baseline_is_exact --lib
cargo test -p validator case::sections::g --lib
cargo test -p validator --lib
git add crates/libs/validator/src/case/sections/{g.rs,mod.rs}
git commit -m "refactor: tableize MFDS G conditioned values"
```

### Task 6: Migrate ICH Presence, Aggregate, and Companion Rules

**Files:**
- Modify: `crates/libs/validator/src/case/sections/c.rs`
- Modify: `crates/libs/validator/src/case/sections/d.rs`
- Modify: `crates/libs/validator/src/case/sections/h.rs`
- Modify: `crates/libs/validator/src/case/sections/n.rs`

**Interfaces:**
- Consumes: `CatalogValueRule<T>`.
- Produces: section-local aggregate rows whose values are present only when the
  existing aggregate predicate is satisfied.

- [ ] **Step 1: Add aggregate and missing-container characterization tests**

Cover these exact direct groups:

```text
ICH.C.1.1.REQUIRED, ICH.C.1.REQUIRED
ICH.C.2.r.2.1.REQUIRED, ICH.C.2.r.4.REQUIRED, ICH.C.2.r.5.REQUIRED
ICH.C.3.1.REQUIRED, ICH.C.3.2.REQUIRED
ICH.C.1.11.2.REQUIRED
ICH.D.1.REQUIRED, ICH.D.1.1.4.REQUIRED
ICH.D.2.2a.REQUIRED, ICH.D.2.2b.REQUIRED
ICH.D.2.2.1a.REQUIRED, ICH.D.2.2.1b.REQUIRED
ICH.D.9.3.REQUIRED
ICH.H.1.REQUIRED, ICH.N.REQUIRED
```

For aggregate rules, include empty collection, nonmatching collection, and one
matching item. For H, preserve `should_require_case_narrative` semantics.

- [ ] **Step 2: Run tests and verify current behavior**

```bash
cargo test -p validator case::sections::c --lib
cargo test -p validator case::sections::d --lib
cargo test -p validator case::sections::h --lib
cargo test -p validator case::sections::n --lib
```

- [ ] **Step 3: Prepare catalog values rather than condition closures**

Use `RuleValue::owned(Some("present".to_string()), None)` only when the actual
aggregate or companion value exists; use `RuleValue::owned(None, None)` when it
does not. Put applicability booleans into the catalog's existing `RuleFacts`
fields. If a required condition lacks a fact binding, stop and correct catalog
parity in a separate reviewed commit before continuing; do not add a table
trigger.

- [ ] **Step 4: Replace branches and update inventories**

Evaluate prepared rows with `eval_catalog_values`. Ensure missing containers
produce exactly the existing root issue and do not duplicate child issues.

- [ ] **Step 5: Verify and commit**

Compute the new direct-only count from the actual set difference in the test;
do not hand-edit the expected count until the test output is reviewed.

```bash
cargo test -p validator case_rule_inventory_baseline_is_exact --lib -- --nocapture
cargo test -p validator --lib
git add crates/libs/validator/src/case/sections/{c.rs,d.rs,h.rs,n.rs,mod.rs}
git commit -m "refactor: tableize case presence and companion rules"
```

### Task 7: Move Algorithmic ICH/MFDS Checks to `ViolationRule`

**Files:**
- Modify: `crates/libs/validator/src/case/sections/c.rs`
- Modify: `crates/libs/validator/src/case/sections/d.rs`
- Modify: `crates/libs/validator/src/case/sections/e.rs`
- Modify: `crates/libs/validator/src/case/sections/g.rs`

**Interfaces:**
- Consumes: `ViolationRule<T>` and `eval_violations<T>`.
- Produces: prepared relation rows with concrete paths and already-normalized
  operands.

- [ ] **Step 1: Add relation characterization tests**

Test both emitting and silent cases for:

```text
ICH.C.1.4.AFTER_C.1.2.FORBIDDEN
ICH.C.1.4.AFTER_C.1.5.FORBIDDEN
ICH.C.1.5.AFTER_C.1.2.FORBIDDEN
ICH.D.8.MPID_PHPID.EXCLUSIVE
ICH.D.10.8.MPID_PHPID.EXCLUSIVE
ICH.E.i.3.2.CRITERIA.REQUIRED
ICH.E.i.3.2.NI.ONLY
```

Also lock the existing MFDS relatedness method/result profile checks that reuse
their required code for an invalid present value.

- [ ] **Step 2: Run tests before refactoring**

```bash
cargo test -p validator case::sections::c --lib
cargo test -p validator case::sections::d --lib
cargo test -p validator case::sections::e --lib
cargo test -p validator case::sections::g --lib
```

- [ ] **Step 3: Define prepared relation rows and static violation tables**

Keep `violated` predicates limited to normalized comparisons. Path resolution
happens before evaluation. For nested D.10 exclusion, omit unresolved owners;
never default parent or child indexes.

- [ ] **Step 4: Replace branches with `eval_violations`**

Remove the migrated codes from direct inventories. Remove the redundant direct
entry for `ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN`; its existing future-date table
remains the sole implementation registration.

- [ ] **Step 5: Verify and commit**

```bash
cargo test -p validator case_rule_inventory_baseline_is_exact --lib -- --nocapture
cargo test -p validator implemented_case_registry_matches_case_validate_catalog --lib
cargo test -p validator --lib
git add crates/libs/validator/src/case/sections/{c.rs,d.rs,e.rs,g.rs,mod.rs}
git commit -m "refactor: tableize algorithmic case violations"
```

### Task 8: Normalize and Tableize the Four FDA C Special Rules

**Files:**
- Modify: `crates/libs/validator/src/case/sections/c.rs`

**Interfaces:**
- Produces: owned `FdaStudyRuleView` rows after asynchronous registration reads.
- Consumes: both shared evaluators.

- [ ] **Step 1: Add full FDA C characterization tests**

Cover six-digit valid/invalid study numbers, IND and pre-ANDA receivers,
registration present/absent, and reporter payload with/without email for:

```text
FDA.C.5.5a.REQUIRED
FDA.C.5.5b.REQUIRED
FDA.C.5.6.r.REQUIRED
FDA.C.2.r.2.EMAIL.REQUIRED
```

- [ ] **Step 2: Run tests and verify sensitivity**

```bash
cargo test -p validator case::sections::c::golden_c1_value_tests --lib
```

- [ ] **Step 3: Build `FdaStudyRuleView` after database reads**

The view stores report/receiver facts, normalized study number, a boolean for
cross-report registration presence, and concrete paths. `collect_fda_issues`
performs the read once and then evaluates static tables.

- [ ] **Step 4: Replace all four direct branches**

Represent six-digit validity as the extracted value: valid values remain
present, invalid values become absent for the existing required policy. Use
catalog facts for applicability. Use `ViolationRule` only if the catalog has a
separate non-presence constraint; do not invent one during this task.

- [ ] **Step 5: Verify and commit**

```bash
cargo test -p validator case::sections::c --lib
cargo test -p validator case_rule_inventory_baseline_is_exact --lib -- --nocapture
cargo test -p validator --lib
git add crates/libs/validator/src/case/sections/{c.rs,mod.rs}
git commit -m "refactor: tableize FDA C special rules"
```

### Task 9: Normalize and Tableize the Eleven FDA G Device Rules

**Files:**
- Modify: `crates/libs/validator/src/case/sections/g.rs`

**Interfaces:**
- Produces: `FdaDrugRuleView` per drug and one aggregate `FdaDrugSetRuleView`.
- Consumes: `CatalogValueRule<T>` for required fields and `ViolationRule<T>` for
  invalid characteristic combinations.

- [ ] **Step 1: Add FDA device characterization matrix tests**

Cover combination-product true/false, local criteria 4/5, malfunction on
suspect/non-suspect drugs, missing device values, and invalid G.k.1.a for all
eleven FDA G direct codes.

- [ ] **Step 2: Run tests before refactoring**

```bash
cargo test -p validator case::sections::g --lib -- --nocapture
```

- [ ] **Step 3: Load characteristics and create owned views**

For each drug, merge stored and derived characteristics exactly once. Populate
the view with malfunction, suspect, brand/common name, product code, problem
code, remedial action, G.k.1.a facts, and concrete paths. Populate the aggregate
view from per-drug views without additional database reads.

- [ ] **Step 4: Evaluate static tables and remove mutable issue-state flags**

Replace `has_malfunction_any`, `has_malfunction_suspect`, `has_gk12r3`,
`has_gk12r11`, and `has_invalid_gk1a` issue branches with values/facts on the two
views. Preserve duplicate legacy codes where the current behavior intentionally
emits both canonical and characteristic-level issues.

- [ ] **Step 5: Verify direct-only reaches zero and commit**

```bash
cargo test -p validator case::sections::g --lib
cargo test -p validator case_rule_inventory_baseline_is_exact --lib -- --nocapture
cargo test -p validator implemented_case_registry_matches_case_validate_catalog --lib
cargo test -p validator --lib
git add crates/libs/validator/src/case/sections/{g.rs,mod.rs}
git commit -m "refactor: tableize FDA G device rules"
```

Expected inventory: 461 union, zero missing, zero unexpected, and zero
direct-only rules.

### Task 10: Remove Direct Inventory and Finalize Documentation

**Files:**
- Modify: `crates/libs/validator/src/case/sections/mod.rs`
- Modify: `crates/libs/validator/src/case/sections/c.rs`
- Modify: `crates/libs/validator/src/case/sections/d.rs`
- Modify: `crates/libs/validator/src/case/sections/e.rs`
- Modify: `crates/libs/validator/src/case/sections/f.rs`
- Modify: `crates/libs/validator/src/case/sections/g.rs`
- Modify: `crates/libs/validator/src/case/sections/h.rs`
- Modify: `crates/libs/validator/src/case/sections/n.rs`
- Modify: `registry/catalog-implementation-inventory.md`

**Interfaces:**
- Removes: section `direct_rule_codes()` functions.
- Keeps: one evaluator-backed `implemented_case_rule_codes()` exact-set registry.

- [ ] **Step 1: Add the final no-direct assertion**

```rust
#[test]
fn case_catalog_has_no_direct_only_implementations() {
    assert!(implemented_direct_rule_codes().is_empty());
    assert_eq!(implemented_table_rule_codes().len(), 461);
}
```

- [ ] **Step 2: Run and verify RED if any direct registration remains**

```bash
cargo test -p validator case_catalog_has_no_direct_only_implementations --lib
```

Expected: fail with the remaining code list, or pass only when every migration
task has removed its direct registrations.

- [ ] **Step 3: Remove empty direct APIs and simplify the final test**

Once the direct set is empty, remove section direct lists and assert the table
registry equals the 461-code catalog set directly. Keep diagnostic missing and
unexpected lists in the failure message.

- [ ] **Step 4: Update implementation inventory**

Document 461 evaluator-backed rules, zero direct-only rules, zero missing, and
zero unexpected. Describe `CatalogValueRule`, `ViolationRule`, and prepared
views, and retain the exact verification command.

- [ ] **Step 5: Run final verification and commit**

```bash
cargo fmt --check
cargo test -p validator implemented_case_registry_matches_case_validate_catalog --lib
cargo test -p validator --lib
cargo test -p validator
git diff --check
git status --short
```

Review `git status` and stage only task-owned files:

```bash
git add crates/libs/validator/src/case/sections registry/catalog-implementation-inventory.md
git commit -m "docs: finalize case rule table coverage"
```

If `cargo clippy -p validator --lib --tests --no-deps -- -D warnings` is run,
report existing unrelated warnings separately; do not change unrelated files as
part of this plan.
