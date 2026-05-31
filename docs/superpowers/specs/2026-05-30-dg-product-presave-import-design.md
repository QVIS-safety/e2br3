# DG Product Presave Import Architecture

## Context

The reference EDU cubeSAFETY DG screen treats `DG_PRD_KEY` as the product-template selector for the current DG drug row. The user opens it from the DG row, selects one visible product from `Select Products`, and the selected product template is applied to that same row.

This was checked through the UI on the three authority variants:

| Reference route | Authority | Selector columns | Observed import shape |
|---|---|---|---|
| `NA/case/786996/detail/DG/1` | ICH | `Brand Name`, `Pre-IP Name`, `Sender`, `Manufacturer` | Current row updated in place. Blank template values cleared mapped DG fields. Existing dosage/case fields remained. |
| `US/case/786994/detail/DG/1` | FDA | Same product selector columns | Same product import behavior. FDA-only DG fields were visible but not populated by the selected product row. |
| `KR/case/786994/detail/DG/1` | MFDS | Same product selector columns | Same product import behavior. MFDS-only DG fields were visible but not populated by the selected product row. |

The current local `SectionG` behavior is different: `handleImportProduct` appends a new drug row and uses fallback unions such as authorization country falling back to manufacturer country. That creates confusing data movement and does not match the reference workflow.

## Goal

Wire product presaves into DG with a precise, one-way import contract:

- selecting a product presave from `DG_PRD_KEY` updates the active DG drug row;
- the product presave identity `productId` is written to the case-level `dgPrdKey`;
- each mapped field is copied directly, with no fallback unions;
- blank product-template values clear the corresponding mapped DG fields;
- authority-specific DG fields are preserved unless the product presave has an exact modeled source for that same DG field.

## Source Of Truth

There are two different concepts that must not be merged:

| Concept | Local source | Local target | Meaning |
|---|---|---|---|
| Product template identity | `product_presaves.product_id` / `ProductPresaveData.productId` | `cases.dg_prd_key` / form `dgPrdKey` | The selected product template key displayed in DG as `DG_PRD_KEY`. |
| Product template row ID | `product_presaves.id` | none in DG | Internal UUID for CRUD, access checks, and deletion protection. It is not the DG product key. |

`ProductPresaveData.dgPrdKey` exists in the frontend type surface, but the current product-presave table does not have `product_presaves.dg_prd_key`. It must not be used as an import fallback. The DG import source for the visible product key is only `productId`.

## Reference-Aligned User Flow

1. User is editing a case DG row.
2. User opens the `DG_PRD_KEY` selector.
3. Selector lists available product presaves with list metadata: brand name, pre-IP name, sender, and manufacturer.
4. User selects one product presave.
5. The active DG row is patched in place.
6. Top-level `dgPrdKey` is set to the selected product's `productId`.
7. Mapped DG product fields are replaced by selected product values.
8. Unmapped case-specific DG fields remain unchanged.

The import must not append a new `drugs[]` row except in an explicit empty-state fallback where there is no active DG row.

## Local Files

Primary frontend files:

- `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/case-form/sections/SectionG.tsx`
- `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/types/presave.ts`
- `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/schemas/presave.ts`
- `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/presave/canonicalMappers.ts`
- `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/presave/canonicalWriteMappers.ts`

Primary backend/schema files:

- `/Users/hyundonghoon/projects/rust/e2br3/e2br3/db/bootstrap/01-safetydb-schema.sql`
- product presave BMC/API model files used by existing presave CRUD
- case BMC/API model files used by SectionG read/write

## Field Mapping

The product import mapper should read a `ProductPresaveData` and return:

- `casePatch`: top-level case values such as `dgPrdKey`;
- `drugPatch`: the replacement values for the active `drugs[index]` product fields;
- `substanceRows`: replacement rows for `drugs[index].activeSubstances`.

### Case-Level Mapping

| Product presave field | DG/case target | Rule |
|---|---|---|
| `productId` | `dgPrdKey` | Direct copy. No fallback to `dgPrdKey`, `mpid`, `medicinalProduct`, or any other field. |

### DG Product Field Mapping

| Product presave field | DG target | Rule |
|---|---|---|
| `drugCharacterization` | `drugs[index].drugCharacterization` | Direct copy. |
| `medicinalProduct` | `drugs[index].medicinalProduct` | Direct copy. |
| `medicinalProductNotation` | `drugs[index].medicinalProductNotation` | Direct copy if the local DG form/model exposes this notation field. |
| `mpidVersionDateNumber` | `drugs[index].mpidVersion` | Direct copy for the non-regional ICH/base MPID field only. Do not use this as storage for regional MFDS product-code fields. |
| `mpid` | `drugs[index].mpid` | Direct copy for the non-regional ICH/base MPID field only. Do not use this as storage for regional MFDS product-code fields. |
| `phpidVersionDateNumber` | `drugs[index].phpidVersion` | Direct copy. |
| `phpid` | `drugs[index].phpid` | Direct copy. |
| `obtainDrugCountry` | `drugs[index].obtainDrugCountry` | Direct copy. |
| `drugBrandName` | `drugs[index].drugBrandName` | Direct copy. DG BMC stores this as `brand_name`. |
| `drugGenericName` | `drugs[index].drugGenericName` | Direct copy. DG BMC stores this as `drug_generic_name`. |
| `investigationalProductBlinded` | `drugs[index].investigationalProductBlinded` | Direct copy; absent/blank clears to undefined. |
| `drugAuthorizationNumber` | `drugs[index].drugAuthorizationNumber` | Direct copy. |
| `drugAuthorizationCountry` | `drugs[index].drugAuthorizationCountry` | Direct copy. |
| `drugAuthorizationHolder` | `drugs[index].drugAuthorizationHolder` | Direct copy. |
| `drugBatchNumber` | `drugs[index].drugBatchNumber` | Direct copy. DG BMC stores this as `batch_lot_number`. |

No field in this table may use an alias fallback. In particular:

- do not read `drugObtainedCountry` when importing `obtainDrugCountry`;
- do not read `authorizationNumber` when importing `drugAuthorizationNumber`;
- do not read `authorizationCountry` or `manufacturerCountry` when importing `drugAuthorizationCountry`;
- do not read `holderApplicantName` or `manufacturerName` when importing `drugAuthorizationHolder`.

Backend/frontend canonical mappers may normalize legacy API aliases at the CRUD boundary, but SectionG's DG import mapper must consume only the canonical product-presave fields above.

### Substance Mapping

Selecting a product presave replaces the active DG row's `activeSubstances` with the product template's `substances`.

| Product substance field | DG substance target | Rule |
|---|---|---|
| `name` | `substanceName` | Direct copy. |
| `termIdVersion` | `substanceTermIdVersion` | Direct copy. |
| `termId` | `substanceTermId` | Direct copy. |
| `strengthNumber` | `substanceStrengthValue` | Parse as number. Empty or unparseable becomes undefined. |
| `strengthUnit` | `substanceStrengthUnit` | Direct copy. |

If the selected product has no substances, the active row's mapped substance values should be cleared/replaced consistently with the reference behavior. Do not preserve old substance values just because the template is blank.

## Regional Field Storage Architecture

DG case storage must not collapse regional fields into generic fields. If a reference field has a regional code, the case BMC, frontend type, schema, save mapper, read mapper, and SectionG binding must expose a distinct field for that code.

Generic fields such as `mpid`, `mpidVersion`, `substanceTermId`, and `substanceTermIdVersion` may remain for ICH/base fields and backwards migration, but they must not be the canonical storage for MFDS-coded fields.

The target architecture is:

| Reference field code | Required canonical DG case target | Legacy/current target to migrate away from |
|---|---|---|
| `G.k.2.1.1a` | `drugs[index].mpidVersion` | same |
| `G.k.2.1.1b` | `drugs[index].mpid` | same |
| `G.k.2.1.KR.1a` | `drugs[index].mfdsProductCodeVersion` | `drugs[index].mpidVersion` |
| `G.k.2.1.KR.1b` | `drugs[index].mfdsProductCode` | `drugs[index].mpid` |
| `G.k.2.3.r.2a` | `drugs[index].activeSubstances[n].substanceTermIdVersion` | same |
| `G.k.2.3.r.2b` | `drugs[index].activeSubstances[n].substanceTermId` | same |
| `G.k.2.3.r.1.KR.1a` | `drugs[index].activeSubstances[n].mfdsSubstanceCodeVersion` | `substanceTermIdVersion` |
| `G.k.2.3.r.1.KR.1b` | `drugs[index].activeSubstances[n].mfdsSubstanceCode` | `substanceTermId` |

The selector workflow is the same for ICH, FDA, and MFDS. Authority affects which DG fields are visible and which explicit regional field is written; it must not change the meaning of a generic field.

The import rule is authority-aware: every product-presave field with an exact DG target must be moved, including FDA and MFDS regional fields. Fields are preserve-only only when the product presave model has no exact source for that DG target, or when the only available source is generic metadata that would require guessing.

## Authority-Specific Behavior

### ICH / NA

Use the base mapping only. Preserve all non-product DG fields, including dosage, indication, assessment, rechallenge, and additional information.

### FDA / US

The reference FDA route shows additional DG fields such as:

- `FDA.G.k.1.a`
- `FDA.G.k.10a`
- `FDA.G.k.10.1`
- `FDA.G.k.12.r.*` device information

The selected product row did not populate those fields during the UI check because the selected template had no visible values for them. The implementation should still move FDA product-presave fields when an exact field-to-field DG mapping exists. It must preserve FDA fields only when no exact product-presave source exists.

Add an FDA-specific product import mapping when all of these are true:

1. the product presave BMC has an exact source field for the FDA DG field;
2. the local SectionG case model has the exact target field;
3. reference UI evidence shows `DG_PRD_KEY` import populates that field.

### MFDS / KR

The reference MFDS route shows additional DG fields such as:

- `G.k.2.1.KR.1a`
- `G.k.2.1.KR.1b`
- `G.k.2.3.r.1.KR.1a`
- `G.k.2.3.r.1.KR.1b`
- KR device fields including `KR_DVC_SN`, `KR_DVC_MFR`, `KR_DVC_MDL`, `KR_DVC_LOT`, `KR_DVC_UDI_DI`, `KR_DVC_UDI_SN`, `KR_DVC_UDI_LOT`, `KR_DVC_UDI_MFD`, `KR_DVC_UDI_EXP`, `KR_DVC_IMPD`, `KR_DVC_PROBC`, `KR_DVC_CIC_TYPE`, `KR_DVC_CIC_ASMT`, `KR_DVC_CIC_CON`, `KR_DVC_HIC_CSC`, `KR_DVC_HIC_HO`, and `KR_DVC_CMPC`.

The selected product row did not populate the KR device block during the UI check because the selected template had no visible values for it. The implementation should still move MFDS product-presave fields when an exact field-to-field DG mapping exists. It must preserve `mfdsDeviceInfo` entries only when no exact product-presave source exists.

MFDS product/substance fields must be separate case fields, not aliases of generic MPID or generic substance term fields.

Required mappings after the DG case model is split:

| Product presave source | DG case target | Reference field |
|---|---|---|
| selected MFDS product code source for the case context | `drugs[index].mfdsProductCode` | `G.k.2.1.KR.1b` |
| selected MFDS product code version source for the case context | `drugs[index].mfdsProductCodeVersion` | `G.k.2.1.KR.1a` |
| selected MFDS ingredient code source for the case context | `drugs[index].activeSubstances[n].mfdsSubstanceCode` | `G.k.2.3.r.1.KR.1b` |
| selected MFDS ingredient code version source for the case context | `drugs[index].activeSubstances[n].mfdsSubstanceCodeVersion` | `G.k.2.3.r.1.KR.1a` |

The source selection among `mfds_domestic_*`, `mfds_udl_*`, `mfds_foreign_ich_*`, and `mfds_foreign_e2b_*` must be deterministic based on current case authority/report-type context. It must not be implemented as a fallback chain.

### MFDS Regional Presave Coverage Matrix

The product presave stores multiple MFDS code families because the same visible DG regional field can be sourced differently depending on the MFDS reporting context. The DG case model must store the selected effective value in explicit regional DG fields, while preserving enough source clarity in the import rule to avoid fallback behavior.

| Product presave field | Role | DG target | Context rule | Import status |
|---|---|---|---|---|
| `mfds_domestic_product_code` | Product code | `drugs[index].mfdsProductCode` (`G.k.2.1.KR.1b`) | Use when MFDS context is domestic/post-market KR for a domestic product. | Mapped. |
| `mfds_domestic_ingredient_code` | Ingredient/substance code | `drugs[index].activeSubstances[n].mfdsSubstanceCode` (`G.k.2.3.r.1.KR.1b`) | Use with the matching domestic product-code context. | Mapped. |
| `mfds_udl_product_code` | Product code | `drugs[index].mfdsProductCode` (`G.k.2.1.KR.1b`) | Use only when MFDS context explicitly selects the UDL source family, such as CT/CU/unapproved-drug-list workflow when confirmed by business/reference rule. | Mapped after explicit context selector is added. |
| `mfds_udl_ingredient_code` | Ingredient/substance code | `drugs[index].activeSubstances[n].mfdsSubstanceCode` (`G.k.2.3.r.1.KR.1b`) | Use with the matching UDL product-code context. | Mapped after explicit context selector is added. |
| `mfds_udl_manufacturer_code` | Manufacturer code | New explicit DG field if reference/BMC confirms one; otherwise not DG product import. | Do not import into holder/manufacturer name or product code. | Not mapped until an exact DG field code is identified. |
| `mfds_udl_manufacturer_name` | Manufacturer name | New explicit DG field if reference/BMC confirms one; otherwise not DG product import. | Do not import into `drugAuthorizationHolder` or FDA/MFDS device manufacturer fields. | Not mapped until an exact DG field code is identified. |
| `mfds_foreign_ich_product_code` | Product code | `drugs[index].mfdsProductCode` (`G.k.2.1.KR.1b`) | Use only when MFDS foreign context explicitly selects the foreign-ICH source family. | Mapped after explicit context selector is added. |
| `mfds_foreign_ich_ingredient_code` | Ingredient/substance code | `drugs[index].activeSubstances[n].mfdsSubstanceCode` (`G.k.2.3.r.1.KR.1b`) | Use with the matching foreign-ICH product-code context. | Mapped after explicit context selector is added. |
| `mfds_foreign_ich_holder_code` | Holder code | New explicit DG field if reference/BMC confirms one; otherwise not DG product import. | Do not import into `drugAuthorizationHolder`; code and name are different concepts. | Not mapped until an exact DG field code is identified. |
| `mfds_foreign_ich_holder_name` | Holder name | New explicit DG field if reference/BMC confirms one; otherwise not DG product import. | Do not fallback into `drugAuthorizationHolder`; only direct-map if this exact regional holder field is modeled. | Not mapped until an exact DG field code is identified. |
| `mfds_foreign_e2b_product_code` | Product code | `drugs[index].mfdsProductCode` (`G.k.2.1.KR.1b`) | Use only when MFDS foreign context explicitly selects the foreign-E2B source family. | Mapped after explicit context selector is added. |
| `mfds_foreign_e2b_ingredient_code` | Ingredient/substance code | `drugs[index].activeSubstances[n].mfdsSubstanceCode` (`G.k.2.3.r.1.KR.1b`) | Use with the matching foreign-E2B product-code context. | Mapped after explicit context selector is added. |
| `mfds_foreign_e2b_holder_code` | Holder code | New explicit DG field if reference/BMC confirms one; otherwise not DG product import. | Do not import into `drugAuthorizationHolder`; code and name are different concepts. | Not mapped until an exact DG field code is identified. |
| `mfds_foreign_e2b_holder_name` | Holder name | New explicit DG field if reference/BMC confirms one; otherwise not DG product import. | Do not fallback into `drugAuthorizationHolder`; only direct-map if this exact regional holder field is modeled. | Not mapped until an exact DG field code is identified. |

The context selector is a required part of implementation. It should produce one source family, for example `domestic`, `udl`, `foreign_ich`, or `foreign_e2b`, based on MFDS case/report context. The importer then reads only that family. It must not scan the families in order and take the first non-empty value.

Version fields are also required for complete regional mapping. The current product presave BMC has product/ingredient code fields but does not expose separate MFDS product-code-version or ingredient-code-version columns for every family. Until those are modeled, `mfdsProductCodeVersion` and `mfdsSubstanceCodeVersion` can only be filled from exact version fields when they exist; they must not be guessed from generic `mpid_version` or generic substance term version.

## Excluded Product Presave Fields

These fields are not part of DG product import in the first aligned implementation:

| Field | Reason |
|---|---|
| `dgPrdKey` | Not persisted on `product_presaves`; using it would create a confusing fallback path. |
| `preApprovalIpName` | Selector/list metadata unless a later DG target is proven. |
| `manufacturerName` | Product/presave manufacturer metadata. Do not import into `drugAuthorizationHolder`; that target has the exact source `drugAuthorizationHolder`. Only map this later if the DG target is explicitly `drugs[index].manufacturerName` in the import path. |
| `manufacturerCountry` | Product/presave manufacturer metadata. Do not import into `drugAuthorizationCountry`; that target has the exact source `drugAuthorizationCountry`. Only map this later if the DG target is explicitly `drugs[index].manufacturerCountry` in the import path. |
| `originalManufacturer` | Metadata; no proven DG target. |
| `sender` | Product scope/list metadata, not DG drug data. |
| `senderPresaveId` | Internal relation/scope metadata, not DG drug data. |
| `productDescription` | Admin/template metadata. |
| `productDeleted` | Admin/template lifecycle metadata. |
| `mfdsStudyNumber`, `mfdsProtocolNumber`, `mfdsOtherStudiesType` | Study/regional C.5 style fields, not DG product import fields. |
| `fdaIndNumberOccurred`, `fdaPreAndaNumberOccurred`, `fdaCrossReportedIndNumbers` | FDA C.5 style fields, not DG product import fields. |
| `mfdsRegionalItems` | Generic regional storage; no exact DG target/import behavior proven. |
| `drugObtainedCountry`, `authorizationNumber`, `authorizationCountry`, `holderApplicantName`, `holderApplicantNameNotation` | Alias/legacy fields; do not read in SectionG import. |

## Frontend Architecture

Create a small mapper instead of keeping import logic inline in `SectionG`.

Suggested shape:

```ts
type DgProductImportPatch = {
  dgPrdKey: string;
  drugPatch: Partial<DrugInformation>;
  activeSubstances: DrugSubstance[];
};

function mapProductPresaveToDgImport(product: ProductPresaveData): DgProductImportPatch;
```

The mapper should:

- read only canonical fields listed in this spec;
- return blank strings for blank string targets so blanks clear existing DG values;
- return undefined for blank optional booleans/numbers;
- replace substances rather than merge;
- write regional fields only to explicit regional case targets, never to generic aliases.

`SectionG.handleImportProduct` should:

1. resolve the active drug index;
2. create a drug row only if no row exists;
3. call the mapper;
4. `setValue("dgPrdKey", patch.dgPrdKey, { shouldDirty: true })`;
5. patch `drugs[index]` mapped fields with `{ shouldDirty: true }`;
6. replace `drugs[index].activeSubstances`;
7. leave all non-product fields on `drugs[index]` untouched.

`PresaveImportPicker` should remain a picker/list concern. It should not know DG field mapping rules.

## Backend/BMC Architecture

Do not add a new backend DG import endpoint for this behavior. The reference workflow is a frontend form import followed by normal case save.

Backend responsibilities:

- product presave CRUD continues to persist template fields in `product_presaves` and child tables;
- case save continues to persist `cases.dg_prd_key` and DG row fields through the existing case write path;
- user/product access and deletion protection may use product presave UUIDs internally, but DG case data stores the product key string in `dg_prd_key`, not the presave UUID.

Backend model changes required by this architecture:

- add explicit DG case BMC fields/columns or structured payload keys for regional field codes, starting with MFDS `G.k.2.1.KR.1a`, `G.k.2.1.KR.1b`, `G.k.2.3.r.1.KR.1a`, and `G.k.2.3.r.1.KR.1b`;
- update case API DTOs, OpenAPI, read mappers, write mappers, validation paths, XML import/export, and tests to use those explicit fields;
- add migration/backfill from current generic `mpid`/`mpidVersion` and substance term fields into the new MFDS fields only when the case/report context proves those values were regional MFDS values.

Backend model cleanup to consider during implementation:

- keep `product_presaves.product_id` as the product-template key source;
- do not add `product_presaves.dg_prd_key` just for this import;
- remove or deprecate frontend-only `ProductPresaveData.dgPrdKey` once canonical mapper usage is verified, or leave it unused with tests proving import does not read it.

## Data Preservation Rules

Product import must preserve these active-row fields:

- dosage information;
- indication rows;
- drug-action and assessment fields;
- rechallenge/recurrence fields;
- additional information;
- FDA additional/device fields that do not have an exact product-presave source;
- MFDS regional/device fields that do not have an exact product-presave source;
- any current row fields not listed in the mapping table.

Product import must replace these active-row fields:

- top-level `dgPrdKey`;
- mapped base product identity/name/code/authorization fields;
- mapped FDA/MFDS regional product fields into explicit regional case targets when exact product-presave sources exist;
- active substances.

## Testing Requirements

Use TDD before implementation.

Add tests with actual SectionG import behavior for at least ICH, FDA, and MFDS authority modes:

1. selecting a product presave updates the active row instead of appending;
2. `productId` writes top-level `dgPrdKey`;
3. `ProductPresaveData.dgPrdKey`, `mpid`, `medicinalProduct`, and other fields are not used as fallback product keys;
4. blank mapped product fields clear existing DG product fields;
5. existing dosage/indication/assessment data remains unchanged;
6. existing FDA fields with no exact product-presave source remain unchanged on FDA import;
7. existing MFDS device/regional fields with no exact product-presave source remain unchanged on MFDS import;
8. substances are replaced from the selected product template;
9. excluded metadata fields do not leak into DG targets;
10. no authorization/manufacturer fallback unions are used.
11. MFDS product/substance code imports write to explicit MFDS case fields, not to generic `mpid` or generic substance term fields.

Mapper unit tests are allowed, but at least one test per authority should exercise the real SectionG import path or the same callback path used by `PresaveImportPicker`.

## Acceptance Criteria

Implementation is complete when:

- DG product selection behaves like the reference: current row update, no append;
- product key import is `productId -> dgPrdKey` only;
- direct mapping clears blanks and avoids fallback unions;
- ICH/FDA/MFDS tests prove authority-specific fields are moved when exactly mapped and preserved only when no exact product-presave source exists;
- regional coded DG fields are stored separately by field code, not overloaded into generic fields;
- MFDS regional import uses an explicit source-family selector and never falls through from domestic to UDL to foreign ICH/E2B values;
- no BMC CRUD behavior changes are required for ordinary product presave create/update/list;
- normal case save persists imported values through the existing case save path.

## Open Questions

The implementation plan must resolve one business/reference rule before coding MFDS regional import: which case/report context selects `udl`, `foreign_ich`, or `foreign_e2b` as the product-presave source family. Domestic KR mapping is explicit. The architecture intentionally avoids guessing from generic metadata or fallback aliases.
