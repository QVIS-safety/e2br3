# Product Presave Final Alignment Matrix

## Scope

This matrix classifies every current local Product Presave field against reference Product Presave and DG import evidence. `dgPrdKey` is excluded because it is case-owned DG state, not a Product Presave field.

Statuses:

- `referenceImportedToDg`: exists in reference Product Presave and is copied into DG by `DG_PRD_KEY` import.
- `referencePreserveOnly`: exists in reference Product Presave, but the reference import does not copy it into DG.
- `localSystemOnly`: local-only support field with a bounded local reason.
- `remove`: local drift; remove from Product Presave unless a separate requirement reintroduces it outside this workflow.

## Matrix

| Local Field | Reference Field / Branch | Reference DG Target | Final Status | Required Local Behavior |
|---|---|---|---|---|
| `productId` | internal reference `PRD_KEY`, not user-entered | none | `localSystemOnly` | Keep only if needed as local template identity/display. Do not import to DG. |
| `drugCharacterization` | none found in reference Product Presave | none | `remove` | Remove from Product Presave and stop importing to DG. |
| `medicinalProduct` | `productDrug.productName` | DG product name `G.k.2.2` | `referenceImportedToDg` | Import exactly. |
| `medicinalProductNotation` | `productDrugNotation.productName` | DG product name notation | `referenceImportedToDg` | Import exactly; add missing DG import if not wired. |
| `drugBrandName` | `brandName` | none | `referencePreserveOnly` | Keep as Product Presave metadata; stop importing to DG. |
| `drugGenericName` | none found in reference Product Presave | none | `remove` | Remove from Product Presave and stop importing to DG. |
| `obtainDrugCountry` | `productDrug.country` | DG country where drug was obtained `G.k.2.4` | `referenceImportedToDg` | Import exactly. |
| `drugAuthorizationCountry` | `productDrug.authNationCode` | DG authorisation/application country `G.k.3.2` | `referenceImportedToDg` | Import exactly. |
| `drugAuthorizationHolder` | `productDrug.authName` | DG holder/applicant name `G.k.3.3` | `referenceImportedToDg` | Import exactly. |
| `drugAuthorizationNumber` | `productDrug.authNumber` | DG authorisation/application number `G.k.3.1` | `referenceImportedToDg` | Import exactly. |
| `drugBatchNumber` | none found in reference Product Presave | none | `remove` | Remove from Product Presave and stop importing to DG. |
| `manufacturerName` | overlaps reference `mnftName` metadata | none | `remove` | Replace with `originalManufacturer`/manufacturer metadata only; do not use as fallback for holder/applicant. |
| `manufacturerCountry` | none found in reference Product Presave | none | `remove` | Remove from Product Presave and stop using as fallback. |
| `preApprovalIpName` | `prdName` | none | `referencePreserveOnly` | Keep as Product Presave metadata; do not import to DG. |
| `originalManufacturer` | `mnftKey`, `mnftName` | none | `referencePreserveOnly` | Keep as Product Presave metadata; do not import to DG. |
| `sender` | `senderName` | none | `referencePreserveOnly` | Keep as Product Presave metadata/display; do not import to DG. |
| `senderPresaveId` | `senderKey` | none | `referencePreserveOnly` | Keep as Product Presave sender link; do not import to DG. |
| `productDescription` | `description` | none | `referencePreserveOnly` | Keep as Product Presave metadata; do not import to DG. |
| `productDeleted` | `delFlag` | none | `referencePreserveOnly` | Keep as Product Presave metadata; do not import to DG. |
| `mpidVersionDateNumber` | `productDrug.mpidVersion` | DG MPID version `G.k.2.1.1a` | `referenceImportedToDg` | Import exactly. |
| `mpid` | `productDrug.mpid` | DG MPID `G.k.2.1.1b` | `referenceImportedToDg` | Import exactly. |
| `mfdsMpidVersion` | `productDrug.mfdsMpidVersion` | KR DG MFDS MPID version `G.k.2.1.KR.1a` | `referenceImportedToDg` | Import exactly for KR/MFDS when present. |
| `mfdsMpid` | `productDrug.mfdsMpid` | KR DG MFDS MPID `G.k.2.1.KR.1b` | `referenceImportedToDg` | Import exactly for KR/MFDS when present. |
| `mfdsDeviceInfo` | `productMedicalDeviceKr` | KR/MFDS DG medical-device graph | `referenceImportedToDg` | Import the full KR device graph. |
| `mfdsDeviceItems` | local flattened storage backing for `productMedicalDeviceKr` | none directly | `localSystemOnly` | Keep only as internal storage/normalization backing for `mfdsDeviceInfo`; do not expose as a separate generic Product Presave concept. |
| `phpidVersionDateNumber` | `productDrug.phpidVersion` | DG PhPID version `G.k.2.1.2a` | `referenceImportedToDg` | Import exactly. |
| `phpid` | `productDrug.phpid` | DG PhPID `G.k.2.1.2b` | `referenceImportedToDg` | Import exactly. |
| `mfdsStudyNumber` | none in reference Product Presave; Study/C.5 domain | none | `remove` | Remove from Product Presave. Belongs to Study Presave if needed. |
| `mfdsProtocolNumber` | none in reference Product Presave; Study/C.5 domain | none | `remove` | Remove from Product Presave. Belongs to Study Presave if needed. |
| `mfdsOtherStudiesType` | none in reference Product Presave; Study/C.5 domain | none | `remove` | Remove from Product Presave. Belongs to Study Presave if needed. |
| `fdaIndNumberOccurred` | none in reference Product Presave; Study/C.5 domain | none | `remove` | Remove from Product Presave. Belongs to Study Presave if needed. |
| `fdaPreAndaNumberOccurred` | none in reference Product Presave; Study/C.5 domain | none | `remove` | Remove from Product Presave. Belongs to Study Presave if needed. |
| `fdaCrossReportedIndNumbers` | none in reference Product Presave; Study/C.5 domain | none | `remove` | Remove from Product Presave. Belongs to Study Presave if needed. |
| `substances` | `substances[]` | DG active-substance rows `G.k.2.3.r`, including KR MFDS substance ID/version | `referenceImportedToDg` | Import exactly, replacing DG substance rows like reference behavior. |
| `mfdsRegionalItems` | none; reference uses explicit `productMedicalDeviceKr` and MFDS drug fields | none | `remove` | Remove generic regional item storage for Product Presave -> DG. |
| `drugObtainedCountry` | none; alias only | none | `remove` | Remove alias and fallback behavior. |
| `investigationalProductBlinded` | `productDrug.blind` | DG investigational product blinded `G.k.2.5` | `referenceImportedToDg` | Import exactly. |
| `authorizationNumber` | none; alias only | none | `remove` | Remove alias and fallback behavior. |
| `authorizationCountry` | none; alias only | none | `remove` | Remove alias and fallback behavior. |
| `holderApplicantName` | none; alias only | none | `remove` | Remove alias and fallback behavior. |
| `holderApplicantNameNotation` | `productDrugNotation.authName` | DG holder/applicant notation | `referenceImportedToDg` | Import exactly; add missing DG import if not wired. |

## Closed Decision

All local Product Presave fields are classified. The remaining implementation work is not further reference investigation; it is local alignment:

- remove local drift fields;
- stop importing reference metadata and aliases into DG;
- import the two notation fields that reference imports but local SectionG currently does not;
- keep the KR/MFDS medical-device graph wired through explicit device fields, not generic regional rows.
