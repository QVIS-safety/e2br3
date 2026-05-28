# Authorityless Canonical Presaves Design

Date: 2026-05-28

## Goal

Canonical INFO presaves should be reusable master-data records with a union field model. A presave is not owned by ICH, FDA, MFDS, US, KR, or USKR. The frontend route authority controls which fields are visible to the user, and the backend BMCs update whichever union fields are present in the save payload.

This replaces the current persisted presave `authority` model for canonical section presaves.

## Decision

Remove persisted `authority` from canonical presave parent records:

- `sender_presaves`
- `receiver_presaves`
- `product_presaves`
- `reporter_presaves`
- `study_presaves`
- `narrative_presaves`

The section BMCs remain section-specific and keep real columns for all fields needed across ICH, FDA, and MFDS. FDA-only and MFDS-only fields are nullable columns on the same canonical record. Saving a presave does not require authority context and does not reject fields because of an authority mismatch.

## Frontend Authority Model

The INFO route authority remains a frontend routing and field-visibility concept:

- `ICH`: show common ICH fields.
- `US`: show common fields plus FDA fields.
- `KR`: show common fields plus MFDS fields.
- `USKR`: show common fields plus FDA and MFDS fields.

The frontend should stop sending presave authority query parameters and stop including `authority` in canonical presave create payloads. Existing route helpers may still normalize `ICH`, `US`, `KR`, and `USKR`, but their output is used for rendering decisions only.

## Backend API Model

Canonical presave endpoints should become authorityless:

- List endpoints return the organization-scoped presaves for the section without `?authority=...` filtering.
- Create endpoints do not require or accept `authority`.
- Detail save endpoints do not require authority context.
- Response DTOs do not include presave `authority`.

Any existing `?authority=` query behavior should be removed from canonical presave section routes. If temporary compatibility is needed during frontend rollout, the server may accept and ignore the parameter, but it must not filter or validate by it.

## BMC Behavior

Presave BMCs update fields according to the submitted payload and existing patch semantics:

- FDA columns are updated when FDA fields are submitted.
- MFDS columns are updated when MFDS fields are submitted.
- Common columns are updated when common fields are submitted.
- Missing fields keep the existing value unless the update type already treats them as explicit null/clear operations.

Remove presave authority-field validation from parent and child BMCs. In particular:

- Product presaves may store FDA fields and MFDS fields on the same row.
- Study presaves may store FDA fields and MFDS fields on the same row.
- Reporter presaves may store MFDS qualification fields without a presave authority.
- FDA child rows such as cross-reported IND numbers are allowed whenever the parent exists.
- MFDS child rows such as regional product items are allowed whenever the parent exists.

This does not change case validation, export validation, or submission validation. Those flows may still evaluate data against ICH/FDA/MFDS authority profiles at their own API boundaries.

## Data Migration

The migration should remove `authority` columns and related indexes/check constraints from canonical presave parent tables. Existing rows should be preserved as single canonical records with their current field values. No duplicate-merging is required in the first implementation because the current schema does not define a stable cross-authority identity for matching separate FDA and MFDS rows.

If duplicate FDA/MFDS rows exist for the same logical template, they will remain separate records after authority removal. Users or a later targeted cleanup can consolidate them using domain-specific matching rules.

## Scope

Included:

- Database schema and compatibility migration for removing presave `authority`.
- Rust model, create, update, list, and response DTO updates.
- Removal of presave authority-field validation.
- REST route updates for authorityless list/create/detail behavior.
- Frontend canonical presave API and mapper updates so authority is not sent.
- Frontend INFO route field visibility remains driven by `ICH`, `US`, `KR`, and `USKR`.
- Backend and frontend tests updated to lock the authorityless model.

Excluded:

- Case editor authority/profile behavior.
- Case validation, export, submission, and XML authority selection.
- Automatic merging of existing duplicate FDA/MFDS presave rows.
- Adding new display-scope metadata such as `visible_appendices`.

## Testing

Backend tests should prove:

- Canonical presave create no longer requires authority.
- List endpoints do not filter by authority.
- Product and Study presaves can store both FDA and MFDS fields on the same row.
- FDA and MFDS child rows can coexist under the same parent where the section supports them.
- Responses no longer expose `authority`.

Frontend tests should prove:

- Canonical presave hooks and write mappers do not send `authority`.
- `/US/.../info/...` renders FDA fields.
- `/KR/.../info/...` renders MFDS fields.
- `/USKR/.../info/...` renders both FDA and MFDS fields.
- Creating or saving from `USKR` sends the same canonical presave payload shape as other routes, with only field values changing.

## Risks

Existing tests and code may assume authority-scoped presave lists. Those tests should be rewritten around field visibility and organization/scope access instead.

Existing data may include separate FDA and MFDS templates that users expected to be distinct. The first implementation intentionally preserves both records and removes only authority semantics; it does not infer that two rows should become one.

Removing authority from presaves must not leak into case validation or export/submission logic. Those domains still need authority profiles because they produce authority-specific validation and output.
