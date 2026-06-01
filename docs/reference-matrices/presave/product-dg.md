# Product Presave Final Alignment Matrix

## Scope

This matrix classifies every current local Product Presave field against reference Product Presave and DG import evidence. `dgPrdKey` is excluded because it is case-owned DG state, not a Product Presave field.

Statuses:

- `referenceImportedToCase`: exists in reference Product Presave and is copied into DG by `DG_PRD_KEY` import.
- `referencePreserveOnly`: exists in reference Product Presave, but the reference import does not copy it into DG.
- `localSystemOnly`: local-only support field with a bounded local reason.
- `removed`: local drift; removed from Product Presave unless a separate requirement reintroduces it outside this workflow.

## Matrix

| field | reference evidence | local frontend | local backend/BMC | case target | category | action |
| --- | --- | --- | --- | --- | --- | --- |
| `productId` | internal reference `PRD_KEY`, not user-entered | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `localSystemOnly` | Keep only if needed as local template identity/display. Do not import to DG. |
| `drugCharacterization` | none found in reference Product Presave | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `removed` | Remove from Product Presave and stop importing to DG. |
| `medicinalProduct` | `productDrug.productName` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | DG product name `G.k.2.2` | `referenceImportedToCase` | Import exactly. |
| `medicinalProductNotation` | `productDrugNotation.productName` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | DG product name notation | `referenceImportedToCase` | Import exactly; add missing DG import if not wired. |
| `drugBrandName` | `brandName` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `referencePreserveOnly` | Keep as Product Presave metadata; stop importing to DG. |
| `drugGenericName` | none found in reference Product Presave | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `removed` | Remove from Product Presave and stop importing to DG. |
| `obtainDrugCountry` | `productDrug.country` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | DG country where drug was obtained `G.k.2.4` | `referenceImportedToCase` | Import exactly. |
| `drugAuthorizationCountry` | `productDrug.authNationCode` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | DG authorisation/application country `G.k.3.2` | `referenceImportedToCase` | Import exactly. |
| `drugAuthorizationHolder` | `productDrug.authName` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | DG holder/applicant name `G.k.3.3` | `referenceImportedToCase` | Import exactly. |
| `drugAuthorizationNumber` | `productDrug.authNumber` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | DG authorisation/application number `G.k.3.1` | `referenceImportedToCase` | Import exactly. |
| `drugBatchNumber` | none found in reference Product Presave | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `removed` | Remove from Product Presave and stop importing to DG. |
| `manufacturerName` | overlaps reference `mnftName` metadata | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `removed` | Replace with `originalManufacturer`/manufacturer metadata only; do not use as fallback for holder/applicant. |
| `manufacturerCountry` | none found in reference Product Presave | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `removed` | Remove from Product Presave and stop using as fallback. |
| `preApprovalIpName` | `prdName` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `referencePreserveOnly` | Keep as Product Presave metadata; do not import to DG. |
| `originalManufacturer` | `mnftKey`, `mnftName` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `referencePreserveOnly` | Keep as Product Presave metadata; do not import to DG. |
| `sender` | `senderName` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `referencePreserveOnly` | Keep as Product Presave metadata/display; do not import to DG. |
| `senderPresaveId` | `senderKey` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `referencePreserveOnly` | Keep as Product Presave sender link; do not import to DG. |
| `productDescription` | `description` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `referencePreserveOnly` | Keep as Product Presave metadata; do not import to DG. |
| `productDeleted` | `delFlag` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `referencePreserveOnly` | Keep as Product Presave metadata; do not import to DG. |
| `mpidVersionDateNumber` | `productDrug.mpidVersion` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | DG MPID version `G.k.2.1.1a` | `referenceImportedToCase` | Import exactly. |
| `mpid` | `productDrug.mpid` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | DG MPID `G.k.2.1.1b` | `referenceImportedToCase` | Import exactly. |
| `mfdsMpidVersion` | `productDrug.mfdsMpidVersion` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | KR DG MFDS MPID version `G.k.2.1.KR.1a` | `referenceImportedToCase` | Import exactly for KR/MFDS when present. |
| `mfdsMpid` | `productDrug.mfdsMpid` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | KR DG MFDS MPID `G.k.2.1.KR.1b` | `referenceImportedToCase` | Import exactly for KR/MFDS when present. |
| `mfdsDeviceInfo` | `productMedicalDeviceKr` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | KR/MFDS DG medical-device graph | `referenceImportedToCase` | Import the full KR device graph. |
| `mfdsDeviceItems` | local flattened storage backing for `productMedicalDeviceKr` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none directly | `localSystemOnly` | Keep only as internal storage/normalization backing for `mfdsDeviceInfo`; do not expose as a separate generic Product Presave concept. |
| `phpidVersionDateNumber` | `productDrug.phpidVersion` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | DG PhPID version `G.k.2.1.2a` | `referenceImportedToCase` | Import exactly. |
| `phpid` | `productDrug.phpid` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | DG PhPID `G.k.2.1.2b` | `referenceImportedToCase` | Import exactly. |
| `mfdsStudyNumber` | none in reference Product Presave; Study/C.5 domain | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `removed` | Remove from Product Presave. Belongs to Study Presave if needed. |
| `mfdsProtocolNumber` | none in reference Product Presave; Study/C.5 domain | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `removed` | Remove from Product Presave. Belongs to Study Presave if needed. |
| `mfdsOtherStudiesType` | none in reference Product Presave; Study/C.5 domain | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `removed` | Remove from Product Presave. Belongs to Study Presave if needed. |
| `fdaIndNumberOccurred` | none in reference Product Presave; Study/C.5 domain | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `removed` | Remove from Product Presave. Belongs to Study Presave if needed. |
| `fdaPreAndaNumberOccurred` | none in reference Product Presave; Study/C.5 domain | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `removed` | Remove from Product Presave. Belongs to Study Presave if needed. |
| `fdaCrossReportedIndNumbers` | none in reference Product Presave; Study/C.5 domain | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `removed` | Remove from Product Presave. Belongs to Study Presave if needed. |
| `substances` | `substances[]` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | DG active-substance rows `G.k.2.3.r`, including KR MFDS substance ID/version | `referenceImportedToCase` | Import exactly, replacing DG substance rows like reference behavior. |
| `mfdsRegionalItems` | none; reference uses explicit `productMedicalDeviceKr` and MFDS drug fields | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `removed` | Remove generic regional item storage for Product Presave -> DG. |
| `drugObtainedCountry` | none; alias only | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `removed` | Remove alias and fallback behavior. |
| `investigationalProductBlinded` | `productDrug.blind` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | DG investigational product blinded `G.k.2.5` | `referenceImportedToCase` | Import exactly. |
| `authorizationNumber` | none; alias only | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `removed` | Remove alias and fallback behavior. |
| `authorizationCountry` | none; alias only | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `removed` | Remove alias and fallback behavior. |
| `holderApplicantName` | none; alias only | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | none | `removed` | Remove alias and fallback behavior. |
| `holderApplicantNameNotation` | `productDrugNotation.authName` | Product Presave frontend surface. | ProductPresave BMC/schema or child graph. | DG holder/applicant notation | `referenceImportedToCase` | Import exactly; add missing DG import if not wired. |

## Closed Decision

All local Product Presave fields are classified. The remaining implementation work is not further reference investigation; it is local alignment:

- removed local drift fields;
- stop importing reference metadata and aliases into DG;
- import the two notation fields that reference imports but local SectionG currently does not;
- keep the KR/MFDS medical-device graph wired through explicit device fields, not generic regional rows.

## Coverage Check

- Total categorized rows: 41
- Uncategorized fields: 0
- Ambiguous fields: 0
