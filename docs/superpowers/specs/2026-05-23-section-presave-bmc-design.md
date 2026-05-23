# Section Presave BMC Design

Date: 2026-05-23

## Goal

Replace the generic `presave_templates` architecture with structured INFO presave master data models. Presaves should be section-specific records with real columns, normal audit trail behavior, authority-aware fields, and import/apply mappings aligned with the existing case BMC/domain field names.

This design covers backend model shape for Sender, Receiver, Product, Reporter, Study, and Narrative. Study and Narrative frontend detail UI is outside the first UI implementation phase, but their backend model shape is included now so the data model is coherent across INFO sections.

## Design Decision

Use separate BMCs and tables per INFO section. Do not add a `PresaveRecordBmc`.

Each section table owns its own metadata:

- `id`
- `organization_id`
- `authority`
- `name`
- `comments`
- `deleted`
- section-specific default flags where needed
- `created_at`
- `updated_at`
- `created_by`
- `updated_by`

Each section table also owns its real fields. Field names should mirror existing case BMC/domain fields wherever the presave is imported from, applied to, or audited against case data. Frontend labels can use reference-style text, but backend field names should stay aligned with case domain names.

## Authority Model

Supported authorities are:

- `ich`
- `fda`
- `mfds`

Each section table contains the union of fields needed across ICH, FDA, and MFDS. Authority-specific fields are nullable and active only for the relevant authority.

Rules:

- `ich`: common/ICH fields active; FDA/MFDS-only fields should be null.
- `fda`: common + FDA fields active; MFDS-only fields should be null.
- `mfds`: common + MFDS fields active; FDA-only fields should be null.

These rules must be enforced in BMC/service validation from the first implementation. DB check constraints are a second hardening phase after the first production data migration confirms the final field list.

## Shared Behavior

Every section BMC should implement:

- create
- get
- list with organization, authority, deleted, and search filters
- update
- soft delete
- hard delete only if existing admin conventions require it
- field audit access through standard `audit_logs`
- import from case where applicable
- apply to case where applicable

Receiver is the exception for import source: receiver presave is primarily submission-routing master data, not a direct case-section import.

## BMCs

Add these primary BMCs:

- `SenderPresaveBmc`
- `ReceiverPresaveBmc`
- `ProductPresaveBmc`
- `ReporterPresaveBmc`
- `StudyPresaveBmc`
- `NarrativePresaveBmc`

Add child BMCs only for repeatable child records:

- `SenderPresaveGatewayBmc`
- `SenderPresaveResponsiblePersonBmc`
- `ReceiverPresaveConsigneeBmc`
- `ProductPresaveSubstanceBmc`
- `ProductPresaveFdaCrossReportedIndBmc`
- `ProductPresaveMfdsRegionalItemBmc`
- `StudyPresaveRegistrationNumberBmc`
- `NarrativePresaveSenderDiagnosisBmc`
- `NarrativePresaveCaseSummaryBmc`

## Sender

Table: `sender_presaves`

Mirrors `SenderInformation` where possible.

Columns:

- metadata columns listed above
- `is_default`
- `sender_type`
- `organization_name`
- `department`
- `street_address`
- `city`
- `state`
- `postcode`
- `country_code`
- `telephone`
- `fax`
- `email`

Authority separation:

- ICH, FDA, and MFDS share the same core sender fields.
- Gateway rows are authority-targeted even when the parent sender presave is ICH, because the reference UI shows FDA, PMDA, MFDS, NMPA, and EMA gateway rows under ICH.
- Default sender behavior should be scoped per target gateway authority, not organization-wide.

Table: `sender_presave_gateways`

Columns:

- `id`
- `sender_presave_id`
- `sequence_number`
- `gateway_authority`
- `sender_identifier`
- `routing_identifier`
- `cde_sender_identifier`
- `cdr_sender_identifier`
- `ema_sender_identifier`
- `is_default_for_authority`
- audit columns

`gateway_authority` values should cover the visible routing targets: `fda`, `pmda`, `mfds`, `nmpa`, `ema`.

Table: `sender_presave_responsible_persons`

Columns:

- `id`
- `sender_presave_id`
- `sequence_number`
- `department`
- `person_title`
- `person_given_name`
- `person_middle_name`
- `person_family_name`
- `is_default`
- audit columns

The case BMC currently stores one responsible person in `SenderInformation`. Presave stores the reference-style table shape, while import/apply uses the default row to map to and from the case fields.

## Receiver

Table: `receiver_presaves`

Receiver presave is submission-routing oriented and is not simply copied from `ReceiverInformation`.

Columns:

- metadata columns listed above
- `receiver_type`
- `organization_name`
- `receiver_identifier`
- `day_count_rule`
- `nsae_solicited_day_count`
- `nsae_solicited_not_applicable`
- `nsae_non_solicited_day_count`
- `nsae_non_solicited_not_applicable`
- `sae_solicited_day_count`
- `sae_solicited_not_applicable`
- `sae_non_solicited_day_count`
- `sae_non_solicited_not_applicable`
- `description`

Authority separation:

- ICH, FDA, and MFDS use the same receiver form shape based on the provided screenshots.
- Submission routing behavior uses `authority`, report type, due-day, and not-applicable fields.

Table: `receiver_presave_consignees`

Columns:

- `id`
- `receiver_presave_id`
- `sequence_number`
- `name`
- `phone`
- `email`
- audit columns

## Product

Table: `product_presaves`

Mirrors `DrugInformation` names where possible. Product can reference sender presave because product records are tied to sender in the INFO workflow.

Columns:

- metadata columns listed above
- `sender_presave_id`
- `drug_characterization`
- `medicinal_product`
- `medicinal_product_notation`
- `preapproval_ip_name`
- `brand_name`
- `drug_generic_name`
- `manufacturer_name`
- `product_description`
- `mpid`
- `mpid_version`
- `phpid`
- `phpid_version`
- `investigational_product_blinded`
- `obtain_drug_country`
- `drug_authorization_number`
- `drug_authorization_country`
- `drug_authorization_holder`
- `holder_applicant_name_notation`
- `fda_ind_number_occurred`
- `fda_pre_anda_number_occurred`
- `mfds_domestic_product_code`
- `mfds_domestic_ingredient_code`
- `mfds_udl_product_code`
- `mfds_udl_ingredient_code`
- `mfds_udl_manufacturer_code`
- `mfds_udl_manufacturer_name`
- `mfds_foreign_ich_product_code`
- `mfds_foreign_ich_ingredient_code`
- `mfds_foreign_ich_holder_code`
- `mfds_foreign_ich_holder_name`
- `mfds_foreign_e2b_product_code`
- `mfds_foreign_e2b_ingredient_code`
- `mfds_foreign_e2b_holder_code`
- `mfds_foreign_e2b_holder_name`

Authority separation:

- ICH: common product fields and substance rows only.
- FDA: common fields plus `fda_*` fields and FDA cross-reported IND rows.
- MFDS: common fields plus `mfds_*` fields and any required MFDS regional child rows.

Table: `product_presave_substances`

Columns:

- `id`
- `product_presave_id`
- `sequence_number`
- `substance_name`
- `substance_termid_version`
- `substance_termid`
- `strength_value`
- `strength_unit`
- audit columns

Table: `product_presave_fda_cross_reported_inds`

Columns:

- `id`
- `product_presave_id`
- `sequence_number`
- `ind_number`
- audit columns

Table: `product_presave_mfds_regional_items`

Columns:

- `id`
- `product_presave_id`
- `sequence_number`
- `item_type`
- `item_value`
- audit columns

`item_type` is an enum-like text value controlled by backend validation. Initial values are `domestic_approval_number`, `foreign_ich_approval_number`, `foreign_e2b_approval_number`, `foreign_ich_product_code`, `foreign_ich_ingredient_code`, `foreign_ich_holder_code`, `foreign_e2b_product_code`, `foreign_e2b_ingredient_code`, and `foreign_e2b_holder_code`. This keeps the repeated MFDS regional blocks normalized without returning to opaque JSON.

## Reporter

Table: `reporter_presaves`

Mirrors `PrimarySource`.

Columns:

- metadata columns listed above
- `reporter_title`
- `reporter_given_name`
- `reporter_middle_name`
- `reporter_family_name`
- `organization`
- `department`
- `street`
- `city`
- `state`
- `postcode`
- `telephone`
- `country_code`
- `email`
- `qualification`
- `qualification_kr1`
- `primary_source_regulatory`

Authority separation:

- ICH and FDA use common fields.
- MFDS uses common fields plus nullable `qualification_kr1`.

## Study

Table: `study_presaves`

Mirrors `StudyInformation`.

Columns:

- metadata columns listed above
- `product_presave_id`
- `study_name`
- `sponsor_study_number`
- `study_type_reaction`
- `study_type_reaction_kr1`
- `edc_sync`

Authority separation:

- ICH and FDA use common study fields.
- MFDS uses common fields plus nullable `study_type_reaction_kr1`.

`product_presave_id` should be nullable initially so existing study presaves can be created before product relationships are fully enforced.

Table: `study_presave_registration_numbers`

Columns:

- `id`
- `study_presave_id`
- `sequence_number`
- `registration_number`
- `country_code`
- audit columns

## Narrative

Table: `narrative_presaves`

Mirrors `NarrativeInformation`.

Columns:

- metadata columns listed above
- `case_narrative`
- `reporter_comments`
- `sender_comments`

Authority separation:

- ICH, FDA, and MFDS share the same core narrative shape in this design. Additional regional narrative fields must be added as explicit nullable columns or child tables with authority validation, not JSON payloads.

Child tables for full H section presaves:

Table: `narrative_presave_sender_diagnoses`

- `id`
- `narrative_presave_id`
- `sequence_number`
- `diagnosis_meddra_version`
- `diagnosis_meddra_code`
- audit columns

Table: `narrative_presave_case_summaries`

- `id`
- `narrative_presave_id`
- `sequence_number`
- `summary_type`
- `language_code`
- `summary_text`
- audit columns

## Audit Trail

Use standard `audit_logs` with normal table and field names. Every visible field’s paper icon should request audit by the table and column that owns that field.

Examples:

- `GET /api/audit-logs/by-record/product_presaves/{id}?field=brand_name`
- `GET /api/audit-logs/by-record/product_presave_substances/{id}?field=substance_name`
- `GET /api/audit-logs/by-record/sender_presave_gateways/{id}?field=sender_identifier`

This is the main reason for preferring section tables over JSON: field-level audit works through existing audit behavior instead of JSON-path-specific audit logic.

## Import And Apply

Each section BMC owns explicit import/apply mapping.

Case-derived presaves:

- `SenderPresaveBmc::create_from_case_sender(case_id)`
- `ProductPresaveBmc::create_from_case_drug(drug_id, authority)`
- `ReporterPresaveBmc::create_from_case_primary_source(primary_source_id)`
- `StudyPresaveBmc::create_from_case_study(study_id)`
- `NarrativePresaveBmc::create_from_case_narrative(case_id)`

Receiver is special:

- `ReceiverPresaveBmc::create_from_submission_route(...)`

Apply methods should mirror the case BMC field names so mappings stay direct and testable.

## Migration Strategy

1. Add new section tables and BMCs behind new endpoints.
2. Keep `presave_templates` as legacy during transition.
3. Build list/detail APIs per section.
4. Update frontend INFO routes to call section-specific APIs.
5. Add import-from-case and apply-to-case methods section by section.
6. Retire `presave_templates` only after existing data migration and frontend switch are complete.

## Testing Strategy

Backend tests should cover:

- CRUD and org isolation per section BMC
- authority filtering per section
- authority-specific null-field enforcement
- field-level audit entries for parent and child fields
- sender default per gateway authority
- product-to-sender FK behavior
- product ICH/FDA/MFDS field separation
- reporter MFDS `qualification_kr1`
- study MFDS `study_type_reaction_kr1`
- import-from-case mappings use case BMC field names
- apply-to-case mappings do not run full regulatory validation

Frontend tests should cover:

- INFO list routes call the correct section endpoint
- detail forms show the correct authority-specific fields
- paper icons request audit for the correct table/record/field
- product sender selection uses sender presave records
- ICH product does not show FDA/MFDS fields
- FDA product does not show MFDS fields
- MFDS product does not show FDA fields
