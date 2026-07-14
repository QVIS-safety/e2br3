# RBAC Architecture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split the monolithic RBAC implementation into focused modules, generate permission groups declaratively, preserve current behavior, and then correct policy gaps proven by tests.

**Architecture:** `types`, `catalog`, `builtin_roles`, `menu_policy`, `dynamic_roles`, and `check` modules remain private behind the existing `acs::*` facade. `catalog` generates every existing public permission constant plus internal ordered resource groups. Built-in roles compose those groups with explicit action filters, and policy corrections occur only after an exhaustive behavior snapshot passes.

**Tech Stack:** Rust, `std::sync::OnceLock`/`RwLock`, Cargo tests

## Global Constraints

- Preserve every existing public permission constant name and value.
- Preserve `lib_core::model::acs::*` import compatibility.
- Preserve permission ordering during the structural phase.
- Dynamic custom roles override built-in roles after canonical normalization.
- No database schema, API payload, role identifier, or menu key changes.
- Policy corrections require independent failing tests and independent commits.

---

### Task 1: Pin exhaustive RBAC behavior

**Files:**
- Modify: `crates/libs/lib-core/src/model/acs/permission.rs`

**Interfaces:**
- Consumes: `role_permissions`, `permissions_for_menu_privileges`, `Permission::to_string`.
- Produces: deterministic FNV-1a snapshots for built-in roles and every menu checkbox combination.

- [ ] **Step 1: Add a deterministic snapshot helper and deliberately failing expectations**

Inside the existing test module, add an FNV-1a hash over each ordered permission's display bytes and separators. Add `rbac_builtin_and_menu_policy_snapshot` covering:

```rust
[
	ROLE_SYSTEM_ADMIN,
	ROLE_SPONSOR_ADMIN_CRO,
	ROLE_SPONSOR_ADMIN_COMPANY,
	ROLE_USER,
	"viewer",
	"unknown",
]
```

For menu keys, cover `home_workflow`, `home_notice`, `home_email`, `case`, `info`, `import`, `export_submission`, `submission`, `export`, `user`, `users`, `audit`, `data`, `terminology`, `admin`, `settings`, `roles`, `organization`, `organizations`, and `unknown`, with each of the four checkbox flags enabled independently and all four enabled together. Initialize expected hashes to zero.

- [ ] **Step 2: Run RED and record baseline hashes**

Run:

```bash
cargo test -p lib-core rbac_builtin_and_menu_policy_snapshot -- --nocapture
```

Expected: FAIL showing the actual nonzero snapshot values.

- [ ] **Step 3: Replace zero expectations with the observed baseline values**

Run the same command. Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/libs/lib-core/src/model/acs/permission.rs
git commit -m "test: snapshot complete RBAC policy"
```

### Task 2: Extract permission types and declarative catalog

**Files:**
- Create: `crates/libs/lib-core/src/model/acs/types.rs`
- Create: `crates/libs/lib-core/src/model/acs/catalog.rs`
- Modify: `crates/libs/lib-core/src/model/acs/mod.rs`
- Modify: `crates/libs/lib-core/src/model/acs/permission.rs`

**Interfaces:**
- Produces: unchanged public `Resource`, `Action`, `Permission`, and all 190 permission constants; internal ordered `*_PERMISSIONS` slices.
- Consumes: no runtime state.

- [ ] **Step 1: Add a failing source-structure test**

Add `crates/libs/lib-core/src/model/acs/tests.rs` with assertions using `include_str!` that require `types.rs`, `catalog.rs`, and the macro invocation marker `permission_group! {`, and reject public permission constants remaining in `permission.rs`.

Run:

```bash
cargo test -p lib-core acs_modules_separate_types_and_catalog -- --nocapture
```

Expected: FAIL because the new modules do not exist yet.

- [ ] **Step 2: Move types without changing their definitions**

Move `Resource`, `Action`, `Permission`, its accessors, and `Display` to `types.rs`. Re-export them from `acs/mod.rs`.

- [ ] **Step 3: Generate ordered permission groups**

In `catalog.rs`, define:

```rust
macro_rules! permission_group {
	(
		$group:ident,
		$resource:ident,
		$( $constant:ident => $action:ident ),+ $(,)?
	) => {
		$(
			pub const $constant: Permission =
				Permission::new(Resource::$resource, Action::$action);
		)+
		pub(crate) const $group: &[Permission] = &[$($constant),+];
	};
}
```

Invoke it once per resource, listing constants/actions in their current order. Re-export all public constants from `acs/mod.rs`.

- [ ] **Step 4: Verify behavior and commit**

```bash
cargo fmt --all -- --check
cargo test -p lib-core rbac_builtin_and_menu_policy_snapshot
cargo check -p web-server
git add crates/libs/lib-core/src/model/acs
git commit -m "refactor: extract declarative permission catalog"
```

### Task 3: Extract and compact built-in role policies

**Files:**
- Create: `crates/libs/lib-core/src/model/acs/builtin_roles.rs`
- Modify: `crates/libs/lib-core/src/model/acs/mod.rs`
- Modify: `crates/libs/lib-core/src/model/acs/permission.rs`
- Modify: `crates/libs/lib-core/src/model/acs/tests.rs`

**Interfaces:**
- Produces: `admin_permissions`, `system_admin_permissions`, `profile_edit_permissions`, `viewer_permissions`, and `role_permissions` with unchanged ordered output.
- Consumes: ordered resource groups from `catalog.rs`.

- [ ] **Step 1: Extend the source-structure test and verify RED**

Require `builtin_roles.rs`, reject `fn admin_permissions` in `permission.rs`, and run `acs_modules_separate_types_and_catalog`. Expected: FAIL.

- [ ] **Step 2: Implement shared group composition**

Use `OnceLock<Vec<Permission>>` per built-in set and a helper that appends selected groups while retaining group and action order. Keep explicit group lists for admin, profile edit, and viewer so intentionally omitted resources remain omitted.

For profile edit, filter normal operational groups to `Create`, `Read`, `Update`, and `List`, then append the current special permissions in their current order. Preserve the presave DELETE exception. For viewer, filter its current explicit groups to `Read` and `List` only.

- [ ] **Step 3: Verify snapshot identity and commit**

```bash
cargo test -p lib-core rbac_builtin_and_menu_policy_snapshot -- --nocapture
cargo test -p lib-core model::acs
cargo check -p web-server
git add crates/libs/lib-core/src/model/acs
git commit -m "refactor: compose built-in RBAC roles from groups"
```

### Task 4: Separate menu policy, dynamic roles, and checks

**Files:**
- Create: `crates/libs/lib-core/src/model/acs/menu_policy.rs`
- Create: `crates/libs/lib-core/src/model/acs/dynamic_roles.rs`
- Create: `crates/libs/lib-core/src/model/acs/check.rs`
- Delete: `crates/libs/lib-core/src/model/acs/permission.rs`
- Modify: `crates/libs/lib-core/src/model/acs/mod.rs`
- Modify: `crates/libs/lib-core/src/model/acs/tests.rs`

**Interfaces:**
- `menu_policy`: `AdminMenuPrivilege`, `permissions_for_menu_privileges`.
- `dynamic_roles`: replace/upsert/remove plus an internal normalized lookup.
- `check`: `role_permissions`, `has_permission`, `has_any_permission`, `has_all_permissions`.

- [ ] **Step 1: Extend the source-structure test and verify RED**

Require the three new modules and require `permission.rs` to be absent. Expected: FAIL.

- [ ] **Step 2: Move menu expansion unchanged**

Move `AdminMenuPrivilege`, `push_unique`, `permissions_for_menu_key`, and `permissions_for_menu_privileges` to `menu_policy.rs`. Keep all aliases and branch conditions unchanged.

- [ ] **Step 3: Move dynamic state and centralize lookup**

Move dynamic-role state and mutations to `dynamic_roles.rs`. Provide an internal closure-based lookup so the `RwLock` guard never escapes. Preserve current normalization and override precedence.

- [ ] **Step 4: Move permission checks**

Move the four public query functions to `check.rs`, using the shared dynamic lookup once per query and falling back to built-in permissions only when no dynamic role exists.

- [ ] **Step 5: Verify and commit**

```bash
cargo fmt --all -- --check
cargo test -p lib-core model::acs -- --nocapture
cargo test -p web-server --test api role_admin -- --test-threads=1
cargo test -p web-server --test authz rbac -- --test-threads=1
cargo check -p web-server
git add crates/libs/lib-core/src/model/acs
git commit -m "refactor: separate RBAC policy and runtime state"
```

### Task 5: Audit and correct proven policy gaps

**Files:**
- Modify: `crates/libs/lib-core/src/model/acs/builtin_roles.rs`
- Modify: `crates/libs/lib-core/src/model/acs/menu_policy.rs`
- Modify: `crates/libs/lib-core/src/model/acs/dynamic_roles.rs`
- Modify: `crates/libs/lib-core/src/model/acs/tests.rs`
- Test: `crates/services/web-server/tests/api/role_admin/privilege_matrix_web.rs`

**Interfaces:**
- Produces: independently tested policy corrections only where expected behavior is demonstrated by route requirements and menu semantics.

- [ ] **Step 1: Prove and fix case-view device-characteristic access**

Add a failing test showing a `case` read privilege grants `DRUG_DEVICE_CHARACTERISTIC_READ` and `DRUG_DEVICE_CHARACTERISTIC_LIST`, matching the device-characteristic GET/list route requirements. Add the device-characteristic group to viewer composition, verify GREEN, and commit:

```bash
git commit -am "fix: include device characteristics in case view access"
```

- [ ] **Step 2: Prove and fix sponsor-admin e-mail permission**

Add a failing test that both sponsor-admin roles include `EMAIL_NOTIFICATION_SEND`, consistent with `admin_permissions` being their full administrative policy. Add the permission, verify GREEN, and commit:

```bash
git commit -am "fix: grant sponsor admins reserved email permission"
```

- [ ] **Step 3: Make equivalent aliases an explicit contract**

Add equality tests for `user/users`, `data/terminology`, and `export_submission/submission/export` across all checkbox combinations. Change policy only if a test reveals a difference. Commit tests separately.

- [ ] **Step 4: Recover poisoned dynamic-role reads and writes**

Refactor lock acquisition through internal helpers that recover `PoisonError::into_inner()` rather than silently dropping updates or falling back to built-in permissions. Add a module-local test that poisons an isolated test lock and proves lookup/update recovery; do not poison the process-global role cache. Commit independently.

- [ ] **Step 5: Report unproven candidates without changing them**

Keep system-admin operational permissions and the profile DELETE asymmetry unchanged unless an existing endpoint/menu contract supplies an explicit contradictory expectation.

### Task 6: Final verification and reduction report

**Files:**
- Verify: `crates/libs/lib-core/src/model/acs/`

- [ ] **Step 1: Run full focused verification**

```bash
cargo fmt --all -- --check
cargo check -p web-server
cargo test -p lib-core model::acs -- --nocapture
cargo test -p web-server --test api role_admin -- --test-threads=1
cargo test -p web-server --test authz rbac -- --test-threads=1
git diff dev...HEAD --check
```

- [ ] **Step 2: Measure total ACS source reduction**

```bash
{ git show dev:crates/libs/lib-core/src/model/acs/mod.rs; git show dev:crates/libs/lib-core/src/model/acs/permission.rs; } | wc -c
find crates/libs/lib-core/src/model/acs -type f -name '*.rs' -exec wc -c {} +
```

Compare all ACS Rust files so splitting cannot disguise net growth.

- [ ] **Step 3: Confirm clean branch**

```bash
git status --short --branch
```
