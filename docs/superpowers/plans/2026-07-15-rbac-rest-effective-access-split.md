# RBAC REST Effective Access Test Split Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the 1,600-line `privilege_matrix_web.rs` with focused effective-access modules while preserving every existing test.

**Architecture:** `effective_access.rs` declares area modules. Each area file keeps its original test bodies and imports existing role-admin helpers through the parent module; no production code changes.

**Tech Stack:** Rust, Axum Router integration tests, PostgreSQL fixtures, `serial_test`.

## Global Constraints

- Preserve every existing test function name and assertion.
- Delete `privilege_matrix_web.rs` only after the inventory matches.
- Do not duplicate request or fixture helpers.
- Do not modify production code.

---

### Task 1: Capture Inventory and Create Module Skeleton

- [ ] Record every `test_*` function in `privilege_matrix_web.rs`.
- [ ] Create `effective_access.rs` declaring `persistence_web`, `case_web`, `information_web`, `transfer_web`, `terminology_web`, `administration_web`, and `dashboard_web`.
- [ ] Replace `mod privilege_matrix_web` in `role_admin/mod.rs` with `mod effective_access`.

### Task 2: Move Existing Tests by Area

- [ ] Move persistence and unsupported-menu validation to `persistence_web.rs`.
- [ ] Move case and workflow tests to `case_web.rs`.
- [ ] Move information/presave tests to `information_web.rs`.
- [ ] Move export and import tests to `transfer_web.rs`.
- [ ] Move terminology tests to `terminology_web.rs`.
- [ ] Move users/roles, settings, and audit tests to `administration_web.rs`.
- [ ] Move notice and e-mail tests to `dashboard_web.rs`.
- [ ] Delete the original file and compare test-name inventories exactly.

### Task 3: Verify and Commit

- [ ] Run `cargo fmt --all`.
- [ ] Run `cargo test -p web-server --test api role_admin -- --nocapture`.
- [ ] Run `cargo test -p web-server --test authz -- --nocapture`.
- [ ] Run `cargo test -p web-server`.
- [ ] Commit with `test: split RBAC REST effective access coverage`.
