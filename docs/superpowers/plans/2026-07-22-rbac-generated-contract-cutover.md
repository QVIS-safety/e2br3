# Generated RBAC Contract Cutover Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the backend Policy Registry-generated PDF grant contract drive the live Role & Privilege UI and the backend legacy privilege adapter, eliminating the independently maintained `ROLE_PRIVILEGE_ROWS` and `MENU_POLICIES` policy tables.

**Architecture:** The Policy Registry owns every PDF row, its UI binding, availability, grant implications, and entitlements. The frontend renders the generated rows directly; the backend translates stored legacy menu flags to canonical grant IDs and compiles those grants through the Registry, with one temporary entitlement-to-legacy-permission compatibility adapter until route authorization is migrated to Action IDs.

**Tech Stack:** Rust 2021, Serde, existing `lib-core` authorization registry, Next.js, TypeScript, React, Jest.

## Global Constraints

- Product behavior follows PDF page 8: exactly 18 rows, including ADMIN Read/Edit and reserved Report Due Mail Read/Send.
- Reserved rows are visible, disabled, and never persisted or granted.
- CASE Read must not grant XML export execution or user discovery.
- Users Edit must not grant role-profile CRUD or role assignment.
- Existing stored aliases are accepted only by the backend compatibility adapter; the UI emits canonical bindings only.
- There is one editable role per user/organization; this cutover does not add multi-role support.
- Every production behavior change follows red-green TDD.

---

### Task 1: Generate Complete UI Bindings from the Registry

**Files:**
- Modify: `crates/libs/lib-core/src/authorization/definitions.rs`
- Modify: `crates/libs/lib-core/src/authorization/registry.rs`
- Modify: `crates/libs/lib-core/src/authorization/contract.rs`
- Modify: `crates/libs/lib-core/src/authorization/tests.rs`
- Modify: `crates/libs/lib-core/tests/authorization_contract_snapshot.rs`
- Regenerate in frontend: `lib/auth/generated-authorization.ts`

**Interfaces:**
- Produces `GrantUiBinding { menu_key, field }` where `field` is one of `can_read`, `can_edit`, `can_review`, or `can_lock`.
- Extends every generated `PdfRolePrivilegeRow` with `menuKey` and `field`.

- [ ] **Step 1: Write failing registry and export tests**

Assert all 18 PDF rows have unique `(menu_key, field)` bindings, CASE QC binds to `case/can_review`, CASE Lock binds to `case/can_lock`, ADMIN binds to `admin/can_read` and `admin/can_edit`, and Report Due Mail binds to reserved `email_report_due/can_read` and `email_report_due/can_edit` rows.

- [ ] **Step 2: Run tests and verify RED**

Run: `cargo test -p lib-core authorization::tests`

Run: `cargo test -p lib-core --test authorization_contract_snapshot`

Expected: FAIL because generated rows do not expose UI bindings and the registry does not validate duplicate bindings.

- [ ] **Step 3: Implement binding types, validation, and TypeScript export**

Add the binding to each canonical grant definition, reject duplicate bindings during registry construction, include the binding in canonical JSON and generated TypeScript, and keep reserved rows entitlement-free and unassignable.

- [ ] **Step 4: Regenerate and verify GREEN**

Run: `./scripts/generate_frontend_authorization.sh /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/.worktrees/rbac-policy-kernel`

Run the two Rust test commands again. Expected: PASS and a second generation produces no diff.

- [ ] **Step 5: Commit backend and generated frontend artifact**

Commit message backend: `feat: generate role privilege UI bindings`

Commit message frontend: `chore: sync role privilege bindings`

### Task 2: Make the Frontend Render the Generated PDF Rows

**Files:**
- Modify: `lib/admin/roleConfig.ts`
- Modify: `app/(protected)/admin/role-privilege/components/RolePrivilegeMatrix.tsx`
- Modify: `app/(protected)/admin/role-privilege/model/rolePrivilegeModel.ts`
- Modify: `app/(protected)/admin/role-privilege/model/effectiveAccessContract.ts`
- Modify: `__tests__/role-privilege-rows.test.ts`
- Modify: `__tests__/integration/role-privilege-effective-access.contract.test.ts`
- Modify or create: `__tests__/auth/generated-role-privilege-cutover.test.ts`

**Interfaces:**
- `ROLE_PRIVILEGE_ROWS` becomes a typed projection of `PDF_ROLE_PRIVILEGE_ROWS`, not a handwritten array.
- Reserved rows expose `disabled: true` and are excluded from save payloads.

- [ ] **Step 1: Write the failing source-of-truth test**

Assert the rendered row keys equal the generated 18-row list exactly, `roleConfig.ts` contains no handwritten row objects, ADMIN Read is present, and both Report Due Mail rows are present but disabled.

- [ ] **Step 2: Run the focused Jest tests and verify RED**

Run: `npm test -- --runInBand __tests__/auth/generated-role-privilege-cutover.test.ts __tests__/role-privilege-rows.test.ts __tests__/integration/role-privilege-effective-access.contract.test.ts`

Expected: FAIL because the current manual table has 16 rows and different ADMIN/E-mail bindings.

- [ ] **Step 3: Replace the manual table and enforce reserved behavior**

Derive labels, ordering, menu keys, fields, and availability from the generated artifact. Render reserved checkboxes disabled with a visible reserved label; sanitize them out of draft and save payloads.

- [ ] **Step 4: Verify focused and RBAC frontend suites**

Run the focused command again, then:

`npm test -- --runInBand __tests__/auth __tests__/rbac-contract __tests__/sidebar.permissions.test.tsx __tests__/static-role-console.test.ts`

Expected: PASS with 18 generated rows and no handwritten PDF row list.

- [ ] **Step 5: Commit**

Commit message: `feat: render generated role privilege rows`

### Task 3: Replace Backend `MENU_POLICIES` with Registry Grant Compilation

**Files:**
- Modify: `crates/libs/lib-core/src/authorization/registry.rs`
- Create: `crates/libs/lib-core/src/model/acs/registry_adapter.rs`
- Modify: `crates/libs/lib-core/src/model/acs/mod.rs`
- Remove: `crates/libs/lib-core/src/model/acs/menu_policy.rs`
- Modify: `crates/libs/lib-core/src/model/acs/tests.rs`
- Modify: `crates/services/web-server/tests/api/role_admin/effective_access/*.rs`

**Interfaces:**
- Produces `canonical_grants_for_menu_privileges(&[AdminMenuPrivilege]) -> Result<Vec<GrantId>, PrivilegeAdapterError>`.
- Produces `legacy_permissions_for_grants(&[GrantId]) -> Result<Vec<Permission>, PrivilegeAdapterError>` as the only temporary compatibility boundary.
- Keeps public `permissions_for_menu_privileges` behavior while changing its implementation to Registry compilation.

- [ ] **Step 1: Write failing parity and no-duplicate-policy tests**

Assert every implemented generated binding compiles through the Registry, reserved bindings grant nothing, unknown keys fail closed, CASE isolation and Users escalation tests remain true, and production sources contain no `MENU_POLICIES` declaration.

- [ ] **Step 2: Run tests and verify RED**

Run: `cargo test -p lib-core model::acs`

Run: `cargo test -p web-server --test api role_admin::effective_access -- --test-threads=1`

Expected: FAIL because the backend still uses the handwritten policy table.

- [ ] **Step 3: Implement the Registry adapter and delete `MENU_POLICIES`**

Translate legacy stored menu flags through registry-owned bindings to canonical grant IDs, expand implied grants and entitlements with `PolicyRegistry::effective_entitlements`, and map entitlements to existing `Permission` constants only at `registry_adapter.rs`. Reject unknown and reserved grants; preserve aliases only on input.

- [ ] **Step 4: Verify backend policy and API parity**

Run the two commands again. Expected: PASS, including CASE Review/Lock separation, no CASE export/user leakage, Users Edit escalation denial, ADMIN Read/Edit behavior, and reserved E-mail denial.

- [ ] **Step 5: Commit**

Commit message: `refactor: compile menu privileges through policy registry`

### Task 4: Prove End-to-End Contract Ownership

**Files:**
- Modify: `scripts/check_legacy_authorization_paths.sh`
- Modify: `crates/libs/lib-core/tests/authorization_contract_snapshot.rs`
- Modify: frontend `__tests__/integration/role-privilege-effective-access.live.test.ts`

**Interfaces:**
- Produces CI invariants that fail if a manual frontend row table or backend `MENU_POLICIES` reappears.

- [ ] **Step 1: Add failing static ownership checks**

Require exactly one Registry PDF grant list, no handwritten frontend row objects, no backend `MENU_POLICIES`, and byte-stable regeneration.

- [ ] **Step 2: Verify the ownership checks fail before final cleanup**

Run the static Rust and Jest tests. Expected: FAIL until all legacy definitions are removed.

- [ ] **Step 3: Remove remaining duplicate definitions and regenerate**

Delete obsolete manual contract fragments and regenerate the frontend artifact once.

- [ ] **Step 4: Run full verification**

Backend: `cargo check -p web-server --all-targets`, Registry/ACS tests, permission-profile tests, and role-admin effective-access API tests.

Frontend: generated-contract, Role & Privilege, RBAC contract, sidebar, static console, and production build.

Live: start the backend and run `npm run test:rbac-integration` against it, verifying every implemented row OFF -> ON -> OFF and both reserved rows remain non-assignable.

- [ ] **Step 5: Commit**

Commit message backend: `test: enforce registry-owned RBAC contract`

Commit message frontend: `test: enforce generated RBAC ownership`
