# Product Presave Reference Evidence

## Workflow Investigated

- Product Presave detail/create/edit workflow: not yet confirmed as an exact route. The route must be recorded by normal authenticated UI navigation through `INFO > PRODUCT`.
- DG Product Presave import workflow: identify authority-specific DG routes where the product lookup/import control can be used from the DG section.
- Authorities checked for route scaffolding: ICH/NA, FDA/US, MFDS/KR.

This Task 2 artifact records the reference workflow routes and blockers only. It does not perform deep bundle analysis, payload extraction, or import field classification. That work belongs to Task 3 and later tasks.

## Operating Constraints

- No direct API calls.
- No manually requested inferred endpoints.
- No replayed, edited, synthesized, or resent requests.
- No scraping, crawling, endpoint enumeration, or unrelated record inspection.
- Product Presave route discovery must use normal authenticated UI navigation only.
- Bundle inspection is Task 3 and must be limited to JavaScript assets naturally loaded by the requested pages.
- Live network capture is allowed only later if bundle evidence cannot answer exact field movement.
- Redact values; preserve field names, types, nesting, route shape, and workflow order.

## Reference Routes

| Authority | Product Presave Route | DG Case Route | Status | Confidence | Evidence |
|---|---|---|---|---|---|
| ICH/NA | `https://edu-safetyr3.crscube.io/57/508/en/US/caseinfo/product/list`; detail `https://edu-safetyr3.crscube.io/57/508/en/US/caseinfo/product/24725/detail` observed under FDA selector | `https://edu-safetyr3.crscube.io/57/508/en/NA/case/786994/detail/DG/1` | DG route confirmed by switching authority selector to `ICH`; Product Presave list/detail route confirmed under current FDA selector | High | Authenticated Chrome, authority selector changed to `ICH`, URL changed to `/NA/`, DG section visible, no login prompt |
| FDA/US | `https://edu-safetyr3.crscube.io/57/508/en/US/caseinfo/product/list`; detail `https://edu-safetyr3.crscube.io/57/508/en/US/caseinfo/product/24725/detail` | `https://edu-safetyr3.crscube.io/57/508/en/US/case/786994/detail/DG/1`; alternate prior route: `https://edu-safetyr3.crscube.io/57/508/en/US/case/786995/detail/DG/4` | DG route confirmed in authenticated Chrome session; Product Presave list/detail route confirmed by normal `INFO > PRODUCT` navigation | High | Authenticated Chrome tab title `EDU - cubeSAFETY`, visible FDA header, visible DG section, visible `DG_PRD_KEY` field, Product tab list/detail opened |
| MFDS/KR | `https://edu-safetyr3.crscube.io/57/508/en/US/caseinfo/product/list`; detail `https://edu-safetyr3.crscube.io/57/508/en/US/caseinfo/product/24725/detail` observed under FDA selector | `https://edu-safetyr3.crscube.io/57/508/en/KR/case/786994/detail/DG/1` | DG route confirmed by switching authority selector to `MFDS`; Product Presave list/detail route confirmed under current FDA selector | High | Authenticated Chrome, authority selector changed to `MFDS`, URL changed to `/KR/`, DG section visible with KR device fields, no login prompt |

## Route Confidence and Blockers

### Confirmed Enough for Task 3 Route Opening

- ICH/NA DG route: `https://edu-safetyr3.crscube.io/57/508/en/NA/case/786996/detail/DG/1`
- MFDS/KR DG route: `https://edu-safetyr3.crscube.io/57/508/en/KR/case/786994/detail/DG/1`

These routes are confirmed from the written plan and prior Product Presave/DG investigation context. They are acceptable starting points for Task 3 static bundle evidence if authenticated Chrome navigation opens the expected DG page.

Authenticated route check refinement: direct URL navigation while the top authority selector is FDA normalizes `/NA/` and `/KR/` URLs back to `/US/`. Switching the visible authority selector through the UI is the correct reference behavior:

- selecting `ICH` changes the DG route to `https://edu-safetyr3.crscube.io/57/508/en/NA/case/786994/detail/DG/1`;
- selecting `MFDS` changes the DG route to `https://edu-safetyr3.crscube.io/57/508/en/KR/case/786994/detail/DG/1`;
- selecting `FDA` changes the DG route to `https://edu-safetyr3.crscube.io/57/508/en/US/case/786994/detail/DG/1`.

### Confirmed FDA Task 3 Evidence Route

- FDA/US DG route: `https://edu-safetyr3.crscube.io/57/508/en/US/case/786994/detail/DG/1`
- FDA/US alternate prior route retained as non-canonical context only: `https://edu-safetyr3.crscube.io/57/508/en/US/case/786995/detail/DG/4`

Evidence: authenticated Chrome tab opened the candidate route with title `EDU - cubeSAFETY`, visible FDA authority header, visible DG tab selection, and visible `DG_PRD_KEY` product lookup/import field.

### Product Presave Route Confirmation

Product Presave route was confirmed by normal UI navigation:

- clicked `INFO` in the left navigation;
- clicked the `PRODUCT` tab;
- list route: `https://edu-safetyr3.crscube.io/57/508/en/US/caseinfo/product/list`;
- clicked visible row `TEST Product`;
- detail route: `https://edu-safetyr3.crscube.io/57/508/en/US/caseinfo/product/24725/detail`.

The detail form shows Product Presave fields including `Pre-approval IP Name`, `Brand Name`, `Original Manufacturer`, required `Sender`, `Description`, `Deleted`, ICH product identity fields, substance rows, country obtained, blinded flag, authorisation fields, and holder/applicant fields.

### Authentication/Browser State Check

Attempted normal browser navigation to the NA/ICH DG route with the available in-app browser:

```text
https://edu-safetyr3.crscube.io/57/508/en/NA/case/786996/detail/DG/1
```

Result:

```text
Redirected to https://edu-safetyr3.crscube.io/login/en
```

Rechecked Google Chrome after the user logged in. Chrome reported an authenticated EDU tab at:

```text
https://edu-safetyr3.crscube.io/57/508/en/US/case/786994/detail/DG/1
```

Visible state confirms the app is logged in and on the FDA DG route. Product Presave route discovery was then completed through normal `INFO > PRODUCT` UI navigation.

## Bundles Inspected

The following files were taken from static JavaScript assets naturally loaded by the confirmed Product Presave and DG pages. No inferred API endpoint was requested directly.

| Bundle | Reference Area | Evidence Use |
|---|---|---|
| `app.e6626b8e.js` | shared service client | Product Presave and DG product-info client methods and endpoint templates |
| `CaseInfo2.613bce31.js` | Product Presave detail route | Product Presave form fields, nested `productDrug`, `productDrugNotation`, `substances`, and KR device graph |
| `CaseDetail1.b5394ea0.js` | DG case section route | `DG_PRD_KEY` lookup behavior and field copy/import behavior |
| `CaseInfo1.31e0f99a.js` | Product Presave list route | Product list/detail route support |
| `Case~CaseDetail1~CaseDetail2~CaseDetail3~CaseDetail4.231e069d.js` | case shared chunks | shared case detail components |
| `Case~CaseDetail1~CaseDetail2~CaseDetail4.ad1fd62c.js` | case shared chunks | shared case detail components |
| `Case~CaseDetail1~CaseDetail3~CaseDetail4.11e1f4b5.js` | case shared chunks | shared case detail components |

Bundle working copy: `/tmp/e2br3-product-presave-ref-bundles-20260531`.

## Relevant Client Methods

| Method | Source | Behavior |
|---|---|---|
| `retrieveCaseInfoProducts` | `app.e6626b8e.js` | loads Product Presave list |
| `retrieveCaseInfoProductOptions` | `app.e6626b8e.js` | loads Product Presave option rows |
| `retrieveCaseInfoProduct` | `app.e6626b8e.js` | loads Product Presave detail |
| `saveCaseInfoProduct` | `app.e6626b8e.js` | saves Product Presave detail |
| `retrieveCaseDGProductInfo` | `app.e6626b8e.js` | loads DG import projection for a selected Product Presave |
| `onSuspectDrugProductSelected` | `CaseDetail1.b5394ea0.js` | receives selected Product Presave row and calls product-info loading with `PRD_KEY` |
| `getProductInfoData` | `CaseDetail1.b5394ea0.js` | calls `retrieveCaseDGProductInfo`, then invokes `putProductInfo` and `putProductInfoSubstances` |
| `putProductInfo` | `CaseDetail1.b5394ea0.js` | copies parent Product Presave drug, notation, and KR medical-device data into DG |
| `putProductInfoSubstances` | `CaseDetail1.b5394ea0.js` | replaces DG substance rows from Product Presave substances |

## API Calls Discovered

Static bundle evidence only. These were not called manually.

| Order | Method | Endpoint Template | Source | Purpose |
|---|---|---|---|---|
| 1 | `GET` | `/1.0/sponsors/{sponsor}/products` | `retrieveCaseInfoProducts` | Product Presave list |
| 2 | `GET` | `/1.1/sponsors/{sponsor}/products/options` | `retrieveCaseInfoProductOptions` | Product Presave lookup options |
| 3 | `GET` | `/1.0/sponsors/{sponsor}/products/{productKey}` | `retrieveCaseInfoProduct` | Product Presave detail |
| 4 | `PUT` | `/1.0/sponsors/{sponsor}/products/{productKey}` | `saveCaseInfoProduct` | Product Presave save |
| 5 | `GET` | `/1.1/sponsors/{sponsor}/products/{productKey}?CaseDrug` | `retrieveCaseDGProductInfo` | DG-specific Product Presave import projection |

## Reference Product Presave Fields

| Reference Field | UI / Element ID | Appendix | Product Presave Area | DG Import Status |
|---|---|---|---|---|
| `prdName` | `PRD_NAME` | all | metadata | not imported to DG |
| `brandName` | `BRAND_NAME` | all | metadata | not imported to DG |
| `mnftKey`, `mnftName` | `ORIGINAL_MANUFACTURER` | all | metadata | not imported to DG |
| `senderKey`, `senderName` | `SENDER` | all when CRO sender is enabled | metadata | not imported to DG |
| `description` | `DESCRIPTION` | all | metadata | not imported to DG |
| `delFlag` | `DELETED` | all | metadata | not imported to DG |
| `productDrug.mpidVersion` | `DG_MPID_VER` | all | product drug | imported to DG |
| `productDrug.mpid` | `DG_MPID` | all | product drug | imported to DG |
| `productDrug.phpidVersion` | `DG_PHPID_VER` | all | product drug | imported to DG |
| `productDrug.phpid` | `DG_PHPID` | all | product drug | imported to DG |
| `productDrug.mfdsMpidVersion` | `DGK_MPID_VER` | KR/MFDS | product drug | imported to DG |
| `productDrug.mfdsMpid` | `DGK_MPID` | KR/MFDS | product drug | imported to DG |
| `productDrug.productName` | `DG_PRODUCT_NAME` | all | product drug | imported to DG |
| `productDrugNotation.productName` | `DG_PRODUCT_NAME_NOTATION` | all | product drug notation | imported to DG |
| `substances[].subsName` | `SUBS_NAME` | all | substances | imported to DG |
| `substances[].mfdsTidVer` | `SUBK_TID_VER` | KR/MFDS | substances | imported to DG |
| `substances[].mfdsTid` | `SUBK_TID` | KR/MFDS | substances | imported to DG |
| `substances[].subsTidVer` | `SUBS_TID_VER` | all | substances | imported to DG |
| `substances[].subsTid` | `SUBS_TID` | all | substances | imported to DG |
| `substances[].subsDose` | `SUBS_DOSE` | all | substances | imported to DG |
| `substances[].subsDoseu` | `SUBS_DOSEU` | all | substances | imported to DG |
| `productDrug.country` | `DG_COUNTRY` | all | product drug | imported to DG |
| `productDrug.blind` | `DG_BLIND` | all | product drug | imported to DG |
| `productDrug.authNumber` | `DG_AUTH_NO` | all | product drug | imported to DG |
| `productDrug.authNationCode` | `DG_AUTH_NATIOIN_CODE` | all | product drug | imported to DG |
| `productDrug.authName` | `DG_AUTH_NAME` | all | product drug | imported to DG |
| `productDrugNotation.authName` | holder/applicant notation | all | product drug notation | imported to DG |
| `productMedicalDeviceKr.dvcSerialNumber` | `KR_DVC_SN` | KR/MFDS | KR medical device | imported to DG |
| `productMedicalDeviceKr.dvcManufacturer` | `KR_DVC_MFR` | KR/MFDS | KR medical device | imported to DG |
| `productMedicalDeviceKr.dvcModels[].dvcModel` | `KR_DVC_MDL` | KR/MFDS | KR medical device model rows | imported to DG |
| `productMedicalDeviceKr.dvcModels[].dvcLot` | `KR_DVC_LOT` | KR/MFDS | KR medical device model rows | imported to DG |
| `productMedicalDeviceKr.dvcModels[].dvcUdiIdentifier` | `KR_DVC_UDI_DI` | KR/MFDS | KR medical device model rows | imported to DG |
| `productMedicalDeviceKr.dvcModels[].dvcUdiSerialNumber` | `KR_DVC_UDI_SN` | KR/MFDS | KR medical device model rows | imported to DG |
| `productMedicalDeviceKr.dvcModels[].dvcUdiLot` | `KR_DVC_UDI_LOT` | KR/MFDS | KR medical device model rows | imported to DG |
| `productMedicalDeviceKr.dvcModels[].dvcUdiManufacturingDate` | `KR_DVC_UDI_MFD` | KR/MFDS | KR medical device model rows | imported to DG |
| `productMedicalDeviceKr.dvcModels[].dvcUdiExpirationDate` | `KR_DVC_UDI_EXP` | KR/MFDS | KR medical device model rows | imported to DG |
| `productMedicalDeviceKr.dvcModels[].dvcImpds[].dvcImpd` | `KR_DVC_IMPD` | KR/MFDS | KR medical device IMPD rows | imported to DG |
| `productMedicalDeviceKr.dvcProblemCodes[].code` | `KR_DVC_PROBC` | KR/MFDS | KR device code rows | imported to DG |
| `productMedicalDeviceKr.dvcCICTypes[].code` | `KR_DVC_CIC_TYPE` | KR/MFDS | KR device code rows | imported to DG |
| `productMedicalDeviceKr.dvcCICAssessments[].code` | `KR_DVC_CIC_ASMT` | KR/MFDS | KR device code rows | imported to DG |
| `productMedicalDeviceKr.dvcCICConclusions[].code` | `KR_DVC_CIC_CON` | KR/MFDS | KR device code rows | imported to DG |
| `productMedicalDeviceKr.dvcHICClinicalSigns[].code` | `KR_DVC_HIC_CSC` | KR/MFDS | KR device code rows | imported to DG |
| `productMedicalDeviceKr.dvcHICOutcomes[].code` | `KR_DVC_HIC_HO` | KR/MFDS | KR device code rows | imported to DG |
| `productMedicalDeviceKr.dvcComponentCodes[].code` | `KR_DVC_CMPC` | KR/MFDS | KR device code rows | imported to DG |

## DG Import Behavior

When the user selects a Product Presave through `DG_PRD_KEY`, the reference uses the selected row's `PRD_KEY` to load a DG-specific product projection. The bundle then imports only these response branches:

- `productDrug`
- `productDrugNotation`
- `substances`
- `productMedicalDeviceKr` for KR/MFDS

Reference import copies:

| Reference Source | Reference DG Target |
|---|---|
| `productDrug.mpidVersion` | DG MPID version |
| `productDrug.mpid` | DG MPID |
| `productDrug.phpidVersion` | DG PhPID version |
| `productDrug.phpid` | DG PhPID |
| `productDrug.mfdsMpidVersion` | KR DG MFDS MPID version |
| `productDrug.mfdsMpid` | KR DG MFDS MPID |
| `productDrug.productName`, `productDrugNotation.productName` | DG medicinal product name and notation |
| `productDrug.country` | DG country where drug was obtained |
| `productDrug.blind` | DG investigational product blinded |
| `productDrug.authNumber` | DG authorisation/application number |
| `productDrug.authNationCode` | DG authorisation/application country |
| `productDrug.authName`, `productDrugNotation.authName` | DG holder/applicant name and notation |
| `substances[]` | DG active substances, including KR MFDS substance ID/version fields |
| `productMedicalDeviceKr` | KR/MFDS DG medical-device rows and code rows |

## Fields Requiring Live Network Evidence

None for the Product Presave -> DG import decision. Static bundle evidence identifies both the DG import method and every copied field branch. Live network capture is not required for the classification matrix.

## Reference Findings

- Reference Product Presave does not expose a user-entered `dgPrdKey`; `PRD_KEY` is an internal Product Presave row key used by the lookup/import control.
- Reference Product Presave does not import metadata fields (`prdName`, `brandName`, `mnftKey/mnftName`, `senderKey/senderName`, `description`, `delFlag`) into DG.
- Reference Product Presave imports product drug fields, product drug notation fields, substance rows, and KR/MFDS medical-device rows into DG.
- Reference Product Presave does not show local Product Presave FDA C.5 study fields; those belong to the Study/C.5 domain, not Product Presave -> DG.
- Reference Product Presave uses exact field families for KR/MFDS drug and medical-device fields. It does not use a generic MFDS regional item table for DG product import.
