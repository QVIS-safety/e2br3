# RBAC Privilege Roundtrip Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the existing Role & Privilege `privileges` value drive frontend visibility and make system-admin role provisioning operate in an explicitly selected sponsor organization.

**Architecture:** The current-user profile will return the assigned role's already-existing normalized `AdminMenuPrivilege[]`; low-level `permissions` remain unchanged for backend action enforcement. Frontend navigation and dashboard actions will consume shared privilege helpers, while permission-profile CRUD and user creation will share an explicit organization target for system administrators.

**Tech Stack:** Rust/Axum/SQLx/PostgreSQL backend, Next.js/React/TypeScript frontend, Cargo tests, Jest, browser E2E.

## Global Constraints

- Do not introduce `roleGrants`, a capability mirror, a new RBAC table, or a client-maintained privilege catalog.
- Persist only the existing normalized `AdminMenuPrivilege[]` shape.
- Keep low-level `permissions` for action/API checks only.
- Keep exactly one active role assignment per user and organization.
- Keep reserved Email rows disabled and absent from persistence payloads.
- Reject custom roles in the system organization.

---

### Task 1: Return Existing Privileges in the Current-User Profile

**Files:**
- Modify: `crates/libs/lib-core/src/model/acs/registry_adapter.rs`
- Modify: `crates/services/web-server/src/web/rest/permission_profile_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/user_rest/dto.rs`
- Modify: `crates/services/web-server/src/web/rest/user_rest/handlers.rs`
- Test: `crates/services/web-server/tests/api/role_admin/effective_access/administration_web.rs`

**Interfaces:**
- Produces: `built_in_menu_privileges(role: &str) -> Vec<AdminMenuPrivilege>`.
- Produces: `CurrentUserProfileView.privileges: Vec<AdminMenuPrivilege>`.
- Consumes: stored `permission_profiles.privileges_json` for custom roles.

- [ ] **Step 1: Write failing backend tests**

Add tests asserting that a current user assigned a custom role containing only `case.read` receives exactly the normalized CASE privilege, and that built-in sponsor/system roles receive the same built-in privileges used by permission-profile responses.

- [ ] **Step 2: Verify RED**

Run: `cargo test -p web-server --test api role_admin::effective_access -- --nocapture`

Expected: compilation or assertion failure because the profile response has no `privileges` field.

- [ ] **Step 3: Implement the shared existing-privilege resolver**

Move built-in privilege construction to the ACS adapter, reuse it from the role API, and resolve custom privileges by assigned profile ID and authenticated organization. Normalize before returning and fail closed to an empty vector for an invalid custom assignment.

- [ ] **Step 4: Verify GREEN**

Run: `cargo test -p web-server --test api role_admin::effective_access -- --nocapture`

Expected: all selected tests pass.

### Task 2: Make Frontend Visibility Consume Privileges

**Files:**
- Modify: `lib/types/api.ts`
- Modify: `lib/api/endpoints/auth.ts`
- Create: `lib/auth/menu-privileges.ts`
- Modify: `lib/contexts/AuthContext.tsx`
- Modify: `components/Sidebar.tsx`
- Modify: `components/dashboard/DashboardPage.tsx`
- Modify: `app/(protected)/admin/AdminWorkspace.tsx`
- Test: `__tests__/auth/menu-privileges.test.ts`
- Test: `__tests__/Sidebar.test.tsx`
- Test: `__tests__/dashboard-page.test.tsx`

**Interfaces:**
- Consumes: `CurrentUserProfile.privileges: AdminMenuPrivilege[]`.
- Produces: `hasMenuPrivilege(privileges, menuKey, field)` and `hasAnyMenuPrivilege(privileges, menuKey)`.
- Produces: `AuthContext.privileges` and privilege-check helpers.

- [ ] **Step 1: Write failing helper and component tests**

Assert that CASE Read makes CASE and HOME available while INFO, ADMIN, Import, Submission, New ICSR, Import XML, Export/Submit, and INFO Master Data remain hidden. Assert that CASE Edit, INFO Read/Edit, Import Edit, Submission Edit/Read, and ADMIN Read/Edit reveal only their corresponding surfaces.

- [ ] **Step 2: Verify RED**

Run: `npm test -- --runInBand __tests__/auth/menu-privileges.test.ts __tests__/Sidebar.test.tsx __tests__/dashboard-page.test.tsx`

Expected: failure because visibility is still inferred from `permissions` and quick actions are unconditional.

- [ ] **Step 3: Implement privilege helpers and switch consumers**

Read menu keys and flags directly from `privileges`. Remove menu visibility dependence on `access-rules.ts`; retain low-level permission helpers for action-specific guards that do not represent menus. Filter Dashboard quick actions by the exact existing privilege and filter ADMIN tabs by `admin` and related existing menu privileges.

- [ ] **Step 4: Verify GREEN**

Run the focused Jest command from Step 2.

Expected: all focused tests pass.

### Task 3: Scope System-Admin Role CRUD to a Sponsor Organization

**Files:**
- Modify: `crates/services/web-server/src/web/rest/permission_profile_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/routes/admin.rs`
- Modify: `crates/libs/lib-core/src/model/permission_profile.rs`
- Test: `crates/services/web-server/tests/authz/rbac_users/permission_profiles_web.rs`

**Interfaces:**
- Consumes: optional query/body `organization_id: Uuid` for system administrators.
- Produces: a validated operation `Ctx` scoped to the selected non-system active organization.
- Sponsor administrators always use their authenticated organization and cannot override it.

- [ ] **Step 1: Write failing organization-scope tests**

Cover missing organization for system admin, system-organization rejection, successful CRUD in a sponsor organization, cross-organization role rejection, and sponsor-admin override rejection/ignore behavior.

- [ ] **Step 2: Verify RED**

Run: `cargo test -p web-server --test authz rbac_users::permission_profiles_web -- --nocapture`

Expected: tests fail because system-admin CRUD currently uses the system organization.

- [ ] **Step 3: Implement one organization-context resolver**

Validate the target organization once per request, construct the scoped context for system administrators, and pass that same context to list/get/create/update/delete. Reject the nil system UUID and inactive targets.

- [ ] **Step 4: Verify GREEN**

Run the focused Cargo command from Step 2.

Expected: all permission-profile organization tests pass.

### Task 4: Share the Selected Organization Across Role and User Forms

**Files:**
- Modify: `lib/api/endpoints/admin.ts`
- Modify: `lib/types/api.ts`
- Modify: `app/(protected)/admin/AdminWorkspace.tsx`
- Modify: `app/(protected)/admin/role/hooks/useAdminRoles.ts`
- Modify: `app/(protected)/admin/role/components/AdminRolesPanel.tsx`
- Modify: `app/(protected)/admin/users/hooks/useAdminUserMutations.ts`
- Modify: `app/(protected)/admin/users/components/UserCreateForm.tsx`
- Test: `__tests__/admin-users.header-filters.test.ts`

**Interfaces:**
- Produces: `selectedOrganizationId: string` owned by `AdminWorkspace`.
- Permission-profile API methods accept `organizationId?: string`.
- User creation includes the same `organizationId` for system administrators.

- [ ] **Step 1: Write failing UI/API payload tests**

Assert that system-admin role loading and mutations include the selected organization, user creation sends the same organization, forms are disabled without selection, and sponsor-admin payloads remain fixed to their authenticated organization.

- [ ] **Step 2: Verify RED**

Run: `npm test -- --runInBand __tests__/admin-users.header-filters.test.ts`

Expected: focused assertions fail because no shared organization selection is present.

- [ ] **Step 3: Implement the shared organization selector and payload flow**

Load active non-system organizations using the existing organizations endpoint. Render one selector for system administrators above the Role/User workspaces, pass its ID into both hooks, and keep sponsor administrators fixed to `user.organizationId` without an editable selector.

- [ ] **Step 4: Verify GREEN**

Run the focused Jest command from Step 2.

Expected: all admin workspace tests pass, including the previously stale Email and bulk-toggle expectations updated to generated rows.

### Task 5: Preserve Startup Ordering and Verify the Full Roundtrip

**Files:**
- Modify: `crates/services/web-server/src/main.rs`
- Modify: `crates/services/web-server/src/bootstrap.rs`
- Modify: `crates/libs/lib-core/src/model/user.rs`
- Test: `crates/services/web-server/tests/authz/authorization_startup.rs`

**Interfaces:**
- Authorization storage initializes before administrator bootstrap.
- Organization membership exists before normalized role assignment.
- The nil UUID is treated as the real system organization, not as missing data.

- [ ] **Step 1: Run startup regression tests**

Run: `cargo test -p web-server --test authz authorization_startup:: -- --nocapture`

Expected: three tests pass with the existing local fixes.

- [ ] **Step 2: Run backend and frontend focused suites**

Run:

```bash
cargo test -p lib-core model::acs -- --nocapture
cargo test -p web-server --test authz rbac_users -- --nocapture
cargo test -p web-server --test api role_admin -- --nocapture
npm test -- --runInBand __tests__/auth __tests__/role-privilege-rows.test.ts __tests__/admin-users.header-filters.test.ts __tests__/Sidebar.test.tsx __tests__/dashboard-page.test.tsx
```

Expected: all selected suites pass.

- [ ] **Step 3: Run dedicated-database browser E2E**

Start backend and frontend against an isolated database. Select a sponsor organization as system admin, create a CASE Read role, assign it to a new user, log in, and verify the exact menu/API acceptance sequence from the design.

- [ ] **Step 4: Review and integrate**

Check both repositories for unrelated changes, commit only RBAC files, merge to `dev`, and push after all verification output is clean.
