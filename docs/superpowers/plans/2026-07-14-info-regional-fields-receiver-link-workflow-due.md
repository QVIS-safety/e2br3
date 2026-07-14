# INFO Regional Fields, Receiver Link, and Workflow Due Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist Product-to-Receiver UUID links, expose missing INFO regional fields, and preserve blank Workflow Due values.

**Architecture:** Store relationships and regional presave values in PostgreSQL through existing `lib-core` models and Axum presave detail graphs. Bind the Next.js canonical mappers and React Hook Form components to those contracts, retaining soft deletion, organization scope, audit metadata, and null semantics.

**Tech Stack:** PostgreSQL, Rust/Axum/SQLx/modql, Next.js 15, React 19, TypeScript, React Hook Form, Zod, Jest.

## Global Constraints

- Backend: `/Users/hyundonghoon/projects/rust/e2br3/e2br3`.
- Frontend: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend`.
- Preserve unrelated worktree changes and commit backend/frontend separately.
- Keep `original_manufacturer`, CASE regional fields, and Sender production behavior unchanged.
- Every task starts with a failing focused test and ends with focused verification.

---

### Task 1: Product-to-Receiver database and model relationship

**Files:**
- Modify: `db/bootstrap/01-safetydb-schema.sql`
- Modify: `crates/libs/lib-core/src/model/presave.rs`
- Modify: `crates/libs/lib-core/tests/section_presave.rs`

**Interfaces:**
- Produces: `ProductPresave.receiver_presave_id: Option<Uuid>` in read/create/insert/update structs.
- Produces: Product create/update validation for an active same-organization Receiver.
- Produces: UUID-first Receiver deletion guard with legacy manufacturer-name fallback.

- [ ] **Step 1: Write failing tests**

Add `product_presave_round_trips_receiver_presave_id`, `product_presave_rejects_deleted_receiver_reference`, `product_presave_rejects_foreign_receiver_reference`, and `receiver_soft_delete_rejects_uuid_linked_product`. The round-trip assertion is:

```rust
input.receiver_presave_id = Some(receiver_id);
let product_id = ProductPresaveBmc::create(&ctx, &mm, input).await?;
assert_eq!(
    ProductPresaveBmc::get(&ctx, &mm, product_id).await?.receiver_presave_id,
    Some(receiver_id),
);
```

Test both `ReceiverPresaveBmc::update(... deleted: Some(true))` and `ReceiverPresaveBmc::delete` return a conflict for an active linked Product.

- [ ] **Step 2: Prove the test fails**

Run `cargo test -p lib-core product_presave_round_trips_receiver_presave_id -- --nocapture`.

Expected: compile failure because Product structs lack `receiver_presave_id`.

- [ ] **Step 3: Add schema and idempotent upgrade SQL**

Add a nullable `receiver_presave_id UUID`, an index, and a composite same-org FK:

```sql
CONSTRAINT product_presaves_receiver_org_fk
  FOREIGN KEY (receiver_presave_id, organization_id)
  REFERENCES receiver_presaves(id, organization_id)
  ON DELETE RESTRICT
```

Ensure `receiver_presaves` has `UNIQUE (id, organization_id)`. Add `ALTER TABLE ... ADD COLUMN IF NOT EXISTS`, an idempotent constraint `DO` block, and a backfill CTE that updates only same-org, active, exactly-one normalized matches between `original_manufacturer` and `organization_name`. Ambiguous/unmatched rows stay null.

- [ ] **Step 4: Implement model validation and guard**

Copy `receiver_presave_id` through all Product structs. Before create/update with a supplied UUID, resolve the Receiver under the current context and reject missing, deleted, or foreign rows with `product requires an active receiver presave in the same organization`.

Change the Receiver reference predicate to:

```rust
row.receiver_presave_id == Some(id) || legacy_original_manufacturer_match
```

- [ ] **Step 5: Verify and commit**

Run `cargo test -p lib-core --test section_presave -- --nocapture` and `cargo fmt --check`.

Commit only the three listed files as `feat: persist product receiver presave links`.

---

### Task 2: Receiver REST and frontend canonical round-trip

**Files:**
- Modify: `crates/services/web-server/tests/api/presave/product_web.rs`
- Modify: `lib/presave/canonicalMappers.ts`
- Modify: `lib/presave/canonicalWriteMappers.ts`
- Modify: `__tests__/dashboard/product-form-alignment.test.tsx`
- Modify: `__tests__/dashboard/info-presave-detail-route.test.tsx`

**Interfaces:**
- Consumes: Task 1 REST field `receiver_presave_id`.
- Produces: `receiverPresaveId` read/write round-trip without deriving it from manufacturer text.

- [ ] **Step 1: Add failing API and mapper assertions**

POST/GET a Product with `receiver_presave_id` and assert the response UUID; then assert both Receiver delete routes return HTTP 409. In Jest assert:

```ts
expect(mapProductPresave({ receiver_presave_id: "receiver-1" }))
  .toMatchObject({ receiverPresaveId: "receiver-1" });
expect(writePayload).toMatchObject({ receiver_presave_id: "receiver-1" });
```

- [ ] **Step 2: Run tests and observe the mapper failure**

Backend: `cargo test -p web-server --test api product_receiver_link_round_trip_and_delete_guard -- --nocapture`.

Frontend: `npm test -- --runInBand __tests__/dashboard/product-form-alignment.test.tsx`.

- [ ] **Step 3: Implement tolerant canonical read mapping**

```ts
receiverPresaveId: stringOf(source, "receiverPresaveId", "receiver_presave_id"),
```

Keep the existing canonical write call `setIfPresent(parent, "receiver_presave_id", data.receiverPresaveId)`.

- [ ] **Step 4: Verify and commit per repository**

Run the two focused frontend tests plus the API test. Commit backend test as `test: cover product receiver REST relationship`; commit frontend mapper/tests as `feat: round trip product receiver master links`.

---

### Task 3: MFDS C.2.r.4.KR.1 in INFO Reporter

**Files:**
- Modify: `lib/types/presave.ts`
- Modify: `lib/schemas/presave.ts`
- Modify: `lib/presave/canonicalMappers.ts`
- Modify: `lib/presave/canonicalWriteMappers.ts`
- Modify: `components/presave/ReporterForm.tsx`
- Modify: `__tests__/dashboard/presave-minimal-form-validation.test.ts`
- Modify: `__tests__/dashboard/presave-field-audit-buttons.test.tsx`

**Interfaces:**
- Consumes: existing backend field `qualification_kr1`.
- Produces: `ReporterPresaveData.qualificationKr1?: "1" | "2"`.

- [ ] **Step 1: Add failing form tests**

Verify the control appears for KR/MFDS/USKR only when Qualification is `3` and no qualification nullFlavor is set. Verify values `1` and `2`, stale-value clearing when Qualification changes, hidden behavior for ICH, submission mapping, and audit field `qualification_kr1`.

- [ ] **Step 2: Run and observe failure**

Run `npm test -- --runInBand __tests__/dashboard/presave-minimal-form-validation.test.ts __tests__/dashboard/presave-field-audit-buttons.test.tsx`.

- [ ] **Step 3: Add contract and mapping**

```ts
qualificationKr1?: "1" | "2";
qualificationKr1: z.enum(["1", "2"]).optional().or(z.literal(""));
qualificationKr1: stringOf(source, "qualificationKr1", "qualification_kr1");
setIfPresent(parent, "qualification_kr1", data.qualificationKr1);
```

Schema refinement rejects a populated KR.1 value unless Qualification is `3` and its nullFlavor is empty.

- [ ] **Step 4: Implement conditional UI**

Use `showQualificationKr1 = ["kr", "mfds", "uskr"].includes(authority) && qualification === "3" && !qualificationNullFlavor`. Clear the field in `useEffect` when false. Render `1: Nurse` and `2: Other` under `Other Health Professional Type (C.2.r.4.KR.1)`.

- [ ] **Step 5: Verify and commit**

Run both Jest files and `npx tsc --noEmit`. Commit as `feat: add MFDS reporter qualification subtype`.

---

### Task 4: FDA Study regional backend contract

**Files:**
- Modify: `db/bootstrap/01-safetydb-schema.sql`
- Modify: `db/bootstrap/10-triggers.sql`
- Modify: `crates/libs/lib-core/src/model/presave.rs`
- Modify: `crates/services/web-server/src/web/rest/section_presave_rest/shared.rs`
- Modify: `crates/services/web-server/src/web/rest/section_presave_rest/study.rs`
- Modify: `crates/services/web-server/src/web/rest/routes/presaves.rs`
- Modify: `crates/libs/lib-core/tests/section_presave.rs`
- Modify: `crates/services/web-server/tests/api/presave/study_web.rs`

**Interfaces:**
- Produces: Study scalar fields `fda_ind_number_occurred` and `fda_pre_anda_number_occurred`.
- Produces: `StudyPresaveFdaCrossReportedInd` child resource and `fda_cross_reported_inds` detail collection.

- [ ] **Step 1: Add failing model/detail tests**

Create a Study with scalar values `123456` and `234567`; create cross-reported IND `345678`; assert GET returns all values. Send `_delete: true` for the child and assert the row remains returned with `deleted=true`.

- [ ] **Step 2: Prove failures**

Run `cargo test -p lib-core study_presave_fda_fields_round_trip -- --nocapture` and `cargo test -p web-server --test api study_presave_fda_detail_graph_round_trip -- --nocapture`.

- [ ] **Step 3: Add schema**

Add the two nullable `VARCHAR(10)` Study columns and idempotent ALTER statements. Add `study_presave_fda_cross_reported_inds` with UUID id, parent UUID, sequence, required `ind_number VARCHAR(10)`, soft-delete/audit timestamps and users, unique parent/sequence constraint, RLS policies, update timestamp trigger, and audit trigger.

- [ ] **Step 4: Add models and validation**

Copy both scalar options through Study read/create/insert/update. Add child read/create/update/BMC structs following `StudyPresaveRegistrationNumberBmc`. Reject scalar or child values over 10 characters and reject an empty child `ind_number`.

- [ ] **Step 5: Extend detail graph and routes**

Add `fda_cross_reported_inds` to load, permission preflight, transactional apply, and response. Add scoped CRUD routes at `/api/presaves/studies/{study_id}/fda-cross-reported-inds[/{id}]`, using soft deletion and parent-scope validation.

- [ ] **Step 6: Verify and commit**

Run `cargo fmt --check`, the two focused tests, `cargo test -p lib-core --test section_presave -- --nocapture`, and `cargo test -p web-server --test api presave::study_web -- --nocapture`. Commit as `feat: add FDA fields to study presaves`.

---

### Task 5: FDA Study regional INFO UI

**Files:**
- Modify: `lib/types/presave.ts`
- Modify: `lib/schemas/presave.ts`
- Modify: `lib/presave/canonicalMappers.ts`
- Modify: `lib/presave/canonicalWriteMappers.ts`
- Modify: `lib/hooks/usePresaveTemplates.ts`
- Modify: `components/presave/StudyForm.tsx`
- Modify: `__tests__/dashboard/info-presave-detail-route.test.tsx`
- Modify: `__tests__/dashboard/presave-field-audit-buttons.test.tsx`

**Interfaces:**
- Consumes: Task 4 scalar and child contracts.
- Produces: `fdaIndNumberOccurred`, `fdaPreAndaNumberOccurred`, and `fdaCrossReportedInds` frontend fields.

- [ ] **Step 1: Add failing visibility and row-lifecycle tests**

Test matrix: US/FDA shows FDA only; KR/MFDS shows MFDS only; USKR shows both; ICH shows neither. Add, load, soft-delete, restore, and submit a cross-reported IND row with stable id and sequence.

- [ ] **Step 2: Run and observe failure**

Run `npm test -- --runInBand __tests__/dashboard/info-presave-detail-route.test.tsx __tests__/dashboard/presave-field-audit-buttons.test.tsx`.

- [ ] **Step 3: Add types, schema, mapping, and hook persistence**

```ts
fdaIndNumberOccurred?: string;
fdaPreAndaNumberOccurred?: string;
fdaCrossReportedInds?: Array<{
  id?: string;
  sequenceNumber: number;
  indNumber: string;
  deleted?: boolean;
  _delete?: boolean;
}>;
```

Apply `.max(10)`, map both snake/camel key forms, and extend detail loading/saving to retain child IDs and call Task 4 routes.

- [ ] **Step 4: Implement FDA UI**

Set `showFdaFields = ["us", "fda", "uskr"].includes(normalizedAuthority)`. Reuse CASE labels for FDA.C.5.5a, FDA.C.5.5b, FDA.C.5.6.r. Use `useSoftDeleteFieldArray`; audit scalars against `study_presaves` and children against `study_presave_fda_cross_reported_inds`.

- [ ] **Step 5: Verify and commit**

Run both Jest files and `npx tsc --noEmit`. Commit as `feat: add FDA regional study presave fields`.

---

### Task 6: Nullable Workflow Due

**Files:**
- Modify: `lib/types/api.ts`
- Modify: `app/(protected)/admin/settings/model/adminSettingsModel.ts`
- Modify: `app/(protected)/admin/settings/hooks/useAdminSettings.ts`
- Modify: `app/(protected)/admin/settings/components/WorkflowStatusesSection.tsx`
- Modify: `__tests__/admin-users.header-filters.test.ts`
- Modify: `crates/services/web-server/tests/api/case_validation_web.rs`

**Interfaces:**
- Produces: `WorkflowStatusConfig.dueDays?: number | null`.
- Consumes: backend `due_days: Option<i32>` without changing null to zero.

- [ ] **Step 1: Add failing tests**

Frontend: blank input submits `dueDays: null`; explicit zero remains zero; `-1` and `1.5` block API submission with a field error. Backend: POST/GET a workflow status with `due_days: null` and assert null round-trip.

- [ ] **Step 2: Run and observe frontend coercion failure**

Run `npm test -- --runInBand __tests__/admin-users.header-filters.test.ts` and `cargo test -p web-server --test api workflow_settings_preserves_null_due_days -- --nocapture`.

- [ ] **Step 3: Implement null-preserving input**

```tsx
value={status.dueDays == null ? "" : String(status.dueDays)}
onChange={(event) => updateWorkflowStatus(index, {
  dueDays: event.target.value === "" ? null : Number(event.target.value),
})}
```

New statuses start at null. Before saving, reject non-null values unless `Number.isInteger(value) && value >= 0`. Serialize with `dueDays: status.dueDays == null ? null : status.dueDays`; remove `Number(status.dueDays || 0)`.

- [ ] **Step 4: Audit runtime consumers**

Run `rg -n "due_days.*unwrap_or\\(0\\)|unwrap_or\\(0\\).*due_days" crates`. Keep fallback only for negativity checks. If deadline creation uses it, first add a test proving null means no deadline and zero means a zero-day deadline, then branch on `Option`.

- [ ] **Step 5: Verify and commit per repository**

Run frontend Jest plus `npx tsc --noEmit`; commit as `fix: preserve blank workflow due values`. Run backend null and negative tests; commit the backend test/runtime change as `test: cover nullable workflow due settings`.

---

### Task 7: Cross-repository regression verification

**Files:**
- No production changes expected.

**Interfaces:**
- Consumes: Tasks 1–6.
- Produces: final verification evidence.

- [ ] **Step 1: Backend verification**

Run:

```bash
cargo fmt --check
cargo test -p lib-core --test section_presave -- --nocapture
cargo test -p web-server --test api presave -- --nocapture
cargo test -p web-server --test api test_workflow_settings -- --nocapture
```

- [ ] **Step 2: Sender regression verification**

Run both `sponsor_company_sender_presave_limited_to_one_active_record` and `sponsor_cro_sender_presave_allows_multiple_active_records`; both must pass without Sender production changes.

- [ ] **Step 3: Frontend verification**

Run the five focused Jest files from Tasks 2, 3, 5, and 6, then `npx tsc --noEmit`.

- [ ] **Step 4: Ownership check**

Run `git status --short` and `git log --oneline -8` in both repositories. Confirm no unrelated user files were staged or committed.
