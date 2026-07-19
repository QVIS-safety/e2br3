# PDF RBAC Compliance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every non-reserved Role & Privilege row in the 18 June 2026 QVIS PDF control its real menu, UI action, and API authorization while closing the known CASE and user-administration over-grants.

**Architecture:** Keep the backend permission catalog and `MENU_POLICIES` as the execution authority, and generate the frontend permission constants from it. Put Review and Lock behind atomic server toggle actions, persist the pre-lock status, and make the frontend consume the returned case state. Separate user CRUD admission from built-in role administration, redact dashboard notices at the runtime-settings boundary, and stage Role & Privilege edits until an explicit Save.

**Tech Stack:** Rust, Axum, SQLx/PostgreSQL, Tokio integration tests, Next.js/React, TypeScript, Jest/Testing Library, npm permission-generator scripts.

## Global Constraints

- `QVIS Safety Database_UI Specification_18JUN2026_Updated.pdf` pages 7, 8, 41, 94, and 95 are the source of truth.
- E-mail keeps the single `Send` row backed by `EmailNotification.Send`; do not restore the unsupported three subscription rows.
- CASE Workflow Read, QC Edit, and Lock Edit are independent rows and permissions.
- Unlock restores the server-persisted status from immediately before Lock; it never guesses `draft` or `validated`.
- Existing user changes in the shared frontend `dev` worktree and untracked backend artifacts must not be modified.
- Production changes follow RED-GREEN-REFACTOR; every behavior must have a failing test first.

## Repository Workspaces

- Backend: `/Users/hyundonghoon/projects/rust/e2br3/e2br3/.worktrees/pdf-rbac-compliance`
- Frontend execution worktree to create from local frontend `dev`: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/.worktrees/pdf-rbac-compliance`
- Design: `docs/superpowers/specs/2026-07-18-pdf-rbac-compliance-design.md`

---

### Task 1: Remove CASE over-grants and split user CRUD from Role administration

**Files:**
- Modify: `crates/libs/lib-core/src/model/acs/builtin_roles.rs`
- Modify: `crates/libs/lib-core/src/model/acs/menu_policy.rs`
- Modify: `crates/libs/lib-rest-core/src/lib.rs`
- Modify: `crates/libs/lib-web/src/middleware/mw_permission.rs`
- Modify: `crates/services/web-server/src/web/rest/user_rest/handlers.rs`
- Modify: `crates/services/web-server/src/web/rest/permission_profile_rest.rs`
- Test: `crates/libs/lib-core/src/model/acs/menu_policy.rs`
- Test: `crates/services/web-server/tests/api/role_admin/effective_access/administration_web.rs`

**Interfaces:**
- Consumes: existing `Permission`, `has_permission`, `Ctx::is_admin()`, and exact per-handler `USER_*` checks.
- Produces: shared `lib_core::model::acs::can_access_user_admin(ctx: &Ctx) -> bool`, `require_user_admin(ctx, mm)`, and `require_role_admin(ctx)`, with Role/Profile endpoints restricted to built-in administrators.

- [ ] **Step 1: Add failing policy tests for the PDF CASE boundary**

```rust
#[test]
fn case_read_excludes_export_execution_and_user_discovery() {
    let permissions = expand_menu_privileges(&[AdminMenuPrivilege {
        menu_key: "case".into(), can_read: true, can_edit: false,
        can_review: false, can_lock: false,
    }]);
    assert!(permissions.contains(&CASE_READ));
    assert!(permissions.contains(&CASE_LIST));
    assert!(!permissions.contains(&XML_EXPORT));
    assert!(!permissions.contains(&USER_READ));
    assert!(!permissions.contains(&USER_LIST));
}

#[test]
fn case_edit_does_not_implicitly_export_xml() {
    let permissions = expand_menu_privileges(&[AdminMenuPrivilege {
        menu_key: "case".into(), can_read: false, can_edit: true,
        can_review: false, can_lock: false,
    }]);
    assert!(permissions.contains(&CASE_UPDATE));
    assert!(!permissions.contains(&XML_EXPORT));
}
```

- [ ] **Step 2: Run the policy tests and verify RED**

Run: `cargo test -p lib-core case_ -- --nocapture`

Expected: FAIL because `viewer_permissions()` includes `USER_PERMISSIONS` and `XML_EXPORT_PERMISSIONS`, and `profile_edit_permissions()` includes export.

- [ ] **Step 3: Narrow the CASE bundles**

Remove these selections from `VIEWER_PERMISSIONS`:

```rust
USER_PERMISSIONS => [Read, List],
XML_EXPORT_PERMISSIONS => [Export],
```

Remove this selection from `PROFILE_EDIT_PERMISSIONS`:

```rust
XML_EXPORT_PERMISSIONS => [Export],
```

Keep the existing constrained `/api/users/workflow-options` endpoint authorized by `CASE_READ` for display names.

- [ ] **Step 4: Add failing administration tests**

Change `test_users_and_roles_matrix_privileges_grant_effective_admin_permissions` into two tests. The users-edit role must create/update users but receive `403` from `/api/admin/permission-profiles`; it must also receive `403` when attempting to change any user's `role`. A built-in sponsor administrator must retain Role/Profile CRUD.

```rust
assert_eq!(create_user_status, StatusCode::CREATED);
assert_eq!(create_profile_status, StatusCode::FORBIDDEN);
assert_eq!(assign_role_status, StatusCode::FORBIDDEN);
assert_eq!(sponsor_create_profile_status, StatusCode::CREATED);
```

- [ ] **Step 5: Run the administration test and verify RED**

Run: `cargo test -p web-server --test api role_admin::effective_access::administration_web::test_users_edit_cannot_manage_roles_or_assign_roles -- --nocapture`

Expected: FAIL because `USER_CREATE` currently satisfies both `RequireAdmin` and `require_admin`, and Role/Profile handlers call both gates.

- [ ] **Step 6: Implement distinct gates and role-assignment protection**

Add the shared predicate in `lib-core::model::acs` and consume it from both `lib-rest-core` and `lib-web`:

```rust
pub fn can_access_user_admin(ctx: &Ctx) -> bool {
    ctx.is_admin()
        || [USER_LIST, USER_READ, USER_CREATE, USER_UPDATE, USER_DELETE]
            .into_iter()
            .any(|permission| has_permission(ctx.permission_subject(), permission))
}
```

In `lib-rest-core`, make the request-level helper call that predicate:

```rust

pub async fn require_user_admin(ctx: &Ctx, mm: &ModelManager) -> Result<()> {
    let _ = mm;
    if can_access_user_admin(ctx) { Ok(()) } else {
        Err(Error::AccessDenied { required_role: "user administration".into() })
    }
}

pub fn require_role_admin(ctx: &Ctx) -> Result<()> {
    if ctx.is_admin() { Ok(()) } else {
        Err(Error::AccessDenied { required_role: "role administration".into() })
    }
}
```

Make `RequireAdmin` delegate to the same `lib-core` predicate so extractor and function cannot drift. User handlers call `require_user_admin` plus their exact `USER_*` permission. Before accepting a non-default `data.role` in create or any `data.role` change in update, return `PermissionDenied` unless `ctx.is_admin()`; non-built-in user editors may create the default user role and manage non-role fields only. Permission-profile handlers remove `RequireAdmin` and call `require_role_admin` exactly once.

- [ ] **Step 7: Run focused and regression tests**

Run: `cargo test -p lib-core model::acs -- --nocapture`

Run: `cargo test -p web-server --test api role_admin::effective_access::administration_web -- --nocapture`

Expected: PASS; users.edit remains operational for user fields but cannot manage or assign roles.

- [ ] **Step 8: Commit Task 1**

```bash
git add crates/libs/lib-core/src/model/acs/builtin_roles.rs crates/libs/lib-core/src/model/acs/menu_policy.rs crates/libs/lib-rest-core/src/lib.rs crates/libs/lib-web/src/middleware/mw_permission.rs crates/services/web-server/src/web/rest/user_rest/handlers.rs crates/services/web-server/src/web/rest/permission_profile_rest.rs crates/services/web-server/tests/api/role_admin/effective_access/administration_web.rs
git commit -m "fix: enforce PDF RBAC administration boundaries"
```

### Task 2: Persist and atomically toggle CASE Review and Lock

**Files:**
- Create: `db/migrations/20260718_case_status_before_lock.sql`
- Modify: `db/bootstrap/01-safetydb-schema.sql`
- Modify: `crates/libs/lib-core/src/model/case.rs`
- Modify: `crates/services/web-server/src/web/rest/case_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/routes/cases.rs`
- Modify: `crates/services/web-server/src/web/rest/permission_contract.rs`
- Test: `crates/services/web-server/tests/api/role_admin/review_lock_web.rs`

**Interfaces:**
- Consumes: `CaseBmc`, RLS transaction setup, `CASE_APPROVE`, `CASE_LOCK`, and the existing case read DTO conversion.
- Produces: `CaseBmc::toggle_review`, `CaseBmc::toggle_lock`, `POST /api/cases/{id}/review/toggle`, and `POST /api/cases/{id}/lock/toggle`, each returning the refreshed case.

- [ ] **Step 1: Rewrite the Review/Lock integration test for toggle semantics**

Add helpers that POST to the two new routes and assert:

```rust
let (_, reviewed) = toggle_review(&app, &reviewer_cookie, case_id).await?;
assert_eq!(reviewed["data"]["status"], "reviewed");
let (_, draft) = toggle_review(&app, &reviewer_cookie, case_id).await?;
assert_eq!(draft["data"]["status"], "draft");

set_case_status_for_test(&mm, case_id, "validated").await?;
let (_, locked) = toggle_lock(&app, &locker_cookie, case_id).await?;
assert_eq!(locked["data"]["status"], "locked");
let (_, restored) = toggle_lock(&app, &locker_cookie, case_id).await?;
assert_eq!(restored["data"]["status"], "validated");
```

Repeat Lock restoration from `draft` and `reviewed`. Assert editor-only cannot Review, reviewer cannot Lock, locker cannot edit `dg_prd_key`, and a legacy locked row with null `status_before_lock` returns `409`.

- [ ] **Step 2: Run the test and verify RED**

Run: `cargo test -p web-server --test api role_admin::review_lock_web -- --nocapture`

Expected: FAIL with 404 for the toggle endpoints and missing `status_before_lock` storage.

- [ ] **Step 3: Add the migration and bootstrap column**

`db/migrations/20260718_case_status_before_lock.sql`:

```sql
ALTER TABLE cases
    ADD COLUMN IF NOT EXISTS status_before_lock VARCHAR(50);

ALTER TABLE cases
    ADD CONSTRAINT case_status_before_lock_valid
    CHECK (
        status_before_lock IS NULL OR
        status_before_lock IN ('draft', 'reviewed', 'validated')
    );
```

Add the same nullable column and constraint to the bootstrap `cases` table. Add `status_before_lock: Option<String>` to `Case` and all explicit case SELECT projections.

- [ ] **Step 4: Implement atomic domain toggles**

Implement a private transaction helper that sets the full RLS context, selects `status, status_before_lock FROM cases WHERE id = $1 FOR UPDATE`, validates the transition, and performs exactly one update:

```rust
pub enum CaseLifecycleAction { ToggleReview, ToggleLock }

pub async fn toggle_lifecycle(
    ctx: &Ctx,
    mm: &ModelManager,
    id: Uuid,
    action: CaseLifecycleAction,
) -> Result<()> {
    // ToggleReview: draft -> reviewed; reviewed|validated -> draft.
    // ToggleLock: draft|reviewed|validated -> locked and save previous;
    //             locked -> saved previous and clear status_before_lock.
    // Return Error::Conflict for terminal/inconsistent states.
}
```

Bind all SQL values; do not interpolate statuses. Update `updated_at` and `updated_by` in the same statement so existing audit infrastructure records the lifecycle change.

- [ ] **Step 5: Add REST handlers and route contracts**

```rust
pub async fn toggle_case_review(State(mm): State<ModelManager>, ctx_w: CtxW, Path(id): Path<Uuid>) -> Result<(StatusCode, Json<DataRestResult<CaseReadResult>>)> {
    let ctx = ctx_w.0;
    require_permission(&ctx, CASE_APPROVE)?;
    CaseBmc::toggle_lifecycle(&ctx, &mm, id, CaseLifecycleAction::ToggleReview).await.map_err(Error::Model)?;
    let row = CaseBmc::get(&ctx, &mm, id).await?;
    Ok((StatusCode::OK, Json(DataRestResult { data: case_to_read_result(&ctx, &mm, row).await? })))
}
```

Add the equivalent Lock handler with `CASE_LOCK`, register both POST routes, and replace the stale `case.approve` permission contract with `case.review.toggle` and `case.lock.toggle` entries.

- [ ] **Step 6: Prevent raw status updates from bypassing toggle rules**

In `update_case`, reject raw transitions entering/leaving `locked` and entering/leaving `reviewed`/`validated` with `400` instructing callers to use the toggle endpoints. Continue allowing non-workflow lifecycle transitions only under `CASE_UPDATE`. This makes the dedicated service the sole Review/Lock state writer.

- [ ] **Step 7: Run migration, lifecycle, validation, and audit tests**

Run: `cargo test -p web-server --test api role_admin::review_lock_web -- --nocapture`

Run: `cargo test -p web-server --test api case_validation_web::test_locked_case_rejects_content_updates -- --nocapture`

Run: `cargo test -p web-server --test api audit -- --nocapture`

Expected: PASS; Lock restores all three supported prior states and Audit Trail stays readable.

- [ ] **Step 8: Commit Task 2**

```bash
git add db/migrations/20260718_case_status_before_lock.sql db/bootstrap/01-safetydb-schema.sql crates/libs/lib-core/src/model/case.rs crates/services/web-server/src/web/rest/case_rest.rs crates/services/web-server/src/web/rest/routes/cases.rs crates/services/web-server/src/web/rest/permission_contract.rs crates/services/web-server/tests/api/role_admin/review_lock_web.rs
git commit -m "fix: toggle case review and lock atomically"
```

### Task 3: Enforce Notice Read and account-scoped Role metadata

**Files:**
- Modify: `crates/services/web-server/src/web/rest/admin_settings_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/permission_profile_rest.rs`
- Test: `crates/services/web-server/tests/api/role_admin/effective_access/dashboard_web.rs`
- Test: `crates/services/web-server/tests/api/role_admin/metadata_web.rs`

**Interfaces:**
- Consumes: `DASHBOARD_NOTICE_READ`, `DASHBOARD_NOTICE_UPDATE`, `Ctx` built-in role predicates, and existing soft-delete/update behavior.
- Produces: runtime settings with permission-redacted `notices`, built-in Role rows appropriate to the current account, and atomic enforcement of the 20-active-custom-role limit on create and restore.

- [ ] **Step 1: Add failing Notice, transfer, and Role metadata tests**

Assert a no-notice role receives runtime settings with `notices: []`, a Notice Read role receives seeded notices, Notice Edit implies both read and update, a CRO sponsor account sees `sponsor_admin_cro` but not system/company roles, and restoring the twenty-first active custom role returns `409`. Preserve the transfer contract: History Read permits list/download and returns `403` for a new Import or Export/Submission execution.

- [ ] **Step 2: Run tests and verify RED**

Run: `cargo test -p web-server --test api role_admin::effective_access::dashboard_web -- --nocapture`

Run: `cargo test -p web-server --test api role_admin::metadata_web -- --nocapture`

Run: `cargo test -p web-server --test api role_admin::effective_access::transfer_web -- --nocapture`

Expected: FAIL because runtime settings expose notices to every authenticated user and built-in role rows are not account-scoped.

- [ ] **Step 3: Redact Notice content at the response boundary**

After loading runtime settings:

```rust
if !has_permission(ctx.permission_subject(), DASHBOARD_NOTICE_READ) {
    payload.notices = Some(Vec::new());
}
```

Keep `update_admin_notices` protected by `DASHBOARD_NOTICE_UPDATE`. Verify `home_notice.can_edit` still expands to both permissions.

- [ ] **Step 4: Return only applicable built-in roles**

Replace the unconditional `built_in_roles()` result with a `visible_built_in_roles(ctx)` match:

```rust
match canonical_role(ctx.role()).as_str() {
    ROLE_SYSTEM_ADMIN => vec![system_admin_row(), sponsor_cro_row(), sponsor_company_row()],
    ROLE_SPONSOR_ADMIN_CRO => vec![sponsor_cro_row()],
    ROLE_SPONSOR_ADMIN_COMPANY => vec![sponsor_company_row()],
    _ => Vec::new(),
}
```

Built-in rows are non-editable. Custom rows remain organization-scoped through existing RLS.

- [ ] **Step 5: Make the active-role limit atomic**

For create and `active: false -> true`, count active custom profiles inside the same transaction used for the write. Return `model::Error::Conflict { message: "active custom role limit is 20" }` at 20. Soft deletion does not erase content; restoration rechecks the limit.

- [ ] **Step 6: Run focused tests and commit**

Run: `cargo test -p web-server --test api role_admin -- --nocapture`

Expected: PASS.

```bash
git add crates/services/web-server/src/web/rest/admin_settings_rest.rs crates/services/web-server/src/web/rest/permission_profile_rest.rs crates/services/web-server/tests/api/role_admin/effective_access/dashboard_web.rs crates/services/web-server/tests/api/role_admin/effective_access/transfer_web.rs crates/services/web-server/tests/api/role_admin/metadata_web.rs
git commit -m "fix: enforce notice and role metadata privileges"
```

### Task 4: Restore the PDF matrix and generated frontend permission contract

**Files:**
- Frontend modify: `lib/admin/roleConfig.ts`
- Frontend modify: `lib/auth/generated-permissions.ts` by generator only
- Frontend modify: `app/(protected)/admin/role-privilege/model/effectiveAccessContract.ts`
- Frontend modify: `__tests__/role-privilege-rows.test.ts`
- Frontend modify: `__tests__/integration/role-privilege-effective-access.contract.test.ts`

**Interfaces:**
- Consumes: backend generated permission JSON and the backend endpoint contract.
- Produces: the exact PDF row list, `Permission.CaseLock`, and effective-access entries for Workflow Read, Review toggle, Lock toggle, and reserved E-mail Send.

- [ ] **Step 1: Write failing matrix and contract assertions**

```ts
expect(findRow("CASE", "Workflow", "Read")).toMatchObject({ menuKey: "case_workflow", field: "canRead" });
expect(findRow("CASE", "QC", "Edit")).toMatchObject({ menuKey: "case", field: "canReview" });
expect(findRow("CASE", "Lock", "Edit")).toMatchObject({ menuKey: "case", field: "canLock" });
expect(findRow("E-MAIL", "E-mail", "Send")).toMatchObject({ menuKey: "home_email", field: "canEdit" });
expect(ROLE_PRIVILEGE_ROWS.some((row) => row.menuKey.startsWith("email_"))).toBe(false);
expect(permissionsFor("case", "canLock")).toEqual(["Case.Lock"]);
```

- [ ] **Step 2: Run frontend tests and verify RED**

Run: `npm test -- --runInBand __tests__/role-privilege-rows.test.ts __tests__/integration/role-privilege-effective-access.contract.test.ts`

Expected: FAIL because local frontend dev contains subscription rows, labels Review rather than QC, and its generated catalog is stale.

- [ ] **Step 3: Regenerate permissions and implement the exact row list**

Run from the clean frontend worktree:

```bash
E2BR3_BACKEND_ROOT=/Users/hyundonghoon/projects/rust/e2br3/e2br3/.worktrees/pdf-rbac-compliance npm run generate:permissions
```

Set the final rows to `case_workflow/Workflow/Read`, `case/QC/Edit`, `case/Lock/Edit`, and `home_email/E-mail/Send`. Remove all `email_report_due`, `email_review`, and `email_lock` rows.

- [ ] **Step 4: Replace static probes with real action contracts**

Use:

```ts
{ row: matrixRow("case", "QC", "Edit"), expectedPermissions: [Permission.CaseApprove], probe: { kind: "endpoint", method: "POST", path: "/api/cases/{id}/review/toggle", allowedStatuses: [200] } }
{ row: matrixRow("case", "Lock", "Edit"), expectedPermissions: [Permission.CaseLock], probe: { kind: "endpoint", method: "POST", path: "/api/cases/{id}/lock/toggle", allowedStatuses: [200] } }
{ row: matrixRow("home_email", "E-mail", "Send"), expectedPermissions: [Permission.EmailNotificationSend], probe: { kind: "reserved", reason: "RE scheduled-mail feature is not implemented" } }
```

Extend `EffectiveAccessProbe` with the explicit reserved variant; the contract test permits a missing endpoint only for that variant.

- [ ] **Step 5: Run tests and permission freshness check**

Run: `npm test -- --runInBand __tests__/role-privilege-rows.test.ts __tests__/integration/role-privilege-effective-access.contract.test.ts`

Run: `E2BR3_BACKEND_ROOT=/Users/hyundonghoon/projects/rust/e2br3/e2br3/.worktrees/pdf-rbac-compliance npm run check:permissions`

Expected: PASS.

- [ ] **Step 6: Commit Task 4 in the frontend repository**

```bash
git add lib/admin/roleConfig.ts lib/auth/generated-permissions.ts 'app/(protected)/admin/role-privilege/model/effectiveAccessContract.ts' __tests__/role-privilege-rows.test.ts __tests__/integration/role-privilege-effective-access.contract.test.ts
git commit -m "fix: align role privilege contract with QVIS PDF"
```

### Task 5: Wire frontend Review/Lock toggles and preserve Audit Trail access

**Files:**
- Frontend modify: `lib/auth/case-permissions.ts`
- Frontend modify: `lib/auth/access-rules.ts`
- Frontend modify: `lib/api/endpoints/cases/core/crud.ts`
- Frontend modify: `components/case-form/CaseHeader.tsx`
- Frontend modify: `components/case-form/CaseFormLayout.tsx`
- Frontend modify: `components/case-form/hooks/useCaseEditorWorkflowActions.ts`
- Frontend test: `__tests__/case-workflow.integration.test.ts`
- Frontend test: `__tests__/case-form/CaseHeader.authority-selector.test.ts`
- Frontend test: `__tests__/audit-trail-commonization.test.ts`

**Interfaces:**
- Consumes: `Permission.CaseApprove`, `Permission.CaseLock`, and the two backend toggle endpoints.
- Produces: `canReviewCase`, `canLockCase`, `api.cases.toggleReview(id)`, and `api.cases.toggleLock(id)`.

- [ ] **Step 1: Add failing UI action tests**

Assert Review is enabled only by `Case.Approve`, Lock only by `Case.Lock`, each click calls its dedicated POST endpoint without calling `saveCaseWorkflow` or `handleSave`, a second click uses the same endpoint, and Audit Trail buttons remain clickable for `reviewed`, `validated`, and `locked` cases.

- [ ] **Step 2: Run tests and verify RED**

Run: `npm test -- --runInBand __tests__/case-workflow.integration.test.ts __tests__/case-form/CaseHeader.authority-selector.test.ts __tests__/audit-trail-commonization.test.ts`

Expected: FAIL because both buttons use `canApproveCase`, Lock sends `locked -> draft`, and the read-only fieldset disables Audit controls.

- [ ] **Step 3: Add permission helpers and API calls**

```ts
export const casePermissions = {
  canRead: (set: PermissionSet) => can(set, Permission.CaseRead),
  canEdit: (set: PermissionSet) => can(set, Permission.CaseUpdate),
  canReview: (set: PermissionSet) => can(set, Permission.CaseApprove),
  canLock: (set: PermissionSet) => can(set, Permission.CaseLock),
};

toggleReview: (id: string) => apiClient.post<BackendCase>(`/api/cases/${id}/review/toggle`),
toggleLock: (id: string) => apiClient.post<BackendCase>(`/api/cases/${id}/lock/toggle`),
```

Add `CaseLock` to CASE route access so a Lock-only role can reach the case it is authorized to lock.

- [ ] **Step 4: Replace client-selected statuses with server toggles**

`handleValidateCase` and `handleLockCase` require a persisted case ID, call the matching toggle once, reset the form from the returned `BackendCase`, and show success text based on the returned status. They do not call `handleSave`; dirty editable data must be saved separately before an action.

- [ ] **Step 5: Keep Audit controls outside blanket disabling**

Remove the disabled `<fieldset>` wrapper from `CaseFormLayout`. Continue passing `isReadOnly` to data-entry controls through existing section/form props, and add a regression assertion that every rendered editable input is disabled while `[aria-label*="audit" i]` buttons are not. Do not disable the Audit Trail modal trigger based on lifecycle state.

- [ ] **Step 6: Run tests and commit**

Run: `npm test -- --runInBand __tests__/case-workflow.integration.test.ts __tests__/case-form/CaseHeader.authority-selector.test.ts __tests__/audit-trail-commonization.test.ts`

Expected: PASS.

```bash
git add lib/auth/case-permissions.ts lib/auth/access-rules.ts lib/api/endpoints/cases/core/crud.ts components/case-form/CaseHeader.tsx components/case-form/CaseFormLayout.tsx components/case-form/hooks/useCaseEditorWorkflowActions.ts __tests__/case-workflow.integration.test.ts __tests__/case-form/CaseHeader.authority-selector.test.ts __tests__/audit-trail-commonization.test.ts
git commit -m "fix: wire dedicated case review and lock actions"
```

### Task 6: Enforce HOME Notice and menu visibility in the frontend

**Files:**
- Frontend modify: `lib/auth/access-rules.ts`
- Frontend modify: `components/case-form/CaseSidebar.tsx`
- Frontend modify: `components/dashboard/NoticePanel.tsx`
- Frontend test: `__tests__/sidebar.permissions.test.tsx`
- Frontend test: `__tests__/dashboard/notice-panel.test.tsx`
- Frontend test: `__tests__/dashboard/home-appendix-workflow.test.ts`
- Frontend test: `__tests__/rbac-contract/import-actions.test.tsx`
- Frontend test: `__tests__/rbac-contract/submission-actions.test.tsx`

**Interfaces:**
- Consumes: redacted runtime settings and `DashboardNotice.Read/Update`.
- Produces: a `home` access rule, hidden HOME/Notice UI without Read, and Edit implying a visible Notice panel.

- [ ] **Step 1: Add failing no-permission and Notice-only tests**

```ts
expect(homeLink(noPermissions)).toBeNull();
expect(homeLink([Permission.DashboardNoticeRead])).toBeTruthy();
expect(renderNoticePanel([]).container).toBeEmptyDOMElement();
expect(renderNoticePanel([Permission.DashboardNoticeUpdate]).getByText("Edit notices")).toBeTruthy();
```

- [ ] **Step 2: Run tests and verify RED**

Run: `npm test -- --runInBand __tests__/sidebar.permissions.test.tsx __tests__/dashboard/notice-panel.test.tsx __tests__/dashboard/home-appendix-workflow.test.ts`

Run: `npm test -- --runInBand __tests__/rbac-contract/import-actions.test.tsx __tests__/rbac-contract/submission-actions.test.tsx`

Expected: FAIL because HOME is unconditional and NoticePanel loads for users without Notice Read.

- [ ] **Step 3: Add and consume the HOME rule**

Add `home` to `AccessRuleKey` with `DashboardNoticeRead`, `DashboardNoticeUpdate`, `CaseRead`, and `CaseList`; attach it to the HOME sidebar item. `NoticePanel` returns `null` unless the user can Read or Update, and treats Update as implicit Read for UI rendering.

- [ ] **Step 4: Re-run tests and commit**

Run: `npm test -- --runInBand __tests__/sidebar.permissions.test.tsx __tests__/dashboard/notice-panel.test.tsx __tests__/dashboard/home-appendix-workflow.test.ts`

Run: `npm test -- --runInBand __tests__/rbac-contract/import-actions.test.tsx __tests__/rbac-contract/submission-actions.test.tsx`

Expected: PASS.

```bash
git add lib/auth/access-rules.ts components/case-form/CaseSidebar.tsx components/dashboard/NoticePanel.tsx __tests__/sidebar.permissions.test.tsx __tests__/dashboard/notice-panel.test.tsx __tests__/dashboard/home-appendix-workflow.test.ts __tests__/rbac-contract/import-actions.test.tsx __tests__/rbac-contract/submission-actions.test.tsx
git commit -m "fix: enforce home notice visibility"
```

### Task 7: Stage Role & Privilege edits until explicit Save

**Files:**
- Frontend modify: `app/(protected)/admin/AdminWorkspace.tsx`
- Frontend modify: `app/(protected)/admin/role/hooks/useAdminRoles.ts`
- Frontend modify: `app/(protected)/admin/role-privilege/hooks/useRolePrivilegeMatrix.ts`
- Frontend modify: `app/(protected)/admin/role-privilege/components/RolePrivilegeMatrix.tsx`
- Frontend modify: `app/(protected)/admin/role/model/adminRolesModel.ts`
- Frontend test: `__tests__/admin-users.header-filters.test.ts`

**Interfaces:**
- Consumes: sanitized `nextRolePrivilegesForCell/Column` and `permissionProfiles.updateProfile`.
- Produces: `draftRoles`, `dirtyRoleIds`, `setDraftRolePrivilege`, `saveRolePrivileges`, and `isSavingRoleId`.

- [ ] **Step 1: Add failing explicit-Save tests**

Render Role & Privilege, change one checkbox, and assert no update request occurs before Save. Click Save and assert one normalized profile update. Make that request fail and assert the checkbox stays changed, the role remains marked unsaved, and the error is visible. Keep existing soft-delete/strikethrough/restore and 20-role tests.

- [ ] **Step 2: Run the focused test and verify RED**

Run: `npm test -- --runInBand __tests__/admin-users.header-filters.test.ts -t 'Role & Privilege|role limit|restore'`

Expected: FAIL because checkbox handlers currently call `updateCustomRoleOptimistically` immediately and rollback on failure.

- [ ] **Step 3: Implement draft state**

In `useRolePrivilegeMatrix`, initialize a draft map from loaded roles and preserve dirty entries across re-renders:

```ts
const [draftPrivileges, setDraftPrivileges] = useState<Record<string, AdminMenuPrivilege[]>>({});
const [dirtyRoleIds, setDirtyRoleIds] = useState<Set<string>>(new Set());

const setDraftRolePrivileges = (roleId: string, privileges: AdminMenuPrivilege[]) => {
  setDraftPrivileges((current) => ({ ...current, [roleId]: privileges }));
  setDirtyRoleIds((current) => new Set(current).add(roleId));
};
```

Cell/column changes update only this map. `saveRolePrivileges(role)` sends the sanitized draft once. Success replaces the server role and clears dirty state; failure retains both draft and dirty state.

- [ ] **Step 4: Render Save and unsaved state**

Add one Save button per editable role column with an accessible label `Save privileges for <role>`. Disable it while that role is saving or when that role is clean. Display `Unsaved changes` for dirty roles.

Derive Role tab access from built-in admin identity rather than `User.Create/Update`, matching the backend Role/Profile gate. Keep User tab access permission-based.

- [ ] **Step 5: Re-run tests and commit**

Run: `npm test -- --runInBand __tests__/admin-users.header-filters.test.ts -t 'Role & Privilege|role limit|restore'`

Expected: PASS.

```bash
git add 'app/(protected)/admin/AdminWorkspace.tsx' 'app/(protected)/admin/role/hooks/useAdminRoles.ts' 'app/(protected)/admin/role-privilege/hooks/useRolePrivilegeMatrix.ts' 'app/(protected)/admin/role-privilege/components/RolePrivilegeMatrix.tsx' 'app/(protected)/admin/role/model/adminRolesModel.ts' __tests__/admin-users.header-filters.test.ts
git commit -m "fix: save role privileges explicitly"
```

### Task 8: Cross-repository verification and contract cleanup

**Files:**
- Frontend modify: `lib/auth/generated-endpoint-permissions.ts` by generator only
- Frontend modify: `__tests__/integration/role-privilege-effective-access.live.test.ts`

**Interfaces:**
- Consumes: all backend endpoint contracts and frontend matrix contracts.
- Produces: reproducible cross-repository freshness checks and final evidence for every PDF row.

- [ ] **Step 1: Run the generators and inspect diffs**

Run in frontend:

```bash
E2BR3_BACKEND_ROOT=/Users/hyundonghoon/projects/rust/e2br3/e2br3/.worktrees/pdf-rbac-compliance npm run generate:permissions
E2BR3_BACKEND_ROOT=/Users/hyundonghoon/projects/rust/e2br3/e2br3/.worktrees/pdf-rbac-compliance npm run generate:endpoint-permissions
```

Expected: only generated permission/endpoint artifacts change; no hand-edited generated file.

- [ ] **Step 2: Run backend verification**

Run:

```bash
cargo fmt --all -- --check
cargo test -p lib-core model::acs -- --nocapture
cargo test -p web-server --test api role_admin -- --nocapture
cargo test -p web-server --test api case_validation_web -- --nocapture
```

Expected: all PASS and no formatting diff.

- [ ] **Step 3: Run frontend verification**

Run:

```bash
npm run lint
npx tsc --noEmit
npm test -- --runInBand __tests__/role-privilege-rows.test.ts __tests__/integration/role-privilege-effective-access.contract.test.ts __tests__/case-workflow.integration.test.ts __tests__/sidebar.permissions.test.tsx __tests__/dashboard/notice-panel.test.tsx __tests__/admin-users.header-filters.test.ts __tests__/rbac-contract/import-actions.test.tsx __tests__/rbac-contract/submission-actions.test.tsx
E2BR3_BACKEND_ROOT=/Users/hyundonghoon/projects/rust/e2br3/e2br3/.worktrees/pdf-rbac-compliance npm run check:permissions
```

Expected: all PASS; the only reserved Role row is E-mail Send.

- [ ] **Step 4: Inspect both worktrees for unintended changes**

Run `git status --short` and `git diff --check` in both worktrees. Confirm no shared-worktree files, generated build directories, PDF extraction files, or unrelated source files are staged.

- [ ] **Step 5: Commit final generated/contract changes in the frontend**

```bash
git add lib/auth/generated-endpoint-permissions.ts __tests__/integration/role-privilege-effective-access.live.test.ts
git commit -m "test: verify PDF RBAC effective access"
```

Do not merge, push, or modify the shared `dev` worktrees without a separate user instruction.
