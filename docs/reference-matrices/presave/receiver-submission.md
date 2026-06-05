# Receiver Presave -> Submission Receiver Routing Reference Matrix

Date: 2026-06-01

Updated: 2026-06-05. The 2026-06-05 bundle investigation supersedes the original recommendation to add `receiver_presave_routes`. Receiver detail owns master plus consignees only. Submission receiver options belong to the submission workflow.

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
| `SUB.RECEIVER` selection | Reference `SUB > SUBMISSION` receiver input opens submission options such as `MFDS(KR)` and `FDA(CBER IND)` through `ReceiverSelectInput` with `type="Submission"`. | Current local submission no longer reads receiver presave `routes`; it uses explicit N.1.4/N.2.r.3 inputs. | No backend receiver route child table. | Submission receiver route selection. | `referenceImportedToCase` | Model as a separate submission receiver option source, not receiver presave children. |
| `SUB.RECEIVER` route label | Reference route labels captured: `MFDS(CF)`, `MFDS(FR)`, `MFDS(KR)`, `MFDS(CU)`, `MFDS(CT)`, `FDA(Postmarket)`, `FDA(CDER IND)`, `FDA(CDER IND-exempt BA/BE)`, `FDA(CBER IND)`. | No active receiver-presave route owner after cleanup. | No backend receiver route child table. | Submission receiver route selection. | `referenceImportedToCase` | Future route option model should be named around submission receiver routing. Do not add `receiver_presave_routes`. |
| route authority | Reference route label and condition field split by authority: MFDS uses MFDS report type; FDA uses FDA report type. | No active receiver-presave route owner after cleanup. | No backend receiver route child table. | Submission receiver route matching. | `referenceImportedToCase` | Store/derive this under a submission receiver option source if local needs persisted options. |
| route condition page | Reference generated row `0-PAGE = CI (C.1)` for MFDS and FDA receiver routes. | No active receiver-presave route owner after cleanup. | No backend receiver route child table. | Submission condition row. | `referenceImportedToCase` | Derived by submission workflow from selected label; not a receiver presave field. |
| route condition item: MFDS | Reference generated row `0-ITEM = MFDS Report Type(-)` for `MFDS(...)` routes. | No active receiver-presave route owner after cleanup. | No backend receiver route child table. | `CI.MFDS_REPORT_TYPE`. | `referenceImportedToCase` | Derived by submission workflow from selected label. |
| route condition item: FDA | Reference generated row `0-ITEM = FDA Report Type(FDA_REPORT_TYPE)` for `FDA(...)` routes. | No active receiver-presave route owner after cleanup. | No backend receiver route child table. | `CI.FDA_REPORT_TYPE`. | `referenceImportedToCase` | Derived by submission workflow from selected label. |
| route condition operator | Reference generated row `0-CONDITION = Equal`. | No active receiver-presave route owner after cleanup. | No backend receiver route child table. | Submission condition row. | `referenceImportedToCase` | Derived by submission workflow from selected label. |
| route condition value: `MFDS(CT)` | Reference bundle maps `MFDS(CT)` to value `1`. | No active receiver-presave route owner after cleanup. | No backend receiver route child table. | `CI.MFDS_REPORT_TYPE = 1`. | `referenceImportedToCase` | Keep in submission receiver option logic if implemented. |
| route condition value: `MFDS(CU)` | Reference bundle maps `MFDS(CU)` to value `2`. | No active receiver-presave route owner after cleanup. | No backend receiver route child table. | `CI.MFDS_REPORT_TYPE = 2`. | `referenceImportedToCase` | Keep in submission receiver option logic if implemented. |
| route condition value: `MFDS(KR)` | Reference bundle maps `MFDS(KR)` to value `3`. | No active receiver-presave route owner after cleanup. | No backend receiver route child table. | `CI.MFDS_REPORT_TYPE = 3`. | `referenceImportedToCase` | Keep in submission receiver option logic if implemented. |
| route condition value: `MFDS(FR)` | Reference bundle maps `MFDS(FR)` to value `4`. | No active receiver-presave route owner after cleanup. | No backend receiver route child table. | `CI.MFDS_REPORT_TYPE = 4`. | `referenceImportedToCase` | Keep in submission receiver option logic if implemented. |
| route condition value: `MFDS(CF)` | Reference bundle maps `MFDS(CF)` to value `5`. | No active receiver-presave route owner after cleanup. | No backend receiver route child table. | `CI.MFDS_REPORT_TYPE = 5`. | `referenceImportedToCase` | Keep in submission receiver option logic if implemented. |
| route condition value: `FDA(CDER IND)` | Reference bundle maps `FDA(CDER IND)` to value `1`. | No active receiver-presave route owner after cleanup. | No backend receiver route child table. | `CI.FDA_REPORT_TYPE = 1`. | `referenceImportedToCase` | Keep in submission receiver option logic if implemented. |
| route condition value: `FDA(CDER IND-exempt BA/BE)` | Reference bundle maps `FDA(CDER IND-exempt BA/BE)` to value `2`. | No active receiver-presave route owner after cleanup. | No backend receiver route child table. | `CI.FDA_REPORT_TYPE = 2`. | `referenceImportedToCase` | Keep in submission receiver option logic if implemented. |
| route condition value: `FDA(CBER IND)` | Reference bundle maps `FDA(CBER IND)` to value `3`. | No active receiver-presave route owner after cleanup. | No backend receiver route child table. | `CI.FDA_REPORT_TYPE = 3`. | `referenceImportedToCase` | Keep in submission receiver option logic if implemented. |
| route condition value: `FDA(Postmarket)` | Reference bundle maps `FDA(Postmarket)` to value `4`. | No active receiver-presave route owner after cleanup. | No backend receiver route child table. | `CI.FDA_REPORT_TYPE = 4`. | `referenceImportedToCase` | Keep in submission receiver option logic if implemented. |
| `message_headers.batch_receiver_identifier` | Reference selection is a submission receiver route; exact N header value is not displayed in the inspected submission UI bundle. | Explicit `routingBatchReceiverIdentifier` writes `batchReceiverIdentifier`. | `message_headers.batch_receiver_identifier`. | Message header receiver routing. | `localSystemOnly` | Keep explicit local submission input until a separate submission receiver option source is implemented. Do not source from receiver presave parent/children. |
| `message_headers.message_receiver_identifier` | Reference receiver route labels encode receiver variants and CI condition values; exact N.2.r.3 write path was not visible in inspected bundle. | Explicit `routingMessageReceiverIdentifier` writes `messageReceiverIdentifier`. | `message_headers.message_receiver_identifier`. | Message header receiver routing. | `localSystemOnly` | Keep explicit local submission input until a separate submission receiver option source is implemented. Do not source from receiver presave parent/children. |
| legacy `routingRules[]` | Not a reference payload shape. | Removed/ignored as receiver presave route source. | No backend receiver route child table. | None. | `removed` | Do not reintroduce. |
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

## Required Architecture Correction

Do not add `receiver_presave_routes`.

The 2026-06-05 receiver-detail and submission-bundle trace showed two separate reference owners:

- `INFO > RECEIVER` uses manufacturer master APIs and owns receiver master fields plus consignees.
- `SUB > SUBMISSION` uses submission receiver options and derives CI report-type condition rows from labels like `MFDS(KR)` and `FDA(CDER IND)`.

Backend receiver presave should therefore remain `parent + consignees`.

Future backend work, if local needs reference-equivalent receiver option selection, should create a submission receiver option source outside receiver presave. The minimum proven option data is:

- receiver option label, such as `MFDS(KR)` or `FDA(Postmarket)`
- authority/reporting family
- CI condition item derived from the label family: `MFDS_REPORT_TYPE` or `FDA_REPORT_TYPE`
- CI condition value derived from the label value
- local N.1.4/N.2.r.3 identifiers only if local export needs explicit message-header writing

Frontend migration should:

- Keep receiver presave master data out of `/api/cases/{caseId}/receiver` during submission routing.
- Keep explicit local N.1.4/N.2.r.3 submission inputs until a separate submission receiver option source exists.
- Remove or ignore receiver-presave `routingRules[]` / `routes[]` as submission route sources.
- Align day-count and NA fields to backend column names.

## Coverage Check

Reference fields: 30

Local frontend receiver presave fields: 33

Backend presave columns/tables and import targets: 49

Categorized matrix rows: 62

Uncategorized fields: 0

Ambiguous fields: 0
