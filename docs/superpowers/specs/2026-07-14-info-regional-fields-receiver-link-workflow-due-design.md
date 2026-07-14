# INFO Regional Fields, Receiver Link, and Workflow Due Design

## Goal

Complete the four confirmed gaps in INFO presaves and workflow settings:

1. Persist the Product-to-Receiver master relationship by UUID and prevent deletion of referenced Receivers.
2. Render and persist MFDS `C.2.r.4.KR.1` in the INFO Reporter form.
3. Render and persist FDA study regional fields in the INFO Study form.
4. Preserve an unspecified Workflow Due value as `null` instead of coercing it to zero.

The existing Sender count policy is out of scope because the backend already enforces one active Sender for pharmaceutical-company sponsor administrators and multiple active Senders for CRO sponsor administrators.

## Repositories

- Backend and database: `/Users/hyundonghoon/projects/rust/e2br3/e2br3`
- Frontend: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend`

Changes must preserve unrelated working-tree modifications in both repositories.

## 1. Product-to-Receiver UUID Relationship

### Data model

Add a nullable `receiver_presave_id UUID` column to `product_presaves` with a foreign key to `receiver_presaves(id)`. Use `ON DELETE RESTRICT`; application deletion remains soft deletion, but the database constraint also prevents accidental physical deletion while referenced.

Keep `original_manufacturer` unchanged. It is an independent ICH Product value and is not the canonical Receiver master relationship.

Expose `receiver_presave_id: Option<Uuid>` consistently in:

- `ProductPresave`
- `ProductPresaveForCreate`
- `ProductPresaveForInsert`
- `ProductPresaveForUpdate`
- the `IntoOrgScopedCreate` mapping

### Scope validation

On Product create or update, a non-null Receiver UUID must resolve to an active Receiver in the same organization. A missing, deleted, or cross-organization Receiver is rejected before writing.

The Product response returns the stored UUID so the existing frontend `receiverPresaveId` mapper can round-trip it.

### Receiver deletion

`ReceiverPresaveBmc::ensure_not_referenced_by_products` first checks active Products whose `receiver_presave_id` equals the Receiver UUID. If any exist, deletion returns a conflict.

The existing normalized comparison between Receiver organization name and Product `original_manufacturer` remains temporarily as a legacy compatibility guard. New behavior and tests must use the UUID relationship.

Both receiver soft-delete entry points—generic update with `deleted=true` and delete—continue to call the model-layer guard. No duplicate REST-only guard is added.

### Migration and backfill

Add the column through the repository's bootstrap/migration SQL conventions. Backfill an existing Product only when all of the following hold:

- `product_presaves.receiver_presave_id` is null;
- Product and Receiver have the same `organization_id`;
- normalized `product_presaves.original_manufacturer` equals normalized `receiver_presaves.organization_name`;
- exactly one active Receiver matches.

Ambiguous or unmatched rows remain null. The migration must not overwrite `original_manufacturer`.

## 2. INFO Reporter MFDS Regional Field

Extend `ReporterPresaveData`, its Zod schema, canonical read/write mappers, and backend Reporter presave model with an optional value for `C.2.r.4.KR.1` using the established backend snake-case name `qualification_kr1` and frontend name `qualificationKr1`.

The INFO Reporter form displays the field only when:

- authority is `KR`, `MFDS`, or `USKR`; and
- Qualification (`C.2.r.4`) equals `3` (Other health professional); and
- Qualification is not suppressed by `nullFlavor=UNK`.

Allowed values are `1` (Nurse) and `2` (Other), matching the MFDS dictionary and the existing CASE Reporter implementation. When the condition becomes false, clear the hidden value so stale regional data is not submitted.

The field receives the same presave audit affordance as adjacent Reporter fields. Existing non-MFDS records remain valid because the value is optional outside the display condition.

## 3. INFO Study FDA Regional Fields

Extend `StudyPresaveData`, its Zod schema, canonical read/write mappers, and backend Study presave model for:

- `fdaIndNumberOccurred` / `fda_ind_number_occurred` (`FDA.C.5.5a`)
- `fdaPreAndaNumberOccurred` / `fda_pre_anda_number_occurred` (`FDA.C.5.5b`)
- repeating cross-reported IND rows for `FDA.C.5.6.r`

The INFO Study form renders these fields only for `US` or `FDA` authority. The combined `USKR` editing context renders both FDA and MFDS regional groups, consistent with its existing use as a combined authority.

Cross-reported IND values use a child table with stable UUID identity, sequence number, soft deletion, audit metadata, and the same create/update/delete detail-graph pattern as other repeating presave rows. Existing rows marked for deletion remain visible with cancelled/struck-through presentation until saved.

Use the existing CASE Study labels and maximum-length rules as the frontend reference. Regional requirement enforcement remains with the case validation profile; INFO presaves accept partial templates and enforce only allowed shape and field length.

## 4. Workflow Due Nullable Semantics

Change the frontend workflow status type to `dueDays?: number | null`. The Due input displays an empty string for null/undefined and maps an empty input to `null`; non-empty input maps to an integer. Before submission, frontend validation rejects non-integer or negative values with a field-level error. It does not clamp invalid values to zero.

Saving settings must preserve `null` instead of using `status.dueDays || 0`. Newly added statuses start with `dueDays: null`. Existing explicit zero remains zero.

The backend already accepts `Option<i32>`. Validation rejects negative non-null values and accepts null. Default built-in statuses may continue to use `Some(0)`; the feature concerns user-configured blank values.

Runtime workflow calculations must treat null as “no configured due interval,” not as an implicit zero-day deadline. Any existing `unwrap_or(0)` used only for negativity validation is acceptable; deadline creation must distinguish null from zero.

## Error Handling

- Referencing an absent, deleted, or foreign-organization Receiver returns a validation/conflict response without persisting the Product change.
- Deleting a referenced Receiver returns `409 Conflict` with a receiver/product relationship message.
- Reporter and Study regional values rejected by schema validation remain attached to their exact form field.
- Workflow Due rejects negative and non-integer values; blank is valid and serialized as JSON null.

## Testing Strategy

### Backend

- Database/model test proving Product create and read round-trip `receiver_presave_id`.
- Tests rejecting deleted and cross-organization Receiver references.
- Test proving both Receiver soft-delete routes reject an active UUID-linked Product.
- Migration/backfill test or deterministic SQL fixture for unique, ambiguous, and unmatched names.
- Reporter presave CRUD/detail test for `qualification_kr1`.
- Study presave detail-graph tests for FDA scalar fields and cross-reported IND create/update/soft-delete.
- Admin settings API test accepting and returning `due_days: null`, while retaining negative-value rejection.
- Existing Sender Company/CRO tests remain green without production changes.

### Frontend

- Product form/mapping test proving selected Receiver UUID is sent and restored.
- Reporter form tests for MFDS authority plus Qualification `3`, hidden states, allowed values, and stale-value clearing.
- Study form tests for FDA-only, MFDS-only, and combined `USKR` visibility and cross-reported row soft deletion.
- Workflow settings tests proving blank remains null, zero remains zero, and negative or non-integer values are rejected before submission.

### Verification

Run targeted backend and frontend suites first, then the full relevant package suites. Verify database bootstrap from an empty database and migration behavior against a fixture containing pre-existing Product/Receiver names.

## Non-goals

- Removing the legacy `original_manufacturer` deletion guard.
- Changing CASE form regional fields or regional validation rules.
- Adding new Sender-count behavior.
- Refactoring unrelated presave APIs or admin settings UI.
