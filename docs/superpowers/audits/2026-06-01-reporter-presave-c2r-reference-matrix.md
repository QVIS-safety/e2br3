# Reporter Presave C.2.r Reference Matrix

Date: 2026-06-01
Reference scope: live reporter presave UI, MFDS authority, English locale
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

The live reporter presave detail page did not expose Reporter's Email. Selecting
Qualification = 3: Other health professional did not reveal a C.2.r.4.KR.1
field in reporter presave.

## Field Matrix

| Local reporter presave field | E2B field | Live reference reporter presave | Case import behavior | Alignment classification |
| --- | --- | --- | --- | --- |
| reporterTitle | C.2.r.1.1 | Present | Import into Section C.2.r | Reference-imported |
| reporterGivenName | C.2.r.1.2 | Present, required | Import into Section C.2.r | Reference-imported |
| reporterMiddleName | C.2.r.1.3 | Present | Import into Section C.2.r | Reference-imported |
| reporterFamilyName | C.2.r.1.4 | Present | Import into Section C.2.r | Reference-imported |
| reporterOrganization | C.2.r.2.1 | Present, required | Import into Section C.2.r | Reference-imported |
| reporterDepartment | C.2.r.2.2 | Present | Import into Section C.2.r | Reference-imported |
| reporterStreet | C.2.r.2.3 | Present | Import into Section C.2.r | Reference-imported |
| reporterCity | C.2.r.2.4 | Present | Import into Section C.2.r | Reference-imported |
| reporterState | C.2.r.2.5 | Present | Import into Section C.2.r | Reference-imported |
| reporterPostcode | C.2.r.2.6 | Present | Import into Section C.2.r | Reference-imported |
| reporterTelephone | C.2.r.2.7 | Present | Import into Section C.2.r | Reference-imported |
| reporterEmail | C.2.r.2.8 / local FDA case field | Not present | Do not import from reporter presave | Case-only, remove from reporter presave |
| reporterCountry | C.2.r.3 | Present | Import into Section C.2.r | Reference-imported |
| qualification | C.2.r.4 | Present, required | Import into Section C.2.r | Reference-imported |
| qualificationKr1 | C.2.r.4.KR.1 / local MFDS case field | Not present | Do not import from reporter presave | Case-only, remove from reporter presave |
| primarySourceForRegulatoryPurposes | C.2.r.5 | Present | Import into Section C.2.r | Reference-imported |
| deleted | Presave management flag | Present | Preserve on template only | Reference-preserved |
| name | Template label | N/A | Preserve on template only | Local metadata |
| comments | Template notes | N/A | Preserve on template only | Local metadata |
| id, organizationId, audit fields | Persistence metadata | N/A | Preserve on template only | Local system metadata |

## Alignment Decision

Reporter presave templates should store and import only the fields visible in the
live reporter presave UI, plus local template metadata. `reporterEmail` and
`qualificationKr1` remain valid case-section fields for FDA/MFDS case editing,
but they should not be stored in reporter presave templates or populated through
reporter presave import.
