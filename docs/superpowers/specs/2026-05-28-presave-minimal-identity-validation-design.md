# Presave Minimal Identity Validation Design

## Goal

Canonical presaves are reusable organization-scoped master data records. Saving a presave should require only the smallest identity needed to find and reuse the record later. Presave save validation must not enforce E2B regulatory completeness.

The backend BMC/domain layer is the source of truth. The frontend mirrors the same rules for user experience, but backend enforcement is required for every canonical write path.

## Scope

In scope:

- Parent canonical presaves: sender, receiver, product, reporter, study, narrative.
- Minimal required identity checks for create/update/details graph saves.
- Duplicate checks within the current organization.
- Frontend form/schema updates that mirror backend identity rules.
- Error mapping for missing identity and duplicates.

Out of scope:

- Regulatory/export completeness validation.
- Child-row duplicate rules, except existing parent and sequence requirements.
- Database unique indexes. These can be added later after BMC behavior is stable.
- Authority-specific presave parent identity. Presaves are no longer separated by authority.

## Architecture

### Backend

Validation belongs in the canonical presave BMC/domain layer, not REST handlers.

Each parent BMC should enforce:

- `create`: validate required identity, then check duplicates.
- `update`: load current row, merge patch into the final candidate, validate the final candidate, then check duplicates excluding the current row.
- details graph parent save: use the same BMC update path so the same rules apply.
- soft-delete: skip identity and duplicate validation when the only effective change is `deleted = true`.

REST remains responsible for request/response mapping and HTTP status mapping only.

### Frontend

Frontend schemas and forms mirror the backend rules:

- Disable or block Save when required identity fields are missing.
- Show clear section-specific validation messages.
- Optionally detect obvious duplicates from already loaded records for fast feedback.
- Still handle backend duplicate conflicts, because backend remains the source of truth.

## Normalization

Duplicate comparisons use normalized identity values:

- trim leading and trailing whitespace
- compare case-insensitively
- treat blank strings as missing
- ignore soft-deleted rows
- compare only rows visible to the current organization scope

No duplicate check should include `authority`.

## Required Identity And Duplicate Rules

### Sender

Required to save:

- `sender_type`
- `organization_name`

Duplicate key within same organization:

- `sender_type`
- normalized `organization_name`

Duplicate condition:

- A non-deleted sender presave already exists in the same organization with the same `sender_type` and normalized `organization_name`.

Frontend fields:

- `senderType`
- `senderOrganization`

### Receiver

Required to save:

- `receiver_type`
- `organization_name`

Duplicate key within same organization:

- `receiver_type`
- normalized `organization_name`

Duplicate condition:

- A non-deleted receiver presave already exists in the same organization with the same `receiver_type` and normalized `organization_name`.

Frontend fields:

- `receiverType`
- `receiverOrganization`

Notes:

- `receiverName` is not canonical identity. The canonical backend field is `organization_name`.

### Product

Required to save:

- `sender_presave_id`
- at least one of:
  - `medicinal_product`
  - `preapproval_ip_name`

Duplicate key within same organization:

- `sender_presave_id`
- product identity, using the provided product identity fields

Duplicate conditions:

- If both `medicinal_product` and `preapproval_ip_name` are present, a duplicate exists when a non-deleted product presave in the same organization has the same `sender_presave_id`, normalized `medicinal_product`, and normalized `preapproval_ip_name`.
- If only `medicinal_product` is present, a duplicate exists when a non-deleted product presave in the same organization has the same `sender_presave_id` and normalized `medicinal_product`.
- If only `preapproval_ip_name` is present, a duplicate exists when a non-deleted product presave in the same organization has the same `sender_presave_id` and normalized `preapproval_ip_name`.

Frontend fields:

- `senderPresaveId`
- `medicinalProduct`
- `preApprovalIpName`

Notes:

- Free-text `sender` does not satisfy the sender requirement. Product must reference a selected canonical sender presave.
- `drugCharacterization` is not required for presave save.

### Reporter

Required to save:

- `reporter_given_name`
- `organization`
- `qualification`

Duplicate key within same organization:

- normalized `reporter_given_name`
- normalized `organization`
- normalized `qualification`

Duplicate condition:

- A non-deleted reporter presave already exists in the same organization with the same normalized `reporter_given_name`, `organization`, and `qualification`.

Frontend fields:

- `reporterGivenName`
- `reporterOrganization`
- `qualification`

### Study

Required to save:

- `product_presave_id`
- at least one of:
  - `sponsor_study_number`
  - `study_name`

Duplicate key within same organization:

- `product_presave_id`
- study identity, using the provided study identity fields

Duplicate conditions:

- If both `sponsor_study_number` and `study_name` are present, a duplicate exists when a non-deleted study presave in the same organization has the same `product_presave_id`, normalized `sponsor_study_number`, and normalized `study_name`.
- If only `sponsor_study_number` is present, a duplicate exists when a non-deleted study presave in the same organization has the same `product_presave_id` and normalized `sponsor_study_number`.
- If only `study_name` is present, a duplicate exists when a non-deleted study presave in the same organization has the same `product_presave_id` and normalized `study_name`.

Frontend fields:

- `productPresaveId`
- `sponsorStudyNumber`
- `studyName`

Notes:

- Study must remain tied to a selected canonical product presave.

### Narrative

Required to save:

- canonical record `name`

Duplicate key within same organization:

- normalized `name`

Duplicate condition:

- A non-deleted narrative presave already exists in the same organization with the same normalized `name`.

Frontend field:

- record name from the presave create/edit shell

Notes:

- `case_narrative` is not required for minimal presave save. Narrative body can be filled later.
- This requires backend narrative validation to stop treating `case_narrative` as the minimal identity if canonical parent `name` is available on the write path.

## Error Semantics

Missing required identity:

- backend domain error category: validation error
- REST status: `400 Bad Request`
- message format: section-specific and field-specific

Duplicate identity:

- backend domain error category: conflict error
- REST status: `409 Conflict`
- message format: section-specific and identity-specific

Example messages:

- `Sender presave requires sender_type and organization_name`
- `Product presave requires sender_presave_id and either medicinal_product or preapproval_ip_name`
- `Product presave already exists for this sender and product identity`
- `Study presave already exists for this product and study identity`

## Update Semantics

Update validation evaluates the final persisted record, not only the patch payload.

Examples:

- If an existing product already has `sender_presave_id` and `medicinal_product`, updating only comments is valid.
- If an update clears the last product identity field, the update is rejected.
- If an update changes product identity to match another non-deleted product under the same sender, the update is rejected with conflict.
- If an update only soft-deletes a record, duplicate validation is skipped.

## Testing Strategy

Backend tests:

- Create rejects missing minimal identity for each section.
- Create rejects duplicate identity within the same organization.
- Create allows same identity across different organizations.
- Update rejects clearing required identity from the final record.
- Update rejects changing identity into an existing duplicate, excluding self.
- Soft-delete allows another record with the same identity to be created afterward.
- Details graph saves use the same validation as direct parent routes.

Frontend tests:

- Section forms block missing minimal identity before submit.
- Product requires selected canonical sender and one product identity.
- Product no longer requires `drugCharacterization`.
- Receiver uses `receiverOrganization` as canonical identity and does not require `receiverName`.
- Reporter requires qualification.
- Backend `409 Conflict` responses are displayed to the user.

## Acceptance Criteria

- No parent presave save path requires authority.
- No presave save path enforces regulatory completeness.
- Backend BMC/domain layer enforces required identity and duplicates.
- Frontend mirrors the same required identity rules.
- Duplicate checks are organization-scoped, normalized, and ignore soft-deleted records.
- Direct parent routes and details graph routes behave consistently.
