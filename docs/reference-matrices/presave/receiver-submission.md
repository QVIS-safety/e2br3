# Receiver Presave -> Submission Receiver Routing Reference Matrix

Date: 2026-06-01

Scope: receiver presave master records and receiver selection in `SUB > SUBMISSION`. The reference workflow was inspected through the logged-in Chrome UI only. No direct API calls and no bundle downloads were used.

Reference routes inspected:

- `INFO > RECEIVER` list and detail pages for `MFDS`, `FDA`, and `HENGRUI`
- `SUB > SUBMISSION` receiver selector and generated routing condition row
- Case edit `CI`, `SD`, and `RE` pages to check visible receiver import targets

Local surfaces inspected:

- Backend schema: `db/bootstrap/01-safetydb-schema.sql:224`
- Backend BMC/model: `crates/libs/lib-core/src/model/presave.rs:596`
- Backend details REST: `crates/services/web-server/src/web/rest/section_presave_rest.rs:294`
- Backend case receiver model: `crates/libs/lib-core/src/model/receiver.rs:14`
- Backend message header model: `crates/libs/lib-core/src/model/message_header.rs:14`
- Frontend type: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/types/presave.ts:82`
- Frontend read mapper: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/presave/canonicalMappers.ts:238`
- Frontend write mapper: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/presave/canonicalWriteMappers.ts:123`
- Frontend submission import flow: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/app/(protected)/submission/page.tsx:505`

## Reference Finding

Receiver presave is not imported into visible case edit receiver fields in the reference flow checked here. `SD (C.3)` showed sender fields only; `RE` showed reporting destination rows such as `RD_RECEIVER`, `RD_RPT_DUE`, and `RD_RPT_DATE`, but no presave import action. Receiver presave is used by `SUB > SUBMISSION` as a receiver route selector, which generates a condition row against `CI (C.1)` report type fields.

The local flow currently writes selected receiver presave data to `/api/cases/{caseId}/receiver` before resolving message header routing. That import path does not match the captured reference behavior.

## Matrix

| field | reference evidence | local frontend | local backend/BMC | case target | category | action |
| --- | --- | --- | --- | --- | --- | --- |
| `MNFT_TYPE` / `receiverType` / `receiver_type` | Receiver detail field `MNFT_TYPE`; examples show `Regulatory Authority`. | `ReceiverPresaveData.receiverType`; `ReceiverForm`; read/write mappers. | `receiver_presaves.receiver_type`; `ReceiverPresave.receiver_type`. | None in captured receiver presave flow. | `referencePreserveOnly` | Keep as receiver master field only. Do not import from receiver presave into C.3 receiver case fields. |
| `MNFT_NAME` / `receiverOrganization` / `organization_name` | Receiver detail field `MNFT_NAME`; examples `MFDS`, `FDA`, `HENGRUI`. | `receiverOrganization`; read mapper from `organizationName`; write mapper to `organization_name`. | `receiver_presaves.organization_name`; `ReceiverPresave.organization_name`. | None in captured receiver presave flow. | `referencePreserveOnly` | Keep as receiver master field only. |
| local `receiverName` / backend `name` | Receiver list/display name is represented by `MNFT_NAME`; no separate case import target was visible. | `receiverName`; read mapper can source `name`; write mapper sends unsupported `receiver_name`. | `receiver_presaves.name` is required template name. | Template label only. | `localSystemOnly` | Keep backend `name` as template identity. Remove unsupported write key `receiver_name`; display should use one canonical label. |
| `MNFT_ID` / `receiverId` / `receiver_identifier` | Receiver detail field `MNFT_ID`; blank for `MFDS` and `FDA`, `HENGRUI` for `HENGRUI`. | `receiverId`; read mapper from `receiverIdentifier`; write mapper to `receiver_identifier`. | `receiver_presaves.receiver_identifier`; `ReceiverPresave.receiver_identifier`. | None in case edit; submission routes use route labels and condition rows. | `referencePreserveOnly` | Keep on parent only as master metadata. Do not use parent-level loose identifier routing. |
| `DAY_TYPE` / `dayCountRule` / `day_count_rule` | Receiver detail field `DAY_TYPE`; `Calendar` for `MFDS` and `FDA`; blank for `HENGRUI`. | `dayCountRule`; read/write mappers. | `receiver_presaves.day_count_rule`; `ReceiverPresave.day_count_rule`. | None in captured receiver presave flow. | `referencePreserveOnly` | Keep as receiver master field only. |
| `AE_SOLI` / `nsaeSolicited` | Reference detail has value field and NA checkbox. Captured value blank. | `nsaeSolicited`; write mapper sends unsupported `nsae_solicited`. | Backend stores `nsae_solicited_day_count`. | None in captured receiver presave flow. | `referencePreserveOnly` | Rename frontend/write shape to day-count field. Stop sending `nsae_solicited`. |
| `AE_SOLI_NA` / `nsae_solicited_not_applicable` | Reference detail NA checkbox checked. | No explicit local frontend boolean field. | `receiver_presaves.nsae_solicited_not_applicable`; BMC field exists. | None in captured receiver presave flow. | `referencePreserveOnly` | Add explicit frontend field if editing receiver master must preserve NA state. |
| `AE_NON_SOLI` / `nsaeNonSolicited` | Reference detail has value field and NA checkbox. Captured value blank. | `nsaeNonSolicited`; write mapper sends unsupported `nsae_non_solicited`. | Backend stores `nsae_non_solicited_day_count`. | None in captured receiver presave flow. | `referencePreserveOnly` | Rename frontend/write shape to day-count field. Stop sending `nsae_non_solicited`. |
| `AE_NON_SOLI_NA` / `nsae_non_solicited_not_applicable` | Reference detail NA checkbox checked. | No explicit local frontend boolean field. | `receiver_presaves.nsae_non_solicited_not_applicable`; BMC field exists. | None in captured receiver presave flow. | `referencePreserveOnly` | Add explicit frontend field if editing receiver master must preserve NA state. |
| `SAE_SOLI` / `saeSolicited` | Reference detail has value field and NA checkbox. Captured value blank. | `saeSolicited`; write mapper sends unsupported `sae_solicited`. | Backend stores `sae_solicited_day_count`. | None in captured receiver presave flow. | `referencePreserveOnly` | Rename frontend/write shape to day-count field. Stop sending `sae_solicited`. |
| `SAE_SOLI_NA` / `sae_solicited_not_applicable` | Reference detail NA checkbox checked. | No explicit local frontend boolean field. | `receiver_presaves.sae_solicited_not_applicable`; BMC field exists. | None in captured receiver presave flow. | `referencePreserveOnly` | Add explicit frontend field if editing receiver master must preserve NA state. |
| `SAE_NONE_SOLI` / `saeNonSolicited` | Reference detail has value field and NA checkbox. Captured value blank. | `saeNonSolicited`; write mapper sends unsupported `sae_non_solicited`. | Backend stores `sae_non_solicited_day_count`. | None in captured receiver presave flow. | `referencePreserveOnly` | Rename frontend/write shape to day-count field. Stop sending `sae_non_solicited`. |
| `SAE_NONE_SOLI_NA` / `sae_non_solicited_not_applicable` | Reference detail NA checkbox checked. | No explicit local frontend boolean field. | `receiver_presaves.sae_non_solicited_not_applicable`; BMC field exists. | None in captured receiver presave flow. | `referencePreserveOnly` | Add explicit frontend field if editing receiver master must preserve NA state. |
| `DEL_FLAG` / `receiverDeleted` / `deleted` | Receiver detail `DEL_FLAG` unchecked; list `Deleted` column is `No`. | `receiverDeleted`; read/write mappers. | `receiver_presaves.deleted`; `ReceiverPresave.deleted`. | Presave list filtering/display. | `referencePreserveOnly` | Keep as receiver master lifecycle field. |
| `DESCRIPTION` / `receiverDescription` / `description` | Receiver detail `DESCRIPTION` blank. | `receiverDescription`; write mapper sends unsupported `receiver_description`. | `receiver_presaves.description`; `ReceiverPresave.description`. | None in captured receiver presave flow. | `referencePreserveOnly` | Write to backend `description`, not `receiver_description`. |
| `comments` | No separate reference receiver detail field; local template comment metadata. | `comments` can be sent through base mapper. | `receiver_presaves.comments`; `ReceiverPresave.comments`. | Template administration only. | `localSystemOnly` | Keep as local template metadata. |
| `authority` | Reference receiver route options are authority-specific (`MFDS(...)`, `FDA(...)`). | Presave APIs carry section authority in create/update. | `receiver_presaves.authority` with `ich`, `fda`, `mfds` check. | Presave filtering/route administration. | `localSystemOnly` | Keep. |
| `id` | Reference UI row number is display only; local UUID is persistence identity. | Template and child row IDs. | `receiver_presaves.id`; `ReceiverPresave.id`. | Persistence only. | `localSystemOnly` | Keep. |
| `organization_id` | Reference organization comes from tenant context, not receiver form field. | Not user-editable in receiver form. | `receiver_presaves.organization_id`; org-scoped BMC insert. | Persistence/tenant isolation. | `localSystemOnly` | Keep. |
| `created_at` | Not a reference receiver form field. | Not edited by receiver form. | `receiver_presaves.created_at`. | Audit metadata. | `localSystemOnly` | Keep. |
| `updated_at` | Not a reference receiver form field. | Not edited by receiver form. | `receiver_presaves.updated_at`. | Audit metadata. | `localSystemOnly` | Keep. |
| `created_by` | Not a reference receiver form field. | Not edited by receiver form. | `receiver_presaves.created_by`. | Audit metadata. | `localSystemOnly` | Keep. |
| `updated_by` | Not a reference receiver form field. | Not edited by receiver form. | `receiver_presaves.updated_by`. | Audit metadata. | `localSystemOnly` | Keep. |
| `receiver_presave_consignees.id` / `consignees[].id` | Consignee grid existed on receiver detail; captured records showed `No data`. | `consignees[].id`. | `receiver_presave_consignees.id`. | Receiver master child identity. | `localSystemOnly` | Keep as child identity if consignee editing stays. |
| `receiver_presave_consignees.receiver_presave_id` | Consignee grid belongs to the receiver master. | Implicit parent in details write flow. | `receiver_presave_consignees.receiver_presave_id`. | Child-parent persistence. | `localSystemOnly` | Keep. |
| `receiver_presave_consignees.sequence_number` / `consignees[].sequenceNumber` | Consignee grid has row order; captured records had no rows. | `consignees[].sequenceNumber`. | `receiver_presave_consignees.sequence_number`. | Receiver master child ordering. | `referencePreserveOnly` | Keep as consignee master field, not case import. |
| `receiver_presave_consignees.name` / `consignees[].consigneeName` | Consignee grid has a name column when rows exist; captured records had no rows. | `consignees[].consigneeName`; write mapper sends `name`. | `receiver_presave_consignees.name`. | Receiver master child row only. | `referencePreserveOnly` | Keep as consignee master field, not case import. |
| `consignees[].consigneeId` / `consignee_id` | No reference consignee ID field was visible in captured receiver records. | `consignees[].consigneeId`; write mapper sends unsupported `consignee_id`. | No backend consignee ID column. | None. | `removed` | Remove from frontend type and write mapper unless a later reference record shows a real consignee ID field. |
| `receiver_presave_consignees.phone` / `consignees[].phone` | Consignee grid has contact columns when rows exist; captured records had no rows. | `consignees[].phone`. | `receiver_presave_consignees.phone`. | Receiver master child row only. | `referencePreserveOnly` | Keep as consignee master field, not case import. |
| `receiver_presave_consignees.email` / `consignees[].email` | Consignee grid has contact columns when rows exist; captured records had no rows. | `consignees[].email`. | `receiver_presave_consignees.email`. | Receiver master child row only. | `referencePreserveOnly` | Keep as consignee master field, not case import. |
| `consignees[].deleted` | No reference consignee delete field was visible in captured records. | Frontend-only child flag. | No backend column. | None. | `removed` | Remove or convert to `_delete` transport only. |
| `consignees[]._delete` | Reference UI delete action not captured as stored field. | Transport deletion flag in frontend details write flow. | REST details endpoint interprets row deletion operations, not a stored column. | Transport only. | `localSystemOnly` | Keep only as local details transport flag. |
| `receiver_presave_consignees.created_at` | Not a reference receiver form field. | Not edited by receiver form. | `receiver_presave_consignees.created_at`. | Audit metadata. | `localSystemOnly` | Keep. |
| `receiver_presave_consignees.updated_at` | Not a reference receiver form field. | Not edited by receiver form. | `receiver_presave_consignees.updated_at`. | Audit metadata. | `localSystemOnly` | Keep. |
| `receiver_presave_consignees.created_by` | Not a reference receiver form field. | Not edited by receiver form. | `receiver_presave_consignees.created_by`. | Audit metadata. | `localSystemOnly` | Keep. |
| `receiver_presave_consignees.updated_by` | Not a reference receiver form field. | Not edited by receiver form. | `receiver_presave_consignees.updated_by`. | Audit metadata. | `localSystemOnly` | Keep. |
| `SUB.RECEIVER` selection | Reference `SUB > SUBMISSION` receiver input opens route options such as `MFDS(KR)` and `FDA(CBER IND)`. | `selectedReceiverTemplate`; `resolveReceiverRouting`. | No backend receiver route child table. | Submission receiver route selection. | `referenceImportedToCase` | Add route child rows under receiver presave; selection should choose one route row. |
| `SUB.RECEIVER` route label | Reference route labels captured: `MFDS(CF)`, `MFDS(FR)`, `MFDS(KR)`, `MFDS(CU)`, `MFDS(CT)`, `FDA(Postmarket)`, `FDA(CDER IND)`, `FDA(CDER IND-exempt BA/BE)`, `FDA(CBER IND)`. | Current `routingRules[]` has no display label field. | No backend route label column/table. | Submission receiver route selection. | `referenceImportedToCase` | Add `receiver_presave_routes.receiver_label`. |
| route authority | Reference route label and condition field split by authority: MFDS uses MFDS report type; FDA uses FDA report type. | `routingRules[].authority`. | No backend route child table. | Submission receiver route matching. | `referenceImportedToCase` | Move from loose frontend-only `routingRules[]` to persisted route row `authority`. |
| route condition page | Reference generated row `0-PAGE = CI (C.1)` for MFDS and FDA receiver routes. | No explicit persisted local field. | No backend route child table. | Submission condition row. | `referenceImportedToCase` | Add `condition_page` route column; seed `CI`. |
| route condition item: MFDS | Reference generated row `0-ITEM = MFDS Report Type(-)` for `MFDS(...)` routes. | Local submission derives report type from case safety report, not from persisted route condition row. | No backend route child table. | `CI.MFDS_REPORT_TYPE`. | `referenceImportedToCase` | Add `condition_field_code = MFDS_REPORT_TYPE` route column. |
| route condition item: FDA | Reference generated row `0-ITEM = FDA Report Type(FDA_REPORT_TYPE)` for `FDA(...)` routes. | Local submission derives report type from case safety report, not from persisted route condition row. | No backend route child table. | `CI.FDA_REPORT_TYPE`. | `referenceImportedToCase` | Add `condition_field_code = FDA_REPORT_TYPE` route column. |
| route condition operator | Reference generated row `0-CONDITION = Equal`. | No explicit persisted local field. | No backend route child table. | Submission condition row. | `referenceImportedToCase` | Add `condition_operator`, fixed to `Equal` for captured routes. |
| route condition value: `MFDS(CT)` | Reference generated value `1` for `MFDS(CT)`. | Current `routingRules[].reportType` can carry a code but lacks route label and condition metadata. | No backend route child table. | `CI.MFDS_REPORT_TYPE = 1`. | `referenceImportedToCase` | Add persisted route row. |
| route condition value: `MFDS(CU)` | Reference generated value `2` for `MFDS(CU)`. | Same as above. | No backend route child table. | `CI.MFDS_REPORT_TYPE = 2`. | `referenceImportedToCase` | Add persisted route row. |
| route condition value: `MFDS(KR)` | Reference generated value `3` for `MFDS(KR)`. | Same as above. | No backend route child table. | `CI.MFDS_REPORT_TYPE = 3`. | `referenceImportedToCase` | Add persisted route row. |
| route condition value: `MFDS(FR)` | Reference generated value `4` for `MFDS(FR)`. | Same as above. | No backend route child table. | `CI.MFDS_REPORT_TYPE = 4`. | `referenceImportedToCase` | Add persisted route row. |
| route condition value: `MFDS(CF)` | Reference generated value `5` for `MFDS(CF)`. | Same as above. | No backend route child table. | `CI.MFDS_REPORT_TYPE = 5`. | `referenceImportedToCase` | Add persisted route row. |
| route condition value: `FDA(CDER IND)` | Reference generated value `1` for `FDA(CDER IND)`. | Same as above. | No backend route child table. | `CI.FDA_REPORT_TYPE = 1`. | `referenceImportedToCase` | Add persisted route row. |
| route condition value: `FDA(CDER IND-exempt BA/BE)` | Reference generated value `2` for `FDA(CDER IND-exempt BA/BE)`. | Same as above. | No backend route child table. | `CI.FDA_REPORT_TYPE = 2`. | `referenceImportedToCase` | Add persisted route row. |
| route condition value: `FDA(CBER IND)` | Reference generated value `3` for `FDA(CBER IND)`. | Same as above. | No backend route child table. | `CI.FDA_REPORT_TYPE = 3`. | `referenceImportedToCase` | Add persisted route row. |
| route condition value: `FDA(Postmarket)` | Reference generated value `4` for `FDA(Postmarket)`. | Same as above. | No backend route child table. | `CI.FDA_REPORT_TYPE = 4`. | `referenceImportedToCase` | Add persisted route row. |
| `message_headers.batch_receiver_identifier` / `batchReceiverId` | Reference selection is a submission receiver route; exact N header value is not displayed in case edit UI, but this is the local N receiver batch target. | `batchReceiverId`; `resolveReceiverRouting` writes `batchReceiverIdentifier`. | `message_headers.batch_receiver_identifier`. | Message header receiver routing. | `referenceImportedToCase` | Make route row provide this value explicitly; stop parent-level implicit identifier substitution. |
| `message_headers.message_receiver_identifier` / route message identifier | Reference receiver route labels encode receiver variants, and the CI warning says Receiver ID applies from MFDS report type. | `receiverId`; `routingRules[].messageReceiverIdentifier`; `resolveReceiverRouting`. | `message_headers.message_receiver_identifier`. | Message header receiver routing. | `referenceImportedToCase` | Make route row provide this value explicitly; stop parent-level implicit identifier substitution. |
| `routingRules[].reportType` | Reference route condition value is per route row. | Current loose frontend child array. | No backend route child table. | Submission route condition value. | `removed` | Replace with persisted route row `condition_value_code`. |
| `routingRules[].batchReceiverIdentifier` | Reference target belongs to selected receiver route, not loose frontend-only data. | Current loose frontend child array. | No backend route child table. | Message header receiver routing. | `removed` | Replace with persisted route row `batch_receiver_identifier`. |
| `routingRules[].messageReceiverIdentifier` | Reference target belongs to selected receiver route, not loose frontend-only data. | Current loose frontend child array. | No backend route child table. | Message header receiver routing. | `removed` | Replace with persisted route row `message_receiver_identifier`. |
| `receiverDepartment` | No receiver presave detail field and no receiver presave import into visible C.3 receiver fields was captured. | Frontend type/form/write mapper. | No `receiver_presaves` column; exists only as case `receiver_information.department`. | C.3 receiver information remains a case field, not receiver presave import target. | `removed` | Remove from receiver presave type/form/write mapper. Keep case receiver field outside presave import. |
| `receiverStreetAddress` | Same as above. | Frontend type/form/write mapper. | No `receiver_presaves` column; case field is `receiver_information.street_address`. | C.3 receiver information only. | `removed` | Remove from receiver presave type/form/write mapper. |
| `receiverCity` | Same as above. | Frontend type/form/write mapper. | No `receiver_presaves` column; case field is `receiver_information.city`. | C.3 receiver information only. | `removed` | Remove from receiver presave type/form/write mapper. |
| `receiverState` | Same as above. | Frontend type/form/write mapper. | No `receiver_presaves` column; case field is `receiver_information.state_province`. | C.3 receiver information only. | `removed` | Remove from receiver presave type/form/write mapper. |
| `receiverPostcode` | Same as above. | Frontend type/form/write mapper. | No `receiver_presaves` column; case field is `receiver_information.postcode`. | C.3 receiver information only. | `removed` | Remove from receiver presave type/form/write mapper. |
| `receiverCountryCode` | Same as above. | Frontend type/form/write mapper. | No `receiver_presaves` column; case field is `receiver_information.country_code`. | C.3 receiver information only. | `removed` | Remove from receiver presave type/form/write mapper. |
| `receiverTelephone` | Same as above. | Frontend type/form/write mapper. | No `receiver_presaves` column; case field is `receiver_information.telephone`. | C.3 receiver information only. | `removed` | Remove from receiver presave type/form/write mapper. |
| `receiverFax` | Same as above. | Frontend type/form/write mapper. | No `receiver_presaves` column; case field is `receiver_information.fax`. | C.3 receiver information only. | `removed` | Remove from receiver presave type/form/write mapper. |
| `receiverEmail` | Same as above. | Frontend type/form/write mapper. | No `receiver_presaves` column; case field is `receiver_information.email`. | C.3 receiver information only. | `removed` | Remove from receiver presave type/form/write mapper. |
| local submission call `upsertReceiverInformation(selectedReceiverTemplate)` | Reference receiver selection did not populate visible C.3 receiver fields in checked case edit sections. | `/submission/page.tsx` calls `api.cases.upsertReceiverInformation` for selected receiver template. | Writes `receiver_information` case table. | C.3 receiver information. | `removed` | Remove this call from receiver presave submission route application. |
| `receiver_information.receiver_type` | Valid local case field, but not a receiver presave import target in captured reference flow. | `upsertReceiverInformation` accepts it. | `receiver_information.receiver_type`. | C.3 receiver information. | `removed` | Do not populate from receiver presave. Keep as case edit field. |
| `receiver_information.organization_name` | Same as above. | `upsertReceiverInformation` accepts it. | `receiver_information.organization_name`. | C.3 receiver information. | `removed` | Do not populate from receiver presave. Keep as case edit field. |
| `receiver_information.department` | Same as above. | `upsertReceiverInformation` accepts it. | `receiver_information.department`. | C.3 receiver information. | `removed` | Do not populate from receiver presave. Keep as case edit field. |
| `receiver_information.street_address` | Same as above. | `upsertReceiverInformation` accepts it. | `receiver_information.street_address`. | C.3 receiver information. | `removed` | Do not populate from receiver presave. Keep as case edit field. |
| `receiver_information.city` | Same as above. | `upsertReceiverInformation` accepts it. | `receiver_information.city`. | C.3 receiver information. | `removed` | Do not populate from receiver presave. Keep as case edit field. |
| `receiver_information.state_province` | Same as above. | `upsertReceiverInformation` accepts it. | `receiver_information.state_province`. | C.3 receiver information. | `removed` | Do not populate from receiver presave. Keep as case edit field. |
| `receiver_information.postcode` | Same as above. | `upsertReceiverInformation` accepts it. | `receiver_information.postcode`. | C.3 receiver information. | `removed` | Do not populate from receiver presave. Keep as case edit field. |
| `receiver_information.country_code` | Same as above. | `upsertReceiverInformation` accepts it. | `receiver_information.country_code`. | C.3 receiver information. | `removed` | Do not populate from receiver presave. Keep as case edit field. |
| `receiver_information.telephone` | Same as above. | `upsertReceiverInformation` accepts it. | `receiver_information.telephone`. | C.3 receiver information. | `removed` | Do not populate from receiver presave. Keep as case edit field. |
| `receiver_information.fax` | Same as above. | `upsertReceiverInformation` accepts it. | `receiver_information.fax`. | C.3 receiver information. | `removed` | Do not populate from receiver presave. Keep as case edit field. |
| `receiver_information.email` | Same as above. | `upsertReceiverInformation` accepts it. | `receiver_information.email`. | C.3 receiver information. | `removed` | Do not populate from receiver presave. Keep as case edit field. |

## Required Backend Migration

Backend architecture migration is needed to match the reference.

Add a receiver routing child table, for example `receiver_presave_routes`, with at least:

- `id`
- `receiver_presave_id`
- `sequence_number`
- `authority`
- `receiver_label`
- `batch_receiver_identifier`
- `message_receiver_identifier`
- `condition_page`
- `condition_field_code`
- `condition_operator`
- `condition_value_code`
- `condition_value_label`
- standard created/updated metadata

Then expose it through receiver presave details in the same style as sender gateways and product child rows. The parent receiver presave remains the preserve-only master record; route rows drive submission receiver selection and N-section message header identifiers.

Frontend migration should:

- Replace loose `routingRules[]` with persisted receiver route rows.
- Stop copying receiver presave parent data into `/api/cases/{caseId}/receiver`.
- Write message header receiver identifiers from the selected route row.
- Remove receiver presave form fields that only belong to case receiver information.
- Align day-count and NA fields to backend column names.

## Coverage Check

Reference fields: 30

Local frontend receiver presave fields: 33

Backend presave columns/tables and import targets: 49

Categorized matrix rows: 62

Uncategorized fields: 0

Ambiguous fields: 0
