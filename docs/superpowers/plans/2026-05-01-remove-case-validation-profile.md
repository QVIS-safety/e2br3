# Remove Case Validation Profile Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the legacy `cases.validation_profile` storage/API path so case validation selection is driven only by `appendices_json`.

**Architecture:** Keep XML import/export history profile fields for historical artifact format metadata, but remove the case-level validation profile column, model fields, public case DTO fields, bootstrap SQL, seed SQL, and tests that use case `validation_profile`. Case creation/import should write `appendices_json` directly.

**Tech Stack:** Rust, sqlx, PostgreSQL bootstrap SQL, Jest/TypeScript frontend endpoint tests.

---

### Task 1: Lock Case Schema Contract

**Files:**
- Test: `crates/services/web-server/tests/api/case_contract_web.rs`
- Test: `__tests__/api.endpoints.test.ts`

- [ ] **Step 1: Write failing tests**

Add/update tests proving public case responses and frontend transforms do not expose or send case `validation_profile`.

- [ ] **Step 2: Run focused tests and verify RED**

Run:
```bash
cargo test -p web-server test_public_case_create_derives_profile_from_appendices -- --nocapture
npm test -- __tests__/api.endpoints.test.ts --runInBand
```

Expected: fail while backend still exposes case `validation_profile` or accepts it as case behavior input.

### Task 2: Remove Case-Level Storage

**Files:**
- Modify: `db/bootstrap/01-safetydb-schema.sql`
- Modify: `db/seed/002-rich-demo-case.sql`
- Modify: `crates/libs/lib-core/src/model/case.rs`

- [ ] **Step 1: Remove bootstrap case column**

Remove `cases.validation_profile`, its check constraint, and index. Keep `xml_import_history.validation_profile` and `xml_export_history.validation_profile`.

- [ ] **Step 2: Remove seed use**

Remove `validation_profile` from the rich demo case insert and snapshot JSON.

- [ ] **Step 3: Remove model fields**

Remove `validation_profile` from `Case`, `CaseForCreate`, `CaseForUpdate`, and `update_touches_non_status_fields`.

### Task 3: Update Backend REST/XML Callers

**Files:**
- Modify: `crates/services/web-server/src/web/rest/case_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/case_intake_rest.rs`
- Modify: `crates/libs/lib-core/src/xml/import.rs`
- Modify tests under `crates/services/web-server/tests` and `crates/libs/lib-core/tests`

- [ ] **Step 1: Remove public case DTO field**

Remove `validation_profile` from public create/update case DTOs and validation payload checks.

- [ ] **Step 2: Derive appendices on import/intake**

When old import input has a requested profile, map it to `appendices_json: ["profile"]`; otherwise infer from header and write appendices JSON.

- [ ] **Step 3: Update tests/fixtures**

Replace case create/update test payloads using `validation_profile` with `appendices_json`.

### Task 4: Verify

**Files:**
- All modified files

- [ ] **Step 1: Format and compile**

Run:
```bash
cargo fmt --check
cargo check -p web-server
npx tsc --noEmit --pretty false
```

- [ ] **Step 2: Run focused suites**

Run:
```bash
cargo test -p web-server test_public_case_create_derives_profile_from_appendices -- --nocapture
cargo test -p web-server submission::tests::selected_submission_authorities -- --nocapture
npm test -- __tests__/api.endpoints.test.ts --runInBand
```

- [ ] **Step 3: Check diffs**

Run:
```bash
git diff --check
rg "validation_profile" db/bootstrap/01-safetydb-schema.sql db/seed/002-rich-demo-case.sql crates/libs/lib-core/src/model/case.rs crates/services/web-server/src/web/rest/case_rest.rs
```

Expected: no case-level `validation_profile` remains.
