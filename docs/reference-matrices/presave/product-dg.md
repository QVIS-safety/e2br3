# Product Presave Final Alignment Matrix

## Scope

This matrix classifies every current local Product Presave field against reference Product Presave and DG import evidence. `dgPrdKey` is excluded because it is case-owned DG state, not a Product Presave field.

Statuses:

- `referenceImportedToCase`: exists in reference Product Presave and is copied into DG by `DG_PRD_KEY` import.
- `referencePreserveOnly`: exists in reference Product Presave, but the reference import does not copy it into DG.
- `localSystemOnly`: local-only support field with a bounded local reason.
- `removed`: local drift; removed from Product Presave unless a separate requirement reintroduces it outside this workflow.

## Matrix

| field | reference evidence | canonical frontend source | canonical backend source | allowed read aliases | allowed write keys | case import target | duplicate sources found | category | decision | tests required |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `productId` | internal reference `PRD_KEY`, not user-entered | `ProductPresaveData.productId` | `product_presaves.product_id` | `id` read only for legacy details identity | `product_id` | none | none | `localSystemOnly` | Keep as template identity/display only. | mapper identity test; DG import negative test |
| `drugCharacterization` | none found in reference Product Presave | none | none | none | none | none | legacy presave import tried to populate DG role | `removed` | Remove from Product Presave; DG import defaults role to suspect. | mapper absence test; DG import negative test |
| `medicinalProduct` | `productDrug.productName` | `ProductPresaveData.medicinalProduct` | `product_presaves.medicinal_product` | none | `medicinal_product` | `drugs[].medicinalProduct` | none | `referenceImportedToCase` | Keep and import exactly. | mapper/write/import tests |
| `medicinalProductNotation` | `productDrugNotation.productName` | `ProductPresaveData.medicinalProductNotation` | `product_presaves.medicinal_product_notation` | none | `medicinal_product_notation` | `drugs[].medicinalProductNotation` | previously missing DG import target | `referenceImportedToCase` | Keep and import exactly. | mapper/write/import tests |
| `drugBrandName` | `brandName` | `ProductPresaveData.drugBrandName` | `product_presaves.brand_name` | `brandName` read only from details/API | `brand_name` | none | previous DG import wrote brand to case drug | `referencePreserveOnly` | Preserve on Product Presave only. | mapper/write test; DG import negative test |
| `drugGenericName` | none found in reference Product Presave | none | none | none | none | none | legacy DG import wrote generic name | `removed` | Remove from Product Presave and do not import. | mapper absence test; DG import negative test |
| `obtainDrugCountry` | `productDrug.country` | `ProductPresaveData.obtainDrugCountry` | `product_presaves.obtain_drug_country` | none | `obtain_drug_country` | `drugs[].obtainDrugCountry` | legacy `drugObtainedCountry` alias existed | `referenceImportedToCase` | Keep canonical source only. | mapper/write/import tests; alias negative test |
| `drugAuthorizationCountry` | `productDrug.authNationCode` | `ProductPresaveData.drugAuthorizationCountry` | `product_presaves.drug_authorization_country` | none | `drug_authorization_country` | `drugs[].drugAuthorizationCountry` | legacy `authorizationCountry` and `manufacturerCountry` fallbacks existed | `referenceImportedToCase` | Keep canonical source only. | mapper/write/import tests; alias negative test |
| `drugAuthorizationHolder` | `productDrug.authName` | `ProductPresaveData.drugAuthorizationHolder` | `product_presaves.drug_authorization_holder` | `authorizationHolder` read only migration alias | `drug_authorization_holder` | `drugs[].drugAuthorizationHolder` | legacy `holderApplicantName` and `manufacturerName` fallbacks existed | `referenceImportedToCase` | Keep canonical source only. | mapper/write/import tests; alias negative test |
| `drugAuthorizationNumber` | `productDrug.authNumber` | `ProductPresaveData.drugAuthorizationNumber` | `product_presaves.drug_authorization_number` | none | `drug_authorization_number` | `drugs[].drugAuthorizationNumber` | legacy `authorizationNumber` fallback existed | `referenceImportedToCase` | Keep canonical source only. | mapper/write/import tests; alias negative test |
| `drugBatchNumber` | none found in reference Product Presave | none | none | none | none | none | legacy DG import wrote batch number | `removed` | Remove from Product Presave and do not import. | mapper absence test; DG import negative test |
| `manufacturerName` | overlaps reference `mnftName` metadata | none | none | none | none | none | legacy fallback for holder/applicant | `removed` | Use `originalManufacturer` only for preserve-only metadata. | mapper absence test; fallback negative test |
| `manufacturerCountry` | none found in reference Product Presave | none | none | none | none | none | legacy fallback for authorization country | `removed` | Remove and do not use as fallback. | mapper absence test; fallback negative test |
| `preApprovalIpName` | `prdName` | `ProductPresaveData.preApprovalIpName` | `product_presaves.preapproval_ip_name` | `preapprovalIpName` read only migration alias | `preapproval_ip_name` | none | none | `referencePreserveOnly` | Preserve on Product Presave only. | mapper/write test; DG import negative test |
| `originalManufacturer` | `mnftKey`, `mnftName` | `ProductPresaveData.originalManufacturer` | `product_presaves.original_manufacturer` | none | `original_manufacturer` | none | previous `manufacturerName` overlap resolved here | `referencePreserveOnly` | Preserve on Product Presave only. | mapper/write test; DG import negative test |
| `sender` | `senderName` | `ProductPresaveData.sender` display derived from selected sender | `product_presaves.sender_presave_id` relation plus sender display in details | none | none for parent content; `sender_presave_id` for relation | none | hidden free-text sender replaced by registered sender relation | `referencePreserveOnly` | Preserve as display metadata only; author relation by `senderPresaveId`. | form sender lookup test; DG import negative test |
| `senderPresaveId` | `senderKey` | `ProductPresaveData.senderPresaveId` | `product_presaves.sender_presave_id` | none | `sender_presave_id` | none | none | `referencePreserveOnly` | Keep as Product Presave sender link only. | mapper/write/form tests; DG import negative test |
| `productDescription` | `description` | `ProductPresaveData.productDescription` | `product_presaves.product_description` | `description` read only migration alias | `product_description` | none | none | `referencePreserveOnly` | Preserve on Product Presave only. | mapper/write test; DG import negative test |
| `productDeleted` | `delFlag` | `ProductPresaveData.productDeleted` | `product_presaves.deleted` | `deleted` read only migration alias | `deleted` | none | none | `referencePreserveOnly` | Preserve deletion state on template only. | mapper/write test; DG import negative test |
| `mpidVersionDateNumber` | `productDrug.mpidVersion` | `ProductPresaveData.mpidVersionDateNumber` | `product_presaves.mpid_version` | `mpidVersion` read only migration alias | `mpid_version` | `drugs[].mpidVersion` | none | `referenceImportedToCase` | Keep and import exactly. | mapper/write/import tests |
| `mpid` | `productDrug.mpid` | `ProductPresaveData.mpid` | `product_presaves.mpid` | none | `mpid` | `drugs[].mpid` | none | `referenceImportedToCase` | Keep and import exactly. | mapper/write/import tests |
| `mfdsMpidVersion` | `productDrug.mfdsMpidVersion` | `ProductPresaveData.mfdsMpidVersion` | `product_presaves.mfds_mpid_version` | none | `mfds_mpid_version` | `drugs[].mfdsMpidVersion` | none | `referenceImportedToCase` | Keep and import exactly for KR/MFDS. | mapper/write/import tests |
| `mfdsMpid` | `productDrug.mfdsMpid` | `ProductPresaveData.mfdsMpid` | `product_presaves.mfds_mpid` | none | `mfds_mpid` | `drugs[].mfdsMpid` | none | `referenceImportedToCase` | Keep and import exactly for KR/MFDS. | mapper/write/import tests |
| `mfdsDeviceInfo` | `productMedicalDeviceKr` | `ProductPresaveData.mfdsDeviceInfo` | `product_presave_mfds_device_items` normalized rows | none | `mfds_device_items[]` generated from `mfdsDeviceInfo` | `drugs[].mfdsDeviceInfo` | flattened storage rows could be mistaken for second authoring source | `referenceImportedToCase` | Author through structured device info; persist as coded rows. | mapper/write/import tests |
| `mfdsDeviceItems` | local flattened storage backing for `productMedicalDeviceKr` | `ProductPresaveData.mfdsDeviceItems` read/update metadata only | `product_presave_mfds_device_items` | `mfds_device_items` read only existing rows | none as independent concept | none directly | duplicate-looking storage for `mfdsDeviceInfo` | `localSystemOnly` | Keep only for row identity and delete reconciliation. | mapper/write reconciliation test; DG import through `mfdsDeviceInfo` |
| `phpidVersionDateNumber` | `productDrug.phpidVersion` | `ProductPresaveData.phpidVersionDateNumber` | `product_presaves.phpid_version` | `phpidVersion` read only migration alias | `phpid_version` | `drugs[].phpidVersion` | none | `referenceImportedToCase` | Keep and import exactly. | mapper/write/import tests |
| `phpid` | `productDrug.phpid` | `ProductPresaveData.phpid` | `product_presaves.phpid` | none | `phpid` | `drugs[].phpid` | none | `referenceImportedToCase` | Keep and import exactly. | mapper/write/import tests |
| `mfdsStudyNumber` | none in reference Product Presave; Study/C.5 domain | none | none | none | none | none | wrong presave domain | `removed` | Remove from Product Presave; Study owns this if needed. | mapper/write absence test |
| `mfdsProtocolNumber` | none in reference Product Presave; Study/C.5 domain | none | none | none | none | none | wrong presave domain | `removed` | Remove from Product Presave; Study owns this if needed. | mapper/write absence test |
| `mfdsOtherStudiesType` | none in reference Product Presave; Study/C.5 domain | none | none | none | none | none | wrong presave domain | `removed` | Remove from Product Presave; Study owns this if needed. | mapper/write absence test |
| `fdaIndNumberOccurred` | none in reference Product Presave; Study/C.5 domain | none | none | none | none | none | wrong presave domain | `removed` | Remove from Product Presave; Study owns this if needed. | mapper/write absence test |
| `fdaPreAndaNumberOccurred` | none in reference Product Presave; Study/C.5 domain | none | none | none | none | none | wrong presave domain | `removed` | Remove from Product Presave; Study owns this if needed. | mapper/write absence test |
| `fdaCrossReportedIndNumbers` | none in reference Product Presave; Study/C.5 domain | none | none | none | none | none | wrong presave domain and legacy child graph | `removed` | Remove from Product Presave; Study owns this if needed. | mapper/write absence test |
| `substances` | `substances[]` | `ProductPresaveData.substances[]` | `product_presave_substances` | `activeSubstances` read only migration alias | `substances[]` | `drugs[].activeSubstances[]` | none | `referenceImportedToCase` | Keep and import exactly, replacing DG substance rows. | mapper/write/import tests |
| `mfdsRegionalItems` | none; reference uses explicit `productMedicalDeviceKr` and MFDS drug fields | none | none | none | none | none | generic regional storage duplicated explicit MFDS fields | `removed` | Remove generic regional item storage. | mapper/write absence test |
| `drugObtainedCountry` | none; alias only | none | none | none | none | none | legacy alias for `obtainDrugCountry` | `removed` | Remove alias and fallback behavior. | alias negative import test |
| `investigationalProductBlinded` | `productDrug.blind` | `ProductPresaveData.investigationalProductBlinded` | `product_presaves.investigational_product_blinded` | none | `investigational_product_blinded` | `drugs[].investigationalProductBlinded` | none | `referenceImportedToCase` | Keep and import exactly. | mapper/write/import tests |
| `authorizationNumber` | none; alias only | none | none | none | none | none | legacy alias for `drugAuthorizationNumber` | `removed` | Remove alias and fallback behavior. | alias negative import test |
| `authorizationCountry` | none; alias only | none | none | none | none | none | legacy alias for `drugAuthorizationCountry` | `removed` | Remove alias and fallback behavior. | alias negative import test |
| `holderApplicantName` | none; alias only | none | none | none | none | none | legacy alias for `drugAuthorizationHolder` | `removed` | Remove alias and fallback behavior. | alias negative import test |
| `holderApplicantNameNotation` | `productDrugNotation.authName` | `ProductPresaveData.holderApplicantNameNotation` | `product_presaves.holder_applicant_name_notation` | none | `holder_applicant_name_notation` | `drugs[].holderApplicantNameNotation` | previously missing DG import target | `referenceImportedToCase` | Keep and import exactly. | mapper/write/import tests |

## One-Source Audit Report

Presave: product

Confirmed 1-1 mappings:

- `medicinalProduct`: `ProductPresaveData.medicinalProduct` -> `product_presaves.medicinal_product` -> `drugs[].medicinalProduct`
- `medicinalProductNotation`: `ProductPresaveData.medicinalProductNotation` -> `product_presaves.medicinal_product_notation` -> `drugs[].medicinalProductNotation`
- `obtainDrugCountry`: `ProductPresaveData.obtainDrugCountry` -> `product_presaves.obtain_drug_country` -> `drugs[].obtainDrugCountry`
- `drugAuthorizationCountry`: `ProductPresaveData.drugAuthorizationCountry` -> `product_presaves.drug_authorization_country` -> `drugs[].drugAuthorizationCountry`
- `drugAuthorizationHolder`: `ProductPresaveData.drugAuthorizationHolder` -> `product_presaves.drug_authorization_holder` -> `drugs[].drugAuthorizationHolder`
- `drugAuthorizationNumber`: `ProductPresaveData.drugAuthorizationNumber` -> `product_presaves.drug_authorization_number` -> `drugs[].drugAuthorizationNumber`
- `mpidVersionDateNumber`: `ProductPresaveData.mpidVersionDateNumber` -> `product_presaves.mpid_version` -> `drugs[].mpidVersion`
- `mpid`: `ProductPresaveData.mpid` -> `product_presaves.mpid` -> `drugs[].mpid`
- `mfdsMpidVersion`: `ProductPresaveData.mfdsMpidVersion` -> `product_presaves.mfds_mpid_version` -> `drugs[].mfdsMpidVersion`
- `mfdsMpid`: `ProductPresaveData.mfdsMpid` -> `product_presaves.mfds_mpid` -> `drugs[].mfdsMpid`
- `phpidVersionDateNumber`: `ProductPresaveData.phpidVersionDateNumber` -> `product_presaves.phpid_version` -> `drugs[].phpidVersion`
- `phpid`: `ProductPresaveData.phpid` -> `product_presaves.phpid` -> `drugs[].phpid`
- `investigationalProductBlinded`: `ProductPresaveData.investigationalProductBlinded` -> `product_presaves.investigational_product_blinded` -> `drugs[].investigationalProductBlinded`
- `holderApplicantNameNotation`: `ProductPresaveData.holderApplicantNameNotation` -> `product_presaves.holder_applicant_name_notation` -> `drugs[].holderApplicantNameNotation`
- `substances`: `ProductPresaveData.substances[]` -> `product_presave_substances` -> `drugs[].activeSubstances[]`
- `mfdsDeviceInfo`: `ProductPresaveData.mfdsDeviceInfo` -> `product_presave_mfds_device_items` -> `drugs[].mfdsDeviceInfo`

Duplicate sources found:

- `drugBrandName`: Product Presave stores `brand_name`, but DG import previously treated it like case drug brand content.
  decision: preserve only on Product Presave; do not import to DG.
- `drugAuthorizationCountry`: legacy `authorizationCountry` and `manufacturerCountry` aliases could compete with canonical `drugAuthorizationCountry`.
  decision: remove aliases and fallback behavior.
- `drugAuthorizationHolder`: legacy `holderApplicantName` and `manufacturerName` aliases could compete with canonical `drugAuthorizationHolder`.
  decision: remove aliases and fallback behavior.
- `drugAuthorizationNumber`: legacy `authorizationNumber` could compete with canonical `drugAuthorizationNumber`.
  decision: remove alias and fallback behavior.
- `mfdsDeviceInfo`: structured frontend source persists as normalized device rows.
  decision: author only through `mfdsDeviceInfo`; keep `mfdsDeviceItems` only for row identity and deletion reconciliation.

Removed fields still present:

- None in canonical Product Presave type, form, schema, write mapper, backend model, or DG import source.

Tests to add or update:

- `canonical-presave-mappers`: product mapper/write tests prove removed fields and aliases do not map or write.
- `field-bindings`: actual `SectionG` import test proves every imported Product Presave field populates DG and preserve-only or removed fields do not.
- `product-form-alignment`: form tests prove sender comes from registered sender master data and MFDS fields are authority scoped.
- `validate_presave_reference_matrices.py`: matrix validator enforces the one-source columns for Product Presave.

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
