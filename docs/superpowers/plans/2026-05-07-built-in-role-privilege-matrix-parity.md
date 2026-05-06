# Built-In Role Privilege Matrix Parity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Admin Role & Privilege show authoritative backend-backed privileges for fixed built-in roles and editable persisted roles without fabricating matrix behavior in the frontend.

**Architecture:** The backend remains the source of truth for every role column in `/api/admin/roles`. Fixed sponsor-administrator roles keep fixed names and fixed full read/edit authority per `docs/requirements/roles.csv`; their matrix cells are populated from backend privilege records but remain read-only. Sponsor administrators continue to create and edit additional roles, including Safety Database Administrator/PVM/PVS-style roles, through persisted `app_roles` records.

**Tech Stack:** Rust/Axum/sqlx backend, `lib_core::model::acs` permission mapping, Next.js Admin page, Jest frontend tests, existing web-server API tests.

---

## Requirement Reading

Source requirement files:

- `docs/requirements/03.csv`, row 26: ADMIN Management General asks for Role creation, per-item permissions (`View`, `Review`, `Lock`), `Role setting`, `Role & Privilege`, requested roles such as PVM/PVS/Head of PV/Sponsor/ADB admin, and Settings.
- `docs/requirements/03.csv`, row 26 follow-up: Create Custom Role should use `role_name` and Description, Role Setting vs Role & Privilege should be clarified, permissions must be set in Role & Privilege per menu (`CASE`, `INFO`, etc.) with `Read`, `Edit`, `Review`, `Lock`, and added roles need editing.
- `docs/requirements/roles.csv`: `System Administrator` grants/revokes Safety Database access to sponsor administrators but has no in-database working authority. `Sponsor Administrator(CRO)` and `Sponsor Administrator (Pharmaceutical Company)` are fixed-name roles with full read/edit access and admin authority. Sponsor administrators can create roles, edit created role menu permissions, assign roles to users, and create roles equivalent to sponsor admin authority.

Important product constraint:

- `roles.csv` says sponsor administrator role names and authority are fixed. Therefore this plan implements built-in role matrix parity as **backend-backed display parity**, not mutable sponsor-admin privilege editing. Editable privilege cells remain for persisted roles created by sponsor administrators. If the product owner wants fixed sponsor-admin privileges to be mutable, that is a requirement change and should be recorded explicitly before implementation.

## File Structure

- Modify `crates/services/web-server/src/web/rest/admin_role_rest.rs`
  - Owns role API response shape and built-in role definitions.
  - Add canonical menu list and built-in privilege helpers.
  - Return one authoritative row per built-in role with full `privileges` and `privilege_map`.
- Modify `crates/libs/lib-core/src/model/acs/permission.rs`
  - Keep `AdminMenuPrivilege` as the shared DTO.
  - Add tests if any privilege helper is moved here; otherwise backend REST tests are enough.
- Modify `crates/services/web-server/tests/api/scope_visibility_web.rs`
  - Extend role admin API coverage for fixed built-in role matrix privileges.
  - Preserve existing assertion that built-in sponsor-admin role updates are rejected.
- Modify `frontend/E2BR3-frontend/app/dashboard/admin/page.tsx`
  - Use backend-provided privileges for all roles in the matrix.
  - Keep cells disabled when `isBuiltin` or `isEditable === false`.
  - Remove frontend assumptions that built-in cells are blank/unavailable.
- Modify `frontend/E2BR3-frontend/__tests__/admin-users.header-filters.test.ts`
  - Update API fixture built-in roles to include authoritative menu privileges.
  - Assert sponsor-admin built-in columns display checked read/edit cells but remain disabled.
  - Assert system admin does not get false in-database working menu privileges.
- Modify `docs/ui-alignment/cubesafety-safetydb-alignment.md`
  - Replace the remaining-gap note after implementation: built-in matrix display parity complete; fixed sponsor-admin privilege editing intentionally not allowed per `roles.csv`.

## Task 1: Backend Built-In Privilege Catalog

**Files:**
- Modify: `crates/services/web-server/src/web/rest/admin_role_rest.rs`
- Test: `crates/services/web-server/tests/api/scope_visibility_web.rs`

- [ ] **Step 1: Write failing backend API test**

Add this assertion block to `test_role_admin_api_exposes_client_role_metadata` after the sponsor role is found:

```rust
let sponsor_privileges = sponsor["privileges"]
    .as_array()
    .ok_or("sponsor privileges should be an array")?;
for menu_key in [
    "case",
    "info",
    "import",
    "export_submission",
    "users",
    "roles",
    "settings",
    "audit",
    "data",
] {
    let privilege = sponsor_privileges
        .iter()
        .find(|row| row["menu_key"] == menu_key)
        .ok_or_else(|| format!("missing sponsor privilege for {menu_key}"))?;
    assert_eq!(privilege["can_read"].as_bool(), Some(true), "{menu_key}");
    assert_eq!(privilege["can_edit"].as_bool(), Some(true), "{menu_key}");
}
assert_eq!(sponsor["is_editable"].as_bool(), Some(false));

let system_privileges = system["privileges"]
    .as_array()
    .ok_or("system privileges should be an array")?;
assert!(
    system_privileges.is_empty(),
    "system admin should not receive Safety DB working menu privileges"
);
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p web-server test_role_admin_api_exposes_client_role_metadata --test api -- --nocapture --test-threads=1
```

Expected: FAIL because sponsor built-in rows currently expose only the broad `admin` privilege instead of one row per menu.

- [ ] **Step 3: Implement backend built-in privilege helpers**

In `crates/services/web-server/src/web/rest/admin_role_rest.rs`, add helpers near `normalize_admin_privileges`:

```rust
const ADMIN_ROLE_MENU_KEYS: &[&str] = &[
    "case",
    "info",
    "import",
    "export_submission",
    "users",
    "roles",
    "settings",
    "audit",
    "data",
];

fn full_menu_privileges() -> Vec<AdminMenuPrivilege> {
    ADMIN_ROLE_MENU_KEYS
        .iter()
        .map(|menu_key| AdminMenuPrivilege {
            menu_key: (*menu_key).to_string(),
            can_read: true,
            can_edit: true,
            can_review: *menu_key == "case",
            can_lock: *menu_key == "case",
        })
        .collect()
}
```

Then change both sponsor-admin built-in rows in `built_in_roles()` from the single `admin` privilege to:

```rust
full_menu_privileges()
```

Keep `ROLE_SYSTEM_ADMIN` with `Vec::new()` because `roles.csv` says it has no in-database working authority.

- [ ] **Step 4: Run backend test to verify pass**

Run:

```bash
cargo test -p web-server test_role_admin_api_exposes_client_role_metadata --test api -- --nocapture --test-threads=1
```

Expected: PASS.

- [ ] **Step 5: Commit backend API slice**

```bash
git add crates/services/web-server/src/web/rest/admin_role_rest.rs crates/services/web-server/tests/api/scope_visibility_web.rs
git commit -m "fix: expose built-in role menu privileges"
```

## Task 2: Frontend Matrix Uses Backend Built-In Privileges

**Files:**
- Modify: `frontend/E2BR3-frontend/app/dashboard/admin/page.tsx`
- Test: `frontend/E2BR3-frontend/__tests__/admin-users.header-filters.test.ts`

- [ ] **Step 1: Write failing frontend test**

In `apiBuiltInRoles`, change sponsor-admin fixtures to include full menu privileges:

```ts
const fullMenuPrivileges = [
  "case",
  "info",
  "import",
  "export_submission",
  "users",
  "roles",
  "settings",
  "audit",
  "data",
].map((menuKey) => ({
  menuKey,
  canRead: true,
  canEdit: true,
  canReview: menuKey === "case",
  canLock: menuKey === "case",
}));
```

Use `privileges: fullMenuPrivileges` for `sponsor_admin_cro` and `sponsor_admin_company`.

Add assertions to the Role & Privilege test:

```ts
expect(matrixCheckbox("CASE", "Read", "Sponsor Administrator (CRO)").checked).toBe(true);
expect(matrixCheckbox("CASE", "Read", "Sponsor Administrator (CRO)").disabled).toBe(true);
expect(matrixCheckbox("Settings", "Edit", "Sponsor Administrator (CRO)").checked).toBe(true);
expect(matrixCheckbox("Settings", "Edit", "Sponsor Administrator (CRO)").disabled).toBe(true);
expect(matrixCheckbox("CASE", "Read", "System Administrator").checked).toBe(false);
expect(matrixCheckbox("CASE", "Read", "System Administrator").disabled).toBe(true);
```

- [ ] **Step 2: Run frontend test to verify it fails**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npx jest --runTestsByPath __tests__/admin-users.header-filters.test.ts --runInBand -t "four-tab"
```

Expected: FAIL because built-in matrix cells are currently always `checked={false}`.

- [ ] **Step 3: Use one role list for matrix columns**

In `app/dashboard/admin/page.tsx`, add a memoized matrix role list near `workflowRoleOptions`:

```ts
const matrixRoles = useMemo(() => {
  const apiRoleByName = new Map(customRoles.map((role) => [role.roleName, role]));
  const staticRows = ROLE_OPTIONS.map((role) => {
    const apiRole = apiRoleByName.get(role.value);
    return apiRole || {
      roleName: role.value,
      displayName: role.label,
      description: role.label,
      privileges: [],
      active: true,
      isBuiltin: true,
      isEditable: false,
    } satisfies AdminRole;
  });
  return [
    ...staticRows,
    ...persistedCustomRoles.filter((role) => !STATIC_ROLE_VALUES.has(role.roleName as UserRole)),
  ];
}, [customRoles, persistedCustomRoles]);
```

- [ ] **Step 4: Render built-in cells from backend privileges**

Replace the two separate `ROLE_OPTIONS.map(...)` and `persistedCustomRoles.map(...)` blocks in the Role & Privilege table header/body with `matrixRoles.map(...)`.

Header:

```tsx
{matrixRoles.map((role) => (
  <th key={role.roleName} className="px-4 py-3">
    {role.description || role.displayName || role.roleName}
  </th>
))}
```

Cell:

```tsx
{matrixRoles.map((role) => {
  const privilege = persistedRolePrivileges(role.privileges).find((item) => item.menuKey === menu.key);
  const isEditableRole = !role.isBuiltin && !role.builtIn && role.isEditable !== false;
  return (
    <td key={role.roleName} className="px-4 py-3 text-center">
      <input
        type="checkbox"
        checked={Boolean(privilege?.[column.key])}
        disabled={!isEditableRole}
        onChange={(e) => void updatePersistedRolePrivilege(role, menu.key, column.key, e.target.checked)}
      />
    </td>
  );
})}
```

This keeps sponsor-admin fixed roles read-only but no longer blank.

- [ ] **Step 5: Run frontend test to verify pass**

Run:

```bash
npx jest --runTestsByPath __tests__/admin-users.header-filters.test.ts --runInBand
```

Expected: PASS.

- [ ] **Step 6: Commit frontend matrix slice**

```bash
git add app/dashboard/admin/page.tsx __tests__/admin-users.header-filters.test.ts
git commit -m "fix: render built-in role privileges from API"
```

## Task 3: Requirement Tracker And Alignment Record

**Files:**
- Modify: `docs/requirements/client_requirements_todo.md`
- Modify: `docs/ui-alignment/cubesafety-safetydb-alignment.md`

- [ ] **Step 1: Update requirement tracker wording**

In `docs/requirements/client_requirements_todo.md`, update the Roles and Privileges completion notes to clarify:

```markdown
- [x] Support per-menu permissions for read, edit, QC/review, and lock instead of only coarse role creation. Backend `/api/admin/roles` exposes authoritative per-menu privileges for fixed sponsor-admin roles and persisted custom roles; the frontend matrix renders fixed roles read-only and persisted roles editable.
```

Do not claim sponsor-admin privilege editing is supported; `roles.csv` says sponsor-admin names and authority are fixed.

- [ ] **Step 2: Update UI alignment record**

In `docs/ui-alignment/cubesafety-safetydb-alignment.md`, change the Admin Four Tabs remaining gap:

```markdown
- Remaining gaps:
  - Built-in role privilege matrix display parity is implemented through backend-provided privilege records. Fixed sponsor-admin roles remain read-only by requirement; custom/persisted roles remain editable.
```

- [ ] **Step 3: Commit docs slice**

```bash
git add docs/requirements/client_requirements_todo.md docs/ui-alignment/cubesafety-safetydb-alignment.md
git commit -m "docs: clarify fixed role privilege matrix parity"
```

## Task 4: Final Verification

**Files:**
- Verify backend and frontend only.

- [ ] **Step 1: Backend targeted verification**

Run:

```bash
cargo test -p web-server test_role_admin_api_exposes_client_role_metadata --test api -- --nocapture --test-threads=1
cargo test -p web-server test_role_admin_api_persists_menu_privileges --test api -- --nocapture --test-threads=1
```

Expected: PASS.

- [ ] **Step 2: Frontend targeted verification**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npx jest --runTestsByPath __tests__/admin-users.header-filters.test.ts --runInBand
npx tsc --noEmit
```

Expected: PASS. If `.next/types` is stale/missing, run `npm run build` once and then rerun `npx tsc --noEmit`.

- [ ] **Step 3: Production build verification**

Run:

```bash
npm run build
```

Expected: PASS. Existing stale browser-data warnings are acceptable if unchanged.

## Self-Review

Spec coverage:

- `roles.csv` fixed sponsor-admin roles: covered by read-only sponsor-admin cells and existing backend update rejection.
- Sponsor-admin full read/edit authority: covered by backend `full_menu_privileges()` API test and frontend checked disabled cells.
- Created/custom role menu permission editing: preserved by existing custom-role API and frontend editable persisted-role matrix.
- System administrator no in-database authority: covered by empty system admin privilege assertion.

Known non-goal:

- Mutating fixed sponsor-admin privileges is not implemented because it conflicts with `roles.csv`. If the client explicitly wants it later, add a requirement-change task first and decide how to migrate fixed roles out of hardcoded authority.
