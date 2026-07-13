# Case–INFO UUID Linkage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Case editor Template application and XML Import persist existing Presave UUID relationships, then authorize Case and Presave visibility with the same UUID scope contract.

**Architecture:** Existing `source_*_presave_id` columns remain the Case relationship source of truth. A shared UUID-scope parser validates new User writes and supports legacy reads; XML Import resolves one authorized Product before Case creation, attaches it to the first imported G.k row, and optionally applies its Sender in the same transaction.

**Tech Stack:** Rust, Axum, SQLx, PostgreSQL, existing web-server integration-test harness.

## Global Constraints

- Do not add a direct Product, Sender, or Study UUID column to `cases`.
- Empty Sender/Product/Study scope means unrestricted access for that dimension.
- `active_sender_identifier` is routing-only and cannot filter Case visibility.
- Blind access remains an independent safety gate.
- New scope writes accept UUID strings; legacy business strings remain readable during compatibility.
- Preserve unrelated existing worktree changes.

---

### Task 1: Unify empty-scope Presave visibility

**Files:**
- Modify: `crates/services/web-server/src/web/rest/section_presave_rest/shared.rs`
- Test: `crates/services/web-server/tests/api/scope_visibility_web.rs`

**Interfaces:**
- Consumes: `allowed_scope_for_section(...) -> Result<Option<HashSet<String>>>`
- Produces: `identifiers_allowed_for_scope(...)` and list filters where an empty set allows every Presave.

- [ ] **Step 1: Add failing integration tests**

Add tests that create a non-admin User with null Sender/Product/Study scopes and assert Sender, Product, and Study Presave list endpoints return the seeded records. Add configured UUID scope assertions for matching and nonmatching Presaves.

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p web-server --test api scope_visibility_web -- --nocapture
```

Expected: the empty-scope Presave assertions fail because the filters currently return no records.

- [ ] **Step 3: Implement the common contract**

Change Presave scope identifiers to include each Presave UUID and retain legacy business identifiers during compatibility. Make `allowed.is_empty()` return allow, and make list filters return every entity when allowed is empty.

- [ ] **Step 4: Verify GREEN**

Run the command from Step 2. Expected: all scope visibility tests pass.

### Task 2: Make Gateway routing independent of Case visibility

**Files:**
- Modify: `crates/libs/lib-rest-core/src/lib.rs`
- Test: `crates/services/web-server/tests/api/scope_visibility_web.rs`

**Interfaces:**
- Produces: `case_matches_user_scope` that checks access dates, UUID/legacy scopes, and blind access, but not `active_sender_identifier`.

- [ ] **Step 1: Add a failing regression test**

Create a Case with a routing Sender identifier and a User whose active Gateway identifier differs. Assert the User can still read the Case when all access scopes permit it.

- [ ] **Step 2: Verify RED**

Run the focused scope test and confirm `Case.Scope` denial.

- [ ] **Step 3: Remove only the visibility call to `selected_sender_matches`**

Keep routing-profile and submission code unchanged.

- [ ] **Step 4: Verify GREEN**

Run the focused test and full scope visibility module.

### Task 3: Validate UUID scope writes and Presave application

**Files:**
- Modify: `crates/libs/lib-core/src/model/user.rs`
- Modify: `crates/services/web-server/src/web/rest/case_editor_rest/dg.rs`
- Modify: the existing C.3 and C.5 editor REST modules located under `crates/services/web-server/src/web/rest/case_editor_rest/`
- Test: `crates/services/web-server/tests/api/case_contract_web.rs`
- Test: `crates/services/web-server/tests/api/scope_visibility_web.rs`

**Interfaces:**
- Produces: new User scope writes serialized as UUID strings; editor create/patch rejects malformed, deleted, cross-tenant, or out-of-scope Presave IDs before persistence.

- [ ] **Step 1: Add failing API tests**

Test Product Template application to a selected G.k row and equivalent Sender/Study source UUID persistence. Add malformed UUID, deleted Presave, cross-organization Presave, and configured-scope mismatch cases that expect 400/403 without modifying the row.

- [ ] **Step 2: Verify RED**

Run the exact new tests with `cargo test -p web-server --test api <test_name> -- --nocapture` and confirm missing validation/source persistence causes each failure.

- [ ] **Step 3: Implement preflight resolvers**

Before editor persistence, parse the source UUID, load the Presave through tenant-scoped BMC access, reject `deleted = true`, and invoke the matching Presave scope check. Preserve the existing snapshot-copy behavior and source UUID in the same request.

- [ ] **Step 4: Verify GREEN**

Run all new editor tests and the complete `case_contract_web` and `scope_visibility_web` modules.

### Task 4: Add Product Presave UUID to XML Import contract

**Files:**
- Modify: `crates/services/web-server/src/web/rest/import_rest.rs`
- Modify: `crates/libs/lib-core/src/xml/import.rs`
- Modify: `crates/libs/lib-core/src/xml/import_runtime/g.rs`
- Modify: `crates/libs/lib-core/src/xml/import_runtime/c.rs`
- Test: `crates/services/web-server/tests/api/case_intake_web.rs`

**Interfaces:**
- Produces: multipart `productPresaveId`; `XmlImportRequest.product_presave_id: Option<Uuid>` plus resolved Product ID and optional resolved Sender snapshot input.

- [ ] **Step 1: Add failing Import tests**

Cover authorized Product selection, first-G.k-only source linkage, `cases.dg_prd_key` business Product ID, no-G.k rollback, deleted/cross-tenant/out-of-scope rejection, and ZIP behavior applying the selected Product independently to each entry.

- [ ] **Step 2: Verify RED**

Run the focused Import tests and confirm the current multipart reader ignores `productPresaveId` and imported source UUIDs remain null.

- [ ] **Step 3: Resolve Product before Case creation**

Parse `productPresaveId` as UUID, load it in the current organization, require `deleted = false`, and enforce Product scope. Pass the resolved UUID and business `product_id` into `XmlImportRequest`; do not reinterpret the existing display `productId` as UUID.

- [ ] **Step 4: Link the first G.k and set Case Product ID**

Extend `import_section_g` to accept the optional Product UUID, reject selected-Product imports with no G.k, and assign the UUID only when creating the first imported row. Set `CaseForCreate.dg_prd_key` and final `CaseForUpdate.dg_prd_key` to the resolved business Product ID.

- [ ] **Step 5: Verify GREEN**

Run focused tests, then the complete `case_intake_web` module.

### Task 5: Apply Product Sender during XML Import

**Files:**
- Modify: `crates/services/web-server/src/web/rest/import_rest.rs`
- Modify: `crates/libs/lib-core/src/xml/import.rs`
- Modify: `crates/libs/lib-core/src/xml/import_runtime/c.rs`
- Test: `crates/services/web-server/tests/api/case_intake_web.rs`

**Interfaces:**
- Consumes: resolved Product and `CImportSettings.apply_sender_info_to_imported_cases`.
- Produces: setting OFF preserves XML C.3 with null source UUID; setting ON replaces C.3 snapshot with Product Sender and stores `source_sender_presave_id`.

- [ ] **Step 1: Add two failing tests**

Assert OFF retains XML Sender and null source UUID. Assert ON applies the selected Product's active same-tenant Sender snapshot and UUID. Add an invalid Sender linkage test that leaves no partial Case.

- [ ] **Step 2: Verify RED**

Run each focused test and confirm setting ON currently does not apply the Presave Sender.

- [ ] **Step 3: Implement Sender resolution and transactional application**

Resolve the Product's Sender only when the setting is ON, reject missing/deleted/cross-tenant linkage, and pass the Sender Presave into C.3 import so snapshot fields and `source_sender_presave_id` are written atomically.

- [ ] **Step 4: Verify GREEN**

Run the focused tests and full Import module.

### Task 6: Full production verification

**Files:**
- Test only; do not modify production code unless a test exposes a root cause and a new RED/GREEN cycle is started.

**Interfaces:**
- Produces: fresh evidence for the approved integration checklist.

- [ ] **Step 1: Run formatting and compile checks**

```bash
cargo fmt --check
cargo check -p web-server
```

- [ ] **Step 2: Run focused integration suites**

```bash
cargo test -p web-server --test api scope_visibility_web -- --nocapture
cargo test -p web-server --test api case_contract_web -- --nocapture
cargo test -p web-server --test api case_intake_web -- --nocapture
```

- [ ] **Step 3: Run the full web-server API integration target**

```bash
cargo test -p web-server --test api -- --nocapture
```

- [ ] **Step 4: Audit the diff**

Use `git diff --check` and `git diff --stat`; verify no unrelated pre-existing files were overwritten and map every approved requirement to a passing test.
