# RBAC Capability Matrix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add backend-derived role capabilities and verify that Role & Privilege rows control backend APIs and frontend read/write UI state consistently.

**Architecture:** Backend remains authoritative: capabilities are derived from the same `has_permission(ctx.permission_subject(), permission)` logic used by API enforcement and are returned in `/api/users/me/profile`. Frontend consumes those capabilities through shared helpers so read permission shows pages and write permission enables mutation controls.

**Tech Stack:** Rust/Axum/SQLx backend, existing permission-profile dynamic role cache, Next.js/React/TypeScript frontend, Jest/Testing Library frontend tests, Rust integration tests.

---

## File Structure

Backend:

- Modify `crates/services/web-server/src/web/rest/user_rest.rs`: add capability DTOs and build current-user/user response capabilities.
- Modify `crates/services/web-server/src/openapi.rs`: document capability response shape.
- Modify `crates/services/web-server/tests/api/scope_visibility_web.rs`: add/extend matrix tests proving backend mapping, API allow/deny, and refresh behavior.
- Optionally modify `crates/libs/lib-core/src/model/acs/permission.rs` only if the matrix reveals a mapping bug. Do not change mappings speculatively.

Frontend:

- Modify `lib/types/api.ts`: add capability types to `CurrentUserProfile`/`User` profile types.
- Create `lib/auth/capabilities.ts`: central capability helpers such as `canCapability(...)`, `canReadModule(...)`, and `canWriteModule(...)`.
- Modify `components/Sidebar.tsx`: show module nav from capabilities instead of static role assumptions where applicable.
- Modify representative pages/components in the first pass:
  - `app/(protected)/admin/AdminWorkspace.tsx`: admin read/update, users, roles controls.
  - `app/(protected)/cases/page.tsx`: case read/create/export/navigation controls.
  - `app/(protected)/dashboard/page.tsx`: home notice edit controls.
- Create or modify frontend tests:
  - `__tests__/capabilities.test.ts`
  - `__tests__/sidebar.capabilities.test.tsx`
  - targeted tests for admin/case/dashboard controls, reusing existing mocks where available.

## Task 1: Backend Capability DTO

**Files:**
- Modify: `crates/services/web-server/src/web/rest/user_rest.rs`
- Modify: `crates/services/web-server/src/openapi.rs`
- Test: `crates/services/web-server/tests/api/scope_visibility_web.rs`

- [ ] **Step 1: Write the failing profile capability test**

Add a focused assertion to `test_permission_profile_admin_privilege_grants_admin_page_access` or a new test in `scope_visibility_web.rs`:

```rust
let (status, value) = request_json(
    &app,
    "GET",
    &custom_cookie,
    "/api/users/me/profile".to_string(),
    None,
)
.await?;
assert_eq!(status, StatusCode::OK, "{value:?}");
assert_eq!(
    value["data"]["capabilities"]["admin"]["read"].as_bool(),
    Some(true),
    "{value:?}"
);
assert_eq!(
    value["data"]["capabilities"]["users"]["create"].as_bool(),
    Some(true),
    "{value:?}"
);
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p web-server --test api scope_visibility_web::test_permission_profile_admin_privilege_grants_admin_page_access -- --nocapture
```

Expected: FAIL because `data.capabilities` is missing.

- [ ] **Step 3: Add capability DTOs**

In `user_rest.rs`, add DTOs near `UserRoleMetadata`:

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleCrudCapabilities {
    pub read: bool,
    pub create: bool,
    pub update: bool,
    pub delete: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseCapabilities {
    pub read: bool,
    pub create: bool,
    pub update: bool,
    pub delete: bool,
    pub review: bool,
    pub lock: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteCapabilities {
    pub read: bool,
    pub execute: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataCapabilities {
    pub read: bool,
    pub import: bool,
    pub approve: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminCapabilities {
    pub read: bool,
    pub update: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserCapabilities {
    pub case: CaseCapabilities,
    pub info: ModuleCrudCapabilities,
    pub import: ExecuteCapabilities,
    pub export_submission: ExecuteCapabilities,
    pub data: DataCapabilities,
    pub admin: AdminCapabilities,
    pub users: ModuleCrudCapabilities,
    pub roles: ModuleCrudCapabilities,
}
```

- [ ] **Step 4: Add capabilities to current profile response**

In `CurrentUserProfileView`, add:

```rust
pub capabilities: UserCapabilities,
```

Add builder:

```rust
fn capabilities_for_subject(subject: &str, is_admin_capable: bool) -> UserCapabilities {
    UserCapabilities {
        case: CaseCapabilities {
            read: has_permission(subject, CASE_READ) || has_permission(subject, CASE_LIST),
            create: has_permission(subject, CASE_CREATE),
            update: has_permission(subject, CASE_UPDATE),
            delete: has_permission(subject, CASE_DELETE),
            review: has_permission(subject, CASE_APPROVE),
            lock: has_permission(subject, CASE_APPROVE),
        },
        info: ModuleCrudCapabilities {
            read: has_permission(subject, PRESAVE_TEMPLATE_READ)
                || has_permission(subject, PRESAVE_TEMPLATE_LIST)
                || has_permission(subject, SENDER_INFORMATION_READ)
                || has_permission(subject, RECEIVER_READ),
            create: has_permission(subject, PRESAVE_TEMPLATE_CREATE)
                || has_permission(subject, SENDER_INFORMATION_CREATE)
                || has_permission(subject, RECEIVER_CREATE),
            update: has_permission(subject, PRESAVE_TEMPLATE_UPDATE)
                || has_permission(subject, SENDER_INFORMATION_UPDATE)
                || has_permission(subject, RECEIVER_UPDATE),
            delete: has_permission(subject, PRESAVE_TEMPLATE_DELETE)
                || has_permission(subject, SENDER_INFORMATION_DELETE)
                || has_permission(subject, RECEIVER_DELETE),
        },
        import: ExecuteCapabilities {
            read: has_permission(subject, XML_IMPORT_READ),
            execute: has_permission(subject, XML_IMPORT),
        },
        export_submission: ExecuteCapabilities {
            read: has_permission(subject, XML_EXPORT_READ),
            execute: has_permission(subject, XML_EXPORT),
        },
        data: DataCapabilities {
            read: has_permission(subject, TERMINOLOGY_READ),
            import: has_permission(subject, TERMINOLOGY_IMPORT),
            approve: has_permission(subject, TERMINOLOGY_APPROVE),
        },
        admin: AdminCapabilities {
            read: is_admin_capable,
            update: has_permission(subject, USER_UPDATE) || has_permission(subject, USER_CREATE),
        },
        users: ModuleCrudCapabilities {
            read: has_permission(subject, USER_READ) || has_permission(subject, USER_LIST),
            create: has_permission(subject, USER_CREATE),
            update: has_permission(subject, USER_UPDATE),
            delete: has_permission(subject, USER_DELETE),
        },
        roles: ModuleCrudCapabilities {
            read: is_admin_capable,
            create: has_permission(subject, USER_CREATE),
            update: has_permission(subject, USER_UPDATE),
            delete: has_permission(subject, USER_DELETE),
        },
    }
}
```

Import the permission constants used above from `lib_core::model::acs`.

- [ ] **Step 5: Wire current-user profile handler**

In `get_current_user_profile`, build capabilities from the request context:

```rust
let capabilities = capabilities_for_subject(
    ctx.permission_subject(),
    lib_rest_core::can_access_admin(&ctx),
);
```

Return it in `CurrentUserProfileView`.

- [ ] **Step 6: Update OpenAPI doc structs**

In `openapi.rs`, add matching doc structs using `#[serde(rename_all = "camelCase")]`, and add `capabilities: UserCapabilitiesDoc` to `CurrentUserProfileView`'s documented equivalent.

- [ ] **Step 7: Run targeted backend test**

Run:

```bash
cargo test -p web-server --test api scope_visibility_web::test_permission_profile_admin_privilege_grants_admin_page_access -- --nocapture
```

Expected: PASS.

- [ ] **Step 8: Commit backend capability contract**

```bash
git add crates/services/web-server/src/web/rest/user_rest.rs crates/services/web-server/src/openapi.rs crates/services/web-server/tests/api/scope_visibility_web.rs
git commit -m "Expose backend-derived user capabilities"
```

## Task 2: Backend Matrix Expansion

**Files:**
- Modify: `crates/services/web-server/tests/api/scope_visibility_web.rs`

- [ ] **Step 1: Add users/roles matrix test**

Create test:

```rust
#[serial]
#[tokio::test]
async fn test_users_and_roles_matrix_privileges_grant_effective_admin_permissions() -> Result<()> {
    let mm = init_test_mm().await?;
    let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
    let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
    let admin_cookie = cookie_header(&admin_token.to_string());
    let app = web_server::app(mm.clone());
    let profile_id = format!("qa_users_roles_matrix_{}", Uuid::new_v4().simple());

    create_empty_custom_role(&app, &admin_cookie, &profile_id).await?;
    let (_custom_user_id, custom_cookie) = custom_role_user(&mm, seed.org_id, &profile_id).await?;

    assert_get_status(&app, &custom_cookie, "/api/users", StatusCode::FORBIDDEN).await?;
    assert_get_status(&app, &custom_cookie, "/api/admin/permission-profiles", StatusCode::FORBIDDEN).await?;

    update_role_privileges(
        &app,
        &admin_cookie,
        &profile_id,
        json!([{ "menu_key": "users", "can_read": true, "can_edit": false, "can_review": false, "can_lock": false }]),
    )
    .await?;
    assert!(has_permission(&profile_id, USER_READ));
    assert!(has_permission(&profile_id, USER_LIST));
    assert!(!has_permission(&profile_id, USER_CREATE));
    assert_get_status(&app, &custom_cookie, "/api/users", StatusCode::FORBIDDEN).await?;

    update_role_privileges(
        &app,
        &admin_cookie,
        &profile_id,
        json!([{ "menu_key": "users", "can_read": true, "can_edit": true, "can_review": false, "can_lock": false }]),
    )
    .await?;
    assert!(has_permission(&profile_id, USER_CREATE));
    assert_get_status(&app, &custom_cookie, "/api/users", StatusCode::OK).await?;

    let next_role = format!("qa_users_roles_child_{}", Uuid::new_v4().simple());
    let (status, value) = request_json(
        &app,
        "POST",
        &custom_cookie,
        "/api/admin/permission-profiles".to_string(),
        Some(json!({ "data": { "profile_id": next_role, "name": "Users Roles Child", "privileges": [] } })),
    )
    .await?;
    assert_eq!(status, StatusCode::CREATED, "{value:?}");

    Ok(())
}
```

- [ ] **Step 2: Run new backend matrix test to verify red/green**

Run:

```bash
cargo test -p web-server --test api scope_visibility_web::test_users_and_roles_matrix_privileges_grant_effective_admin_permissions -- --nocapture
```

Expected after Task 1: PASS. If it fails, fix the smallest backend permission/capability mismatch shown by the assertion.

- [ ] **Step 3: Strengthen existing matrix tests with capability response assertions**

For existing tests (`case`, `info`, `import`, `export_submission`, `data`, `settings`), after each `update_role_privileges`, call `/api/users/me/profile` as the custom user and assert the matching capability:

```rust
let (status, profile) = request_json(
    &app,
    "GET",
    &custom_cookie,
    "/api/users/me/profile".to_string(),
    None,
)
.await?;
assert_eq!(status, StatusCode::OK, "{profile:?}");
assert_eq!(profile["data"]["capabilities"]["case"]["read"].as_bool(), Some(true));
assert_eq!(profile["data"]["capabilities"]["case"]["update"].as_bool(), Some(false));
```

Use the module/action names from Task 1. Keep assertions local to the behavior each test already covers.

- [ ] **Step 4: Run backend role matrix group**

Run:

```bash
cargo test -p web-server --test api role_admin_api -- --nocapture --test-threads=1
cargo test -p web-server --test api matrix_privileges -- --nocapture --test-threads=1
```

If the second filter does not match all matrix tests, run the specific matrix filters used in `scope_visibility_web.rs`.

- [ ] **Step 5: Commit backend matrix tests**

```bash
git add crates/services/web-server/tests/api/scope_visibility_web.rs
git commit -m "Verify role privilege capability matrix"
```

## Task 3: Frontend Capability Types and Helpers

**Files:**
- Modify: `lib/types/api.ts`
- Create: `lib/auth/capabilities.ts`
- Test: `__tests__/capabilities.test.ts`

- [ ] **Step 1: Write failing helper tests**

Create `__tests__/capabilities.test.ts`:

```ts
import { canCapability, canReadModule, canMutateModule } from "@/lib/auth/capabilities";
import type { UserCapabilities } from "@/lib/types";

const capabilities: UserCapabilities = {
  case: { read: true, create: false, update: false, delete: false, review: false, lock: false },
  info: { read: true, create: true, update: true, delete: false },
  import: { read: true, execute: false },
  exportSubmission: { read: false, execute: false },
  data: { read: true, import: false, approve: false },
  admin: { read: true, update: false },
  users: { read: true, create: false, update: false, delete: false },
  roles: { read: true, create: false, update: false, delete: false },
};

describe("capabilities", () => {
  it("reads module capability state", () => {
    expect(canCapability(capabilities, "case", "read")).toBe(true);
    expect(canCapability(capabilities, "case", "update")).toBe(false);
    expect(canReadModule(capabilities, "case")).toBe(true);
    expect(canMutateModule(capabilities, "case")).toBe(false);
    expect(canMutateModule(capabilities, "info")).toBe(true);
  });

  it("denies missing capability data", () => {
    expect(canCapability(undefined, "case", "read")).toBe(false);
    expect(canReadModule(undefined, "case")).toBe(false);
    expect(canMutateModule(undefined, "case")).toBe(false);
  });
});
```

- [ ] **Step 2: Run helper tests to verify failure**

Run:

```bash
npm test -- __tests__/capabilities.test.ts --runInBand
```

Expected: FAIL because `lib/auth/capabilities.ts` and `UserCapabilities` do not exist.

- [ ] **Step 3: Add types**

In `lib/types/api.ts`, add:

```ts
export interface ModuleCrudCapabilities {
  read: boolean;
  create: boolean;
  update: boolean;
  delete: boolean;
}

export interface CaseCapabilities extends ModuleCrudCapabilities {
  review: boolean;
  lock: boolean;
}

export interface ExecuteCapabilities {
  read: boolean;
  execute: boolean;
}

export interface DataCapabilities {
  read: boolean;
  import: boolean;
  approve: boolean;
}

export interface AdminCapabilities {
  read: boolean;
  update: boolean;
}

export interface UserCapabilities {
  case: CaseCapabilities;
  info: ModuleCrudCapabilities;
  import: ExecuteCapabilities;
  exportSubmission: ExecuteCapabilities;
  data: DataCapabilities;
  admin: AdminCapabilities;
  users: ModuleCrudCapabilities;
  roles: ModuleCrudCapabilities;
}
```

Add `capabilities?: UserCapabilities` to the current-user profile type used by `/api/users/me/profile`.

- [ ] **Step 4: Add helper implementation**

Create `lib/auth/capabilities.ts`:

```ts
import type { UserCapabilities } from "@/lib/types";

export type CapabilityModule = keyof UserCapabilities;
export type CapabilityAction<M extends CapabilityModule = CapabilityModule> = keyof UserCapabilities[M];

export function canCapability<M extends CapabilityModule>(
  capabilities: UserCapabilities | undefined | null,
  module: M,
  action: keyof UserCapabilities[M],
): boolean {
  return Boolean(capabilities?.[module]?.[action]);
}

export function canReadModule(
  capabilities: UserCapabilities | undefined | null,
  module: CapabilityModule,
): boolean {
  return canCapability(capabilities, module, "read" as never);
}

export function canMutateModule(
  capabilities: UserCapabilities | undefined | null,
  module: CapabilityModule,
): boolean {
  const moduleCapabilities = capabilities?.[module] as Record<string, boolean> | undefined;
  if (!moduleCapabilities) return false;
  return Object.entries(moduleCapabilities).some(
    ([action, allowed]) => action !== "read" && allowed,
  );
}
```

- [ ] **Step 5: Run helper tests**

Run:

```bash
npm test -- __tests__/capabilities.test.ts --runInBand
```

Expected: PASS.

- [ ] **Step 6: Commit frontend capability helpers**

```bash
git add lib/types/api.ts lib/auth/capabilities.ts __tests__/capabilities.test.ts
git commit -m "Add frontend capability helpers"
```

## Task 4: Frontend Navigation and Admin Controls

**Files:**
- Modify: `components/Sidebar.tsx`
- Modify: `app/(protected)/admin/AdminWorkspace.tsx`
- Test: `__tests__/sidebar.capabilities.test.tsx`
- Test existing: admin route tests under `__tests__/admin-*.test.tsx`

- [ ] **Step 1: Write sidebar capability test**

Create `__tests__/sidebar.capabilities.test.tsx` with existing auth mocks. If no shared render helper exists, use this structure:

```tsx
import { render, screen } from "@testing-library/react";
import Sidebar from "@/components/Sidebar";

jest.mock("next/navigation", () => ({
  usePathname: () => "/dashboard",
}));

const logout = jest.fn();
let mockUser: any = null;

jest.mock("@/lib/contexts/AuthContext", () => ({
  useAuth: () => ({ user: mockUser, logout }),
}));

describe("Sidebar capability navigation", () => {
  it("shows admin for custom users with admin read capability", () => {
    mockUser = {
      role: "user",
      roleMeta: { canAdmin: true },
      capabilities: {
        admin: { read: true, update: false },
        case: { read: true },
        info: { read: false },
        import: { read: false },
        exportSubmission: { read: false },
        data: { read: false },
        users: { read: false },
        roles: { read: false },
      },
    };
    render(<Sidebar />);
    expect(screen.getByText("ADMIN")).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run sidebar test to verify failure or current behavior gap**

Run:

```bash
npm test -- __tests__/sidebar.capabilities.test.tsx --runInBand
```

Expected: FAIL until `Sidebar` reads module capabilities consistently.

- [ ] **Step 3: Update sidebar module visibility**

In `Sidebar.tsx`, prefer capabilities for each module:

```ts
const canRead = (module: CapabilityModule) => canReadModule(user?.capabilities, module);
```

Map nav:

- HOME: always visible for authenticated users.
- CASE: `canRead("case")`.
- INFO: `canRead("info")`.
- DATA: keep system-admin-only unless data page is intentionally capability-driven; if capability-driven, use `canRead("data")`.
- IMPORT: `canRead("import")`.
- EXPORT/SUBMISSION: `canRead("exportSubmission")`.
- ADMIN: `canRead("admin") || canAccessAdmin(user)`.

- [ ] **Step 4: Update AdminWorkspace controls**

Derive:

```ts
const capabilities = currentUser?.capabilities;
const canUpdateAdmin = canCapability(capabilities, "admin", "update");
const canCreateUsers = canCapability(capabilities, "users", "create");
const canUpdateUsers = canCapability(capabilities, "users", "update");
const canDeleteUsers = canCapability(capabilities, "users", "delete");
const canCreateRoles = canCapability(capabilities, "roles", "create");
const canUpdateRoles = canCapability(capabilities, "roles", "update");
const canDeleteRoles = canCapability(capabilities, "roles", "delete");
```

Apply:

- Hide/disable Create User button when `!canCreateUsers`.
- Disable user role/scope mutation controls when `!canUpdateUsers`.
- Hide/disable Delete user actions when `!canDeleteUsers`.
- Hide/disable Create Role when `!canCreateRoles`.
- Disable role matrix checkboxes when `!canUpdateRoles` or role is built-in.
- Hide/disable role delete actions when `!canDeleteRoles`.
- Disable Save Settings when `!canUpdateAdmin`.

- [ ] **Step 5: Guard mutation handlers**

At the start of handlers, add early guards:

```ts
if (!canCreateUsers) {
  toast.error("You do not have permission to create users.");
  return;
}
```

Use matching messages for role/settings/user update/delete.

- [ ] **Step 6: Run admin/frontend tests**

Run:

```bash
npm test -- __tests__/sidebar.capabilities.test.tsx __tests__/admin-user-detail.route.test.ts __tests__/admin-users.header-filters.test.ts --runInBand
npx tsc --noEmit --project tsconfig.jest.json
```

Expected: PASS.

- [ ] **Step 7: Commit frontend navigation/admin controls**

```bash
git add components/Sidebar.tsx app/'(protected)'/admin/AdminWorkspace.tsx __tests__/sidebar.capabilities.test.tsx
git commit -m "Apply capabilities to admin navigation and controls"
```

## Task 5: Representative Read-Only Page Controls

**Files:**
- Modify: `app/(protected)/cases/page.tsx`
- Modify: `app/(protected)/dashboard/page.tsx`
- Add tests depending on existing page test patterns.

- [ ] **Step 1: Add case page read-only test**

Create or extend a cases page test to mock a user with:

```ts
capabilities: {
  case: { read: true, create: false, update: false, delete: false, review: false, lock: false }
}
```

Assert:

```ts
expect(screen.queryByRole("button", { name: /new case/i })).not.toBeInTheDocument();
expect(screen.queryByRole("button", { name: /delete/i })).not.toBeInTheDocument();
```

If buttons remain visible by design, assert `toBeDisabled()` instead.

- [ ] **Step 2: Add dashboard notice read-only test**

Mock `home_notice` read-only capability via:

```ts
capabilities: {
  admin: { read: false, update: false },
  // include any module fields needed by the type
}
```

Assert the edit/save notice controls are absent or disabled for non-edit capability.

- [ ] **Step 3: Implement case page control gating**

Use shared helpers:

```ts
const canCreateCase = canCapability(user?.capabilities, "case", "create");
const canUpdateCase = canCapability(user?.capabilities, "case", "update");
const canDeleteCase = canCapability(user?.capabilities, "case", "delete");
```

Hide/disable Create Case, edit, and delete controls accordingly.

- [ ] **Step 4: Implement dashboard notice control gating**

Replace system-admin-only notice edit logic with capability-driven logic once backend exposes home notice capability. If backend does not expose home capabilities in Task 1, keep existing behavior and add a follow-up task before changing dashboard.

- [ ] **Step 5: Run representative frontend tests**

Run:

```bash
npm test -- __tests__/capabilities.test.ts __tests__/sidebar.capabilities.test.tsx --runInBand
npx tsc --noEmit --project tsconfig.jest.json
```

Also run any case/dashboard tests touched by this task.

- [ ] **Step 6: Commit read-only page controls**

```bash
git add app/'(protected)'/cases/page.tsx app/'(protected)'/dashboard/page.tsx __tests__
git commit -m "Respect read-only capabilities in page controls"
```

## Task 6: Final Verification and Push

**Files:**
- No code changes unless verification finds a defect.

- [ ] **Step 1: Run backend focused verification**

Run:

```bash
cargo fmt --check
cargo test -p web-server --test api role_admin_api -- --nocapture --test-threads=1
cargo test -p web-server --test api scope_visibility_web::test_permission_profile_admin_privilege_grants_admin_page_access -- --nocapture
cargo test -p web-server --test api scope_visibility_web::test_users_and_roles_matrix_privileges_grant_effective_admin_permissions -- --nocapture
```

Expected: PASS. If unrelated DB deadlocks appear, rerun the single failing test after ensuring no other backend tests are running and report the exact database error separately.

- [ ] **Step 2: Run frontend focused verification**

Run:

```bash
npm test -- __tests__/capabilities.test.ts __tests__/role-access.test.ts __tests__/sidebar.capabilities.test.tsx __tests__/admin-user-detail.route.test.ts __tests__/admin-users.header-filters.test.ts --runInBand
npx tsc --noEmit --project tsconfig.jest.json
```

Expected: PASS.

- [ ] **Step 3: Inspect diffs and status**

Run in backend repo:

```bash
git status --short --branch
git diff --stat
```

Run in frontend repo:

```bash
git status --short --branch
git diff --stat
```

Confirm unrelated dirty files are not staged.

- [ ] **Step 4: Push commits**

Run in each repo that has new commits:

```bash
git push origin main
```

Expected: push succeeds.

## Self-Review

Spec coverage:

- Backend-derived capabilities: Task 1.
- Backend matrix tests: Task 2.
- Frontend shared capability layer: Task 3.
- Navigation and admin controls: Task 4.
- Representative read-only/write UI controls: Task 5.
- Verification and push: Task 6.

No placeholders remain. The plan intentionally starts with backend/user/admin/case representative coverage, because those are the highest-risk drift points. Additional module-specific UI coverage can be added after the shared capability model is in place without changing the architecture.
