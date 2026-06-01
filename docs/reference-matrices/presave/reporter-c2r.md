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

| field | reference evidence | local frontend | local backend/BMC | case target | category | action |
| --- | --- | --- | --- | --- | --- | --- |
| reporterTitle | C.2.r.1.1; Present | Reporter Presave frontend surface. | ReporterPresave BMC/schema. | Import into Section C.2.r | referenceImportedToCase | Store in reporter presave and import into Section C.2.r. |
| reporterGivenName | C.2.r.1.2; Present, required | Reporter Presave frontend surface. | ReporterPresave BMC/schema. | Import into Section C.2.r | referenceImportedToCase | Store in reporter presave and import into Section C.2.r. |
| reporterMiddleName | C.2.r.1.3; Present | Reporter Presave frontend surface. | ReporterPresave BMC/schema. | Import into Section C.2.r | referenceImportedToCase | Store in reporter presave and import into Section C.2.r. |
| reporterFamilyName | C.2.r.1.4; Present | Reporter Presave frontend surface. | ReporterPresave BMC/schema. | Import into Section C.2.r | referenceImportedToCase | Store in reporter presave and import into Section C.2.r. |
| reporterOrganization | C.2.r.2.1; Present, required | Reporter Presave frontend surface. | ReporterPresave BMC/schema. | Import into Section C.2.r | referenceImportedToCase | Store in reporter presave and import into Section C.2.r. |
| reporterDepartment | C.2.r.2.2; Present | Reporter Presave frontend surface. | ReporterPresave BMC/schema. | Import into Section C.2.r | referenceImportedToCase | Store in reporter presave and import into Section C.2.r. |
| reporterStreet | C.2.r.2.3; Present | Reporter Presave frontend surface. | ReporterPresave BMC/schema. | Import into Section C.2.r | referenceImportedToCase | Store in reporter presave and import into Section C.2.r. |
| reporterCity | C.2.r.2.4; Present | Reporter Presave frontend surface. | ReporterPresave BMC/schema. | Import into Section C.2.r | referenceImportedToCase | Store in reporter presave and import into Section C.2.r. |
| reporterState | C.2.r.2.5; Present | Reporter Presave frontend surface. | ReporterPresave BMC/schema. | Import into Section C.2.r | referenceImportedToCase | Store in reporter presave and import into Section C.2.r. |
| reporterPostcode | C.2.r.2.6; Present | Reporter Presave frontend surface. | ReporterPresave BMC/schema. | Import into Section C.2.r | referenceImportedToCase | Store in reporter presave and import into Section C.2.r. |
| reporterTelephone | C.2.r.2.7; Present | Reporter Presave frontend surface. | ReporterPresave BMC/schema. | Import into Section C.2.r | referenceImportedToCase | Store in reporter presave and import into Section C.2.r. |
| reporterEmail | C.2.r.2.8 / FDA case field; Not present in reporter presave; present in FDA case reporter section | Reporter Presave frontend surface. | ReporterPresave BMC/schema. | Do not import from reporter presave | removed | Do not store in reporter presave or populate through reporter presave import. |
| reporterCountry | C.2.r.3; Present | Reporter Presave frontend surface. | ReporterPresave BMC/schema. | Import into Section C.2.r | referenceImportedToCase | Store in reporter presave and import into Section C.2.r. |
| qualification | C.2.r.4; Present, required | Reporter Presave frontend surface. | ReporterPresave BMC/schema. | Import into Section C.2.r | referenceImportedToCase | Store in reporter presave and import into Section C.2.r. |
| qualificationKr1 | C.2.r.4.KR.1 / local MFDS case field; Not present | Reporter Presave frontend surface. | ReporterPresave BMC/schema. | Do not import from reporter presave | removed | Do not store in reporter presave or populate through reporter presave import. |
| primarySourceForRegulatoryPurposes | C.2.r.5; Present | Reporter Presave frontend surface. | ReporterPresave BMC/schema. | Import into Section C.2.r | referenceImportedToCase | Store in reporter presave and import into Section C.2.r. |
| deleted | Presave management flag; Present | Reporter Presave frontend surface. | ReporterPresave BMC/schema. | Preserve on template only | referencePreserveOnly | Preserve on reporter presave only; do not import as case content. |
| name | Template label; N/A | Reporter Presave frontend surface. | ReporterPresave BMC/schema. | Preserve on template only | localSystemOnly | Keep as local template or persistence metadata only. |
| comments | Template notes; N/A | Reporter Presave frontend surface. | ReporterPresave BMC/schema. | Preserve on template only | localSystemOnly | Keep as local template or persistence metadata only. |
| id, organizationId, audit fields | Persistence metadata; N/A | Reporter Presave frontend surface. | ReporterPresave BMC/schema. | Preserve on template only | localSystemOnly | Keep as local template or persistence metadata only. |

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
