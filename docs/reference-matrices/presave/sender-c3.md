# Sender Presave -> C.3 Reference Matrix

## Scope

Sender Presave master data and import into the case Sender section, `C.3 - Information on Sender of Case Safety Report`, plus the FDA/MFDS gateway value used for `messageHeader.messageSenderIdentifier`.

Reference basis:

- Live reference UI inspection on 2026-06-01 at `edu-safetyr3.crscube.io/57/508/en/KR/caseinfo/sender/508/detail`.
- Existing local C.3 import behavior in `frontend/E2BR3-frontend/components/case-form/sections/SectionC3.tsx`.
- Existing backend canonical sender schema in `sender_presaves`, `sender_presave_gateways`, and `sender_presave_responsible_persons`.

The live reference also shows PMDA/NMPA/EMA gateway rows; those are intentionally out of local scope. Local sender gateway alignment is limited to FDA and MFDS.

## Matrix

| field | reference evidence | local frontend | local backend/BMC | case target | category | action |
| --- | --- | --- | --- | --- | --- | --- |
| `senderType` | C.3.1 sender type | Canonical presave frontend surface; see current section implementation. | `sender_presaves.sender_type` | `safetyReportIdentification.senderType` | `referenceImportedToCase` | import exactly |
| `senderOrganization` | C.3.2 sender organisation | Canonical presave frontend surface; see current section implementation. | `sender_presaves.organization_name` | `safetyReportIdentification.senderOrganization` | `referenceImportedToCase` | import exactly |
| `senderOrganizationNotation` | live detail shows C.3.2 `Notation` under Sender's Organisation | Canonical presave frontend surface; see current section implementation. | `sender_presaves.organization_name_notation` | no current local case notation field | `referencePreserveOnly` | preserve in sender presave |
| `senderDepartment` | C.3.3.1 sender department | Canonical presave frontend surface; authored on responsible-person rows. | `sender_presave_responsible_persons.department` | `safetyReportIdentification.senderDepartment` | `referenceImportedToCase` | import default responsible row |
| `senderPersonTitle` | C.3.3.2 sender title | Canonical presave frontend surface; see current section implementation. | `sender_presave_responsible_persons.person_title` | `safetyReportIdentification.senderPersonTitle` | `referenceImportedToCase` | import default responsible row |
| `senderPersonGivenName` | C.3.3.3 sender given name | Canonical presave frontend surface; see current section implementation. | `sender_presave_responsible_persons.person_given_name` | `safetyReportIdentification.senderPersonGivenName` | `referenceImportedToCase` | import default responsible row when present |
| `senderPersonMiddleName` | C.3.3.4 sender middle name | Canonical presave frontend surface; see current section implementation. | `sender_presave_responsible_persons.person_middle_name` | `safetyReportIdentification.senderPersonMiddleName` | `referenceImportedToCase` | import default responsible row |
| `senderPersonFamilyName` | C.3.3.5 sender family name | Canonical presave frontend surface; see current section implementation. | `sender_presave_responsible_persons.person_family_name` | `safetyReportIdentification.senderPersonFamilyName` | `referenceImportedToCase` | import default responsible row |
| `senderStreetAddress` | C.3.4.1 street | Canonical presave frontend surface; see current section implementation. | `sender_presaves.street_address` | `safetyReportIdentification.senderStreetAddress` | `referenceImportedToCase` | import exactly |
| `senderCity` | C.3.4.2 city | Canonical presave frontend surface; see current section implementation. | `sender_presaves.city` | `safetyReportIdentification.senderCity` | `referenceImportedToCase` | import exactly |
| `senderState` | C.3.4.3 state | Canonical presave frontend surface; see current section implementation. | `sender_presaves.state` | `safetyReportIdentification.senderState` | `referenceImportedToCase` | import exactly |
| `senderPostcode` | C.3.4.4 postcode | Canonical presave frontend surface; see current section implementation. | `sender_presaves.postcode` | `safetyReportIdentification.senderPostcode` | `referenceImportedToCase` | import exactly |
| `senderCountryCode` | C.3.4.5 country | Canonical presave frontend surface; see current section implementation. | `sender_presaves.country_code` | `safetyReportIdentification.senderCountryCode` | `referenceImportedToCase` | import exactly |
| `senderTelephone` | C.3.4.6 telephone | Canonical presave frontend surface; see current section implementation. | `sender_presaves.telephone` | `safetyReportIdentification.senderTelephone` | `referenceImportedToCase` | import exactly |
| `senderFax` | C.3.4.7 fax | Canonical presave frontend surface; see current section implementation. | `sender_presaves.fax` | `safetyReportIdentification.senderFax` | `referenceImportedToCase` | import exactly |
| `senderEmail` | C.3.4.8 email | Canonical presave frontend surface; see current section implementation. | `sender_presaves.email` | `safetyReportIdentification.senderEmail` | `referenceImportedToCase` | import exactly |
| `regulatorGateways[].senderId` | FDA/MFDS Sender Id rows | Canonical presave frontend surface; see current section implementation. | `sender_presave_gateways.sender_identifier` | `messageHeader.messageSenderIdentifier` for matching authority | `referenceImportedToCase` | import exact FDA/MFDS authority row |
| `regulatorGateways[].as2RoutingId` | FDA/MFDS AS2 routing row value | Canonical presave frontend surface; see current section implementation. | `sender_presave_gateways.routing_identifier` | none | `referencePreserveOnly` | preserve, do not import to C.3 |
| `regulatorGateways[].cdeSenderIdentifier` | NMPA CDE row value; local backend stores child field | Canonical presave frontend surface; see current section implementation. | `sender_presave_gateways.cde_sender_identifier` | none | `referencePreserveOnly` | preserve if present |
| `regulatorGateways[].cdrSenderIdentifier` | NMPA CDR row value; local backend stores child field | Canonical presave frontend surface; see current section implementation. | `sender_presave_gateways.cdr_sender_identifier` | none | `referencePreserveOnly` | preserve if present |
| `regulatorGateways[].isDefaultForAuthority` | live default selector on gateway rows | Canonical presave frontend surface; see current section implementation. | `sender_presave_gateways.is_default_for_authority` | row selection only | `referencePreserveOnly` | persist from UI default choice |
| `senderPersons[].isDefault` | live default selector on responsible-person rows | Canonical presave frontend surface; see current section implementation. | `sender_presave_responsible_persons.is_default` | row selection only | `referencePreserveOnly` | persist from UI default choice |
| `linkedOrganizationName` / `linkedOrganizationType` | no live reference sender field | Canonical presave frontend surface; see current section implementation. | none | none | `removed` | remove from sender presave path |
| parent `cdrSenderId` | CDR belongs to gateway child row | Canonical presave frontend surface; see current section implementation. | none | none | `removed` | remove parent field |
| `regulatorGatewayDefaultIndex` / `senderPersonDefaultIndex` | local UI-only indices | Canonical presave frontend surface; see current section implementation. | none | none | `removed` | replace with row booleans |

## Coverage Check

- FDA/MFDS live-relevant sender fields are classified.
- Local-only drift fields are classified as removed.
- PMDA/NMPA/EMA are explicitly out of scope.
- Total categorized rows: 25
- Uncategorized fields: 0
- Ambiguous fields: 0
