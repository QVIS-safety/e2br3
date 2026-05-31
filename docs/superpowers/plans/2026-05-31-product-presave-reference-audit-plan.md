# Product Presave Reference Audit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce a closed, evidence-backed Product Presave -> DG alignment matrix with zero ambiguous rows, so every local and reference Product Presave field is either imported, preserved with proof, local-system-only with rationale, removed, or added as missing.

**Architecture:** This is an investigation-first plan. It creates durable audit artifacts before any code changes, compares local field ownership against reference UI/bundle/network evidence, then outputs a final decision matrix and implementation action list. Reference inspection must follow `api-workflow-payload-investigator`: inspect naturally loaded bundles first, use Chrome UI/network capture only for unresolved fields, and never call inferred APIs directly.

**Tech Stack:** Rust backend, React/TypeScript frontend, Jest/Cargo tests for local verification, Chrome/DevTools or approved browser tooling for passive reference workflow observation, Markdown/CSV audit artifacts.

---

## Files

- Read: `/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/superpowers/specs/2026-05-31-product-presave-reference-audit-design.md`
- Create: `/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/superpowers/audits/2026-05-31-product-presave-local-inventory.md`
- Create: `/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/superpowers/audits/2026-05-31-product-presave-reference-evidence.md`
- Create: `/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/superpowers/audits/2026-05-31-product-presave-final-matrix.md`
- Create: `/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/superpowers/audits/2026-05-31-product-presave-implementation-actions.md`
- Read: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/types/presave.ts`
- Read: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/schemas/presave.ts`
- Read: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/presave/ProductForm.tsx`
- Read: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/presave/canonicalMappers.ts`
- Read: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/presave/canonicalWriteMappers.ts`
- Read: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/case-form/sections/SectionG.tsx`
- Read: backend Product Presave model/API/test files under `/Users/hyundonghoon/projects/rust/e2br3/e2br3/crates`

---

### Task 1: Create the Local Product Presave Inventory

**Files:**
- Create: `/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/superpowers/audits/2026-05-31-product-presave-local-inventory.md`
- Read: local Product Presave frontend/backend files listed above

- [x] **Step 1: Create the audit directory**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/e2br3
mkdir -p docs/superpowers/audits
```

Expected: directory exists.

- [x] **Step 2: Extract local frontend field ownership**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3
rg -n "export interface ProductPresaveData|productPresaveSchema|register\\(|setIfPresent\\(parent|mapProductPresave|handleImportProduct|productPresaveDgImportTargets|preserveOnly|importedToDg" \
  frontend/E2BR3-frontend/lib/types/presave.ts \
  frontend/E2BR3-frontend/lib/schemas/presave.ts \
  frontend/E2BR3-frontend/components/presave/ProductForm.tsx \
  frontend/E2BR3-frontend/lib/presave/canonicalMappers.ts \
  frontend/E2BR3-frontend/lib/presave/canonicalWriteMappers.ts \
  frontend/E2BR3-frontend/components/case-form/sections/SectionG.tsx \
  frontend/E2BR3-frontend/__tests__/ui-binding/field-bindings.test.ts
```

Expected: output identifies local type fields, schema fields, form controls, canonical mapper keys, write mapper keys, and current DG import/test category.

- [x] **Step 3: Extract backend Product Presave persistence fields**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/e2br3
rg -n "ProductPresave|product_presave|mfds_mpid|product_id|preapproval|fda_ind|mfds_device|mfds_regional|substance" \
  crates/libs/lib-core/src/model \
  crates/services/web-server/src/web/rest \
  crates/services/web-server/tests/api \
  crates/libs/lib-core/tests \
  db/bootstrap/01-safetydb-schema.sql
```

Expected: output identifies backend DTO/model/API/schema/test persistence paths for Product Presave parent fields and child rows.

- [x] **Step 4: Write the local inventory artifact**

Create `/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/superpowers/audits/2026-05-31-product-presave-local-inventory.md` with this structure:

```markdown
# Product Presave Local Inventory

## Scope

Product Presave -> DG only. `dgPrdKey` is case-owned and is not a Product Presave field.

## Current Local Fields

| Local Field | Type | ProductForm Control | Schema Key | Read Mapper | Write Mapper | Backend Field/Table | Current Local DG Target | Current Category | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| productId | string | Product ID input | productId | productId/product_id | product_id | product_presaves.product_id | none | preserveOnly | file refs |
| drugCharacterization | string | ProductForm field if present | drugCharacterization | drug_characterization | drug_characterization | product_presaves.drug_characterization | drugs.0.drugCharacterization | importedToDg | file refs |
```

Add one row for every current field:

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

- [x] **Step 5: Verify no local Product Presave `dgPrdKey` ownership remains**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3
rg -n "dgPrdKey|dg_prd_key" \
  frontend/E2BR3-frontend/lib/types/presave.ts \
  frontend/E2BR3-frontend/lib/schemas/presave.ts \
  frontend/E2BR3-frontend/lib/presave \
  frontend/E2BR3-frontend/components/presave \
  frontend/E2BR3-frontend/__tests__/ui-binding/field-bindings.test.ts
```

Expected: no hits in Product Presave type/schema/mappers/UI. Test sentinel hits in `field-bindings.test.ts` are acceptable only when they explicitly prove `dgPrdKey` is ignored.

---

### Task 2: Identify Reference Product Presave and DG Workflows

**Files:**
- Create or update: `/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/superpowers/audits/2026-05-31-product-presave-reference-evidence.md`

- [x] **Step 1: Start the reference evidence document**

Create `/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/superpowers/audits/2026-05-31-product-presave-reference-evidence.md`:

```markdown
# Product Presave Reference Evidence

## Workflow Investigated

- Product Presave detail/create/edit workflow:
- DG Product Presave import workflow:
- Authorities checked: ICH, FDA, MFDS

## Operating Constraints

- No direct API calls.
- No replayed requests.
- Bundle inspection first.
- Live network capture only for unresolved field movement.
- Redact values; preserve field names, types, nesting, and workflow order.

## Reference Routes

| Authority | Product Presave Route | DG Case Route | Status |
|---|---|---|---|
| ICH/NA | record exact route from INFO > PRODUCT UI navigation | https://edu-safetyr3.crscube.io/57/508/en/NA/case/786996/detail/DG/1 | pending |
| FDA/US | record exact route from INFO > PRODUCT UI navigation | stop and ask user if no authenticated US DG route is available in browser history | pending |
| MFDS/KR | record exact route from INFO > PRODUCT UI navigation | https://edu-safetyr3.crscube.io/57/508/en/KR/case/786994/detail/DG/1 | pending |
```

- [x] **Step 2: Confirm reference routes before opening Chrome**

Use the routes already discussed in prior investigation where applicable:

```text
NA/ICH DG baseline: https://edu-safetyr3.crscube.io/57/508/en/NA/case/786996/detail/DG/1
US/FDA DG route: use the known US DG case route from prior DG checks or ask user if unavailable.
KR/MFDS DG route: https://edu-safetyr3.crscube.io/57/508/en/KR/case/786994/detail/DG/1
Product Presave route: use the INFO > PRODUCT UI route reached by normal navigation from the authenticated reference app.
```

Expected: a concrete Product Presave route and DG route per authority are recorded. If a route is missing, stop and ask for only that route.

- [x] **Step 3: Open each route through Chrome UI only**

Use Chrome with the user-authenticated session. Interact only through normal UI navigation. Do not use direct HTTP/API requests.

Expected: pages load or the audit records authentication/navigation blocker with date/time and exact route.

---

### Task 3: Static Bundle Evidence Pass

**Files:**
- Update: `/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/superpowers/audits/2026-05-31-product-presave-reference-evidence.md`
- Temporary only: `/tmp/e2br3-product-presave-reference-bundles-*`

- [x] **Step 1: Capture naturally loaded bundle names**

From Chrome DevTools or approved browser tooling, record only JS files naturally loaded by:

```text
Product Presave page
NA/ICH DG page
US/FDA DG page
KR/MFDS DG page
```

Expected: evidence document contains a `Bundles Inspected` table:

```markdown
## Bundles Inspected

| Page | Bundle/Chunk | How Identified | Relevant Symbols |
|---|---|---|---|
```

- [x] **Step 2: Inspect only relevant loaded bundles**

Save relevant loaded JS bundles to `/tmp/e2br3-product-presave-reference-bundles-<timestamp>/`.

Search bundle copies for these terms:

```text
Product
PRODUCT
DG
DG_PRD_KEY
mpid
phpid
preApproval
manufacturer
authorization
substance
IND
ANDA
MFDS
device
import
presave
template
```

Expected: no unrelated routes are crawled or downloaded.

- [x] **Step 3: Extract Product Presave field evidence**

For every reference Product Presave field found in bundle code, add:

```markdown
## Reference Product Presave Fields

| Authority | Reference Label/Code | Payload Key | Type | Required? | Bundle Evidence |
|---|---|---|---|---:|---|
```

Evidence must cite bundle file name and symbol/method name. Do not paste large minified snippets.

- [x] **Step 4: Extract DG import behavior evidence**

For each Product Presave import method or field-copy method found, add:

```markdown
## Reference DG Import Behavior

| Authority | Source Payload Key | DG Target Field/Code | Import Behavior | Bundle Evidence |
|---|---|---|---|---|
```

Expected: fields resolved by static evidence are marked `resolvedByBundle`.

---

### Task 4: UI/Network Evidence Pass for Unresolved Fields

**Files:**
- Update: `/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/superpowers/audits/2026-05-31-product-presave-reference-evidence.md`

- [x] **Step 1: List unresolved fields**

Append:

```markdown
## Fields Requiring Live UI/Network Evidence

| Field | Why Bundle Evidence Was Insufficient | Authority |
|---|---|---|
```

Expected: only unresolved fields are listed. Fields already resolved by bundle evidence are not retested through network.

- [x] **Step 2: Prepare safe sentinel values** (not required; bundle evidence closed all fields)

Use synthetic values that do not contain personal data:

```text
AUD-PROD-001
AUD-BRAND-001
AUD-MPID-001
AUD-PHPID-001
AUD-AUTH-001
AUD-MFG-001
AUD-FDA-IND-001
AUD-MFDS-001
AUD-DEVICE-001
```

Expected: no real patient, sender, reporter, or free-text case content is captured.

- [x] **Step 3: Perform UI-only Product Presave import** (not required; no unresolved fields remained after bundle evidence)

For each authority with unresolved fields:

1. Open Product Presave UI.
2. Create or select a Product Presave using sentinel values only.
3. Open DG case route.
4. Click Product Presave import control through the UI.
5. Select the Product Presave through the UI.
6. Observe relevant request/response shapes and DG state.
7. Do not save unrelated case changes unless the workflow requires saving to observe import. If saving is required, stop and ask the user first.

Expected: evidence document records passive observation only.

- [x] **Step 4: Record live evidence** (not required; no live network evidence was used)

For every unresolved field observed live, add:

```markdown
## Live UI/Network Observations

| Authority | UI Action | Request Shape | Response Shape | DG Field Updated? | Evidence |
|---|---|---|---|---:|---|
```

Use redacted examples:

```json
{
  "fieldName": "<string>",
  "items": [
    {
      "nestedField": "<string>"
    }
  ]
}
```

Expected: field names, nesting, and types are preserved; sensitive values are redacted.

---

### Task 5: Build the Closed Final Matrix

**Files:**
- Create: `/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/superpowers/audits/2026-05-31-product-presave-final-matrix.md`

- [x] **Step 1: Create the matrix artifact**

Create `/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/superpowers/audits/2026-05-31-product-presave-final-matrix.md`:

```markdown
# Product Presave Final Alignment Matrix

## Status Values

- `referenceImportedToDg`
- `referencePreserveOnly`
- `localSystemOnly`
- `remove`
- `missingLocal`

## Matrix

| Field | Local? | Reference Product Presave? | Reference Payload Key | Reference DG Target | Reference Imports? | Local DG Target | Final Status | Evidence |
|---|---:|---:|---|---|---:|---|---|---|
```

- [x] **Step 2: Add all local fields**

Copy every field from local inventory into the final matrix.

Expected: every current local field appears exactly once.

- [x] **Step 3: Add reference-only fields**

For any reference Product Presave field not in local inventory, add a row with:

```text
Local? = no
Final Status = missingLocal
```

Expected: reference-only fields are not lost just because local code lacks them.

- [x] **Step 4: Apply final statuses**

Use these exact rules:

```text
referenceImportedToDg: reference has field and reference imports it into DG
referencePreserveOnly: reference has field and reference does not import it into DG
localSystemOnly: reference lacks field, local mechanics require it
remove: reference lacks field and local mechanics do not require it
missingLocal: reference has field and local lacks field
```

Expected: no row has empty status, `preserveOnly`, `importedToDg`, or `ambiguous` as a final status.

- [x] **Step 5: Validate zero ambiguous rows**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/e2br3
rg -n "\\bambiguous\\b|\\bpreserveOnly\\b|\\bimportedToDg\\b|\\|\\s*\\|\\s*$" docs/superpowers/audits/2026-05-31-product-presave-final-matrix.md
```

Expected:
- No `ambiguous`.
- No provisional `preserveOnly` or `importedToDg` final status.
- No blank final-status cells.
- It is acceptable for the status legend to include only approved final status values.

---

### Task 6: Derive Implementation Actions

**Files:**
- Create: `/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/superpowers/audits/2026-05-31-product-presave-implementation-actions.md`

- [x] **Step 1: Create action artifact**

Create `/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/superpowers/audits/2026-05-31-product-presave-implementation-actions.md`:

```markdown
# Product Presave Implementation Actions

## Import To DG

| Field | DG Target | Evidence | Required Local Change | Test |
|---|---|---|---|---|

## Keep Reference Preserve-Only

| Field | Evidence Reference Does Not Import | Local Test |
|---|---|---|

## Keep Local System-Only

| Field | Local Rationale | Local Test |
|---|---|---|

## Remove

| Field | Reason | Files To Change | Test |
|---|---|---|---|

## Add Missing Local

| Field | Reference Evidence | Files To Change | Test |
|---|---|---|---|
```

- [x] **Step 2: Fill each action list from the final matrix**

Expected:
- `referenceImportedToDg` rows appear under `Import To DG`.
- `referencePreserveOnly` rows appear under `Keep Reference Preserve-Only`.
- `localSystemOnly` rows appear under `Keep Local System-Only`.
- `remove` rows appear under `Remove`.
- `missingLocal` rows appear under `Add Missing Local`.

- [x] **Step 3: Specify test expectations**

For each action, name the future test class:

```text
referenceImportedToDg -> SectionG Product Presave import test
referencePreserveOnly -> negative import assertion with reference evidence
localSystemOnly -> negative import assertion plus local mechanics test
remove -> TypeScript/schema/mappers no-hit search and relevant Jest/Cargo tests
missingLocal -> ProductForm/schema/mapper/API round-trip and import test if reference imports it
```

Expected: every action has a test expectation.

---

### Task 7: Review and Commit Audit Artifacts

**Files:**
- Review:
  - `/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/superpowers/audits/2026-05-31-product-presave-local-inventory.md`
  - `/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/superpowers/audits/2026-05-31-product-presave-reference-evidence.md`
  - `/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/superpowers/audits/2026-05-31-product-presave-final-matrix.md`
  - `/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/superpowers/audits/2026-05-31-product-presave-implementation-actions.md`

- [ ] **Step 1: Self-review for forbidden outcomes**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/e2br3
rg -n "UNRESOLVED|AMBIGUOUS|NEEDS_EVIDENCE|PLACEHOLDER|empty final status" docs/superpowers/audits/2026-05-31-product-presave-*.md
```

Expected: no unresolved placeholders. If any are found, either resolve the row with evidence or stop and report the exact blocker.

- [ ] **Step 2: Validate every matrix row has an action or proof**

Manually compare:

```text
final-matrix.md
implementation-actions.md
```

Expected:
- Every `referenceImportedToDg`, `remove`, and `missingLocal` row has an implementation action.
- Every `referencePreserveOnly` and `localSystemOnly` row has evidence/rationale and a negative local test expectation.

- [ ] **Step 3: Force-add ignored docs artifacts**

Because `docs/` is ignored in this repo, run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/e2br3
git add -f \
  docs/superpowers/audits/2026-05-31-product-presave-local-inventory.md \
  docs/superpowers/audits/2026-05-31-product-presave-reference-evidence.md \
  docs/superpowers/audits/2026-05-31-product-presave-final-matrix.md \
  docs/superpowers/audits/2026-05-31-product-presave-implementation-actions.md
```

Expected: only audit docs are staged for this commit.

- [ ] **Step 4: Commit audit artifacts**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/e2br3
git commit -m "docs: add product presave reference audit matrix"
```

Expected: commit succeeds with only audit artifacts.

---

## Self-Review

- Spec coverage: This plan creates the local inventory, reference evidence, final zero-ambiguous matrix, implementation action list, and audit commit required by the approved design.
- Placeholder scan: The plan contains no open-ended implementation placeholders; any unresolved reference evidence becomes an explicit blocker rather than an accepted final row.
- Type consistency: Final statuses match the approved design exactly: `referenceImportedToDg`, `referencePreserveOnly`, `localSystemOnly`, `remove`, and `missingLocal`.
