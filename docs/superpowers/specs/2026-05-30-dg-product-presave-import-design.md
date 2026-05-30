# DG Product Presave Import Design

## Context

The reference EDU cubeSAFETY UI treats `DG_PRD_KEY` in the DG section as a product selector for the current drug row. In the checked route, double-clicking the `DG_PRD_KEY` value opened a `Select Products` modal with product rows identified by `Brand Name`, `Pre-IP Name`, `Sender`, and `Manufacturer`. Selecting a row populated the current DG row from that product template. When the selected product template did not contain values for some product fields, those DG fields were cleared rather than preserved.

Our current SectionG product import does not match that behavior. `handleImportProduct` appends a new drug row and uses several fallback chains when copying product presave fields. It also does not set the top-level `dgPrdKey` that the local DG Product ID display reads.

## Goal

Align local DG product-presave import with the reference architecture:

- selecting a product presave from the DG product selector updates the active/current DG drug row;
- `productId` from the selected product presave becomes the displayed case-level `dgPrdKey`;
- mapped fields are assigned directly, including blank values, so a blank presave field clears the DG field;
- product presave metadata that is not a DG field is not imported into DG.

## Non-Goals

- Do not make product presave cover every DG field. DG includes case-specific fields such as dosage, indications, drug-reaction matrix, rechallenge, additional drug information, and device sections. Those remain case data.
- Do not add direct API probing or replay behavior to the investigation workflow.
- Do not import sender scope, sender presave identity, deleted flags, comments, or admin-only product metadata into DG.
- Do not use fallback unions such as `authorizationCountry || manufacturerCountry` in the DG import path.

## Reference Behavior

The relevant UI behavior is:

1. User is on a DG drug row.
2. User opens the `DG_PRD_KEY` product selector.
3. The selector lists available products using product-template/list columns.
4. User selects one product.
5. The current DG drug row is updated in place.
6. Product-template blanks clear corresponding DG fields.
7. Existing unrelated case-specific DG fields remain outside the product import contract.

## Local Architecture

Relevant local files:

- `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/case-form/sections/SectionG.tsx`
- `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/types/presave.ts`
- `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/schemas/presave.ts`
- `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/presave/canonicalMappers.ts`
- `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/presave/canonicalWriteMappers.ts`
- `/Users/hyundonghoon/projects/rust/e2br3/e2br3/db/bootstrap/01-safetydb-schema.sql`

Important local observations:

- `ProductPresaveData.productId` is the product template identity visible in the product presave form.
- `ProductPresaveData.dgPrdKey` exists in frontend type/schema/write mapper, but the current product presave DB table does not define a product-presave `dg_prd_key` column. The case table has `cases.dg_prd_key`.
- SectionG displays Product ID from top-level `dgPrdKey`.
- SectionG currently appends a new drug during product import.

## Field Mapping

The DG product import mapper should update the active drug row and top-level case field as follows.

| Product presave field | DG/case target | Notes |
|---|---|---|
| `productId` | `dgPrdKey` | Direct assignment. This drives the visible DG Product ID / `DG_PRD_KEY`. |
| `drugCharacterization` | `drugs[index].drugCharacterization` | Direct assignment. |
| `medicinalProduct` | `drugs[index].medicinalProduct` | Direct assignment. |
| `mpidVersionDateNumber` | `drugs[index].mpidVersion` | Direct assignment. |
| `mpid` | `drugs[index].mpid` | Direct assignment. |
| `phpidVersionDateNumber` | `drugs[index].phpidVersion` | Direct assignment. |
| `phpid` | `drugs[index].phpid` | Direct assignment. |
| `obtainDrugCountry` | `drugs[index].obtainDrugCountry` | Direct assignment. |
| `investigationalProductBlinded` | `drugs[index].investigationalProductBlinded` | Direct assignment; blank/undefined clears to undefined. |
| `drugAuthorizationNumber` | `drugs[index].drugAuthorizationNumber` | Direct assignment. |
| `drugAuthorizationCountry` | `drugs[index].drugAuthorizationCountry` | Direct assignment. |
| `drugAuthorizationHolder` | `drugs[index].drugAuthorizationHolder` | Direct assignment. |
| `substances` | `drugs[index].activeSubstances` | Replace the substance rows from the selected product template. |

Substance row mapping:

| Product substance field | DG substance target |
|---|---|
| `name` | `substanceName` |
| `termIdVersion` | `substanceTermIdVersion` |
| `termId` | `substanceTermId` |
| `strengthNumber` | `substanceStrengthValue` as number when parseable, otherwise undefined |
| `strengthUnit` | `substanceStrengthUnit` |

## Excluded Fields

The following product presave fields should not be imported into DG by this workflow unless a later reference check shows a DG target:

- `dgPrdKey` on product presave: local storage does not support this as product template identity; use `productId`.
- `preApprovalIpName`: selector/list metadata, not a confirmed DG case target.
- `drugBrandName`, `drugGenericName`, `drugBatchNumber`: local SectionG has fields, but the reference `DG_PRD_KEY` import evidence for this workflow maps the product template into the official DG product fields above. These should remain out of the first alignment pass unless a reference product row demonstrates they populate DG.
- `manufacturerName`, `manufacturerCountry`, `originalManufacturer`: selector/list or product metadata; do not fallback into authorization holder/country.
- `sender`, `senderPresaveId`: product list/scope metadata, not DG drug data.
- `productDescription`, `productDeleted`: product-template administration data.
- `mfdsStudyNumber`, `mfdsProtocolNumber`, `mfdsOtherStudiesType`: study/regional C.5 data, not DG product import data.
- `fdaIndNumberOccurred`, `fdaPreAndaNumberOccurred`, `fdaCrossReportedIndNumbers`: FDA C.5 data, not DG product import data.
- `mfdsRegionalItems`: not part of this DG product import until a concrete DG target is modeled.
- Alias-only fields such as `drugObtainedCountry`, `authorizationNumber`, `authorizationCountry`, `holderApplicantName`, and `holderApplicantNameNotation`: canonical mapping can normalize backend aliases, but the DG import mapper should read the direct canonical fields only.

## Behavior Details

Importing a product presave should:

1. Determine the active DG drug index from the currently edited/open row.
2. Update that row in place instead of appending.
3. Set every mapped target, even if the selected value is empty.
4. Replace active substances with the selected product template's substance rows.
5. Mark changed fields dirty so the user can save the imported values.
6. Keep unrelated existing DG data intact, including dosage, indications, drug reaction assessments, additional information, and device-specific sections.
7. Keep product selector/list columns as display metadata only.

If there is no active drug row, the implementation may create one and then apply the selected product. That fallback should be reserved for explicit empty-state behavior and covered by tests.

## Testing

Add TDD tests around the actual SectionG import path:

- selecting a product presave updates the current DG row, not a new appended row;
- `productId` sets top-level `dgPrdKey`;
- blank product fields clear existing current-row values;
- active substances are replaced from the product presave;
- unrelated current-row case fields, such as dosage and indications, survive the import;
- excluded metadata fields do not leak into DG targets;
- direct mapping is used without fallback unions.

Prefer a focused component-level test using the real SectionG import picker path, plus small pure mapper tests if the implementation extracts a helper.

## Implementation Notes

Introduce a small product-to-DG mapper/helper rather than expanding `handleImportProduct` inline. The helper should accept a `ProductPresaveData` and return the precise field patch for a DG row plus the top-level `dgPrdKey` update. Keeping this logic isolated makes the blank-clearing and excluded-field contract testable.

`PresaveImportPicker` may need a mode or callback context so SectionG can use the same picker UI while applying the result to the active row. Avoid changing global presave CRUD behavior.

## Open Questions

No open functional questions. The chosen behavior is to match reference: selected product blanks clear mapped DG fields.
