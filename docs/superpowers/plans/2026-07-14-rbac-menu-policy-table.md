# Declarative RBAC Menu Policy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the procedural menu-key dispatcher with one declarative policy table without changing any permission result or public API.

**Architecture:** Private `MenuPolicy`, `FlagPolicy`, `PermissionSource`, and `PermissionBundle` values describe aliases and checkbox mappings. One evaluator resolves fixed slices or existing role bundles and appends them with stable first-occurrence deduplication.

**Tech Stack:** Rust, standard library static slices, existing `lib-core` RBAC tests.

## Global Constraints

- Keep `AdminMenuPrivilege` and `permissions_for_menu_privileges` signatures unchanged.
- Preserve trimmed lookup, unknown-key empty output, output ordering, and deduplication.
- Preserve policy fingerprint `5602083785880063594`.
- Add no dependency and expose no new public implementation type.

---

### Task 1: Enforce Declarative Policy Structure

**Files:**
- Modify: `crates/libs/lib-core/src/model/acs/tests.rs`
- Test: `crates/libs/lib-core/src/model/acs/tests.rs`

**Interfaces:**
- Consumes: filesystem path `src/model/acs/menu_policy.rs`
- Produces: structural regression test `menu_policy_is_declarative`

- [ ] **Step 1: Write the failing structural test**

```rust
#[test]
fn menu_policy_is_declarative() {
    let source = fs::read_to_string(acs_dir().join("menu_policy.rs")).unwrap();
    assert!(source.contains("static MENU_POLICIES:"));
    assert!(!source.contains("match menu_key"));
}
```

Extract the existing ACS directory construction into `fn acs_dir() -> PathBuf`
so both structural tests use the same helper.

- [ ] **Step 2: Run the test and verify RED**

Run: `cargo test -p lib-core menu_policy_is_declarative -- --nocapture`

Expected: FAIL because `MENU_POLICIES` does not exist.

- [ ] **Step 3: Commit only after Task 2 turns GREEN**

```bash
git add crates/libs/lib-core/src/model/acs/tests.rs \
  crates/libs/lib-core/src/model/acs/menu_policy.rs
git commit -m "refactor: declare RBAC menu policies as data"
```

---

### Task 2: Replace Dispatcher With Policy Data

**Files:**
- Modify: `crates/libs/lib-core/src/model/acs/menu_policy.rs`
- Test: `crates/libs/lib-core/src/model/acs/menu_policy.rs`
- Test: `crates/libs/lib-core/src/model/acs/tests.rs`

**Interfaces:**
- Consumes: `viewer_permissions()`, `profile_edit_permissions()`, `admin_permissions()`
- Produces: unchanged `pub fn permissions_for_menu_privileges(&[AdminMenuPrivilege]) -> Vec<Permission>`

- [ ] **Step 1: Add private policy representation**

```rust
#[derive(Clone, Copy)]
enum PermissionBundle { Viewer, ProfileEdit, Admin }

#[derive(Clone, Copy)]
enum PermissionSource {
    Fixed(&'static [Permission]),
    Bundle(PermissionBundle),
}

struct FlagPolicy {
    read: &'static [PermissionSource],
    edit: &'static [PermissionSource],
    review: &'static [PermissionSource],
    lock: &'static [PermissionSource],
}

struct MenuPolicy {
    keys: &'static [&'static str],
    flags: FlagPolicy,
}
```

- [ ] **Step 2: Define all existing menu mappings in `MENU_POLICIES`**

Declare one entry for each semantic family: `home_workflow`, `home_notice`,
`home_email`, `case`, `info`, `import`, export aliases, user aliases, `audit`,
terminology aliases, `admin`, `settings`, and `roles`. Do not add organization
or unknown entries because their existing result is empty.

- [ ] **Step 3: Implement generic source resolution and evaluation**

```rust
fn resolve(source: PermissionSource) -> &'static [Permission] {
    match source {
        PermissionSource::Fixed(permissions) => permissions,
        PermissionSource::Bundle(PermissionBundle::Viewer) => viewer_permissions(),
        PermissionSource::Bundle(PermissionBundle::ProfileEdit) => profile_edit_permissions(),
        PermissionSource::Bundle(PermissionBundle::Admin) => admin_permissions(),
    }
}
```

Select enabled sources in read, edit, review, lock order and append each resolved
slice through `push_unique`. Delete `permissions_for_menu_key` and its
`match menu_key` body.

- [ ] **Step 4: Run targeted tests and verify GREEN**

Run:

```bash
cargo fmt --all
cargo test -p lib-core menu_policy_is_declarative -- --nocapture
cargo test -p lib-core rbac_builtin_and_menu_policy_snapshot -- --nocapture
cargo test -p lib-core model::acs:: -- --nocapture
```

Expected: all pass and the fingerprint remains `5602083785880063594`.

- [ ] **Step 5: Run complete verification**

Run: `cargo test -p lib-core`

Expected: 124 tests pass and 10 existing database-dependent tests remain ignored.

- [ ] **Step 6: Measure the result**

Run:

```bash
wc -c crates/libs/lib-core/src/model/acs/*.rs
git diff --stat HEAD^..HEAD
git status --short
```

Expected: clean worktree after commit and no regression in the complete suite.
