# Structured Save Constraint Errors Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reject forced invalid Case Editor writes with a structured HTTP 422 constraint-violation response and let the frontend place that unexpected response on the existing field and section error path.

**Architecture:** The existing Catalog-backed portable save constraint guard remains the sole pre-mutation backend defense and returns one transport-neutral `ConstraintViolation`. `lib-web` maps it to the established error envelope, while the frontend API/result-guard layers preserve and recognize the typed detail before the Case Editor reuses React Hook Form and section-error state. The case-level business-rule Validation engine and its cache are not changed.

**Tech Stack:** Rust, Axum, Serde, Tokio integration tests, TypeScript, React Hook Form, Jest, Node test runner

## Global Constraints

- Reserve the term `Validation` for the case-level business-rule engine and its `CaseValidationReport`/`ValidationIssue` values.
- Name the pre-mutation backend defense `save constraint guard` and its failure a `constraint violation`; do not introduce `ValidationError` naming for it.
- A rejected forced API save returns HTTP `422 Unprocessable Entity`, never a 2xx response.
- The public body is exactly the existing envelope with `error.message = "CONSTRAINT_VIOLATION"` and `error.data.detail = { ruleCode, path, message }`.
- Return the first violation as one object; do not add aggregation, a new frontend store, runtime Catalog fetches, or old string parsing.
- Normal browser behavior remains local field error plus section indicator plus disabled Save/Save Next and no save API call.
- The guard must reject before model mutation and before any business-validation cache refresh.
- Preserve unrelated changes in both worktrees. Backend worktree: `/Users/hyundonghoon/projects/rust/e2br3/e2br3`. Frontend dev worktree: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/.worktrees/merge-local-dev`.

---

## File Structure

- `crates/libs/lib-rest-core/src/error.rs`: owns the transport-neutral constraint detail and REST error variant.
- `crates/libs/lib-rest-core/src/lib.rs`: publicly re-exports the constraint detail for callers.
- `crates/services/web-server/src/web/rest/case_editor_rest/portable_save.rs`: converts the existing Catalog constraint failure into the typed REST error.
- `crates/libs/lib-web/src/error.rs`: defines the stable public `CONSTRAINT_VIOLATION` discriminator.
- `crates/libs/lib-web/src/middleware/mw_res_map.rs`: maps both wrapped and direct REST errors to HTTP 422 and structured detail.
- `crates/services/web-server/tests/api/case_editor_contract_web.rs`: proves direct-field and repeatable-row API rejection, exact JSON, no persistence, and no cache refresh.
- `lib/types/common.ts`: describes the three-field structured detail.
- `node-tests/api.client.test.ts`: proves the API client preserves the 422 code, message, and nested detail.
- `node-tests/error-messages/validation.syntax.message-conformance.test.ts` and `node-tests/validation.domain-format.parity.test.ts`: repair existing Catalog-union test narrowing so the Node suite can execute.
- `lib/case-save/resultGuards.ts`: preserves API error metadata in a typed thrown error and extracts a trusted constraint detail.
- `__tests__/case-save/resultGuards.test.ts`: tests metadata preservation and strict constraint-detail recognition.
- `components/case-form/hooks/useCaseEditorValidationState.ts`: factors the existing field/section error application path for reuse.
- `components/case-form/hooks/useCaseEditorSave.ts`: handles only typed constraint rejections as field/section fallback; generic failures retain toast behavior.
- `components/case-form/CaseEditor.tsx`: wires the reused error applicator into the save hook.
- `__tests__/case-form/CaseEditor.validation-errors.integration.test.ts`: proves local no-request behavior and unexpected server fallback behavior.

---

### Task 1: Typed Backend Constraint Failure

**Files:**
- Modify: `crates/libs/lib-rest-core/src/error.rs`
- Modify: `crates/libs/lib-rest-core/src/lib.rs`
- Modify: `crates/services/web-server/src/web/rest/case_editor_rest/portable_save.rs`
- Test: `crates/services/web-server/src/web/rest/case_editor_rest/portable_save.rs`

**Interfaces:**
- Consumes: existing Catalog-derived `rule_code`, concrete frontend `path`, and Catalog `message` passed to `portable_save::violation`.
- Produces: `lib_rest_core::ConstraintViolation { rule_code: String, path: String, message: String }` and `lib_rest_core::Error::ConstraintViolation(ConstraintViolation)`.

- [ ] **Step 1: Replace string-matching unit assertions with exact typed assertions**

  Add a helper in `portable_save_tests` and update the direct/repeatable/nested tests to compare each field:

  ```rust
  fn constraint_violation(error: Error) -> ConstraintViolation {
      match error {
          Error::ConstraintViolation(detail) => detail,
          other => panic!("expected constraint violation, got {other:?}"),
      }
  }

  fn portable_constraint_message(code: &str) -> String {
      portable_constraints()
          .into_iter()
          .find(|constraint| constraint.code == code)
          .expect("portable Catalog constraint exists")
          .message
  }

  let detail = constraint_violation(error);
  assert_eq!(detail.rule_code, "ICH.E.i.1.1a.LENGTH.MAX");
  assert_eq!(detail.path, "reactions.0.primarySourceReaction");
  assert_eq!(
      detail.message,
      portable_constraint_message("ICH.E.i.1.1a.LENGTH.MAX")
  );
  ```

  Use the existing public `portable_constraints()` projection as shown; do not duplicate the Catalog message copy in the test.

- [ ] **Step 2: Run the focused unit test and verify RED**

  Run: `cargo test -p web-server portable_save_rejects_repeatable_row_values`

  Expected: compilation/test failure because `ConstraintViolation` and `Error::ConstraintViolation` do not exist yet.

- [ ] **Step 3: Add the minimal serializable detail and error variant**

  In `lib-rest-core/src/error.rs`:

  ```rust
  #[derive(Debug, Clone, Serialize, PartialEq, Eq)]
  #[serde(rename_all = "camelCase")]
  pub struct ConstraintViolation {
      pub rule_code: String,
      pub path: String,
      pub message: String,
  }

  #[derive(Debug, From, Serialize)]
  pub enum Error {
      // existing variants
      ConstraintViolation(ConstraintViolation),
  }
  ```

  Re-export it from `lib-rest-core/src/lib.rs`:

  ```rust
  pub use self::error::{ConstraintViolation, Error, Result};
  ```

  Change only the existing `violation` helper in `portable_save.rs`:

  ```rust
  use lib_rest_core::ConstraintViolation;

  fn violation(rule_code: &str, path: &str, message: &str) -> Error {
      Error::ConstraintViolation(ConstraintViolation {
          rule_code: rule_code.to_owned(),
          path: path.to_owned(),
          message: message.to_owned(),
      })
  }
  ```

- [ ] **Step 4: Run all portable save guard unit tests and verify GREEN**

  Run: `cargo test -p web-server portable_save_tests`

  Expected: every `portable_save_tests` test passes with exact typed details.

- [ ] **Step 5: Commit the backend typed-error unit**

  ```bash
  git add crates/libs/lib-rest-core/src/error.rs crates/libs/lib-rest-core/src/lib.rs crates/services/web-server/src/web/rest/case_editor_rest/portable_save.rs
  git commit -m "feat: type save constraint violations"
  ```

---

### Task 2: HTTP 422 Mapping and Backend Contract

**Files:**
- Modify: `crates/libs/lib-web/src/error.rs`
- Modify: `crates/libs/lib-web/src/middleware/mw_res_map.rs`
- Test: `crates/services/web-server/tests/api/case_editor_contract_web.rs`

**Interfaces:**
- Consumes: `lib_rest_core::Error::ConstraintViolation(ConstraintViolation)` from Task 1.
- Produces: HTTP 422 with `ClientError::CONSTRAINT_VIOLATION` and camel-case detail under `error.data.detail`.

- [ ] **Step 1: Tighten direct-field and repeatable-row API tests to the exact contract**

  For `ci_patch_rejects_catalog_constraint_before_write` and `portable_ae_patch_rejects_before_write`, pass `mm.clone()` into `web_server::app`, call `GET /api/cases/{case_id}/validation?authority=ich` once after fixture setup to establish a non-stale cache row, and capture the stale-summary count before the rejected PATCH. Assert it is zero, then assert:

  ```rust
  let stale_before = stale_validation_summary_count(
      &mm,
      seed.admin.id,
      seed.org_id,
      &case_id,
  )
  .await?;
  assert_eq!(stale_before, 0);

  assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
  assert_eq!(body["error"]["message"], "CONSTRAINT_VIOLATION");
  assert_eq!(
      body["error"]["data"]["detail"]["ruleCode"],
      "ICH.E.i.1.1a.LENGTH.MAX"
  );
  assert_eq!(
      body["error"]["data"]["detail"]["path"],
      "reactions.0.primarySourceReaction"
  );
  assert_eq!(
      body["error"]["data"]["detail"]["message"],
      portable_constraint_message("ICH.E.i.1.1a.LENGTH.MAX")
  );
  assert!(body["error"]["data"]["req_uuid"].is_string());
  ```

  Import `validator::portable_constraints` and add the same local `portable_constraint_message` helper shown in Task 1. Keep the existing GET/read-back assertion and add `stale_after == stale_before`; this detects an accidental cache-stale operation on rejection. Apply the same exact assertions to CI using `ICH.C.1.9.1.ALLOWED.VALUE` and `safetyReportIdentification.otherCaseIdentifiersExist`. Leave the NR coverage intact, but update its expected status/body discriminator so every portable endpoint shares the contract.

- [ ] **Step 2: Run the two contract tests and verify RED**

  Run: `cargo test -p web-server --test api case_editor_contract_web::ci_patch_rejects_catalog_constraint_before_write -- --exact --nocapture`

  Run: `cargo test -p web-server --test api case_editor_contract_web::portable_ae_patch_rejects_before_write -- --exact --nocapture`

  Expected: both fail because the mapper still returns its fallback status/body.

- [ ] **Step 3: Add the stable client discriminator and map both REST-error paths**

  Add `CONSTRAINT_VIOLATION` to `ClientError` in `lib-web/src/error.rs`.

  In both `Error::Rest(rest_err)` and direct `rest_error` matches in `mw_res_map.rs`, add an explicit arm before fallbacks:

  ```rust
  lib_rest_core::Error::ConstraintViolation(detail) => (
      StatusCode::UNPROCESSABLE_ENTITY,
      ClientError::CONSTRAINT_VIOLATION,
      Some(serde_json::to_value(detail).expect("constraint detail serializes")),
  ),
  ```

  The direct branch assigns the serialized value to `debug_detail` and returns the two-tuple. Do not make the detail conditional on `E2BR3_DEBUG_ERRORS`; it is public contract data.

- [ ] **Step 4: Run all three rejection contracts and verify GREEN**

  Run: `cargo test -p web-server --test api case_editor_contract_web::ci_patch_rejects_catalog_constraint_before_write -- --exact --nocapture`

  Run: `cargo test -p web-server --test api case_editor_contract_web::portable_ae_patch_rejects_before_write -- --exact --nocapture`

  Run: `cargo test -p web-server --test api case_editor_contract_web::portable_direct_rows_patch_rejects_before_write -- --exact --nocapture`

  Expected: all pass; response is 422, stored value is unchanged, and stale-summary count does not change.

- [ ] **Step 5: Run REST/web library tests and commit**

  Run: `cargo test -p lib-rest-core -p lib-web`

  Expected: both packages pass.

  ```bash
  git add crates/libs/lib-web/src/error.rs crates/libs/lib-web/src/middleware/mw_res_map.rs crates/services/web-server/tests/api/case_editor_contract_web.rs
  git commit -m "feat: return 422 for save constraint violations"
  ```

---

### Task 3: Frontend API Detail Preservation and Node Test Repair

**Files:**
- Modify: `lib/types/common.ts`
- Test: `node-tests/api.client.test.ts`
- Test: `node-tests/error-messages/validation.syntax.message-conformance.test.ts`
- Test: `node-tests/validation.domain-format.parity.test.ts`

**Interfaces:**
- Consumes: backend `error.data.detail` from Task 2.
- Produces: `ConstraintViolationDetail` and an `ApiError` with `code === "CONSTRAINT_VIOLATION"`, human message, and `details.detail` preserving all fields.

- [ ] **Step 1: Add the 422 API client contract test**

  Add a fetch mock returning the approved envelope and assert:

  ```ts
  assert.equal(res.status, "error");
  assert.equal(res.error?.code, "CONSTRAINT_VIOLATION");
  assert.equal(res.error?.message, "The value exceeds the maximum length.");
  assert.deepEqual(res.error?.details?.detail, {
    ruleCode: "ICH.E.i.1.1a.LENGTH.MAX",
    path: "reactions.0.primarySourceReaction",
    message: "The value exceeds the maximum length.",
  });
  ```

- [ ] **Step 2: Run the Node suite and record the existing RED state**

  Run from the frontend dev worktree: `npm run test:node`

  Expected before repair: TypeScript `TS2339` failures where Catalog-union rules are read as though every rule has `.message` or `.regex`. The new API-client test must not be silently skipped.

- [ ] **Step 3: Narrow the existing union-aware tests without changing production behavior**

  In the message-conformance test, import `catalogConstraints` from `../../lib/zod/generated/catalogConstraints` and resolve the displayed message by rule kind:

  ```ts
  const catalogMessages = new Map(
    catalogConstraints.map((constraint) => [constraint.code, constraint.message]),
  );

  function messageForRule(rule: FrontendSyntaxFieldRule): string {
    if (rule.kind !== "catalog") return rule.message;
    const message = catalogMessages.get(rule.ruleCode);
    assert.notEqual(message, undefined, `Missing Catalog rule ${rule.ruleCode}`);
    return message as string;
  }
  ```

  Use `messageForRule(rule)` in both loops. In the domain-format parity test, add `if (rule.kind === "catalog") continue;` before reading `.regex` in both loops because Catalog regex sources are checked by the Catalog parity suite.

  Add the public type in `lib/types/common.ts`:

  ```ts
  export interface ConstraintViolationDetail {
    ruleCode: string;
    path: string;
    message: string;
  }
  ```

  Do not add a second API-client normalization path: the current client already derives the message from object `detail`, the code from `error.message`, and details from `error.data`.

- [ ] **Step 4: Run the Node suite and verify GREEN**

  Run: `npm run test:node`

  Expected: all Node tests pass, including the structured 422 test and the two repaired Catalog-union suites.

- [ ] **Step 5: Commit the frontend contract test and test-infrastructure repair**

  ```bash
  git add lib/types/common.ts node-tests/api.client.test.ts node-tests/error-messages/validation.syntax.message-conformance.test.ts node-tests/validation.domain-format.parity.test.ts
  git commit -m "test: preserve save constraint response details"
  ```

---

### Task 4: Typed Frontend Save-Result Error

**Files:**
- Modify: `lib/case-save/resultGuards.ts`
- Test: `__tests__/case-save/resultGuards.test.ts`

**Interfaces:**
- Consumes: `ApiError { message, code, details }` and `ConstraintViolationDetail` from Task 3.
- Produces: `ApiResultError extends Error` carrying `code?: string` and `details?: Record<string, unknown>`, plus `constraintViolationFromError(error: unknown): ConstraintViolationDetail | null`.

- [ ] **Step 1: Write failing metadata and strict-parser tests**

  Cover these cases:

  ```ts
  expect(() => assertNoApiErrors([constraintResult], fallback)).toThrow(
    ApiResultError,
  );

  try {
    assertNoApiErrors([constraintResult], fallback);
  } catch (error) {
    expect(constraintViolationFromError(error)).toEqual({
      ruleCode: "ICH.E.i.1.1a.LENGTH.MAX",
      path: "reactions.0.primarySourceReaction",
      message: "The value exceeds the maximum length.",
    });
  }
  ```

  Also prove it returns `null` for a non-constraint code, missing `details.detail`, or any non-string required field, and retain all legacy string/fallback-message assertions.

- [ ] **Step 2: Run the focused Jest test and verify RED**

  Run: `npm test -- --runInBand __tests__/case-save/resultGuards.test.ts`

  Expected: failure because `ApiResultError` and `constraintViolationFromError` are not exported and metadata is currently discarded.

- [ ] **Step 3: Implement the minimal typed thrown error and parser**

  Expand the local error shape to include `code` and `details`. Throw `ApiResultError` for object-shaped API errors while preserving current string and fallback messages. Implement a strict, side-effect-free parser:

  ```ts
  export function constraintViolationFromError(
    error: unknown,
  ): ConstraintViolationDetail | null {
    if (!(error instanceof ApiResultError) || error.code !== "CONSTRAINT_VIOLATION") {
      return null;
    }
    const detail = error.details?.detail;
    if (!detail || typeof detail !== "object") return null;
    const { ruleCode, path, message } = detail as Record<string, unknown>;
    return typeof ruleCode === "string" &&
      typeof path === "string" &&
      typeof message === "string"
      ? { ruleCode, path, message }
      : null;
  }
  ```

- [ ] **Step 4: Run the focused guard suite and verify GREEN**

  Run: `npm test -- --runInBand __tests__/case-save/resultGuards.test.ts`

  Expected: all guard tests pass.

- [ ] **Step 5: Commit the typed result guard**

  ```bash
  git add lib/case-save/resultGuards.ts __tests__/case-save/resultGuards.test.ts
  git commit -m "feat: preserve typed save constraint failures"
  ```

---

### Task 5: Case Editor Field/Section Fallback

**Files:**
- Modify: `components/case-form/hooks/useCaseEditorValidationState.ts`
- Modify: `components/case-form/hooks/useCaseEditorSave.ts`
- Modify: `components/case-form/CaseEditor.tsx`
- Test: `__tests__/case-form/CaseEditor.validation-errors.integration.test.ts`

**Interfaces:**
- Consumes: `constraintViolationFromError` from Task 4 and existing `SyntaxIssue { path, section, message }`/`toSyntaxSection(path)`.
- Produces: `markConstraintIssues(issues: SyntaxIssue[]): void`, reusing the existing React Hook Form and `syntaxSectionErrors` state path without new storage.

- [ ] **Step 1: Strengthen the local gate test and add a failing server-fallback test**

  In the existing `blocks an edited portable violation and clears it after correction` case, attempt save while the value is still `bad`, then assert every persistence mock—including `mockPatchEditorPageProjection`—was not called and `saveDisabled` remains true before correcting the field.

  Add a persisted-case test where local collectors return no issue, the section PATCH returns:

  ```ts
  {
    status: "error",
    error: {
      code: "CONSTRAINT_VIOLATION",
      message: "The value exceeds the maximum length.",
      details: {
        req_uuid: "request-id",
        detail: {
          ruleCode: "ICH.C.1.1.LENGTH.MAX",
          path: "safetyReportIdentification.safetyReportId",
          message: "The value exceeds the maximum length.",
        },
      },
    },
  }
  ```

  Edit the Safety Report ID, save, then assert save status is `unsaved`, the matching field exposes that message, `validationErrors["case-identification"]` is true, and `mockToastError` was not called. Retain the existing generic `HTTP_400` test as the regression proof that ordinary failures still toast.

- [ ] **Step 2: Run the focused integration tests and verify RED**

  Run: `npm test -- --runInBand __tests__/case-form/CaseEditor.validation-errors.integration.test.ts`

  Expected: the new server fallback fails because the catch block only emits a generic toast and does not apply field/section errors.

- [ ] **Step 3: Factor and expose the existing field/section applicator**

  In `useCaseEditorValidationState.ts`, extract the shared body from `markCreateGateIssues` into one callback that accepts the RHF error type. Keep `markCreateGateIssues` as a wrapper with type `"createGate"` and add `markConstraintIssues` as a wrapper with type `"constraint"`. Both must update the existing `syntaxErrorPathsRef` and `setSyntaxSectionErrors`; do not add state.

- [ ] **Step 4: Handle only the typed constraint response in the save catch path**

  Wire `markConstraintIssues` through `CaseEditor.tsx` into `useCaseEditorSave`. Before the generic catch behavior:

  ```ts
  const violation = constraintViolationFromError(error);
  if (violation) {
    markConstraintIssues([
      {
        path: violation.path,
        section: toSyntaxSection(violation.path),
        message: violation.message,
      },
    ]);
    saveTiming.finish({
      result: "constraint-violation",
      error: violation.ruleCode,
    });
    return false;
  }
  ```

  Keep `setSaveStatus("unsaved")` before this branch. Do not set business semantic status to unknown and do not toast for this typed branch. Leave the existing semantic-status/toast behavior unchanged for all other errors.

- [ ] **Step 5: Run Case Editor and result-guard suites and verify GREEN**

  Run: `npm test -- --runInBand __tests__/case-form/CaseEditor.validation-errors.integration.test.ts __tests__/case-save/resultGuards.test.ts`

  Expected: local invalid input sends no request; typed 422 marks the exact field/section without toast; generic HTTP errors still toast.

- [ ] **Step 6: Commit the Case Editor fallback**

  ```bash
  git add components/case-form/hooks/useCaseEditorValidationState.ts components/case-form/hooks/useCaseEditorSave.ts components/case-form/CaseEditor.tsx __tests__/case-form/CaseEditor.validation-errors.integration.test.ts
  git commit -m "feat: surface save constraint failures on fields"
  ```

---

### Task 6: Cross-Repository Verification

**Files:**
- Verify only; no new production files.

**Interfaces:**
- Consumes: Tasks 1–5.
- Produces: evidence that the Catalog artifacts, backend guard, HTTP contract, frontend fallback, and business Validation boundary remain intact.

- [ ] **Step 1: Verify backend formatting and focused suites**

  Run from the backend worktree:

  ```bash
  cargo fmt --all -- --check
  cargo test -p web-server portable_save_tests
  cargo test -p web-server --test api case_editor_contract_web::ci_patch_rejects_catalog_constraint_before_write -- --exact --nocapture
  cargo test -p web-server --test api case_editor_contract_web::portable_ae_patch_rejects_before_write -- --exact --nocapture
  cargo test -p web-server --test api case_editor_contract_web::portable_direct_rows_patch_rejects_before_write -- --exact --nocapture
  cargo test -p validator
  ```

  Expected: all commands pass. The validator suite proves the business-rule Validation engine remains green without being modified for transport errors.

- [ ] **Step 2: Verify generated Catalog parity and frontend suites**

  Run from the frontend dev worktree:

  ```bash
  BACKEND_REPO=/Users/hyundonghoon/projects/rust/e2br3/e2br3 npm run check:validation-catalog
  npm run test:node
  npm test -- --runInBand __tests__/case-save/resultGuards.test.ts __tests__/case-form/CaseEditor.validation-errors.integration.test.ts
  ```

  Expected: Catalog parity passes; Node tests compile and pass; all focused Jest tests pass.

- [ ] **Step 3: Review the diff for terminology and scope**

  Run in each worktree: `git diff --check` and `git status --short`.

  Expected: no whitespace errors; no new save-side type/function/test uses `ValidationError` or calls the constraint guard “validation”; unrelated pre-existing files remain untouched.

- [ ] **Step 4: Commit only if verification required a scoped correction**

  If formatting or a test-only correction changed tracked files, stage only those named files and commit with `chore: verify structured save constraint errors`. Otherwise, do not create an empty commit.
