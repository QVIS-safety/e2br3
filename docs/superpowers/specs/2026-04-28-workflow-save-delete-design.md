# Workflow Save/Delete Stabilization Design

## Summary

This design defines the first implementation slice for the client requirements backlog: stabilizing case save and delete behavior. The target is a consistent backend lifecycle path for manual and imported cases, predictable user-facing save/delete results, and regression coverage that proves the lifecycle works across the currently supported case origins.

## Problem Statement

The current backlog identifies save and delete instability as a P0 case-lifecycle issue:

- page-level save is not reliable for both directly entered and imported cases
- normal case save can surface irrelevant batch/header-style errors
- delete behavior is not yet aligned with compliance-oriented case lifecycle expectations
- case-area list/history behavior after save/delete still needs verification

The codebase already has significant workflow, export, and role/scope infrastructure, but the save/delete path still behaves like a set of partially overlapping flows instead of one explicit lifecycle model. That creates risk that fixes for manual cases do not apply to imported cases, or that delete handling bypasses compliance/audit expectations.

## Goals

- Make case save reliable for both manual and imported cases.
- Ensure normal case save returns only relevant validation and persistence errors.
- Make delete behavior consistent, auditable, and aligned with case lifecycle rules.
- Verify that save/delete outcomes remain visible and understandable in list/history surfaces.
- Add focused automated regression coverage for both case-origin paths.

## Non-Goals

- Follow-up draft creation.
- Broad QC/lock redesign.
- Full workflow policy cleanup outside save/delete touchpoints.
- Form-wide validation re-audit.
- Export/submission UX redesign.
- INFO/admin terminology cleanup.

## Scope

This slice covers the backend and contract behavior for:

- saving existing cases created manually
- saving cases whose source was import
- delete action semantics for cases
- filtering irrelevant save-time error noise from import/batch contexts
- verifying list/history-visible consequences of save/delete

This slice may include minimal frontend changes only if required to consume corrected backend behavior or remove obviously incorrect save/delete messaging. It does not include general UI redesign.

## Design Approach

### 1. Unify case lifecycle handling

Save and delete should flow through one shared lifecycle path rather than separate origin-specific logic. Manual and imported cases can still carry different metadata, but persistence rules, compliance hooks, and response shaping should be centralized enough that the same invariants apply to both.

Required lifecycle invariants:

- save operates on a resolvable case identity regardless of origin
- save rejects only real persistence or validation problems for that case
- delete uses explicit lifecycle semantics rather than ad hoc record removal
- audit/compliance metadata is collected at the lifecycle boundary, not only in one origin path

### 2. Separate case-save errors from import/batch errors

Imported cases may have import-time or batch/header problems associated with their source history, but a user performing a normal case save should not receive unrelated import pipeline errors unless those errors still block persistence of the current case state. The save path should distinguish:

- current case validation failures
- current persistence/update failures
- historical import/batch issues that belong in import/export history, not save responses

### 3. Treat delete as a controlled lifecycle transition

Delete should behave like a controlled case action with compliance-aware semantics. This design intentionally avoids defining a broader retention policy beyond this slice, but it requires that delete no longer behave as a loose removal path. The implementation should make it explicit:

- what state change occurs when a case is deleted
- what audit/compliance data is recorded
- what list/history surfaces still expose the deleted case state
- what API response the client receives after a successful delete

### 4. Verify downstream visibility

This slice is not complete if save/delete only work at the endpoint level while case lists or history views become misleading. Verification must cover at least:

- case remains queryable or intentionally excluded according to the chosen lifecycle semantics
- delete state is observable in the relevant case-facing surfaces
- save results do not create contradictory workflow/list state for imported versus manual cases

## Affected Areas

Based on current repo structure, implementation is expected to center on:

- `crates/services/web-server/src/web/rest/case_rest.rs`
- `crates/services/web-server/src/web/rest/case_validation_rest.rs`
- `crates/services/web-server/src/web/rest/compliance.rs`
- `crates/services/web-server/src/web/rest/import_rest.rs`
- `crates/services/web-server/tests/api/case_contract_web.rs`
- `crates/services/web-server/tests/api/import_contract_web.rs`
- `crates/services/web-server/tests/api/import_history_web.rs`
- related helper/test fixtures under `crates/services/web-server/tests/helpers` and `crates/services/web-server/tests/common`

The exact file list may expand after implementation-level tracing, but the change should remain centered on case lifecycle and API contract tests rather than broad subsystem refactors.

## Expected Behavior

### Save

- Manual case save succeeds when the case payload is valid and persists through the same lifecycle rules used for imported cases.
- Imported case save succeeds when the current editable case payload is valid, without surfacing unrelated historical import/batch/header errors.
- Save failures return actionable case-level validation or persistence errors only.
- Save behavior should not diverge based solely on whether the case originated from manual entry or import.

### Delete

- Delete succeeds through an explicit lifecycle action, not an origin-specific shortcut.
- Delete records the compliance/audit information required by the existing system conventions.
- Delete leaves the system in a consistent state for subsequent list/history reads.
- Delete behavior is identical from the perspective of API contract regardless of original case origin, unless the business rule explicitly forbids deletion for a subset of cases. If such a rule exists in the code, it must be enforced consistently and covered by tests.

## Testing Strategy

This slice should be implemented test-first where practical and validated with contract-level API coverage.

Minimum regression coverage:

- save manual case success
- save imported case success
- save imported case does not leak unrelated import/batch/header errors
- delete manual case produces expected lifecycle result
- delete imported case produces expected lifecycle result
- post-delete list/history read reflects the intended state consistently

Testing should prefer existing API/integration harnesses in `crates/services/web-server/tests/api` rather than isolated unit tests alone, because the failure mode described by the backlog is behavioral and cross-layer.

## Risks and Constraints

- The existing code may couple save/delete behavior to workflow, import history, or compliance hooks in ways that are not obvious from the REST layer alone.
- There may be implicit assumptions in tests or fixtures that imported cases are immutable or handled differently; those assumptions need to be made explicit instead of preserved accidentally.
- Changing delete semantics can affect export/history/list behavior, so regression scope must include downstream reads, not only write endpoints.

## Success Criteria

This slice is complete when:

- manual and imported case save behavior is consistent and reliable
- normal case save no longer returns irrelevant import/batch/header errors
- delete behaves as one explicit lifecycle action with stable API behavior
- automated tests cover the manual/imported save/delete matrix and pass
- no unrelated requirement areas are partially refactored under the banner of this slice

## Decomposition For Planning

This spec is intentionally limited to one implementation plan and can be divided into the following independent execution tasks:

1. Trace and document the current save/delete lifecycle path and identify origin-specific branching.
2. Add failing tests that capture the current instability and noise.
3. Normalize backend save behavior across manual and imported cases.
4. Normalize delete lifecycle behavior and compliance/audit integration.
5. Verify list/history consequences and close remaining contract gaps.

These tasks are suitable for a subagent-driven execution plan after this spec is approved.
