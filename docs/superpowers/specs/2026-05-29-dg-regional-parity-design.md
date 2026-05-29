# DG Regional Field Parity Design

Date: 2026-05-29

## Objective

Close the regional-field gap in the DG case section before extending product presave import. The work covers both authority-specific DG rendering and save/readback parity for FDA and MFDS routes.

The immediate problem is that product presave import cannot be made complete if the case DG section itself does not expose all regional fields that the reference application exposes. The implementation should first make the case section capable of storing and editing those fields, then presave import can map into those visible case paths.

## Reference Evidence

The reference DG route exposes regional fields by authority.

For FDA/US, the reference bundle includes:

- `DGU_ADD_INFO_CODE`
- `DGU_PRD_CAT`
- `DGU_DVC_INFO_MALF`
- `DGU_DVC_PROB_FU_TYPE`
- `DGU_DVC_PROB_CODE`
- `DGU_DVC_INFO_BRD_NAME`
- `DGU_DVC_INFO_PRD_NAME`
- `DGU_DVC_INFO_PRD_CODE`
- `DGU_DVC_MNFT_NAME`
- `DGU_DVC_MNFT_ADDR`
- `DGU_DVC_MNFT_CITY`
- `DGU_DVC_MNFT_STATE`
- `DGU_DVC_MNFT_COUNTRY`
- `DGU_DVC_USAGE`
- `DGU_DVC_LOT_NO`
- `DGU_DVC_OPERATOR`
- `DGU_DVC_REMEDIAL`

For MFDS/KR, the reference DG screen includes a KR medical-device subsection. The bundle and UI show fields including:

- `KR_DVC_IMPD`
- `KR_DVC_MFR`
- `KR_DVC_SN`
- `KR_DVC_LOT`
- `KR_DVC_UDI_DI`
- `KR_DVC_UDI_SN`
- `KR_DVC_UDI_LOT`
- `KR_DVC_UDI_MFD`
- `KR_DVC_UDI_EXP`
- `KR_DVC_MDL`
- `KR_DVC_PROBC`
- `KR_DVC_CIC_TYPE`
- `KR_DVC_CIC_ASMT`
- `KR_DVC_CIC_CON`
- `KR_DVC_HIC_CSC`
- `KR_DVC_HIC_HO`
- `KR_DVC_CMPC`

The local frontend currently has substantial FDA DG support, but only partial MFDS DG support. Local MFDS support covers product code/version and relatedness fields, but not the KR medical-device subsection.

## Local Architecture

Frontend DG lives primarily in:

- `frontend/E2BR3-frontend/components/case-form/sections/SectionG.tsx`
- `frontend/E2BR3-frontend/lib/types/e2br3.ts`
- `frontend/E2BR3-frontend/lib/schemas/e2br3.ts`
- `frontend/E2BR3-frontend/lib/api/endpoints/cases/core/detail.drugs.ts`
- `frontend/E2BR3-frontend/lib/api/endpoints/cases/subresources/drug.ts`

Backend DG persistence lives primarily in:

- `db/bootstrap/07-drug-information.sql`
- `crates/libs/lib-core/src/model/drug.rs`
- `crates/services/web-server/src/web/rest/drug_rest.rs`
- `crates/libs/lib-core/src/validation/case/sections/g.rs`
- `crates/libs/lib-core/src/xml/import_sections/g_drug.rs`
- `crates/libs/lib-core/src/xml/import_runtime/g.rs`
- `crates/libs/lib-core/src/xml/export/sections/g.rs`

Existing backend storage already has two relevant shapes:

- Top-level `drug_information` columns for standard G.k fields and structured FDA JSON.
- Generic `drug_device_characteristics` rows with code/value columns.

## Design Decision

Use the existing generic `drug_device_characteristics` table as the durable storage layer for regional device characteristics, including new MFDS/KR device fields.

Do not add one column per KR device field in `drug_information` for this pass. The reference fields are characteristic-like repeatable regional data, and the existing characteristic table already handles FDA device rows with `code`, `value_code`, and `value_value`. Reusing that table keeps the backend model smaller and keeps FDA/MFDS behavior under one regional characteristic abstraction.

FDA remains backed by the current structured `fdaDeviceInfo` frontend object and `fda_device_info_json` / derived device-characteristic mapping. The FDA pass should audit and fill any missing field mappings against the reference identifiers above, not replace the current model.

MFDS gets a new structured frontend object named `mfdsDeviceInfo`, which maps to `drug_device_characteristics` rows with `code = KR_DVC_*`.

## Frontend Behavior

`SectionG` should render regional subsections by selected authority:

- ICH: common DG fields only.
- US/FDA: common DG plus FDA regional fields.
- KR/MFDS: common DG plus MFDS regional fields.
- USKR: common DG plus both FDA and MFDS regional fields.

The MFDS subsection should follow the reference grouping:

- Device information: manufacturer, serial, lot, UDI, model.
- Implant/device date fields.
- Problem codes.
- Cause investigation code groups: type, assessment, conclusion.
- Health impact code groups: clinical signs/conditions and outcomes.
- Component code group.

Repeatable groups should use the existing local `useFieldArray` pattern used by FDA device code arrays and active substances.

## Data Mapping

FDA mapping should be audited against the reference identifiers:

- `DGU_ADD_INFO_CODE` maps to `fdaAdditionalInfoCoded`.
- `DGU_PRD_CAT` maps to `fdaSpecializedProductCategory`.
- `DGU_DVC_INFO_*`, `DGU_DVC_PROB_*`, `DGU_DVC_REMEDIAL` map to `fdaDeviceInfo`.

MFDS mapping should use a deterministic characteristic map:

| Reference Field | Frontend Path | Storage |
|---|---|---|
| `KR_DVC_IMPD` | `drugs[].mfdsDeviceInfo.implantDates[]` | `drug_device_characteristics.code = KR_DVC_IMPD`, `value_value` |
| `KR_DVC_MFR` | `drugs[].mfdsDeviceInfo.manufacturer` | `code = KR_DVC_MFR`, `value_value` |
| `KR_DVC_SN` | `drugs[].mfdsDeviceInfo.serialNumber` | `code = KR_DVC_SN`, `value_value` |
| `KR_DVC_LOT` | `drugs[].mfdsDeviceInfo.lotNumber` | `code = KR_DVC_LOT`, `value_value` |
| `KR_DVC_UDI_DI` | `drugs[].mfdsDeviceInfo.udiDeviceIdentifier` | `code = KR_DVC_UDI_DI`, `value_value` |
| `KR_DVC_UDI_SN` | `drugs[].mfdsDeviceInfo.udiSerialNumber` | `code = KR_DVC_UDI_SN`, `value_value` |
| `KR_DVC_UDI_LOT` | `drugs[].mfdsDeviceInfo.udiLotNumber` | `code = KR_DVC_UDI_LOT`, `value_value` |
| `KR_DVC_UDI_MFD` | `drugs[].mfdsDeviceInfo.udiManufacturingDate` | `code = KR_DVC_UDI_MFD`, `value_value` |
| `KR_DVC_UDI_EXP` | `drugs[].mfdsDeviceInfo.udiExpirationDate` | `code = KR_DVC_UDI_EXP`, `value_value` |
| `KR_DVC_MDL` | `drugs[].mfdsDeviceInfo.models[]` | `code = KR_DVC_MDL`, `value_value` |
| `KR_DVC_PROBC` | `drugs[].mfdsDeviceInfo.problemCodes[]` | `code = KR_DVC_PROBC`, `value_code` |
| `KR_DVC_CIC_TYPE` | `drugs[].mfdsDeviceInfo.causeInvestigationTypes[]` | `code = KR_DVC_CIC_TYPE`, `value_code` |
| `KR_DVC_CIC_ASMT` | `drugs[].mfdsDeviceInfo.causeInvestigationAssessments[]` | `code = KR_DVC_CIC_ASMT`, `value_code` |
| `KR_DVC_CIC_CON` | `drugs[].mfdsDeviceInfo.causeInvestigationConclusions[]` | `code = KR_DVC_CIC_CON`, `value_code` |
| `KR_DVC_HIC_CSC` | `drugs[].mfdsDeviceInfo.healthImpactClinicalSigns[]` | `code = KR_DVC_HIC_CSC`, `value_code` |
| `KR_DVC_HIC_HO` | `drugs[].mfdsDeviceInfo.healthImpactOutcomes[]` | `code = KR_DVC_HIC_HO`, `value_code` |
| `KR_DVC_CMPC` | `drugs[].mfdsDeviceInfo.componentCodes[]` | `code = KR_DVC_CMPC`, `value_code` |

The read path should parse matching `drug_device_characteristics` rows into `mfdsDeviceInfo`. The save path should upsert/delete the corresponding characteristic rows when `mfdsDeviceInfo` changes.

## Backend Behavior

The backend should not need a new BMC model if the existing `DrugDeviceCharacteristicBmc` can safely store these rows. If current REST helpers only expose generic create/update/list/delete, use them from the frontend save coordinator. If the save coordinator lacks row-level reconciliation for device characteristics, add a focused reconciliation helper for regional device characteristic rows.

Backend validation can initially treat these as persisted/rendered fields, not add new blocking rules unless an existing MFDS rule already requires them. Requiredness should come from the validation catalog only when a known rule exists. UI red-dot behavior should follow existing backend validation results.

XML import/export support should be audited after UI save/readback is in place. If local XML mapping lacks canonical MFDS paths for `KR_DVC_*`, keep XML support out of the first implementation plan and document it as a follow-up instead of inventing unsupported mappings.

## Presave Implication

Product presave import must wait for visible case paths before importing these regional fields. After DG regional parity lands:

- FDA presave fields can map into existing/verified FDA `SectionG` paths.
- MFDS product presave fields can map only into matching visible MFDS case paths.
- Do not import fields into hidden generic metadata or paths the case editor cannot display.

## Testing Strategy

Use TDD.

Frontend tests:

- Add/extend regional rendering tests for `SectionG`:
  - ICH does not show FDA/MFDS regional sections.
  - US shows FDA regional fields.
  - KR shows MFDS KR device fields.
  - USKR shows both.
- Add binding tests for each new MFDS field group using the actual `SectionG` component.
- Add detail builder tests proving `drug_device_characteristics` rows with `KR_DVC_*` parse into `mfdsDeviceInfo`.
- Add save payload tests proving `mfdsDeviceInfo` writes characteristic rows with the expected `code`, `value_code`, and `value_value`.
- Add FDA parity tests for any missing reference fields found during the audit.

Backend tests:

- Add model/BMC tests showing `DrugDeviceCharacteristicBmc` accepts KR device characteristic codes without FDA-only assumptions.
- Add API contract tests for listing/upserting/deleting device characteristic rows used by KR fields if existing coverage is insufficient.
- Add validation visibility tests only when backend validation rules reference these paths.

Manual/browser verification:

- Open local DG for ICH, US, KR, and USKR.
- Confirm the regional sections render only in the matching authorities.
- Save a KR case with multiple repeatable KR device rows.
- Reload and verify values remain in the same fields.

## Risks

The largest risk is conflating MFDS product-code fields with MFDS medical-device fields. They are separate:

- `G.k.2.1.KR.1a/b` product code/version already maps to `mpidVersion` / `mpid`.
- `KR_DVC_*` fields are the missing KR device subsection and should map to device characteristics.

Another risk is overfitting to one reference screen. The first implementation should include the fields proven by the reference bundle and screenshot, then leave XML-specific or appendix-specific unmapped fields as documented follow-ups.

## Approval Gate

After this spec is approved, the implementation plan should start with the MFDS KR device characteristic parser/writer tests, then render the fields in `SectionG`, then audit FDA parity.
