# RBAC Dynamic Role Tests Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add isolated integration coverage for dynamic-role lifecycle operations and precedence over built-in roles.

**Architecture:** One integration test binary owns all mutations to the process-global dynamic-role registry. A cleanup guard empties the registry on return or panic, while the test exercises only exported ACS APIs.

**Tech Stack:** Rust integration tests and the existing `lib-core` ACS API.

## Global Constraints

- Do not modify production RBAC behavior or public interfaces.
- Do not require `--test-threads=1`.
- Restore the global dynamic-role registry after every test outcome.
- Keep the existing RBAC policy fingerprint unchanged.

---

### Task 1: Dynamic Role Lifecycle and Precedence Integration Test

**Files:**
- Create: `crates/libs/lib-core/tests/rbac_dynamic_roles.rs`

**Interfaces:**
- Consumes: `replace_dynamic_roles`, `upsert_dynamic_role_permissions`, `remove_dynamic_role`, `has_permission`, `has_any_permission`, `has_all_permissions`
- Produces: integration test `dynamic_role_lifecycle_and_builtin_precedence`

- [ ] **Step 1: Write the integration test and cleanup guard**

Create a `RegistryCleanup` guard whose `Drop` implementation calls
`replace_dynamic_roles(HashMap::new())`. In one test, clear the registry, then
assert registration with a whitespace-and-uppercase custom role, replacement
of that role's permissions, removal, full-map replacement, and disappearance
of entries omitted from a subsequent replacement.

For precedence, insert `[CASE_READ]` for `ROLE_SPONSOR_ADMIN_CRO`; assert
`CASE_READ` is granted and the built-in `USER_CREATE` is denied. Remove the
dynamic entry and assert built-in `USER_CREATE` is restored.

- [ ] **Step 2: Run the new test**

Run: `cargo test -p lib-core --test rbac_dynamic_roles -- --nocapture`

Expected: one test passes without test-thread serialization flags.

- [ ] **Step 3: Prove repeatability**

Run twice:

```bash
cargo test -p lib-core --test rbac_dynamic_roles -- --nocapture
cargo test -p lib-core --test rbac_dynamic_roles -- --nocapture
```

Expected: both runs pass, demonstrating cleanup does not leak state.

- [ ] **Step 4: Run RBAC and full regression suites**

```bash
cargo test -p lib-core model::acs:: -- --nocapture
cargo test -p lib-core
```

Expected: all tests pass; only the existing database-dependent tests remain ignored.

- [ ] **Step 5: Commit**

```bash
git add crates/libs/lib-core/tests/rbac_dynamic_roles.rs
git commit -m "test: cover dynamic RBAC role lifecycle"
```
