# Presave Lifecycle Delete Guard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Route every presave archive and physical delete through one atomic lifecycle service, protect every new UUID reference with database triggers, and enforce UUID-only user presave scopes.

**Architecture:** A new `lib-core` lifecycle service owns dependency checks and transaction-local mutations for Sender, Receiver, Product, Study, Reporter, and Narrative presaves. PostgreSQL triggers reject references to archived presaves and synchronize reference creation with lifecycle row locks. REST remains responsible for permissions and visible scope, but both PATCH `deleted=true` and DELETE invoke the same lifecycle command.

**Tech Stack:** Rust, Axum, SQLx, PostgreSQL RLS/triggers, modql, serial_test.

## Global Constraints

- Preserve all current endpoint paths and successful response shapes.
- Return HTTP 409 for used, assigned, or inactive-reference conflicts.
- Treat existing display-name scope entries as immediately ineffective; do not migrate or delete them.
- Accept only UUID strings in new `access_sender_ids`, `access_product_ids`, and `access_study_ids` writes.
- Do not call public BMC or `base_uuid` mutation methods from lifecycle transaction-local mutation helpers.
- Do not normalize user scope columns into join tables in this change.
- Use TDD for every behavior change and commit each task independently.

---

## File Map

- Create `crates/libs/lib-core/src/model/presave_lifecycle.rs`: lifecycle kinds, atomic archive/delete commands, SQL `EXISTS` policies, and private raw mutations.
- Modify `crates/libs/lib-core/src/model/mod.rs`: export the lifecycle module.
- Modify `crates/libs/lib-core/src/model/presave.rs`: reject direct `deleted=true`, delegate public hard deletes, and remove duplicated relationship scans.
- Modify `crates/libs/lib-core/src/model/error.rs`: resolve SQLSTATE `P2001` to `Error::Conflict`.
- Modify `db/bootstrap/10-triggers.sql`: active-presave reference trigger function and triggers.
- Create `db/migrations/20260714_presave_lifecycle_guards.sql`: idempotent production trigger migration.
- Modify `crates/libs/lib-core/tests/section_presave.rs`: lifecycle, trigger, physical-delete, legacy Receiver, and concurrency coverage.
- Modify `crates/services/web-server/src/web/rest/section_presave_rest/{shared,sender,receiver,product,study}.rs`: route archive requests into lifecycle and remove REST-owned dependency SQL.
- Modify `crates/services/web-server/src/web/rest/section_presave_rest/{reporter,narrative}.rs`: use the lifecycle-aware common handlers.
- Modify `crates/services/web-server/tests/api/presave/delete_constraints_web.rs`: DELETE/PATCH parity and 409 contracts.
- Modify `crates/services/web-server/src/web/rest/user_rest/validation.rs`: UUID-only scope input validation.
- Modify `crates/services/web-server/tests/authz/rbac_users/create_validation_web.rs`: reject non-UUID scope values.
- Modify `crates/services/web-server/tests/api/scope_visibility_web.rs`: replace legacy display-name grants with UUID grants and assert display names are ineffective.

---

### Task 1: UUID-only Presave Scope Contract

**Files:**
- Modify: `crates/services/web-server/src/web/rest/user_rest/validation.rs`
- Modify: `crates/services/web-server/src/web/rest/section_presave_rest/shared.rs`
- Modify: `crates/libs/lib-core/src/model/presave.rs`
- Test: `crates/services/web-server/tests/authz/rbac_users/create_validation_web.rs`
- Test: `crates/services/web-server/tests/api/scope_visibility_web.rs`

**Interfaces:**
- Consumes: `ScopeListInput` values accepted by user create/update handlers.
- Produces: `validate_presave_scope_uuids(field, input) -> Result<()>` and UUID-only matching shared by visibility and lifecycle work.

- [ ] **Step 1: Add failing validation tests**

Add user create and update cases that submit display names and expect HTTP 400:

```rust
json!({
    "access_sender_ids": ["Sender Org A"],
    "access_product_ids": ["Brand Alpha"],
    "access_study_ids": ["STUDY-ALPHA"]
})
```

Add a positive case using three `Uuid::new_v4().to_string()` values.

- [ ] **Step 2: Run the validation tests and verify RED**

Run:

```bash
cargo test -p web-server --test authz rbac_users::create_validation_web -- --nocapture
```

Expected: non-UUID values are currently accepted, so the new 400 assertions fail.

- [ ] **Step 3: Implement UUID-only input validation**

Add a helper that consumes the existing two `ScopeListInput` variants through `parse_scope_input`, then parses every value as `Uuid`:

```rust
fn validate_uuid_scope(field: &str, input: Option<ScopeListInput>) -> Result<()> {
    for value in parse_scope_input(input).unwrap_or_default() {
        Uuid::parse_str(value.trim()).map_err(|_| Error::BadRequest {
            message: format!("{field} accepts UUID values only"),
        })?;
    }
    Ok(())
}
```

Invoke it for sender, product, and study scopes in both create and update validation paths.

- [ ] **Step 4: Remove display-name visibility matching**

Change `sender_scope_identifiers`, `product_scope_identifiers`, and `study_scope_identifiers` to return only `entity.id.to_string()`. Replace `any_user_scope_contains` with a SQL `EXISTS` helper that compares the canonical lowercase UUID string and active users only.

- [ ] **Step 5: Update visibility fixtures and verify legacy values are ignored**

For tests that intend to grant access, create the presave first and store its returned UUID in the user scope. Retain one explicit display-name test and assert that it grants no visibility. Do not transform existing database values.

- [ ] **Step 6: Run GREEN verification**

Run:

```bash
cargo test -p web-server --test authz rbac_users::create_validation_web -- --nocapture
cargo test -p web-server --test api scope_visibility -- --nocapture
```

Expected: UUID writes pass, display-name writes return 400, and stored legacy names match no presave.

- [ ] **Step 7: Commit**

```bash
git add crates/services/web-server/src/web/rest/user_rest/validation.rs \
  crates/services/web-server/src/web/rest/section_presave_rest/shared.rs \
  crates/libs/lib-core/src/model/presave.rs \
  crates/services/web-server/tests/authz/rbac_users/create_validation_web.rs \
  crates/services/web-server/tests/api/scope_visibility_web.rs
git commit -m "refactor: enforce UUID-only presave scopes"
```

---

### Task 2: Database Active-Presave Reference Boundary

**Files:**
- Modify: `db/bootstrap/10-triggers.sql`
- Create: `db/migrations/20260714_presave_lifecycle_guards.sql`
- Modify: `crates/libs/lib-core/src/model/error.rs`
- Test: `crates/libs/lib-core/tests/section_presave.rs`

**Interfaces:**
- Produces: PostgreSQL trigger function `require_active_presave_reference()` and SQLSTATE `P2001`.
- Produces: `Error::resolve_inactive_presave_reference()` mapping `P2001` to `Error::Conflict`.

- [ ] **Step 1: Add failing trigger matrix tests**

Create an archived row for every parent kind and attempt inserts/updates for all protected columns:

```text
sender_information.source_sender_presave_id
drug_information.source_product_presave_id
study_information.source_study_presave_id
primary_sources.source_reporter_presave_id
narrative_information.source_narrative_presave_id
product_presaves.sender_presave_id
product_presaves.receiver_presave_id
study_presaves.product_presave_id
study_presave_products.product_presave_id
study_presave_reporters.reporter_presave_id
```

Assert each failure exposes PostgreSQL code `P2001`. Also verify null references and active references succeed.

In the same test module, add the two-connection race test before the trigger implementation. Use two independent `ModelManager::new_with_txn()` values and synchronization channels to cover both orderings: archive row lock first and reference key-share lock first.

- [ ] **Step 2: Run trigger tests and verify RED**

Run:

```bash
cargo test -p lib-core --test section_presave inactive_presave_reference -- --ignored --nocapture
```

Expected: inserts currently succeed or fail without `P2001` because triggers do not exist, and the race test can commit an invalid archived-and-referenced state.

- [ ] **Step 3: Implement the trigger function and idempotent migration**

Define one trigger function receiving target table and organization expression through `TG_ARGV`. For non-null UUID values, execute a typed query equivalent to:

```sql
SELECT id
FROM <presave_table>
WHERE id = <new_reference>
  AND organization_id = <owning_organization>
  AND deleted = false
FOR KEY SHARE;
```

If no row is returned:

```sql
RAISE EXCEPTION USING
    ERRCODE = 'P2001',
    MESSAGE = 'inactive presave reference',
    DETAIL = format('%s:%s', TG_ARGV[0], referenced_id);
```

Install `BEFORE INSERT OR UPDATE OF <column>` triggers for all ten columns. Use `DROP TRIGGER IF EXISTS` before creation in the migration; mirror the definitions in bootstrap SQL.

- [ ] **Step 4: Add explicit model error resolution**

Implement:

```rust
pub fn resolve_inactive_presave_reference(self) -> Self {
    match self.as_database_error().and_then(|error| error.code()) {
        Some(code) if code == "P2001" => Error::Conflict {
            message: "inactive presave reference".to_string(),
        },
        _ => self,
    }
}
```

Call the resolver at model boundaries that create or update the protected references.

- [ ] **Step 5: Apply migration and verify GREEN**

Run:

```bash
psql "$SERVICE_DB_URL" -v ON_ERROR_STOP=1 \
  -f db/migrations/20260714_presave_lifecycle_guards.sql
cargo test -p lib-core --test section_presave inactive_presave_reference -- --ignored --nocapture
```

Expected: all ten archived-target writes fail with `P2001`; active and null references pass.

- [ ] **Step 6: Commit**

```bash
git add db/bootstrap/10-triggers.sql \
  db/migrations/20260714_presave_lifecycle_guards.sql \
  crates/libs/lib-core/src/model/error.rs \
  crates/libs/lib-core/tests/section_presave.rs
git commit -m "feat: guard inactive presave references in database"
```

---

### Task 3: Atomic Presave Lifecycle Service

**Files:**
- Create: `crates/libs/lib-core/src/model/presave_lifecycle.rs`
- Modify: `crates/libs/lib-core/src/model/mod.rs`
- Modify: `crates/libs/lib-core/src/model/presave.rs`
- Test: `crates/libs/lib-core/tests/section_presave.rs`

**Interfaces:**
- Produces: `PresaveKind`.
- Produces: `PresaveLifecycleService::{archive, hard_delete}`.
- Produces private transaction-local `archive_row_in_current_txn` and `delete_row_in_current_txn`.

- [ ] **Step 1: Add failing lifecycle policy tests**

Build a table-driven test over all six kinds. Assert that unreferenced rows archive successfully and that these references return `Error::Conflict`:

```text
Sender -> Case, Product, active user UUID scope
Receiver -> Product UUID, Product legacy name only when receiver_presave_id IS NULL
Product -> Case, Study parent, Study child, active user UUID scope
Study -> Case, active user UUID scope
Reporter -> Case, Study reporter child
Narrative -> Case
```

Add direct BMC `delete()` tests for the same dependencies.

- [ ] **Step 2: Run lifecycle tests and verify RED**

Run:

```bash
cargo test -p lib-core --test section_presave presave_lifecycle -- --ignored --nocapture
```

Expected: no lifecycle API exists and current BMC behavior is inconsistent.

- [ ] **Step 3: Implement `PresaveKind` and dependency SQL**

In `presave_lifecycle.rs`, map every kind to its table and stable conflict messages. Use one `SELECT EXISTS` query per dependency category; do not call list BMCs. Receiver legacy matching must be constrained as follows:

```sql
receiver_presave_id = $1
OR (
    receiver_presave_id IS NULL
    AND lower(btrim(original_manufacturer)) = lower(btrim($2))
)
```

Only active referencing rows (`deleted=false`) block lifecycle operations.

- [ ] **Step 4: Implement one transaction around lock, guards, and mutation**

The public command must:

```rust
let tx_mm = mm.new_with_txn()?;
let dbx = tx_mm.dbx();
dbx.begin_txn().await?;
set_full_context_from_ctx_dbx(dbx, ctx).await?;
// SELECT target ... FOR UPDATE
// dependency EXISTS checks
// raw UPDATE or DELETE using dbx.execute
dbx.commit_txn().await?;
```

On every intermediate error, roll back before returning. `archive_row_in_current_txn` must set `deleted=true`, `updated_by`, and `updated_at` so audit triggers preserve the current contract. Neither raw mutation helper may call `base_uuid`, a public BMC, or lifecycle recursively.

- [ ] **Step 5: Restrict direct BMC mutation paths**

For all six parent BMCs:

- Return `Error::Validation` when ordinary `update()` receives `deleted=Some(true)`.
- Delegate public `delete()` to `PresaveLifecycleService::hard_delete`.
- Remove old in-memory `ensure_not_referenced_*` methods and UUID-only list scans.
- Preserve `deleted=Some(false)` restore behavior.

- [ ] **Step 6: Verify lifecycle GREEN**

Run:

```bash
cargo test -p lib-core --test section_presave presave_lifecycle -- --ignored --nocapture
cargo test -p lib-core --test section_presave -- --nocapture
cargo check -p lib-core
```

Expected: lifecycle and direct BMC paths enforce the same policy; the section suite has zero failures.

- [ ] **Step 7: Commit**

```bash
git add crates/libs/lib-core/src/model/presave_lifecycle.rs \
  crates/libs/lib-core/src/model/mod.rs \
  crates/libs/lib-core/src/model/presave.rs \
  crates/libs/lib-core/tests/section_presave.rs
git commit -m "refactor: centralize presave deletion lifecycle"
```

---

### Task 4: REST Delete and Archive Parity

**Files:**
- Modify: `crates/services/web-server/src/web/rest/section_presave_rest/shared.rs`
- Modify: `crates/services/web-server/src/web/rest/section_presave_rest/sender.rs`
- Modify: `crates/services/web-server/src/web/rest/section_presave_rest/receiver.rs`
- Modify: `crates/services/web-server/src/web/rest/section_presave_rest/product.rs`
- Modify: `crates/services/web-server/src/web/rest/section_presave_rest/study.rs`
- Modify: `crates/services/web-server/src/web/rest/section_presave_rest/reporter.rs`
- Modify: `crates/services/web-server/src/web/rest/section_presave_rest/narrative.rs`
- Test: `crates/services/web-server/tests/api/presave/delete_constraints_web.rs`

**Interfaces:**
- Consumes: `PresaveLifecycleService::archive`.
- Produces: identical guards for PATCH `deleted=true`, details PUT parent `deleted=true`, and DELETE.

- [ ] **Step 1: Add failing HTTP parity tests**

For each relevant relationship, issue both:

```http
DELETE /api/presaves/{kind}/{id}
PATCH /api/presaves/{kind}/{id}
Content-Type: application/json

{"data":{"deleted":true}}
```

Assert both return 409 with the same message. Add a payload such as `{"deleted":true,"name":"changed"}` and assert 400. Cover details PUT parent deletion for Sender, Receiver, Product, and Study.

- [ ] **Step 2: Run REST tests and verify RED**

Run:

```bash
cargo test -p web-server --test api presave::delete_constraints_web -- --nocapture
```

Expected: PATCH/DELETE behavior differs for Sender, Study, Reporter, and Narrative.

- [ ] **Step 3: Route every logical delete to lifecycle**

Update common and custom handlers so that:

- REST still checks UPDATE/DELETE permissions and entity visibility.
- A payload containing only `deleted=true` invokes `archive`.
- `deleted=true` plus any other business field returns `Error::BadRequest`.
- DELETE invokes the same `archive` method.
- Details PUT checks the parent update before applying children and invokes lifecycle when the parent is deletion-only.
- REST-owned `*_used_by_cases` SQL and duplicate user-assignment checks are removed after lifecycle coverage exists.

- [ ] **Step 4: Verify REST GREEN**

Run:

```bash
cargo test -p web-server --test api presave::delete_constraints_web -- --nocapture
cargo test -p web-server --test api presave -- --nocapture
cargo check -p web-server
```

Expected: all deletion entry points return the same status and conflict message.

- [ ] **Step 5: Commit**

```bash
git add crates/services/web-server/src/web/rest/section_presave_rest \
  crates/services/web-server/tests/api/presave/delete_constraints_web.rs
git commit -m "refactor: route presave deletion through lifecycle"
```

---

### Task 5: Concurrency Proof and Final Verification

**Files:**
- Modify: `crates/libs/lib-core/tests/section_presave.rs`

**Interfaces:**
- Consumes: lifecycle `FOR UPDATE` and trigger `FOR KEY SHARE` behavior.
- Produces: executable proof that archived-and-referenced cannot be the final committed state.

- [ ] **Step 1: Re-run the two-connection concurrency proof written in Task 2**

Confirm that the test uses two independent `ModelManager::new_with_txn()` values and synchronization channels and exercises both orderings:

1. Archive locks first; reference insert waits and then fails with `P2001`.
2. Reference trigger locks first; archive waits and then fails with `Error::Conflict`.

After each ordering, query the committed state and assert exactly one invariant:

```rust
assert!((presave.deleted && !reference_exists) || (!presave.deleted && reference_exists));
```

- [ ] **Step 2: Run the proof on the completed implementation**

Run:

```bash
cargo test -p lib-core --test section_presave presave_lifecycle_reference_race -- --ignored --nocapture
```

Expected: PASS. If it fails, stop completion, preserve the failure evidence, and return to the failing task's RED/GREEN cycle rather than changing multiple layers here.

- [ ] **Step 3: Run complete backend verification**

Run:

```bash
cargo fmt --all --check
cargo check -p lib-core -p web-server
cargo test -p lib-core --test section_presave -- --nocapture
cargo test -p web-server --test api presave -- --nocapture
cargo test -p web-server --test authz -- --nocapture
```

Expected: zero failures. Document intentionally ignored tests and run the new database-dependent tests explicitly with `--ignored`.

- [ ] **Step 4: Inspect migration idempotency and working tree scope**

Run the lifecycle migration twice against the test database and require both executions to succeed. Then run:

```bash
git diff --check
git status --short
git diff --stat
```

Expected: only lifecycle, trigger, scope, tests, and approved documentation changes are present; unrelated user files remain untouched.

- [ ] **Step 5: Commit the finalized concurrency test if Task 2 did not already include its final assertions**

Only when the test file has an uncommitted assertion or synchronization correction:

```bash
git add crates/libs/lib-core/tests/section_presave.rs
git commit -m "test: prove atomic presave lifecycle guards"
```

If no source changes were required, do not create an empty commit.
