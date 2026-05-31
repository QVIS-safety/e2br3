# Product Presave Implementation Actions

## Goal

Align local Product Presave and DG import behavior to the reference architecture without fallback fields, fake fields, or ambiguous generic regional storage.

## Actions

### 1. Tighten DG Import to Reference-Copied Fields Only

Update `components/case-form/sections/SectionG.tsx` Product Presave import so it copies only:

- `medicinalProduct`
- `medicinalProductNotation`
- `obtainDrugCountry`
- `investigationalProductBlinded`
- `mpidVersionDateNumber`
- `mpid`
- `phpidVersionDateNumber`
- `phpid`
- `mfdsMpidVersion`
- `mfdsMpid`
- `drugAuthorizationNumber`
- `drugAuthorizationCountry`
- `drugAuthorizationHolder`
- `holderApplicantNameNotation`
- `substances`
- `mfdsDeviceInfo`

Remove current DG import of:

- `drugCharacterization`
- `drugBrandName`
- `drugGenericName`
- `drugBatchNumber`
- fallback aliases such as `drugObtainedCountry`, `authorizationNumber`, `authorizationCountry`, `holderApplicantName`, `manufacturerName`, and `manufacturerCountry`

### 2. Add Missing Notation Imports

Reference import copies notation fields:

- `productDrugNotation.productName` -> DG product name notation
- `productDrugNotation.authName` -> DG holder/applicant notation

Local import should wire:

- `medicinalProductNotation`
- `holderApplicantNameNotation`

If the local DG form model does not currently store the corresponding notation paths, add those case fields explicitly rather than dropping notation during import.

### 3. Keep Reference Metadata as Product Presave Metadata

Keep these fields on Product Presave, but never import them into DG:

- `preApprovalIpName`
- `drugBrandName`
- `originalManufacturer`
- `sender`
- `senderPresaveId`
- `productDescription`
- `productDeleted`

Normalize naming so original manufacturer uses one Product Presave field family. Do not keep both `originalManufacturer` and `manufacturerName` as independent sources for the same reference `mnftName` concept.

### 4. Remove Drift Fields from Product Presave

Remove these from frontend type/schema/forms/mappers/tests and backend model/bootstrap/API where present:

- `drugCharacterization`
- `drugGenericName`
- `drugBatchNumber`
- `manufacturerName`
- `manufacturerCountry`
- `mfdsStudyNumber`
- `mfdsProtocolNumber`
- `mfdsOtherStudiesType`
- `fdaIndNumberOccurred`
- `fdaPreAndaNumberOccurred`
- `fdaCrossReportedIndNumbers`
- `mfdsRegionalItems`
- `drugObtainedCountry`
- `authorizationNumber`
- `authorizationCountry`
- `holderApplicantName`

Study/FDA C.5 fields should be handled in Study Presave if required, not Product Presave.

### 5. Preserve Local-System-Only Fields with Tests

Keep local-only fields only when they have a bounded implementation reason:

- `productId`: local template identity/display only; not imported to DG.
- `mfdsDeviceItems`: storage/normalization backing for `mfdsDeviceInfo`; not a separate generic Product Presave concept.

Tests should assert that these are not imported directly into DG.

### 6. Test Matrix

Add or update TDD tests so Product Presave import is checked by authority:

| Authority | Required Assertions |
|---|---|
| ICH/NA | imports common product drug fields, notation fields, substances; does not import metadata or aliases |
| FDA/US | same as ICH for DG; FDA C.5 study fields are absent from Product Presave |
| MFDS/KR | imports common fields, notation fields, MFDS MPID fields, MFDS substance fields, and full KR device graph |

Also add negative tests for removed/fallback fields:

- no `dgPrdKey` from Product Presave;
- no alias fallback behavior;
- no metadata-to-DG import;
- no generic `mfdsRegionalItems` import.

### 7. Backend and Bootstrap Cleanup

Because the current DB is a test DB, update initial/bootstrap scripts directly instead of adding migrations:

- remove drift columns/tables for Product Presave;
- keep explicit Product Presave substance rows;
- keep explicit MFDS device backing rows only if they are required to reconstruct `mfdsDeviceInfo`;
- remove generic MFDS regional item Product Presave storage.

### 8. Verification

Run at minimum:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npm test -- __tests__/dashboard/canonical-presave-mappers.test.ts --runInBand
npm test -- __tests__/ui-binding/field-bindings.test.ts --runInBand --testNamePattern="product presave|Product Presave|MFDS device|source-family"
npm test -- __tests__/dashboard/info-presave-detail-route.test.tsx --runInBand --testNamePattern="ProductForm MFDS device"
```

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/e2br3
cargo test -p web-server --test api product_presave_details -- --nocapture
cargo test -p lib-core --test section_presave product_presave_mfds_device_items_round_trip -- --nocapture
cargo fmt --check
```

If frontend lint remains blocked by the interactive Next.js ESLint setup prompt, record it explicitly rather than claiming lint passed.
