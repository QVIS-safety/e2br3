# Product Presave Reference Audit Design

## Goal

Create a closed, evidence-backed contract for **Product Presave -> DG import** so the answer to "are all Product Presave fields imported/aligned?" is deterministic and does not restart the same investigation loop.

The audit must compare three sources of truth:

1. Local Product Presave contract.
2. Reference Product Presave UI/payload behavior.
3. Reference DG import behavior after selecting a Product Presave.

The result is a matrix with zero ambiguous rows. Implementation work happens only after the matrix is complete.

## Non-Goals

- Do not audit Sender or Study in this pass.
- Do not decide behavior from local code alone.
- Do not scrape, crawl, download unrelated data, replay requests, or call inferred reference APIs directly.
- Do not add migrations for this audit.
- Do not preserve fields only because they already exist locally.

## Current Problem

The current local test classifies Product Presave fields into `importedToDg` and `preserveOnly`. That is useful accounting, but it is not enough.

`preserveOnly` is currently provisional because it has not been proven against the reference. A field may be in `preserveOnly` for one of three very different reasons:

- The reference also keeps it only on the Product Presave template.
- The reference imports it to DG, and our local mapping is missing.
- The field is local-only or obsolete and should be removed.

The audit resolves those cases with evidence.

## Closed Classification

Every field must end in exactly one status:

| Status | Meaning | Required Action |
|---|---|---|
| `referenceImportedToDg` | Field exists in reference Product Presave and reference import populates DG. | Local Product Presave must import it to the matching DG field. |
| `referencePreserveOnly` | Field exists in reference Product Presave but reference import does not populate DG. | Keep locally only if our behavior matches reference. |
| `localSystemOnly` | Field does not exist in reference, but local system mechanics require it. | Keep with explicit local rationale and tests proving it does not import to DG. |
| `remove` | Field does not exist in reference and has no required local role. | Remove from type/schema/UI/API/mappers/tests. |
| `missingLocal` | Field exists in reference Product Presave but is missing locally. | Add local field and persistence; map import only if reference imports it. |

`ambiguous` is allowed only during investigation. The final matrix must contain zero `ambiguous` rows.

## Local Inventory Scan

Build the local side of the matrix from these files:

- `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/types/presave.ts`
- `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/schemas/presave.ts`
- `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/presave/ProductForm.tsx`
- `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/presave/canonicalMappers.ts`
- `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/presave/canonicalWriteMappers.ts`
- `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/case-form/sections/SectionG.tsx`
- Backend Product Presave model/API/test files under `/Users/hyundonghoon/projects/rust/e2br3/e2br3/crates`

For each local field, record:

- `localField`
- `localType`
- `productFormControl`
- `frontendSchemaKey`
- `frontendReadMapperKey`
- `frontendWriteMapperKey`
- `backendDtoField`
- `backendColumnOrChildTable`
- `currentLocalImportTarget`
- `currentLocalCategory`

The current local Product Presave fields are:

```text
productId
drugCharacterization
medicinalProduct
medicinalProductNotation
drugBrandName
drugGenericName
obtainDrugCountry
drugAuthorizationCountry
drugAuthorizationHolder
drugAuthorizationNumber
drugBatchNumber
manufacturerName
manufacturerCountry
preApprovalIpName
originalManufacturer
sender
senderPresaveId
productDescription
productDeleted
mpidVersionDateNumber
mpid
mfdsMpidVersion
mfdsMpid
mfdsDeviceInfo
mfdsDeviceItems
phpidVersionDateNumber
phpid
mfdsStudyNumber
mfdsProtocolNumber
mfdsOtherStudiesType
fdaIndNumberOccurred
fdaPreAndaNumberOccurred
fdaCrossReportedIndNumbers
substances
mfdsRegionalItems
drugObtainedCountry
investigationalProductBlinded
authorizationNumber
authorizationCountry
holderApplicantName
holderApplicantNameNotation
```

`dgPrdKey` is not a local Product Presave field. It may appear only as a sentinel proving Product Presave import ignores case-owned DG row keys.

## Reference Investigation Rules

Use `api-workflow-payload-investigator`.

Allowed:

- Open the specific reference Product Presave UI and DG case route in Chrome.
- Inspect JavaScript bundles naturally loaded by those pages.
- Download only those naturally loaded static JS assets to a temporary directory outside the repo.
- Search bundles for Product Presave form fields, save/load payload keys, DG import methods, import-copy functions, endpoint paths, and response field usage.
- Use live Chrome network capture only when bundle inspection cannot determine exact field movement.
- If live capture is needed, perform only normal UI actions on the requested record/workflow and passively observe relevant request/response shapes.

Forbidden:

- Direct `fetch`, `curl`, browser-address-bar API calls, generated clients, or replayed requests.
- Broad crawling, endpoint enumeration, unrelated route inspection, data extraction, or scraping.
- Capturing unrelated user data.
- Keeping secrets, tokens, cookies, IDs, emails, names, or free-text case values in the report.

All captured values must be redacted while preserving shape and type.

## Reference Scan Workflow

The reference audit has two passes.

### Pass 1: Static Bundle Evidence

1. Open the reference Product Presave page in Chrome.
2. Identify loaded route/app JS bundles.
3. Inspect only relevant loaded bundles.
4. Extract evidence for:
   - Product Presave form field names.
   - Product Presave save/load payload keys.
   - Product Presave list/display keys.
   - Authority-specific Product Presave fields for ICH, FDA, and MFDS.
5. Open the reference DG route.
6. Inspect loaded DG bundles.
7. Extract evidence for:
   - Product Presave import UI action.
   - Import endpoint or client method.
   - Response fields consumed by DG.
   - Copy/mapping function from selected Product Presave into DG case state.

### Pass 2: UI/Network Evidence

Use this only for fields still unresolved by bundle evidence.

1. Create or select a Product Presave containing safe sentinel values for unresolved fields.
2. Import it into DG through the reference UI.
3. Observe relevant network calls and UI state through DevTools.
4. Record only:
   - request field names and types
   - response field names and types
   - DG target field names or field codes
   - whether the sentinel value appeared in DG
5. Redact all real data.

## Matrix Format

The final artifact must be a markdown table with one row per field, including reference-only fields not currently local.

| Field | Local? | Reference Product Presave? | Reference Payload Key | Reference DG Target | Reference Imports? | Local DG Target | Final Status | Evidence |
|---|---:|---:|---|---|---:|---|---|---|

Evidence must cite one or more of:

- local file path and line or symbol
- reference bundle name and symbol/client method
- reference UI/network observation summary
- local test name

No row may have empty `Final Status`.
No row may have `ambiguous` in the final approved matrix.

## Decision Rules

1. If a field is in reference Product Presave and reference imports it to DG, local must import it.
2. If a field is in reference Product Presave and reference does not import it to DG, local may keep it as `referencePreserveOnly`.
3. If a field is local-only, keep it only when it is required for local mechanics such as record identity, relationship, deletion state, or audit/list display.
4. If a local-only field is merely historical, redundant, or unreferenced, remove it.
5. If reference has a field we lack, add it as `missingLocal`; decide import target by reference import behavior.
6. Fallback mappings are allowed only when reference demonstrates equivalent fallback behavior or the local field is explicitly a synonym for the same reference field.
7. Authority-specific fields must be evaluated per authority: ICH, FDA, MFDS.

## Deliverables

1. Local inventory matrix.
2. Reference evidence summary.
3. Final closed classification matrix.
4. List of implementation actions:
   - fields to import to DG
   - fields to keep preserve-only
   - fields to keep local-system-only
   - fields to remove
   - fields to add
5. Test update plan for enforcing the matrix.

## Done Criteria

Product Presave -> DG alignment investigation is complete only when:

- Every local Product Presave field is in the matrix.
- Every reference Product Presave field found in the audited workflow is in the matrix.
- Every field has exactly one final status.
- There are zero `ambiguous` rows.
- Every `referencePreserveOnly` row has reference evidence that import does not populate DG.
- Every `localSystemOnly` row has a local rationale.
- Every `referenceImportedToDg` row has a local DG target or an implementation action to add it.
- `dgPrdKey` remains outside Product Presave and is documented as case-owned.

After this, the answer to "are all Product Presave fields aligned/imported?" must be:

```text
Yes, for Product Presave -> DG, per the approved matrix: every reference-owned field is imported or documented as non-importing reference behavior; every local-only field is justified or removed; zero ambiguous rows remain.
```

or:

```text
No, these exact matrix rows remain unresolved: <list>.
```

No memory-based or informal answer is acceptable.
