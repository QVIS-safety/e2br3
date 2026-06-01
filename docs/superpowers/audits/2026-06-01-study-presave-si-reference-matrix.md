# Study Presave -> SI Reference Matrix

Date: 2026-06-01

## Scope

Reference routes:
- Study Presave list: `/:sponsorKey/:sessionSenderKey/:language/:appendix/caseinfo/study/list`
- Study Presave detail: `/:sponsorKey/:sessionSenderKey/:language/:appendix/caseinfo/study/:studyKey/detail`
- Case SI import workflow: case SI `C.5` library button -> Study select dialog -> `GET /1.1/sponsors/{sponsorKey}/studies/{studyKey}?CaseStudy`

Local modules compared:
- Frontend types/schema/mappers/forms: `lib/types/presave.ts`, `lib/schemas/presave.ts`, `lib/presave/canonicalMappers.ts`, `lib/presave/canonicalWriteMappers.ts`, `components/presave/StudyForm.tsx`
- Frontend SI import: `components/case-form/sections/SectionStudy.tsx`
- Backend model/schema/REST details: `crates/libs/lib-core/src/model/presave.rs`, `crates/services/web-server/src/web/rest/section_presave_rest.rs`, `db/bootstrap/01-safetydb-schema.sql`

## Reference Evidence

- UI list route is `CaseInfoStudyList`; visible list columns are `No.`, `Sponsor Study No.`, `Study Name`, `Study Type`, `EDC Sync`, `Product`, and `Deleted`.
- Bundle route evidence maps Study list/detail to `CaseInfo2` chunks and uses:
  - `GET /1.0/sponsors/{sponsor}/studies`
  - `GET /1.0/sponsors/{sponsor}/studies/{study}`
  - `PUT /1.0/sponsors/{sponsor}/studies/{study}`
  - `GET /1.1/sponsors/{sponsor}/studies/options`
  - `GET /1.1/sponsors/{sponsor}/studies/{study}?CaseStudy` for SI import.
- Reference Study Presave detail fields from `CaseInfo2.613bce31.js`:
  - Product selector: `selectedProducts[]`, submitted as `products[]` with `prdKey`, `prdName`.
  - Registration repeat table: `registrations[].stdRegNo`, `registrations[].stdRegCountry`, `registrations[].delFlag`.
  - `stdName` and `studyInfoNotation.stdName`.
  - `stdSpnNo` and KR-only `stdOid` selector.
  - `stdReactionType`.
  - `sync`, `excludeCaseKeyFromSync`, `delFlag`.
  - Reporter repeat table: `reporters[].rptKey`, `rptOrgan`, `rptGivName`, `rptQualify`, `delFlag`.
- Reference SI import from `CaseDetail1.b5394ea0.js`:
  - Opens Study select dialog, then calls `retrieveCaseSiStudyInfo(sponsorKey, STD_KEY)`.
  - Imports `registrations[].stdRegNo` -> `STD_REG_NO`.
  - Imports `registrations[].stdRegCountry` -> `STD_REG_COUNTRY`.
  - Imports `registrations[].delFlag` into the case registration row deleted state.
  - Imports `stdName` and `studyInfoNotation.stdName` into `C.5.2`.
  - Imports `stdSpnNo` into `C.5.3`.
  - Imports `stdReactionType` into `C.5.4`.
  - Imports `stdOid` into the KR-visible study number OID selector.
  - Does not import Study Presave product rows, EDC sync flags, deleted flag, reporters, FDA C.5.5/C.5.6 fields, or MFDS C.5.4.KR.1.

## Matrix

| field | reference evidence | local frontend | local backend/BMC | case target | category | action |
|---|---|---|---|---|---|---|
| Reference `STD_KEY` / local `id` | Study list/detail key and select dialog key. | `StudyPresaveData.id`. | `study_presaves.id`. | Selector identity only. | `localSystemOnly` | Keep as local identity. |
| `organization_id` | Reference sponsor route scopes data. | Not exposed. | `study_presaves.organization_id` and RLS policies. | None. | `localSystemOnly` | Keep. |
| `name` | Local canonical template name, not visible as reference Study field. | Base presave name. | `study_presaves.name`. | None. | `localSystemOnly` | Keep as local template identity. |
| `comments` | No reference Study detail evidence. | Base comments/description path only. | `study_presaves.comments`. | None. | `localSystemOnly` | Keep only as local template metadata. |
| Reference `products[].prdKey` / `products[].prdName` | Detail product selector; required; submitted from `selectedProducts[]`; list column `Product`. | `studyProducts[]` maps `productPresaveId`, `productName`, `deleted`, row metadata; selected product writes a child row. | `study_presave_products` with REST/details graph and RLS/audit triggers. | Not imported to SI. | `referencePreserveOnly` | Wired as preserve-only child rows. |
| `product_presave_id` / `productPresaveId` | Local normalized FK for current required Study identity and product lookup. | Used by Study form and write mapper; child row also stores selected product FK. | `study_presaves.product_presave_id`; required for local identity. | None. | `localSystemOnly` | Keep as local identity/search helper while product rows carry reference preserve-only relationship. |
| `productId` | No reference field with this local canonical name; display-only value copied from Product Presave selector. | Present only as local form display state; no canonical details parent write. | No backend column. | None. | `localSystemOnly` | Keep only as UI display helper, not a stored/imported Study Presave field. |
| `productName` | Reference has product display name inside `products[].prdName`, not parent `productName`. | Present only as selected product display state; child row persists `productName`. | No backend parent column. | None. | `localSystemOnly` | Keep only as UI display helper. |
| `studyProductName` | No reference parent field; local display compatibility alias. | Present only as UI display state. | No backend column. | None. | `localSystemOnly` | Keep only as UI display helper. |
| `studyProducts[].id` / `sequenceNumber` | Reference product row order. | Mapped and written. | `study_presave_products.id`, `sequence_number`. | None. | `localSystemOnly` | Keep as persistence mechanics. |
| `studyProducts[].productPresaveId` / `productName` | Reference product row key/name. | Mapped and written. | `study_presave_products.product_presave_id`, `product_name`. | None. | `referencePreserveOnly` | Wired. |
| `studyProducts[].deleted` / `_delete` | Reference product row deleted state. | Mapped and written. | `study_presave_products.deleted`. | None. | `referencePreserveOnly` | Wired. |
| Reference `registrations[].rowNo` / local `sequenceNumber` | Reference repeat row order. | `studyRegistrations[].sequenceNumber`; write mapper emits `sequence_number`. | `study_presave_registration_numbers.sequence_number`. | Case repeat row order. | `localSystemOnly` | Keep sequence as persistence/import mechanics. |
| `registrations[].stdRegNo` / `studyRegistrations[].registrationNumber` | Reference imports into SI `STD_REG_NO`; max length 50. | Imported to `studyInformation.studyRegistrationNumbers[].registrationNumber`. | `study_presave_registration_numbers.registration_number` currently `VARCHAR(255)`. | `C.5.1.r.1`. | `referenceImportedToCase` | Keep import; align max length to 50. |
| `registrations[].stdRegCountry` / `studyRegistrations[].registrationCountry` | Reference imports into SI `STD_REG_COUNTRY`. | Imported to `studyInformation.studyRegistrationNumbers[].countryCode`. | `study_presave_registration_numbers.country_code`. | `C.5.1.r.2`. | `referenceImportedToCase` | Keep exact country mapping. |
| `registrations[].delFlag` / `studyRegistrations[].deleted` | Reference imports deleted state into case registration row state. | Local filters deleted rows out before import. | Child table has `deleted`. | Case registration repeat row deleted state. | `referenceImportedToCase` | Change import tests/code to preserve deleted row state instead of silently filtering. |
| `_delete` on study registration rows | Local transient client mutation flag. | Present in type/import filter. | Not persisted. | None. | `localSystemOnly` | Keep transiently if used by details update; do not expose as reference field. |
| `stdName` / `studyName` | Reference imports into SI `C.5.2`. | Required and imported to `studyInformation.studyName`. | `study_presaves.study_name`. | `C.5.2`. | `referenceImportedToCase` | Keep. |
| `studyInfoNotation.stdName` / `studyNameNotation` | Reference imports notation with `onLoad(stdName, notation)`. | Preserved in Study form/schema/write mapper but not imported by `SectionStudy`. | `study_presaves.study_name_notation`. | `C.5.2` notation. | `referenceImportedToCase` | Add section import test and mapping for study name notation. |
| `stdSpnNo` / `sponsorStudyNumber` | Reference imports into SI `STD_SPN_NO`; max length 50. | Required and imported. | `study_presaves.sponsor_study_number` currently `VARCHAR(100)`. | `C.5.3`. | `referenceImportedToCase` | Keep import; align max length to 50. |
| `stdOid` / local `sponsorStudyNumberKind` | Reference KR-visible OID selector imported into case study number OID; default `PROTOCOL_NO`; code list `STD_OID`. | Local uses enum `study_no`/`protocol_no`; not imported to SI. | `sponsor_study_number_kind` with local check constraint. | KR `STD_OID` selector on `C.5.3`. | `referenceImportedToCase` | Replace local enum semantics with reference `stdOid`/code-list values and import to case. |
| `stdReactionType` / `studyTypeReaction` | Reference imports into SI `STD_REACTION_TYPE`. | Required and imported to `studyInformation.studyTypeReaction`. | `study_presaves.study_type_reaction`. | `C.5.4`. | `referenceImportedToCase` | Keep. |
| `sync` / `edcSync` | Reference Study detail switch and list column; not imported by SI workflow. | Present as `edcSync`. | `study_presaves.edc_sync`. | None. | `referencePreserveOnly` | Keep as preserve-only. |
| `excludeCaseKeyFromSync` | Reference Study detail switch; not imported by SI workflow. | `excludeCaseKeyFromSync` maps to parent payload. | `study_presaves.exclude_case_key_from_sync`. | None. | `referencePreserveOnly` | Wired as preserve-only. |
| `delFlag` / `deleted` | Reference Study detail switch and list column; not imported to SI. | `deleted`. | `study_presaves.deleted`. | None. | `referencePreserveOnly` | Keep. |
| `reporters[].rowNo` | Reference reporter repeat row order. | `studyReporters[].sequenceNumber` maps and writes. | `study_presave_reporters.sequence_number`. | None. | `referencePreserveOnly` | Wired. |
| `reporters[].rptKey` | Reference reporter selector key. | `studyReporters[].reporterPresaveId` maps and writes. | `study_presave_reporters.reporter_presave_id`. | None. | `referencePreserveOnly` | Wired. |
| `reporters[].rptOrgan` | Reference displays reporter organization. | `studyReporters[].reporterOrganization` maps and writes. | `study_presave_reporters.reporter_organization`. | None. | `referencePreserveOnly` | Wired. |
| `reporters[].rptGivName` | Reference displays reporter given name. | `studyReporters[].reporterGivenName` maps and writes. | `study_presave_reporters.reporter_given_name`. | None. | `referencePreserveOnly` | Wired. |
| `reporters[].rptQualify` | Reference displays reporter qualification. | `studyReporters[].reporterQualification` maps and writes. | `study_presave_reporters.reporter_qualification`. | None. | `referencePreserveOnly` | Wired. |
| `reporters[].delFlag` | Reference reporter row deleted state. | `studyReporters[].deleted` / `_delete` maps and writes. | `study_presave_reporters.deleted`. | None. | `referencePreserveOnly` | Wired. |
| `studyTypeReactionKr1` / `mfdsOtherStudiesType` | Reference SI has C.5.4.KR.1, but Study Presave detail and SI import do not set it from Study Presave. | Present and currently imported into SI. | `study_presaves.study_type_reaction_kr1`. | None from Study Presave. | `removed` | Remove from Study Presave and section import path; keep only as case SI field. |
| `mfdsStudyNumber` | No reference Study detail or SI import field. | Present; form strips before submit. | `study_presaves.mfds_study_number`. | None. | `removed` | Remove. |
| `mfdsProtocolNumber` | No reference Study detail or SI import field. | Present; form strips before submit. | `study_presaves.mfds_protocol_number`. | None. | `removed` | Remove. |
| `fdaIndNumberOccurred` | Reference SI has FDA.C.5.5a, but Study Presave detail and SI import do not set it from Study Presave. | Present and currently imported into SI. | `study_presaves.fda_ind_number_occurred`. | None from Study Presave. | `removed` | Remove from Study Presave and section import path; keep only as case SI field. |
| `fdaPreAndaNumberOccurred` | Reference SI has FDA.C.5.5b, but Study Presave detail and SI import do not set it from Study Presave. | Present and currently imported into SI. | `study_presaves.fda_pre_anda_number_occurred`. | None from Study Presave. | `removed` | Remove from Study Presave and section import path; keep only as case SI field. |
| `fdaCrossReportedIndNumbers[].id` | Local child row ID only; reference Study Presave has no FDA cross IND child table. | Present. | `study_presave_fda_cross_reported_inds.id`. | None from Study Presave. | `removed` | Remove with the FDA child table. |
| `fdaCrossReportedIndNumbers[].sequenceNumber` | Local child row order only for unsupported child table. | Present. | `study_presave_fda_cross_reported_inds.sequence_number`. | None from Study Presave. | `removed` | Remove with the FDA child table. |
| `fdaCrossReportedIndNumbers[].indNumber` | Reference SI has FDA.C.5.6.r, but Study Presave detail and SI import do not set it from Study Presave. | Present and currently imported into SI. | `study_presave_fda_cross_reported_inds.ind_number`. | None from Study Presave. | `removed` | Remove from Study Presave and section import path; keep only as case SI field. |
| `fdaCrossReportedIndNumbers[].deleted` / `_delete` | Local row state for unsupported child table. | Present. | `study_presave_fda_cross_reported_inds.deleted`. | None. | `removed` | Remove with the FDA child table. |
| `created_at`, `updated_at`, `created_by`, `updated_by` | Reference has audit trail buttons, but these raw columns are local persistence metadata. | Not exposed directly. | Present on parent/child tables. | None. | `localSystemOnly` | Keep audit metadata for retained tables. |

## Category Summary

`referenceImportedToCase`:
`studyRegistrations[].registrationNumber`, `studyRegistrations[].registrationCountry`, `studyRegistrations[].deleted`, `studyName`, `studyNameNotation`, `sponsorStudyNumber`, `sponsorStudyNumberKind/stdOid`, `studyTypeReaction`

`referencePreserveOnly`:
`products[]`, `studyProducts[].productPresaveId`, `studyProducts[].productName`, `studyProducts[].deleted`, `edcSync`, `excludeCaseKeyFromSync`, `deleted`, `reporters[]`, `studyReporters[].reporterPresaveId`, `studyReporters[].reporterOrganization`, `studyReporters[].reporterGivenName`, `studyReporters[].reporterQualification`, `studyReporters[].deleted`

`localSystemOnly`:
`id`, `organization_id`, `name`, `comments`, `productPresaveId`, `productId` display helper, `productName` display helper, `studyProductName` display helper, product `id`/`sequenceNumber`, registration `sequenceNumber`, registration `_delete`, reporter `id`/`sequenceNumber`, audit metadata

`removed`:
`studyProducts[]` legacy aliases (`templateId`, `productId`, `medicinalProduct`, `drugBrandName`), `mfdsOtherStudiesType`, `mfdsStudyNumber`, `mfdsProtocolNumber`, `fdaIndNumberOccurred`, `fdaPreAndaNumberOccurred`, `fdaCrossReportedIndNumbers[]`

## Coverage Check

- Total reference fields/groups: 22
- Total local fields/groups: 43
- Total categorized rows: 43
- Uncategorized fields: 0
- Ambiguous fields: 0
