# Reporter Presave C.2.r Reference Matrix

Date: 2026-06-01
Reference scope: live reporter presave UI, MFDS authority, English locale; FDA authority cross-check through the live page bundle used by the reference UI
Local scope: reporter presave templates and import into Case Section C.2.r

## Live Reference Evidence

The live reporter presave detail page exposes these reporter fields:

- Reporter's Title (C.2.r.1.1)
- Reporter's Given Name (C.2.r.1.2)
- Reporter's Middle Name (C.2.r.1.3)
- Reporter's Family Name (C.2.r.1.4)
- Reporter's Organisation (C.2.r.2.1)
- Reporter's Department (C.2.r.2.2)
- Reporter's Street (C.2.r.2.3)
- Reporter's City (C.2.r.2.4)
- Reporter's State or Province (C.2.r.2.5)
- Reporter's Postcode (C.2.r.2.6)
- Reporter's Telephone (C.2.r.2.7)
- Reporter's Country Code (C.2.r.3)
- Qualification (C.2.r.4)
- Primary Source for Regulatory Purposes (C.2.r.5)
- Deleted

The live MFDS reporter presave detail page did not expose Reporter's Email.
Selecting Qualification = 3: Other health professional did not reveal a
C.2.r.4.KR.1 field in reporter presave.

FDA cross-check: the live reference page bundle for the case reporter section
does include the FDA-only C.2.r.2.8 `reporterEmail` case field, but the
case-info reporter presave component contains no `RPT_EMAIL`/`rptEmail` field.
That keeps Reporter's Email classified as a case-section field, not a reporter
presave field.

## Field Matrix

| field | reference evidence | canonical frontend source | canonical backend source | allowed read aliases | allowed write keys | case import target | duplicate sources found | category | decision | tests required |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| reporterTitle | C.2.r.1.1; Present | `ReporterPresaveData.reporterTitle`; `ReporterForm` input `reporterTitle`; `reporterPresaveSchema.reporterTitle` | `reporter_presaves.reporter_title`; `ReporterPresave.reporter_title`; `ReporterPresaveForCreate/Update.reporter_title` | `reporter_title`, `title` read only | `reporter_title` | `primarySources[].reporterTitle` | None | referenceImportedToCase | Keep one source and import to C.2.r. | Mapper/write mapper, form, backend API/model, Section C.2 import |
| reporterGivenName | C.2.r.1.2; Present, required | `ReporterPresaveData.reporterGivenName`; `ReporterForm` input `reporterGivenName`; `reporterPresaveSchema.reporterGivenName` | `reporter_presaves.reporter_given_name`; `ReporterPresave.reporter_given_name`; `ReporterPresaveForCreate/Update.reporter_given_name` | `reporter_given_name`, `givenName` read only | `reporter_given_name` | `primarySources[].reporterGivenName` | None | referenceImportedToCase | Keep one source and import to C.2.r. | Mapper/write mapper, required form validation, backend identity validation, Section C.2 import |
| reporterMiddleName | C.2.r.1.3; Present | `ReporterPresaveData.reporterMiddleName`; `ReporterForm` input `reporterMiddleName`; `reporterPresaveSchema.reporterMiddleName` | `reporter_presaves.reporter_middle_name`; `ReporterPresave.reporter_middle_name`; `ReporterPresaveForCreate/Update.reporter_middle_name` | `reporter_middle_name`, `middleName` read only | `reporter_middle_name` | `primarySources[].reporterMiddleName` | None | referenceImportedToCase | Keep one source and import to C.2.r. | Mapper/write mapper, form, backend API/model, Section C.2 import |
| reporterFamilyName | C.2.r.1.4; Present | `ReporterPresaveData.reporterFamilyName`; `ReporterForm` input `reporterFamilyName`; `reporterPresaveSchema.reporterFamilyName` | `reporter_presaves.reporter_family_name`; `ReporterPresave.reporter_family_name`; `ReporterPresaveForCreate/Update.reporter_family_name` | `reporter_family_name`, `familyName` read only | `reporter_family_name` | `primarySources[].reporterFamilyName` | None | referenceImportedToCase | Keep one source and import to C.2.r. | Mapper/write mapper, form, backend API/model, Section C.2 import |
| reporterOrganization | C.2.r.2.1; Present, required | `ReporterPresaveData.reporterOrganization`; `ReporterForm` input `reporterOrganization`; `reporterPresaveSchema.reporterOrganization` | `reporter_presaves.organization`; `ReporterPresave.organization`; `ReporterPresaveForCreate/Update.organization` | `organization`, `organizationName`, `organization_name` read only | `organization` | `primarySources[].reporterOrganization` | None | referenceImportedToCase | Keep one source and import to C.2.r. | Mapper/write mapper, required form validation, backend identity validation, Section C.2 import |
| reporterDepartment | C.2.r.2.2; Present | `ReporterPresaveData.reporterDepartment`; `ReporterForm` input `reporterDepartment`; `reporterPresaveSchema.reporterDepartment` | `reporter_presaves.department`; `ReporterPresave.department`; `ReporterPresaveForCreate/Update.department` | `department` | `department` | `primarySources[].reporterDepartment` | None | referenceImportedToCase | Keep one source and import to C.2.r. | Mapper/write mapper, form, backend API/model, Section C.2 import |
| reporterStreet | C.2.r.2.3; Present | `ReporterPresaveData.reporterStreet`; `ReporterForm` input `reporterStreet`; `reporterPresaveSchema.reporterStreet` | `reporter_presaves.street`; `ReporterPresave.street`; `ReporterPresaveForCreate/Update.street` | `street` | `street` | `primarySources[].reporterStreet` | None | referenceImportedToCase | Keep one source and import to C.2.r. | Mapper/write mapper, form, backend API/model, Section C.2 import |
| reporterCity | C.2.r.2.4; Present | `ReporterPresaveData.reporterCity`; `ReporterForm` input `reporterCity`; `reporterPresaveSchema.reporterCity` | `reporter_presaves.city`; `ReporterPresave.city`; `ReporterPresaveForCreate/Update.city` | `city` | `city` | `primarySources[].reporterCity` | None | referenceImportedToCase | Keep one source and import to C.2.r. | Mapper/write mapper, form, backend API/model, Section C.2 import |
| reporterState | C.2.r.2.5; Present | `ReporterPresaveData.reporterState`; `ReporterForm` input `reporterState`; `reporterPresaveSchema.reporterState` | `reporter_presaves.state`; `ReporterPresave.state`; `ReporterPresaveForCreate/Update.state` | `state` | `state` | `primarySources[].reporterState` | None | referenceImportedToCase | Keep one source and import to C.2.r. | Mapper/write mapper, form, backend API/model, Section C.2 import |
| reporterPostcode | C.2.r.2.6; Present | `ReporterPresaveData.reporterPostcode`; `ReporterForm` input `reporterPostcode`; `reporterPresaveSchema.reporterPostcode` | `reporter_presaves.postcode`; `ReporterPresave.postcode`; `ReporterPresaveForCreate/Update.postcode` | `postcode` | `postcode` | `primarySources[].reporterPostcode` | None | referenceImportedToCase | Keep one source and import to C.2.r. | Mapper/write mapper, form, backend API/model, Section C.2 import |
| reporterTelephone | C.2.r.2.7; Present | `ReporterPresaveData.reporterTelephone`; `ReporterForm` input `reporterTelephone`; `reporterPresaveSchema.reporterTelephone` | `reporter_presaves.telephone`; `ReporterPresave.telephone`; `ReporterPresaveForCreate/Update.telephone` | `telephone` | `telephone` | `primarySources[].reporterTelephone` | None | referenceImportedToCase | Keep one source and import to C.2.r. | Mapper/write mapper, form, backend API/model, Section C.2 import |
| reporterEmail | C.2.r.2.8 / FDA case field; Not present in reporter presave; present in FDA case reporter section | None in reporter presave; case field only | None in reporter presave; `reporter_presaves.email` drop SQL only | None | None | Empty; case C.2.r field only | Legacy local presave drift removed | removed | Remove from reporter presave; keep only as case C.2.r.2.8 field. | Form absence, read/write mapper rejection, backend column absence/API ignore, Section C.2 negative import |
| reporterCountry | C.2.r.3; Present | `ReporterPresaveData.reporterCountry`; `ReporterForm` control `reporterCountry`; `reporterPresaveSchema.reporterCountry` | `reporter_presaves.country_code`; `ReporterPresave.country_code`; `ReporterPresaveForCreate/Update.country_code` | `country_code`, `countryCode` read only | `country_code` | `primarySources[].reporterCountry` | None | referenceImportedToCase | Keep one source and import to C.2.r. | Mapper/write mapper, country selector form test, backend API/model, Section C.2 import |
| qualification | C.2.r.4; Present, required | `ReporterPresaveData.qualification`; `ReporterForm` radio group `qualification`; `reporterPresaveSchema.qualification` | `reporter_presaves.qualification`; `ReporterPresave.qualification`; `ReporterPresaveForCreate/Update.qualification` | `qualification` | `qualification` | `primarySources[].qualification` | None | referenceImportedToCase | Keep one source and import to C.2.r. | Mapper/write mapper, required form validation, backend identity validation, Section C.2 import |
| qualificationKr1 | C.2.r.4.KR.1 / local MFDS case field; Not present | None in reporter presave; case field only | None in reporter presave; `reporter_presaves.qualification_kr1` drop SQL only | None | None | Empty; case C.2.r field only | Legacy local presave drift removed | removed | Remove from reporter presave; keep only as case C.2.r.4.KR.1 field. | Form absence, read/write mapper rejection, backend column absence/API ignore, Section C.2 negative import |
| primarySourceForRegulatoryPurposes | C.2.r.5; Present | `ReporterPresaveData.primarySourceForRegulatoryPurposes`; `ReporterForm` radio group `primarySourceForRegulatoryPurposes`; `reporterPresaveSchema.primarySourceForRegulatoryPurposes` | `reporter_presaves.primary_source_regulatory`; `ReporterPresave.primary_source_regulatory`; `ReporterPresaveForCreate/Update.primary_source_regulatory` | `primary_source_regulatory` read only | `primary_source_regulatory` | `primarySources[].primarySourceForRegulatoryPurposes` | None | referenceImportedToCase | Keep one source and import to C.2.r. | Mapper/write mapper, form, backend API/model, Section C.2 import |
| deleted | Presave management flag; Present | `ReporterPresaveData.deleted`; `ReporterForm` checkbox `deleted`; `reporterPresaveSchema.deleted` | `reporter_presaves.deleted`; `ReporterPresave.deleted`; `ReporterPresaveForUpdate.deleted` | `deleted` | `deleted` | Empty | None | referencePreserveOnly | Preserve on reporter presave only; do not import as case content. | Form, mapper/write mapper, API soft-delete, no case import |
| name | Template label; N/A | Presave create/update wrapper `name` | `reporter_presaves.name` | wrapper `name` | `name` | Empty | None | localSystemOnly | Keep as local template metadata only. | API create/update/list identity tests |
| comments | Template notes; N/A | Presave create/update wrapper `comments` or `description` compatibility | `reporter_presaves.comments` | wrapper `comments`, `description` compatibility | `comments` | Empty | None | localSystemOnly | Keep as local template metadata only. | API create/update/list metadata tests |
| id, organizationId, audit fields | Persistence metadata; N/A | Template identity, tenancy, audit metadata | `reporter_presaves.id`, `organization_id`, `created_at`, `updated_at`, `created_by`, `updated_by` | REST response metadata only | Empty | Empty | None | localSystemOnly | Keep as local persistence metadata only. | Backend RLS, audit, and organization isolation tests |

## Alignment Decision

Reporter presave templates should store and import only the fields visible in the
live reporter presave UI, plus local template metadata. `reporterEmail` and
`qualificationKr1` remain valid case-section fields for FDA/MFDS case editing,
but they should not be stored in reporter presave templates or populated through
reporter presave import.

## Coverage Check

- Total categorized rows: 20
- Uncategorized fields: 0
- Ambiguous fields: 0
