# Case–INFO UUID Linkage Design

## Status

Approved for implementation on 2026-07-13.

## Problem

User access scopes currently contain business strings, while Case visibility compares Sender organization name, Product/brand text, Study number, and an active Gateway identifier. Existing Case source UUID columns are not consistently populated, especially during XML import. This makes visibility dependent on mutable display values and causes newly imported or newly assigned cases to disappear unexpectedly.

## Decisions

1. Reuse existing UUID primary and foreign keys. Do not add direct UUID columns to `cases`, `sender_presaves`, `product_presaves`, or `study_presaves`.
2. Treat Presave UUIDs as relationship and authorization identifiers. Business identifiers remain display, XML snapshot, search, and duplicate-detection values.
3. Empty Sender/Product/Study access scope means unrestricted access for that dimension in Case visibility and Presave selection/list APIs.
4. `active_sender_identifier` is Gateway routing context only and must not participate in Case visibility.
5. Blind access remains an independent safety gate.

## Existing Relationship Model

- User → Sender: `users.access_sender_ids` contains `sender_presaves.id` values.
- User → Product: `users.access_product_ids` contains `product_presaves.id` values.
- User → Study: `users.access_study_ids` contains `study_presaves.id` values.
- Product → Sender: `product_presaves.sender_presave_id`.
- Study → Product: `study_presaves.product_presave_id` and, for repeated products, `study_presave_products.product_presave_id`.
- Case C.3 → Sender: `sender_information.source_sender_presave_id`.
- Case G.k → Product: `drug_information.source_product_presave_id`.
- Case C.5 → Study: `study_information.source_study_presave_id`.

The three User scope columns remain TEXT-encoded JSON arrays during the compatibility phase. New writes accept UUID strings only. A normalized User access join-table migration is a future hardening option, not part of this implementation.

## Case Editor Template Application

Applying a Product Template from a specific G.k row stores the selected `product_presaves.id` in that row's `source_product_presave_id` and copies the Product snapshot fields into the same row. Applying Sender or Study Templates similarly records the corresponding source UUID in C.3 or C.5 while preserving copied snapshot values.

The API must reject a Presave UUID when it is malformed, belongs to another organization, is deleted, or is outside the User's configured scope. An empty scope does not reject the Presave.

## XML Import

The Import UI displays Product ID, IP Name, Sender, and manufacturer, but submits the selected `product_presaves.id` as `productPresaveId`. The backend resolves and authorizes the UUID before creating a Case.

For each imported XML entry:

1. Create G.k rows in XML sequence order.
2. Link the selected Product UUID to the first imported G.k row only.
3. Set `cases.dg_prd_key` to the selected Presave's business `product_id` for display, search, and duplicate matching.
4. Do not copy the Product UUID to remaining G.k rows.
5. Reject the entry if no G.k row exists.
6. Do not infer a Study UUID from XML strings.

When `apply_sender_info_to_imported_cases` is disabled, retain the XML C.3 snapshot and leave `source_sender_presave_id` null. When enabled, resolve the selected Product's Sender, apply that Sender snapshot to C.3, and store `sender_presaves.id` in `source_sender_presave_id`. Missing, deleted, cross-tenant, or invalid Sender linkage rejects that import entry rather than silently applying partial data.

ZIP imports apply the same selected Product to the first G.k row of every successfully imported XML entry. Each entry remains independently reported in import history.

## Visibility Contract

For each of Sender, Product, and Study:

- Empty User scope: allow that dimension.
- Configured User scope and a matching Case source UUID: allow that dimension.
- Configured User scope and a nonmatching Case source UUID: deny.
- Configured User scope and no Case source UUID: allow during the legacy compatibility phase, with metrics identifying unresolved cases.

This preserves visibility for legacy/imported Cases while UUID source coverage is backfilled. After coverage reaches the approved threshold, removing the null-source compatibility behavior requires a separate migration decision.

`active_sender_identifier` does not affect the contract. `access_blind_allowed` continues to deny blinded Cases independently.

## Compatibility and Migration

1. Accept legacy business-string scope values for reads while requiring UUID values for new writes.
2. Resolve legacy scope strings only when the match is unique within the User's organization; ambiguous and unmatched values remain in an operational report.
3. Write Case source UUIDs on every new Template application and Product-selected XML import.
4. Shadow-evaluate legacy and UUID visibility before disabling legacy string reads.
5. Retain a feature flag that can restore legacy matching during the observation window.

## Error Handling and Transactions

- Validate Product UUID, tenant, deletion state, and scope before XML Case creation.
- Perform Case creation, G.k source linkage, optional Sender application, Case version creation, and final Case update in one transaction.
- Roll back the complete XML entry on any linkage or persistence error.
- Return a stable per-entry import error and record it in import history without exposing another tenant's identifiers.

## Required Integration Tests

1. Empty scope User can list Sender/Product/Study Presaves and read Cases.
2. Configured scope User sees only matching UUID Presaves and Cases.
3. Cross-organization UUID is rejected.
4. Deleted Presave UUID is rejected.
5. Product-selected XML import links the Product UUID to the first G.k row only and sets `cases.dg_prd_key` to the business Product ID.
6. XML import with no G.k is rejected without a partial Case.
7. Sender setting OFF retains XML Sender and leaves source Sender UUID null.
8. Sender setting ON applies the Product's Sender snapshot and source Sender UUID.
9. Changing the active Gateway identifier does not change Case visibility.
10. Legacy Case with null source UUID remains visible during compatibility mode.
11. Blind Case remains denied without blind permission.
12. A failed import entry leaves no partial Case or source linkage.

## Out of Scope

- Automatic Study matching from XML strings.
- Adding a direct Product foreign key to `cases`.
- Applying one selected Product to every imported G.k row.
- Replacing User scope TEXT columns with join tables in this change.
- Redesigning Gateway submission routing.
