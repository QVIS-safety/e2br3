# Focused RBAC Dynamic Role Tests Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the combined dynamic-role test with focused lifecycle and permission-profile tests organized in a dedicated folder.

**Architecture:** A top-level integration target declares focused modules under `tests/rbac_dynamic_roles/`. Shared support serializes registry mutations, resets global state with a panic-safe guard, expands menu profiles, installs them, and provides complete positive permission assertions.

**Tech Stack:** Rust integration tests, `serial_test`, and exported `lib-core::model::acs` APIs.

## Global Constraints

- Do not modify production RBAC code or public interfaces.
- Every registry-mutating test uses `#[serial]` and `RegistryGuard`.
- Tests run without `--test-threads=1`.
- The existing RBAC policy fingerprint remains unchanged.

---

### Task 1: Test Harness and Lifecycle Modules

**Files:**
- Modify: `crates/libs/lib-core/tests/rbac_dynamic_roles.rs`
- Create: `crates/libs/lib-core/tests/rbac_dynamic_roles/support.rs`
- Create: `registration.rs`, `update.rs`, `removal.rs`, `replacement.rs`, `precedence.rs` in the same folder

**Interfaces:**
- `RegistryGuard::new()` clears the registry and clears it again from `Drop`.
- `install(role, permissions)` delegates to `upsert_dynamic_role_permissions`.

- [ ] **Step 1:** Replace the combined test with module declarations.
- [ ] **Step 2:** Implement the cleanup guard and helper functions in `support.rs`.
- [ ] **Step 3:** Add one or more focused tests per lifecycle file, using `#[serial]`.
- [ ] **Step 4:** Run `cargo test -p lib-core --test rbac_dynamic_roles -- --nocapture`; expect all lifecycle tests to pass.
- [ ] **Step 5:** Commit with `test: split dynamic RBAC lifecycle coverage`.

---

### Task 2: Operational Permission Profiles

**Files:**
- Create: `crates/libs/lib-core/tests/rbac_dynamic_roles/case_profile.rs`
- Create: `crates/libs/lib-core/tests/rbac_dynamic_roles/information_profile.rs`
- Create: `crates/libs/lib-core/tests/rbac_dynamic_roles/transfer_profile.rs`
- Modify: `crates/libs/lib-core/tests/rbac_dynamic_roles.rs`
- Modify: `crates/libs/lib-core/tests/rbac_dynamic_roles/support.rs`

**Interfaces:**
- `profile(menu_key, read, edit, review, lock) -> Vec<Permission>` expands one `AdminMenuPrivilege`.
- `install_profile(role, permissions)` installs and returns the vector for complete positive assertions.

- [ ] **Step 1:** Add separate case read, edit, review, and lock tests. Assert every expanded permission is granted and representative disabled writes are denied.
- [ ] **Step 2:** Add separate information read and edit tests with disabled-write checks.
- [ ] **Step 3:** Add import and export read/execute tests plus export alias equality tests.
- [ ] **Step 4:** Run the integration target; expect all profile tests to pass.
- [ ] **Step 5:** Commit with `test: cover operational dynamic permission profiles`.

---

### Task 3: Administrative and Dashboard Profiles

**Files:**
- Create: `crates/libs/lib-core/tests/rbac_dynamic_roles/administration_profile.rs`
- Create: `crates/libs/lib-core/tests/rbac_dynamic_roles/dashboard_profile.rs`
- Modify: `crates/libs/lib-core/tests/rbac_dynamic_roles.rs`

**Interfaces:**
- Reuses `RegistryGuard`, `profile`, and `install_profile` from `support.rs`.

- [ ] **Step 1:** Add users, audit, terminology, settings, roles, and admin profile tests with positive and disabled/unrelated negative assertions.
- [ ] **Step 2:** Add workflow, notice, and e-mail profile tests plus unsupported-menu empty-profile coverage.
- [ ] **Step 3:** Run the integration target twice; expect both runs to pass without leaked state.
- [ ] **Step 4:** Run `cargo test -p lib-core model::acs:: -- --nocapture` and `cargo test -p lib-core`; expect zero failures.
- [ ] **Step 5:** Commit with `test: cover administrative dynamic permission profiles`.
