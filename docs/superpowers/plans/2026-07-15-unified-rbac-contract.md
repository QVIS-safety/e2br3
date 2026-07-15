# Unified RBAC Contract Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace frontend capability and role-name authorization with the backend's granular `Resource.Action` permissions, including policy-version synchronization and cross-layer drift tests.

**Architecture:** Rust `Permission` values are the sole authorization vocabulary. The backend returns the effective permission set and a database-backed policy version; a generated TypeScript catalog feeds one frontend permission boundary used by routes, menus, and actions. Backend guards remain authoritative, while version headers keep the frontend permission snapshot synchronized.

**Tech Stack:** Rust/Axum/SQLx/PostgreSQL, Next.js 15/React 19/TypeScript, Cargo tests, Jest/Testing Library.

## Global Constraints

- Frontend and backend deploy together; remove legacy `capabilities` without a runtime fallback.
- Do not introduce mock authorization logic in production code.
- Preserve organization, sender, product, study, and blind-data scope behavior.
- Role names remain business data for display and workflow assignment, but never authorize UI or REST access.
- Every production change follows red-green TDD and is committed at its task boundary.
- Work in isolated worktrees created at execution time; the backend and frontend repositories need matching feature branches.
- Preserve unrelated dirty files in both repositories.

---

## File Map

### Backend repository: `/Users/hyundonghoon/projects/rust/e2br3/e2br3`

- `crates/libs/lib-core/src/model/acs/catalog.rs`: enumerate and serialize the complete permission catalog.
- `crates/libs/lib-core/src/model/acs/dynamic_roles.rs`: cache effective dynamic permissions with policy version.
- `crates/libs/lib-core/src/model/acs/types.rs`: canonical `Permission` string representation.
- `crates/services/web-server/src/web/rest/user_rest/dto.rs`: current-profile permission contract.
- `crates/services/web-server/src/web/rest/user_rest/handlers.rs`: resolve and return effective permissions/version.
- `crates/services/web-server/src/web/rest/user_rest/capabilities.rs`: delete after profile migration.
- `crates/services/web-server/src/web/rest/permission_profile_rest.rs`: increment policy version atomically with policy mutations.
- `crates/libs/lib-web/src/middleware/mw_res_map.rs`: stable permission-denied payload and policy-version response header.
- `db/bootstrap/01-safetydb-schema.sql`: bootstrap authorization policy state.
- `db/migrations/20260715_rbac_policy_version.sql`: existing database migration.
- `scripts/generate_frontend_permissions.rs`: generate the frontend catalog from Rust values.
- `crates/services/web-server/tests/api/role_admin/`: REST/profile/version contract tests.

### Frontend repository: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend`

- `lib/auth/generated-permissions.ts`: generated Permission union and constants.
- `lib/auth/permissions.ts`: `can`, `canAny`, and `canAll` pure functions.
- `lib/auth/PermissionGate.tsx`: declarative action gate.
- `lib/auth/access-rules.ts`: route, sidebar, and module action declarations.
- `lib/contexts/AuthContext.tsx`: effective permission set and atomic profile refresh.
- `lib/api/client.ts`: policy-version header observation and stable 403 parsing.
- `lib/auth/capabilities.ts`: delete after callers migrate.
- `lib/auth/roleAccess.ts`: retain business role helpers only; remove authorization helpers.
- `lib/auth/routeAccess.ts`: replace capability rules with Permission declarations.
- `components/Sidebar.tsx`: use shared route/menu declarations.
- `components/presave/`: gate INFO CRUD actions.
- `components/case-form/`: combine permission and lifecycle write constraints.
- `app/(protected)/import/page.tsx`: split history and execute permissions.
- `app/(protected)/submission/page.tsx`: split history and export permissions.
- `app/(protected)/data/page.tsx`: replace system-admin check with terminology permissions.
- `app/(protected)/admin/`: migrate administration actions to Permission values.
- `__tests__/auth/`: permission core, synchronization, and generated-contract tests.
- `__tests__/rbac-contract/`: module action and cross-layer manifest tests.

---

### Task 1: Canonical backend permission catalog

**Files:**
- Modify: `crates/libs/lib-core/src/model/acs/catalog.rs`
- Modify: `crates/libs/lib-core/src/model/acs/types.rs`
- Modify: `crates/libs/lib-core/src/model/acs/mod.rs`
- Test: `crates/libs/lib-core/src/model/acs/tests.rs`

**Interfaces:**
- Consumes: existing `Permission(Resource, Action)` constants.
- Produces: `pub fn all_permissions() -> &'static [Permission]` and deterministic `Permission::to_string()` values.

- [ ] **Step 1: Write the failing catalog completeness test**

Add a test that asserts uniqueness, stable `Resource.Action` syntax, and inclusion of representative edge permissions:

```rust
#[test]
fn permission_catalog_is_complete_unique_and_stable() {
    let values = all_permissions()
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let unique = values.iter().collect::<std::collections::HashSet<_>>();
    assert_eq!(unique.len(), values.len());
    assert!(values.iter().all(|value| {
        let mut parts = value.split('.');
        parts.next().is_some_and(|part| !part.is_empty())
            && parts.next().is_some_and(|part| !part.is_empty())
            && parts.next().is_none()
    }));
    for required in ["Case.Read", "StudyRegistration.Update", "XmlImport.Import", "XmlExport.Export"] {
        assert!(values.iter().any(|value| value == required), "missing {required}");
    }
}
```

- [ ] **Step 2: Run the test and verify it fails**

Run: `cargo test -p lib-core permission_catalog_is_complete_unique_and_stable -- --nocapture`

Expected: FAIL because `all_permissions` is not exported.

- [ ] **Step 3: Export one complete catalog**

Define a single `ALL_PERMISSIONS` slice in `catalog.rs`, constructed from every public permission constant, and expose:

```rust
pub fn all_permissions() -> &'static [Permission] {
    ALL_PERMISSIONS
}
```

Keep `Display` in `types.rs` as the only string formatter. Do not add a second string table.

- [ ] **Step 4: Run focused and library tests**

Run:

```bash
cargo test -p lib-core permission_catalog_is_complete_unique_and_stable -- --nocapture
cargo test -p lib-core model::acs -- --nocapture
```

Expected: both PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/libs/lib-core/src/model/acs
git commit -m "refactor: expose canonical permission catalog"
```

### Task 2: Effective permissions in the current-user profile

**Files:**
- Modify: `crates/services/web-server/src/web/rest/user_rest/dto.rs`
- Modify: `crates/services/web-server/src/web/rest/user_rest/handlers.rs`
- Delete: `crates/services/web-server/src/web/rest/user_rest/capabilities.rs`
- Modify: `crates/services/web-server/src/web/rest/user_rest/mod.rs`
- Modify: `crates/services/web-server/src/web/openapi.rs`
- Test: `crates/services/web-server/tests/api/role_admin/helpers.rs`
- Test: `crates/services/web-server/tests/api/role_admin/effective_access/information_web.rs`

**Interfaces:**
- Consumes: `all_permissions()` and `has_permission(subject, permission)`.
- Produces: `CurrentUserProfileView.permissions: Vec<String>` and no `capabilities` field.

- [ ] **Step 1: Change the profile helper test to require exact permissions**

Replace capability assertions with:

```rust
pub(super) async fn assert_profile_permissions(
    app: &Router,
    cookie: &str,
    present: &[&str],
    absent: &[&str],
) -> Result<Value> {
    let (status, profile) = request_json(
        app, "GET", cookie, "/api/users/me/profile".to_string(), None,
    ).await?;
    assert_eq!(status, StatusCode::OK, "{profile:?}");
    assert!(profile["data"].get("capabilities").is_none(), "{profile:?}");
    let permissions = profile["data"]["permissions"]
        .as_array().ok_or("missing permissions")?
        .iter().filter_map(Value::as_str).collect::<HashSet<_>>();
    for permission in present { assert!(permissions.contains(permission), "missing {permission}"); }
    for permission in absent { assert!(!permissions.contains(permission), "unexpected {permission}"); }
    Ok(profile)
}
```

Update the INFO read-only test to require `PresaveTemplate.Read/List` and reject its Create/Update/Delete permissions.

- [ ] **Step 2: Run the REST test and verify it fails**

Run: `cargo test -p web-server --test api role_admin::effective_access::information_web -- --nocapture`

Expected: FAIL because the profile still returns capabilities and lacks permissions.

- [ ] **Step 3: Implement deterministic effective permissions**

In `handlers.rs`, replace capability construction with:

```rust
let mut permissions = all_permissions()
    .iter()
    .copied()
    .filter(|permission| has_permission(ctx.permission_subject(), *permission))
    .map(|permission| permission.to_string())
    .collect::<Vec<_>>();
permissions.sort_unstable();
permissions.dedup();
```

Replace the DTO field with `pub permissions: Vec<String>`, delete capability DTOs and `capabilities.rs`, and update OpenAPI schemas in the same change.

- [ ] **Step 4: Run role-admin and OpenAPI verification**

Run:

```bash
cargo test -p web-server --test api role_admin::effective_access -- --nocapture
cargo check -p web-server
```

Expected: 13 effective-access tests PASS and web-server checks successfully.

- [ ] **Step 5: Commit**

```bash
git add crates/services/web-server/src/web/rest/user_rest crates/services/web-server/src/web/openapi.rs crates/services/web-server/tests/api/role_admin
git commit -m "feat: expose effective permissions in user profile"
```

### Task 3: Database-backed RBAC policy version

**Files:**
- Create: `db/migrations/20260715_rbac_policy_version.sql`
- Modify: `db/bootstrap/01-safetydb-schema.sql`
- Modify: `crates/libs/lib-core/src/model/acs/dynamic_roles.rs`
- Modify: `crates/services/web-server/src/web/rest/permission_profile_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/user_rest/dto.rs`
- Modify: `crates/services/web-server/src/web/rest/user_rest/handlers.rs`
- Test: `crates/services/web-server/tests/api/role_admin/effective_access/persistence_web.rs`

**Interfaces:**
- Consumes: permission-profile create/update/delete/replace transactions.
- Produces: `policyVersion: i64` in the profile and one monotonic database value.

- [ ] **Step 1: Write a failing policy-version mutation test**

The test must load `/api/users/me/profile`, mutate a permission profile, reload the profile, and assert:

```rust
let before = profile_before["data"]["policyVersion"]
    .as_i64().ok_or("missing policyVersion")?;
// update_role_privileges(...)
let after = profile_after["data"]["policyVersion"]
    .as_i64().ok_or("missing policyVersion")?;
assert!(after > before, "policy version did not advance: {before} -> {after}");
```

- [ ] **Step 2: Run the test and verify it fails**

Run: `cargo test -p web-server --test api role_admin::effective_access::persistence_web -- --nocapture`

Expected: FAIL because `policyVersion` is absent.

- [ ] **Step 3: Add the policy-state row and transactional bump**

Use one singleton row:

```sql
CREATE TABLE IF NOT EXISTS rbac_policy_state (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    version bigint NOT NULL DEFAULT 1 CHECK (version > 0),
    updated_at timestamptz NOT NULL DEFAULT now()
);
INSERT INTO rbac_policy_state (singleton, version)
VALUES (true, 1) ON CONFLICT (singleton) DO NOTHING;
```

In every permission-profile mutation transaction execute:

```sql
UPDATE rbac_policy_state
SET version = version + 1, updated_at = now()
WHERE singleton = true
RETURNING version
```

Refresh the process dynamic-role cache only after the transaction commits. Read the version for the profile response; do not invent per-process counters.

- [ ] **Step 4: Run migration and mutation tests**

Run:

```bash
cargo test -p web-server --test api role_admin::effective_access::persistence_web -- --nocapture
cargo test -p web-server --test api role_admin::crud_web -- --nocapture
```

Expected: PASS; create, update, delete, and replacement each advance the version.

- [ ] **Step 5: Commit**

```bash
git add db crates/libs/lib-core/src/model/acs/dynamic_roles.rs crates/services/web-server/src/web/rest/permission_profile_rest.rs crates/services/web-server/src/web/rest/user_rest crates/services/web-server/tests/api/role_admin
git commit -m "feat: version RBAC policy mutations"
```

### Task 4: Generate the TypeScript permission catalog

**Files:**
- Create: `crates/libs/lib-core/examples/export_permission_catalog.rs`
- Create: `scripts/generate_frontend_permissions.sh`
- Create in frontend: `lib/auth/generated-permissions.ts`
- Create in frontend: `__tests__/auth/generated-permissions.test.ts`
- Modify in frontend: `package.json`

**Interfaces:**
- Consumes: backend `all_permissions()`.
- Produces: frontend `PermissionValue`, `Permission`, and `ALL_PERMISSIONS`.

- [ ] **Step 1: Write a failing frontend catalog test**

```ts
import { ALL_PERMISSIONS, Permission } from "@/lib/auth/generated-permissions";

test("generated permission catalog is unique and contains endpoint permissions", () => {
  expect(new Set(ALL_PERMISSIONS).size).toBe(ALL_PERMISSIONS.length);
  expect(ALL_PERMISSIONS).toEqual(expect.arrayContaining([
    Permission.CaseRead,
    Permission.StudyRegistrationUpdate,
    Permission.XmlImportImport,
    Permission.XmlExportExport,
  ]));
});
```

- [ ] **Step 2: Run the test and verify it fails**

Run in frontend: `npm test -- --runInBand __tests__/auth/generated-permissions.test.ts`

Expected: FAIL because the module does not exist.

- [ ] **Step 3: Implement deterministic generation**

The Rust example prints sorted JSON from `all_permissions()`. The shell script transforms it into:

```ts
export const Permission = {
  CaseRead: "Case.Read",
  // every generated entry
} as const;
export type PermissionValue = typeof Permission[keyof typeof Permission];
export const ALL_PERMISSIONS: readonly PermissionValue[] = Object.values(Permission);
```

Add `npm run generate:permissions` and `npm run check:permissions`; the check regenerates to a temporary file and compares bytes with the committed artifact.

- [ ] **Step 4: Run generation and freshness tests**

Run:

```bash
./scripts/generate_frontend_permissions.sh /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/auth/generated-permissions.ts
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npm run check:permissions
npm test -- --runInBand __tests__/auth/generated-permissions.test.ts
```

Expected: all PASS and a second generation produces no diff.

- [ ] **Step 5: Commit both repositories**

Backend:

```bash
git add crates/libs/lib-core/examples/export_permission_catalog.rs scripts/generate_frontend_permissions.sh
git commit -m "build: export frontend permission catalog"
```

Frontend:

```bash
git add lib/auth/generated-permissions.ts __tests__/auth/generated-permissions.test.ts package.json
git commit -m "build: add generated permission catalog"
```

### Task 5: Frontend permission core and route/menu declarations

**Files:**
- Create: `lib/auth/permissions.ts`
- Create: `lib/auth/PermissionGate.tsx`
- Create: `lib/auth/access-rules.ts`
- Modify: `lib/types/api.ts`
- Modify: `lib/api/endpoints/auth.ts`
- Modify: `lib/contexts/AuthContext.tsx`
- Modify: `lib/auth/routeAccess.ts`
- Modify: `lib/hooks/useProtectedRoute.ts`
- Modify: `components/Sidebar.tsx`
- Test: `__tests__/auth/permissions.test.ts`
- Test: `__tests__/route-access.test.ts`
- Test: `__tests__/sidebar.capabilities.test.tsx` (rename to `sidebar.permissions.test.tsx`)

**Interfaces:**
- Consumes: `CurrentUserProfile.permissions: PermissionValue[]`.
- Produces: pure `can`, `canAny`, `canAll`, `PermissionGate`, and data-driven access rules.

- [ ] **Step 1: Write failing permission and route tests**

Cover fail-closed behavior and the reproduced execute-only cases:

```ts
expect(can(new Set([Permission.XmlImportImport]), Permission.XmlImportImport)).toBe(true);
expect(can(new Set(), Permission.XmlImportImport)).toBe(false);
expect(canAny(new Set([Permission.XmlImportImport]), [
  Permission.XmlImportRead, Permission.XmlImportImport,
])).toBe(true);
expect(canAccessProtectedPath(userWith([Permission.XmlImportImport]), "/import")).toBe(true);
expect(canAccessProtectedPath(userWith([Permission.TerminologyRead]), "/data")).toBe(true);
```

- [ ] **Step 2: Run focused tests and verify failures**

Run: `npm test -- --runInBand __tests__/auth/permissions.test.ts __tests__/route-access.test.ts __tests__/sidebar.permissions.test.tsx`

Expected: FAIL because permission helpers and permission-based fixtures do not exist.

- [ ] **Step 3: Implement one authorization boundary**

Use these exact pure signatures:

```ts
export type PermissionSet = ReadonlySet<PermissionValue>;
export const can = (set: PermissionSet, permission: PermissionValue) => set.has(permission);
export const canAny = (set: PermissionSet, required: readonly PermissionValue[]) =>
  required.some((permission) => set.has(permission));
export const canAll = (set: PermissionSet, required: readonly PermissionValue[]) =>
  required.every((permission) => set.has(permission));
```

Expose the current set and these bound helpers from `AuthContext`. Make `routeAccess` and `Sidebar` consume the same `ACCESS_RULES` declarations. Do not retain capability fallback branches.

- [ ] **Step 4: Run focused frontend tests**

Run: `npm test -- --runInBand __tests__/auth/permissions.test.ts __tests__/route-access.test.ts __tests__/sidebar.permissions.test.tsx __tests__/api.endpoints.test.ts`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add lib/auth lib/types/api.ts lib/api/endpoints/auth.ts lib/contexts/AuthContext.tsx lib/hooks/useProtectedRoute.ts components/Sidebar.tsx __tests__
git commit -m "refactor: centralize frontend permission checks"
```

### Task 6: Migrate INFO, IMPORT, SUBMISSION, and DATA

**Files:**
- Modify: `components/presave/InfoPresaveListRoute.tsx`
- Modify: `components/presave/InfoPresaveDetailRoute.tsx`
- Modify: `app/(protected)/import/page.tsx`
- Modify: `app/(protected)/submission/page.tsx`
- Modify: `app/(protected)/data/page.tsx`
- Test: `__tests__/rbac-contract/info-actions.test.tsx`
- Test: `__tests__/rbac-contract/import-actions.test.tsx`
- Test: `__tests__/rbac-contract/submission-actions.test.tsx`
- Test: `__tests__/rbac-contract/data-actions.test.tsx`

**Interfaces:**
- Consumes: bound `can/canAny` from AuthContext and generated Permission constants.
- Produces: exact action gates for the four confirmed mismatch modules.

- [ ] **Step 1: Write the four failing reproduction tests**

Use concrete permission fixtures. Required assertions include:

```ts
// INFO read only
expect(screen.queryByRole("button", { name: /new/i })).not.toBeInTheDocument();
expect(screen.queryByRole("button", { name: /delete sender/i })).not.toBeInTheDocument();

// IMPORT execute only
expect(screen.getByLabelText(/upload/i)).toBeEnabled();
expect(screen.queryByText(/import history/i)).not.toBeInTheDocument();

// SUBMISSION read only
expect(screen.queryByRole("button", { name: /export xml/i })).not.toBeInTheDocument();
expect(screen.queryByRole("button", { name: /submit cases/i })).not.toBeInTheDocument();

// DATA dynamic reader
expect(screen.getByRole("heading", { name: "DATA" })).toBeInTheDocument();
```

- [ ] **Step 2: Run tests and verify the reproduced failures**

Run: `npm test -- --runInBand __tests__/rbac-contract/{info,import,submission,data}-actions.test.tsx`

Expected: FAIL in the same directions as the confirmed production mismatches.

- [ ] **Step 3: Add exact action gates**

Map each action to generated constants. INFO section contracts gain explicit read/create/update/delete permission fields. IMPORT gates history with `XmlImportRead` and upload with `XmlImportImport`. SUBMISSION gates history with `XmlExportRead` and execution with `XmlExportExport`. DATA removes role-name redirects and uses terminology permissions.

- [ ] **Step 4: Run module and existing regression tests**

Run:

```bash
npm test -- --runInBand __tests__/rbac-contract __tests__/dashboard/info-presave-list-route.test.tsx __tests__/route-access.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add components/presave app/'(protected)' __tests__/rbac-contract
git commit -m "fix: align module actions with backend permissions"
```

### Task 7: Migrate CASE and administration composite permissions

**Files:**
- Create: `lib/auth/case-permissions.ts`
- Modify: `components/case-form/CaseEditor.tsx`
- Modify: `components/case-form/CaseFormLayout.tsx`
- Modify: `components/case-form/CaseHeader.tsx`
- Modify: `components/case-form/hooks/useCaseEditorWorkflowActions.ts`
- Modify: `app/(protected)/admin/AdminWorkspace.tsx`
- Modify: `app/(protected)/admin/organization/`
- Modify: `app/(protected)/admin/settings/`
- Modify: `app/(protected)/admin/users/`
- Modify: `app/(protected)/admin/role/`
- Test: `__tests__/rbac-contract/case-actions.test.tsx`
- Test: `__tests__/rbac-contract/admin-actions.test.tsx`

**Interfaces:**
- Consumes: generated permissions and `canAll`.
- Produces: `CASE_PAGE_ACCESS[pageId] = { read: PermissionValue[], update: PermissionValue[] }` and exact admin action gates.

- [ ] **Step 1: Write failing read-only and composite-permission tests**

Assert that lifecycle state and authorization both participate:

```ts
expect(caseAccess.canSave("CI", setOf(Permission.CaseRead))).toBe(false);
expect(caseAccess.canSave("CI", setOf(
  Permission.CaseUpdate,
  Permission.SafetyReportUpdate,
))).toBe(true);
expect(caseAccess.canApprove(setOf(Permission.CaseUpdate))).toBe(false);
expect(caseAccess.canApprove(setOf(Permission.CaseApprove))).toBe(true);
```

Admin tests must prove dynamic terminology/settings/users permissions work without built-in role names and organization mutation remains unavailable without its explicit permission.

- [ ] **Step 2: Run tests and verify failures**

Run: `npm test -- --runInBand __tests__/rbac-contract/case-actions.test.tsx __tests__/rbac-contract/admin-actions.test.tsx`

Expected: FAIL because CASE ignores user permissions and admin still uses roles/capabilities.

- [ ] **Step 3: Implement explicit composites**

Keep lifecycle read-only as a separate boolean:

```ts
const isAuthorizationReadOnly = !canAll(permissionSet, CASE_PAGE_ACCESS[pageId].update);
const isReadOnly = isLifecycleReadOnly || isAuthorizationReadOnly;
```

Gate workflow review/lock with `Case.Approve`. Replace administration role/capability checks with the exact User, Settings, AuditLog, and Organization permissions used by REST handlers.

- [ ] **Step 4: Run CASE/admin regression suites**

Run:

```bash
npm test -- --runInBand __tests__/rbac-contract/case-actions.test.tsx __tests__/rbac-contract/admin-actions.test.tsx __tests__/case-form __tests__/admin-user-detail.route.test.ts __tests__/admin-users.header-filters.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add lib/auth/case-permissions.ts components/case-form app/'(protected)'/admin __tests__/rbac-contract
git commit -m "fix: enforce exact CASE and admin permissions"
```

### Task 8: Policy-version headers, synchronization, and denial diagnostics

**Files:**
- Modify backend: `crates/libs/lib-web/src/middleware/mw_res_map.rs`
- Modify backend: `crates/libs/lib-web/src/error.rs`
- Test backend: `crates/services/web-server/tests/api/role_admin/effective_access/persistence_web.rs`
- Modify frontend: `lib/api/client.ts`
- Modify frontend: `lib/contexts/AuthContext.tsx`
- Create frontend: `lib/auth/policy-sync.ts`
- Test frontend: `__tests__/auth/policy-sync.test.ts`
- Test frontend: `__tests__/api.endpoints.test.ts`

**Interfaces:**
- Consumes: profile `policyVersion` and database policy state.
- Produces: `X-RBAC-Policy-Version`, deduplicated profile refresh, and stable `PERMISSION_DENIED` details.

- [ ] **Step 1: Write failing backend and frontend synchronization tests**

Backend asserts every authenticated success and 403 carries the version header and denial JSON includes `requiredPermission` and `policyVersion`.

Frontend test simulates two concurrent responses with a newer header and asserts exactly one refresh:

```ts
const refresh = jest.fn().mockResolvedValue(undefined);
const sync = createPolicySynchronizer({ currentVersion: () => 4, refresh });
await Promise.all([sync.observe("5"), sync.observe("5")]);
expect(refresh).toHaveBeenCalledTimes(1);
```

- [ ] **Step 2: Run both tests and verify failures**

Run:

```bash
cargo test -p web-server --test api role_admin::effective_access::persistence_web -- --nocapture
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npm test -- --runInBand __tests__/auth/policy-sync.test.ts __tests__/api.endpoints.test.ts
```

Expected: FAIL because headers and synchronizer are absent.

- [ ] **Step 3: Implement response metadata and deduplicated refresh**

Add the header after authorization context is available. Normalize 403 details into camelCase at the HTTP boundary. Implement one in-flight refresh promise; observe headers after every authenticated response. On mismatch disable new mutations through AuthContext until the refreshed profile is atomically installed. Do not retry the denied mutation.

- [ ] **Step 4: Run synchronization and session tests**

Run:

```bash
cargo test -p web-server --test api role_admin::effective_access -- --nocapture
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npm test -- --runInBand __tests__/auth __tests__/api.endpoints.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit both repositories**

Backend:

```bash
git add crates/libs/lib-web crates/services/web-server/tests/api/role_admin
git commit -m "feat: report RBAC policy version on responses"
```

Frontend:

```bash
git add lib/api/client.ts lib/contexts/AuthContext.tsx lib/auth/policy-sync.ts __tests__
git commit -m "feat: synchronize frontend RBAC policy state"
```

### Task 9: Cross-layer manifest and legacy removal

**Files:**
- Create backend: `crates/services/web-server/src/web/rest/permission_contract.rs`
- Create backend: `crates/services/web-server/examples/export_permission_contract.rs`
- Create frontend: `lib/auth/generated-endpoint-permissions.ts`
- Create frontend: `__tests__/rbac-contract/endpoint-manifest.test.ts`
- Delete frontend: `lib/auth/capabilities.ts`
- Modify frontend: `lib/auth/roleAccess.ts`
- Modify frontend: all remaining capability/authorization role-name callers found by `rg`.
- Modify backend: remaining capability references in tests/OpenAPI.

**Interfaces:**
- Consumes: endpoint permission declarations and frontend action declarations.
- Produces: generated endpoint/action drift check and zero legacy authorization paths.

- [ ] **Step 1: Write failing static guard and manifest tests**

The static guard scans production frontend files:

```ts
expect(matches("canCapability|canReadModule|capabilities\\??\\.|isSystemAdminRole\\(|roleMeta\\?\\.canAdmin")).toEqual([]);
```

The manifest test loads generated endpoint permissions and asserts every declared UI action references the same permission set as its called endpoint.

- [ ] **Step 2: Run tests and verify failures**

Run: `npm test -- --runInBand __tests__/rbac-contract/endpoint-manifest.test.ts`

Expected: FAIL listing remaining capabilities, direct authorization role checks, and missing endpoint declarations.

- [ ] **Step 3: Export endpoint permission declarations and remove legacy paths**

Centralize REST endpoint permission metadata in `permission_contract.rs` and use those constants at route guards where practical. Generate the frontend manifest from the same declarations. Remove `capabilities.ts`, authorization-only exports from `roleAccess.ts`, stale types, fixtures, and tests. Preserve sponsor/workflow role comparisons only where they select business data rather than grant access.

- [ ] **Step 4: Run full verification in both repositories**

Backend:

```bash
cargo fmt --all -- --check
cargo test -p lib-core
cargo test -p web-server --test api role_admin -- --nocapture
cargo check -p web-server
```

Frontend:

```bash
npm run check:permissions
npm test -- --runInBand
npm run build
```

Expected: every command exits 0; static guard finds no legacy authorization paths; generated files have no diff.

- [ ] **Step 5: Commit both repositories**

Backend:

```bash
git add crates/services/web-server crates/libs db scripts
git commit -m "test: enforce backend RBAC permission contract"
```

Frontend:

```bash
git add -A lib app components __tests__ package.json
git commit -m "refactor: remove legacy frontend authorization paths"
```

### Task 10: Coordinated end-to-end verification and release handoff

**Files:**
- Modify only if verification exposes a defect covered by a new failing test.

**Interfaces:**
- Consumes: completed backend and frontend branches.
- Produces: verified coordinated deployment candidates.

- [ ] **Step 1: Start clean backend and frontend services**

Use isolated test databases and the feature worktrees. Confirm the frontend proxy points to the feature backend.

- [ ] **Step 2: Exercise the confirmed role scenarios**

For INFO read-only, CASE read-only, IMPORT execute-only, SUBMISSION read-only, DATA reader, settings editor, and system admin, record:

```text
profile permissions + policyVersion
route/menu result
visible enabled actions
API status and requiredPermission for one denied direct request
```

Expected: UI and API decisions match for every scenario.

- [ ] **Step 3: Exercise live permission changes without logout**

Keep a restricted-user tab open, update its dynamic role from an admin session, trigger one authenticated request or focus transition, and verify the tab installs the new policy version and recomputes controls without reload or logout.

- [ ] **Step 4: Run final clean verification**

Repeat Task 9's complete command set from clean worktrees. Confirm `git status --short` contains only intended committed state.

- [ ] **Step 5: Review and coordinated integration**

Use `superpowers:requesting-code-review`, address only verified findings, then use `superpowers:finishing-a-development-branch`. Merge or publish backend and frontend branches together so neither legacy contract is deployed alone.
