# PDF RBAC Compliance Design

**Date:** 2026-07-18

**Source of truth:** `QVIS Safety Database_UI Specification_18JUN2026_Updated.pdf`, especially PDF pages 7, 8, 41, 94, and 95. When the current code, generated contracts, or tests disagree with the PDF's latest annotation, the PDF wins.

## Goal

Make Role & Privilege settings control the menus, screens, API operations, and CASE workflow actions described by the PDF. Close the privilege-escalation and over-grant paths found during the audit, and prevent the frontend and backend from drifting again.

## Problem Summary and Root Cause

Earlier RBAC work solved important structural problems: it introduced a declarative backend menu policy, exposed raw effective permissions, generated the frontend permission catalog, and split CASE review and lock permissions in the backend. It did not complete the end-to-end behavior required by the PDF.

The remaining defects have four root causes:

1. The Role & Privilege matrix, generated permission catalog, UI action gates, and API authorization tests were updated at different times. A passing static matrix test can therefore describe behavior that the server no longer implements.
2. CASE Review and Lock are represented as lifecycle status edits through the general case update path. The UI still gates both actions with `Case.Approve`, cannot reliably unlock, and requires unrelated `Case.Update` behavior during cancellation or save.
3. Several menu privileges grant broad bundles for convenience. `case.read` includes XML export execution and user discovery, while `users.edit` is also treated as admission to role administration. These bundles cross the PDF's menu boundaries.
4. Some visible matrix rows have no consuming operation. Notice Read is not enforced at the dashboard boundary, and the E-mail Send permission is reserved because the RE mail delivery feature is not present.

## PDF Privilege Contract

Each PDF row is an independent contract. A role receives only the permissions required by that row and any explicit prerequisite listed below.

| PDF menu and row | Stored privilege | Effective behavior |
|---|---|---|
| HOME / Notice / Read | `home_notice.can_read` | View dashboard notices |
| HOME / Notice / Edit | `home_notice.can_edit` | View and update dashboard notices; Edit implies Read |
| HOME / My To Do / Read | `home_workflow.can_read` | View workflow-assigned cases |
| CASE / Case / Read | `case.can_read` | List and read cases only |
| CASE / Case / Edit | `case.can_edit` | Create cases and update case fields |
| CASE / Workflow / Read | `case_workflow.can_read` | View per-case workflow state and workflow columns |
| CASE / QC / Edit | `case.can_review` | Apply and cancel Review/QC |
| CASE / Lock / Edit | `case.can_lock` | Apply and cancel Lock |
| INFO / Case Info / Read | `info.can_read` | View INFO data |
| INFO / Case Info / Edit | `info.can_edit` | Create and update INFO data |
| IMPORT / Import Files / Edit | `import.can_edit` | View the import action screen and execute import |
| IMPORT / Import History / Read | `import.can_read` | View import history and use history downloads, but not import |
| EXPORT/SUBMISSION / Export/Submit / Edit | `export_submission.can_edit` | View and execute export or submission |
| EXPORT/SUBMISSION / History / Read | `export_submission.can_read` | View export/submission history and use history downloads, but not execute a new export/submission |
| E-mail / Send | `home_email.can_edit` | Grant `EmailNotification.Send` for the future RE mail feature |

`case_workflow` remains a distinct menu key. It must never be stored as `case.can_review`. Review and Lock flags are accepted only for the `case` menu key; backend normalization rejects or strips them everywhere else.

The PDF's latest page 95 annotation retains E-mail Send and deletes the proposed Settings Read item. The unmerged three-row E-mail subscription model is therefore not adopted.

## Single Execution Authority

The backend permission catalog and declarative menu policy remain the execution authority. The frontend generated permission enum is regenerated from that catalog and checked for freshness in CI.

The PDF matrix row declaration remains a frontend presentation concern, but every row is paired with:

- its expected backend permissions;
- a real API or action probe when a consuming feature exists; and
- an explicit `reserved` marker only for E-mail Send while the RE mail feature is absent.

A reserved row cannot be reported as operational. Once the RE mail feature is implemented, its send operation must require `EmailNotification.Send` and the reservation is removed.

## CASE Review and Lock Actions

Review and Lock are explicit server actions rather than client-selected generic status edits.

### Review toggle

- `draft -> reviewed` applies Review.
- `reviewed -> draft` cancels Review.
- `validated -> draft` cancels the completed QC state exposed by the same PDF button.
- The action requires `Case.Approve` and does not require `Case.Update`.
- Review-only users cannot change ordinary case fields.

### Lock toggle

- On lock, the server persists the current lifecycle status in a nullable `status_before_lock` column and changes the status to `locked` in the same transaction.
- On unlock, the server restores `status_before_lock` and clears it in the same transaction.
- The action requires `Case.Lock` and does not require `Case.Update` or `Case.Approve`.
- A lock action from an unsupported terminal state is rejected.
- A locked row without a valid saved prior state is treated as inconsistent data and returns a conflict instead of guessing a destination.

The server locks the case row while deciding either toggle. Concurrent clicks therefore operate on the latest committed state and cannot overwrite the saved pre-lock state.

The general case update route continues to validate status changes through the same lifecycle domain service, so callers cannot bypass Review or Lock authorization by submitting a raw `status` field.

Every successful toggle writes the existing audit record with the actor, previous status, next status, and reason metadata. Failure leaves both lifecycle state and audit history unchanged.

## CASE User Interface

The frontend derives three separate booleans from raw permissions:

- `canViewCaseWorkflow` from `Case.Read` and `Case.List` for workflow state display;
- `canReviewCase` from `Case.Approve`;
- `canLockCase` from `Case.Lock`.

The Review and Lock buttons call their dedicated toggle actions. They do not save unrelated dirty fields as a side effect. Ordinary editable fields remain controlled by `Case.Update`.

Review and Lock may make data inputs read-only, but Audit Trail controls are not data inputs. Audit buttons and modals remain enabled and fetchable for a user who can read the case, including while the case is reviewed, validated, or locked.

## Notice and Menu Visibility

Dashboard Notice content is omitted from the dashboard response unless the user has `DashboardNotice.Read`. Notice update requires `DashboardNotice.Update`; Edit grants both Read and Update.

Routes, sidebar items, page actions, and API handlers use the same effective permissions. A role with no privileges sees no protected menu other than the unavoidable authenticated shell. HOME is not an unconditional sidebar exception.

Import and Export/Submission split viewing history from executing an action. History downloads remain part of Read, as specified by the note on PDF page 8.

## Administration Boundaries

`users.edit` grants user create, update, and delete operations only. It does not satisfy the admin gate for permission-profile or role CRUD. Role administration requires a built-in administrator or explicit role-administration permissions. The two existing admin gate implementations must call one shared predicate so they cannot drift.

`case.read` grants case list and case read only. It does not grant XML export execution or user read/list permissions. Screens that need user names obtain a constrained display projection through an appropriately authorized endpoint rather than widening CASE access.

Built-in role visibility follows the current account context:

- a system administrator may administer the built-in roles allowed by the platform;
- a sponsor administrator sees only the sponsor administrator type assigned to the account plus its custom roles;
- unrelated built-in administrator roles are not returned merely because they exist globally.

## Role Editor Behavior

Role privilege checkboxes edit a local draft. Changes are persisted only when the user clicks the explicit Save button required by PDF page 95.

On Save, the client sends a sanitized payload and the backend normalizes it again. On failure, the draft remains visible, the role remains marked unsaved, and the error is shown. On success, the response replaces the draft and clears the dirty indicator.

Custom role deletion remains soft deletion: the row and its contents remain visible with strikethrough and can be restored. The maximum is 20 active custom roles per applicable account scope. Creating or restoring a role rechecks the limit atomically.

## Error Semantics

- Missing privilege: HTTP 403.
- Valid request against an incompatible or concurrently changed lifecycle state: HTTP 409.
- Invalid request shape or unknown role privilege key: HTTP 400.
- Role-limit breach: HTTP 409 with a stable role-limit error code.
- Server failures never produce a false saved or toggled UI state.

## Verification Strategy

Implementation follows test-driven development. Each behavior begins with a test that fails for the current defect.

### Backend policy tests

- Assert the exact effective permission set for every PDF row.
- Assert non-CASE Review/Lock flags cannot survive normalization.
- Assert `case.read` excludes XML export and user permissions.
- Assert `users.edit` does not satisfy role-administration access.

### Backend API tests

- Exercise Review apply/cancel with a Review-only role.
- Exercise Lock apply/cancel from draft, reviewed, and validated states with a Lock-only role, including restoration after a fresh request context.
- Reject Review with Edit-only and Lock with Review-only roles.
- Prove ordinary fields cannot be modified by Review-only or Lock-only roles.
- Prove Audit Trail remains readable in reviewed, validated, and locked states.
- Prove Notice Read and Edit independently at the response and update endpoints.
- Prove Import/Export history readers cannot execute new actions.
- Prove user editors cannot create roles, update their own role assignment, or enter permission-profile CRUD.
- Prove role limit and restore behavior.

### Frontend tests

- Assert the PDF row labels and menu keys, including `CASE / Workflow / Read`, `CASE / QC / Edit`, `CASE / Lock / Edit`, and `E-mail / Send`.
- Assert Review and Lock use separate generated permissions and separate action calls.
- Assert a second click cancels Review or restores the prior Lock state returned by the server.
- Assert Audit Trail controls remain enabled when data fields are read-only.
- Assert menu and route visibility for no-permission, history-only, action-only, and Notice-only roles.
- Assert checkbox edits remain local until Save and survive a failed Save.
- Assert soft delete, strikethrough, restore, account-specific built-in visibility, and the 20-role limit UI.

### Cross-repository contract checks

- Regenerate and diff the frontend permission catalog from the current backend catalog.
- Fail when any non-reserved matrix row lacks an effective-access probe.
- Fail when a contract expects permissions not produced by the backend menu policy.
- Replace the stale test that claimed Workflow, Review, and Lock shared `Case.Approve + Case.Update`.

## Migration and Compatibility

Add `cases.status_before_lock` as nullable text through the normal database migration and bootstrap schema paths. Existing unlocked cases require no backfill. Existing locked cases have no reliable prior status; they remain readable and auditable, but unlock returns a conflict until an administrator resolves the inconsistent lifecycle state. The implementation must not silently assume `validated` or `draft` for those rows.

Legacy permission profiles are normalized when read for effective access and when saved. Unsupported Review/Lock flags never grant access even before a profile is resaved.

## Scope Boundaries

This work implements the PDF's RBAC and Role & Privilege behavior. It does not invent the absent RE scheduled-mail delivery system; it retains and accurately labels the E-mail Send permission for that future feature. It also avoids unrelated cleanup of duplicate API response aliases or dead code unless a specific item must change to enforce the approved permission boundary.

## Completion Criteria

The work is complete when:

1. every non-reserved PDF privilege row controls its named visible UI and server operation;
2. Review and Lock apply and cancel with separate permissions;
3. Lock restores the persisted pre-lock state;
4. Audit Trail remains available in reviewed, validated, and locked states;
5. the known `case.read` and `users.edit` over-grants are closed;
6. Role editing has explicit Save, soft delete/restore, account-scoped built-in visibility, and a 20-active-role limit;
7. E-mail Send remains present and explicitly reserved rather than being replaced by unsupported subscription rows; and
8. backend, frontend, and cross-repository tests pass from clean isolated worktrees.
