# Unified Section Delta Save Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every case editor section save through one consistent request-driven authority architecture, sending `profiles` plus only the changed fields for direct pages and repeatable rows.

**Architecture:** The URL authority remains the single frontend source for validation/render profiles: `ICH -> ["ich"]`, `US -> ["fda"]`, `KR -> ["mfds"]`, `USKR -> ["fda", "mfds"]`. All section saves use one canonical patch envelope, `CaseEditorPagePatchRequest`, with `profiles`, `changes`, and `rows`; section-specific code only maps form paths to backend patch keys. Backend projection/validation continues to use the request `profiles`; no case-level appendix metadata is reintroduced.

**Tech Stack:** Next.js/React frontend, TypeScript, Jest, Rust Axum backend, SQLx/Postgres, existing case editor projection endpoints.

---

## File Structure

Frontend repository: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend`

- Modify: `lib/case-save/pages/direct-page-patch.ts`
  - Owns the unified delta builder entry points for direct and repeatable section PATCH requests.
- Create: `lib/case-save/pages/field-delta.ts`
  - Small shared helpers for dirty-tree traversal, nested value lookup, and `CaseEditorFieldPatch` construction.
- Create: `__tests__/case-save/direct-page-delta.test.ts`
  - Contract tests for every direct section: `CI`, `RP`, `SD`, `LR`, `SI`, `DM`, `NR`.
- Create: `__tests__/case-save/repeatable-row-delta.test.ts`
  - Contract tests for repeatable row deltas: `AE`, `LB`, `DG`, `DH`.
- Modify: `__tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts`
  - Assert save orchestration still calls page/row PATCH with `profiles` and delta payloads.
- Modify only if required by API type gaps: `lib/api/endpoints/cases/core/editor.ts`
  - Keep `CaseEditorPagePatchRequest` stable; no old `appendix`, `focusedAppendix`, or selected metadata.

Backend repository: `/Users/hyundonghoon/projects/rust/e2br3/e2br3`

- Modify: `crates/services/web-server/src/web/rest/case_editor_rest.rs`
  - Accept delta payloads for all direct/repeatable sections.
  - Keep compatibility with existing row-shaped payloads only if tests show current frontend still needs it during transition.
- Modify: `crates/services/web-server/tests/api/case_editor_contract_web.rs`
  - Add contract coverage proving direct and repeatable sections accept `profiles + changes` and reject invalid `profiles` before mutation.

---

## Canonical Request Shape

All section save builders must return this shape:

```ts
type CaseEditorFieldPatch = {
  value?: unknown;
  nullFlavor?: string | null;
  notation?: string | null;
};

type CaseEditorPagePatchRequest = {
  profiles: ValidationProfile[];
  changes: Record<string, CaseEditorFieldPatch>;
  rows: Record<string, unknown>;
};
```

Direct page saves:

```json
{
  "profiles": ["fda", "mfds"],
  "changes": {
    "reporterGivenName": { "value": "Jane" },
    "reporterCountry": { "value": "US" }
  },
  "rows": {}
}
```

Repeatable row saves:

```json
{
  "profiles": ["fda"],
  "changes": {
    "reactionTerm": { "value": "Headache" }
  },
  "rows": {}
}
```

No section may introduce a second architecture. Section-specific code may only define field maps.

---

## Task 1: Add Shared Field Delta Helpers

**Files:**
- Create: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/case-save/pages/field-delta.ts`
- Test: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/case-save/direct-page-delta.test.ts`

- [ ] **Step 1: Write failing tests for nested dirty extraction**

Add the first tests to `__tests__/case-save/direct-page-delta.test.ts`:

```ts
import {
  buildFieldDelta,
  getValueAtPath,
  hasDirtyAtPath,
} from "@/lib/case-save/pages/field-delta";

describe("field delta helpers", () => {
  it("reads nested form values by path", () => {
    expect(
      getValueAtPath(
        {
          primarySources: [
            {
              reporterGivenName: "Jane",
            },
          ],
        },
        ["primarySources", 0, "reporterGivenName"],
      ),
    ).toBe("Jane");
  });

  it("detects dirty leaves by matching path", () => {
    expect(
      hasDirtyAtPath(
        {
          primarySources: [
            {
              reporterGivenName: true,
            },
          ],
        },
        ["primarySources", 0, "reporterGivenName"],
      ),
    ).toBe(true);
  });

  it("builds a patch only for dirty mapped fields", () => {
    const delta = buildFieldDelta({
      data: {
        primarySources: [
          {
            reporterGivenName: "Jane",
            reporterFamilyName: "Doe",
          },
        ],
      },
      dirty: {
        primarySources: [
          {
            reporterGivenName: true,
          },
        ],
      },
      fields: [
        {
          patchKey: "reporterGivenName",
          path: ["primarySources", 0, "reporterGivenName"],
        },
        {
          patchKey: "reporterFamilyName",
          path: ["primarySources", 0, "reporterFamilyName"],
        },
      ],
    });

    expect(delta).toEqual({
      reporterGivenName: { value: "Jane" },
    });
  });
});
```

- [ ] **Step 2: Run the failing test**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npm test -- --runTestsByPath __tests__/case-save/direct-page-delta.test.ts --runInBand
```

Expected: fail because `field-delta.ts` does not exist.

- [ ] **Step 3: Implement the helper module**

Create `lib/case-save/pages/field-delta.ts`:

```ts
import type { CaseEditorFieldPatch } from "@/lib/api/endpoints/cases/core/editor";

export type FieldPath = readonly (string | number)[];

export type FieldDeltaMapping = {
  patchKey: string;
  path: FieldPath;
};

export function getValueAtPath(value: unknown, path: FieldPath): unknown {
  let current = value;
  for (const segment of path) {
    if (current == null) return undefined;
    if (typeof segment === "number") {
      if (!Array.isArray(current)) return undefined;
      current = current[segment];
      continue;
    }
    if (typeof current !== "object" || Array.isArray(current)) return undefined;
    current = (current as Record<string, unknown>)[segment];
  }
  return current;
}

export function hasDirtyAtPath(dirty: unknown, path: FieldPath): boolean {
  const value = getValueAtPath(dirty, path);
  if (value === true) return true;
  if (Array.isArray(value)) return value.some(Boolean);
  if (value && typeof value === "object") {
    return Object.values(value as Record<string, unknown>).some(Boolean);
  }
  return false;
}

export function fieldPatch(value: unknown): CaseEditorFieldPatch {
  return { value: value ?? null };
}

export function buildFieldDelta(args: {
  data: Record<string, unknown>;
  dirty: unknown;
  fields: readonly FieldDeltaMapping[];
}): Record<string, CaseEditorFieldPatch> {
  return Object.fromEntries(
    args.fields
      .filter((field) => hasDirtyAtPath(args.dirty, field.path))
      .map((field) => [field.patchKey, fieldPatch(getValueAtPath(args.data, field.path))]),
  );
}
```

- [ ] **Step 4: Run the helper test**

Run:

```bash
npm test -- --runTestsByPath __tests__/case-save/direct-page-delta.test.ts --runInBand
```

Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add lib/case-save/pages/field-delta.ts __tests__/case-save/direct-page-delta.test.ts
git commit -m "Add shared case editor field delta helpers"
```

---

## Task 2: Extend Direct Page Delta Builders Consistently

**Files:**
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/case-save/pages/direct-page-patch.ts`
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/case-save/direct-page-delta.test.ts`

- [ ] **Step 1: Add failing tests for all direct sections**

Append these tests to `__tests__/case-save/direct-page-delta.test.ts`:

```ts
import { buildDirectPagePatchRequest } from "@/lib/case-save/pages/direct-page-patch";

describe("direct page delta builders", () => {
  it("builds RP reporter deltas with authority profiles", () => {
    const request = buildDirectPagePatchRequest({
      section: "RP",
      authority: "USKR",
      data: {
        primarySources: [{ reporterGivenName: "Jane", reporterFamilyName: "Doe" }],
      },
      dirty: {
        primarySources: [{ reporterGivenName: true }],
      },
    });

    expect(request).toEqual({
      profiles: ["fda", "mfds"],
      changes: { reporterGivenName: { value: "Jane" } },
      rows: {},
    });
  });

  it("builds SD sender/message deltas", () => {
    const request = buildDirectPagePatchRequest({
      section: "SD",
      authority: "US",
      data: {
        senderInformation: { senderOrganization: "Sender Org" },
        receiverInformation: { receiverOrganization: "Receiver Org" },
      },
      dirty: {
        senderInformation: { senderOrganization: true },
      },
    });

    expect(request.profiles).toEqual(["fda"]);
    expect(request.changes).toEqual({
      senderOrganization: { value: "Sender Org" },
    });
    expect(request.rows).toEqual({});
  });

  it("builds LR literature deltas", () => {
    const request = buildDirectPagePatchRequest({
      section: "LR",
      authority: "KR",
      data: { literatureReferences: [{ literatureReference: "PMID:1" }] },
      dirty: { literatureReferences: [{ literatureReference: true }] },
    });

    expect(request).toEqual({
      profiles: ["mfds"],
      changes: { literatureReference: { value: "PMID:1" } },
      rows: {},
    });
  });

  it("builds SI study deltas", () => {
    const request = buildDirectPagePatchRequest({
      section: "SI",
      authority: "ICH",
      data: { studyInformation: { studyName: "Protocol A" } },
      dirty: { studyInformation: { studyName: true } },
    });

    expect(request).toEqual({
      profiles: ["ich"],
      changes: { studyName: { value: "Protocol A" } },
      rows: {},
    });
  });

  it("builds DM patient deltas", () => {
    const request = buildDirectPagePatchRequest({
      section: "DM",
      authority: "US",
      data: { patientInformation: { patientInitials: "AB" } },
      dirty: { patientInformation: { patientInitials: true } },
    });

    expect(request.changes).toEqual({
      patientInitials: { value: "AB" },
    });
    expect(request.rows).toEqual({});
  });

  it("builds NR narrative deltas", () => {
    const request = buildDirectPagePatchRequest({
      section: "NR",
      authority: "US",
      data: { narrative: { caseNarrative: "Narrative text" } },
      dirty: { narrative: { caseNarrative: true } },
    });

    expect(request.changes).toEqual({
      caseNarrative: { value: "Narrative text" },
    });
    expect(request.rows).toEqual({});
  });
});
```

- [ ] **Step 2: Run the direct section tests**

Run:

```bash
npm test -- --runTestsByPath __tests__/case-save/direct-page-delta.test.ts --runInBand
```

Expected: fail because `buildDirectPagePatchRequest` does not yet accept `dirty`, and non-CI sections still populate `rows`.

- [ ] **Step 3: Update the direct page builder signature**

In `lib/case-save/pages/direct-page-patch.ts`, change:

```ts
export function buildDirectPagePatchRequest(args: {
  section: DirectEditorSectionCode;
  authority: CaseEditAuthority;
  data: Record<string, unknown>;
}): CaseEditorPagePatchRequest {
```

to:

```ts
export function buildDirectPagePatchRequest(args: {
  section: DirectEditorSectionCode;
  authority: CaseEditAuthority;
  data: Record<string, unknown>;
  dirty?: unknown;
}): CaseEditorPagePatchRequest {
```

- [ ] **Step 4: Add direct field maps**

In `direct-page-patch.ts`, import the helper:

```ts
import {
  buildFieldDelta,
  type FieldDeltaMapping,
} from "@/lib/case-save/pages/field-delta";
```

Add maps near the top of the file:

```ts
const DIRECT_FIELD_MAPS: Record<DirectEditorSectionCode, readonly FieldDeltaMapping[]> = {
  CI: [
    { patchKey: "reportType", path: ["safetyReportIdentification", "reportType"] },
    { patchKey: "fulfilExpeditedCriteria", path: ["safetyReportIdentification", "fulfilExpeditedCriteria"] },
    { patchKey: "localCriteriaReportType", path: ["safetyReportIdentification", "localCriteriaReportType"] },
    { patchKey: "combinationProductReportIndicator", path: ["safetyReportIdentification", "combinationProductReportIndicator"] },
  ],
  RP: [
    { patchKey: "reporterGivenName", path: ["primarySources", 0, "reporterGivenName"] },
    { patchKey: "reporterFamilyName", path: ["primarySources", 0, "reporterFamilyName"] },
    { patchKey: "reporterOrganization", path: ["primarySources", 0, "reporterOrganization"] },
    { patchKey: "reporterCountry", path: ["primarySources", 0, "reporterCountry"] },
    { patchKey: "qualification", path: ["primarySources", 0, "qualification"] },
  ],
  SD: [
    { patchKey: "senderOrganization", path: ["senderInformation", "senderOrganization"] },
    { patchKey: "senderDepartment", path: ["senderInformation", "senderDepartment"] },
    { patchKey: "senderCountry", path: ["senderInformation", "senderCountry"] },
    { patchKey: "receiverOrganization", path: ["receiverInformation", "receiverOrganization"] },
    { patchKey: "receiverCountry", path: ["receiverInformation", "receiverCountry"] },
  ],
  LR: [
    { patchKey: "literatureReference", path: ["literatureReferences", 0, "literatureReference"] },
  ],
  SI: [
    { patchKey: "studyName", path: ["studyInformation", "studyName"] },
    { patchKey: "sponsorStudyNumber", path: ["studyInformation", "sponsorStudyNumber"] },
    { patchKey: "studyType", path: ["studyInformation", "studyType"] },
  ],
  DM: [
    { patchKey: "patientInitials", path: ["patientInformation", "patientInitials"] },
    { patchKey: "patientSex", path: ["patientInformation", "patientSex"] },
    { patchKey: "patientBirthDate", path: ["patientInformation", "patientBirthDate"] },
    { patchKey: "patientAge", path: ["patientInformation", "patientAge"] },
  ],
  NR: [
    { patchKey: "caseNarrative", path: ["narrative", "caseNarrative"] },
    { patchKey: "reporterComments", path: ["narrative", "reporterComments"] },
    { patchKey: "senderComments", path: ["narrative", "senderComments"] },
  ],
};
```

- [ ] **Step 5: Return deltas from every direct section**

Replace the current `rowsBySection` return logic in `buildDirectPagePatchRequest` with:

```ts
export function buildDirectPagePatchRequest(args: {
  section: DirectEditorSectionCode;
  authority: CaseEditAuthority;
  data: Record<string, unknown>;
  dirty?: unknown;
}): CaseEditorPagePatchRequest {
  const changes = args.dirty
    ? buildFieldDelta({
        data: args.data,
        dirty: args.dirty,
        fields: DIRECT_FIELD_MAPS[args.section],
      })
    : args.section === "CI"
      ? buildCaseIdentificationChanges(args.data)
      : {};

  return {
    profiles: authorityToProfiles(args.authority),
    changes,
    rows: {},
  };
}
```

- [ ] **Step 6: Run direct section tests**

Run:

```bash
npm test -- --runTestsByPath __tests__/case-save/direct-page-delta.test.ts --runInBand
```

Expected: pass.

- [ ] **Step 7: Commit**

```bash
git add lib/case-save/pages/direct-page-patch.ts __tests__/case-save/direct-page-delta.test.ts
git commit -m "Use direct page field deltas for editor saves"
```

---

## Task 3: Wire Dirty State Into Section PATCH Builders

**Files:**
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/case-form/CaseFormWizardNew.tsx`
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts`

- [ ] **Step 1: Add a failing orchestration assertion**

In `__tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts`, update the direct page PATCH expectation to include only dirty fields:

```ts
expect(patchEditorPageProjection).toHaveBeenCalledWith("case-1", "NR", {
  profiles: ["fda", "mfds"],
  changes: {
    caseNarrative: { value: "Updated narrative" },
  },
  rows: {},
});
```

Use the existing test named `saves direct section routes through page projection patch`.

- [ ] **Step 2: Run the orchestration test**

Run:

```bash
npm test -- --runTestsByPath __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts --runInBand
```

Expected: fail because `saveDirectSectionPagePatch` does not pass `dirty` to the builder.

- [ ] **Step 3: Pass dirty state through save helpers**

In `components/case-form/CaseFormWizardNew.tsx`, change `saveDirectSectionPagePatch`:

```ts
export async function saveDirectSectionPagePatch({
  currentCaseId,
  sectionCode,
  authority,
  data,
  dirty,
}: {
  currentCaseId: string;
  sectionCode: DirectEditorSectionCode;
  authority: CaseEditAuthority;
  data: Record<string, unknown>;
  dirty?: unknown;
}) {
  return api.cases.patchEditorPageProjection(
    currentCaseId,
    sectionCode,
    buildDirectPagePatchRequest({
      section: sectionCode,
      authority,
      data,
      dirty,
    }),
  );
}
```

Change `saveSectionScopedPagePatch` to accept and pass `dirty`:

```ts
export async function saveSectionScopedPagePatch({
  currentCaseId,
  sectionScopedEditor,
  data,
  dirty,
}: {
  currentCaseId: string;
  sectionScopedEditor?: SectionScopedEditor;
  data: Record<string, unknown>;
  dirty?: unknown;
}) {
```

Inside the direct branch:

```ts
return saveDirectSectionPagePatch({
  currentCaseId,
  sectionCode: directPagePatchSection,
  authority: sectionScopedEditor.authority,
  data,
  dirty,
});
```

At the call site in `handleSave`, change:

```ts
const scopedPatchResult = await saveSectionScopedPagePatch({
  currentCaseId,
  sectionScopedEditor,
  data: data as Record<string, unknown>,
  dirty,
});
```

- [ ] **Step 4: Run orchestration test**

Run:

```bash
npm test -- --runTestsByPath __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts --runInBand
```

Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add components/case-form/CaseFormWizardNew.tsx __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts
git commit -m "Pass dirty state into editor section patches"
```

---

## Task 4: Add Repeatable Row Delta Builders

**Files:**
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/case-save/pages/direct-page-patch.ts`
- Create: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/case-save/repeatable-row-delta.test.ts`
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/case-form/CaseFormWizardNew.tsx`

- [ ] **Step 1: Write failing repeatable row delta tests**

Create `__tests__/case-save/repeatable-row-delta.test.ts`:

```ts
import { buildRepeatablePageRowPatchRequest } from "@/lib/case-save/pages/direct-page-patch";

describe("repeatable row delta builders", () => {
  it("builds AE reaction row deltas", () => {
    const request = buildRepeatablePageRowPatchRequest({
      section: "AE",
      authority: "US",
      rowId: "rx-1",
      data: {
        reactions: [{ id: "rx-1", reactionTerm: "Headache", outcome: "1" }],
      },
      dirty: {
        reactions: [{ reactionTerm: true }],
      },
    });

    expect(request).toEqual({
      profiles: ["fda"],
      changes: { reactionTerm: { value: "Headache" } },
      rows: {},
    });
  });

  it("builds LB test row deltas", () => {
    const request = buildRepeatablePageRowPatchRequest({
      section: "LB",
      authority: "KR",
      rowId: "lb-1",
      data: {
        testResults: [{ id: "lb-1", testName: "ALT", resultValue: "10" }],
      },
      dirty: {
        testResults: [{ resultValue: true }],
      },
    });

    expect(request).toEqual({
      profiles: ["mfds"],
      changes: { resultValue: { value: "10" } },
      rows: {},
    });
  });

  it("builds DG drug row deltas", () => {
    const request = buildRepeatablePageRowPatchRequest({
      section: "DG",
      authority: "USKR",
      rowId: "dg-1",
      data: {
        drugs: [{ id: "dg-1", medicinalProduct: "Drug A" }],
      },
      dirty: {
        drugs: [{ medicinalProduct: true }],
      },
    });

    expect(request).toEqual({
      profiles: ["fda", "mfds"],
      changes: { medicinalProduct: { value: "Drug A" } },
      rows: {},
    });
  });

  it("builds DH past drug row deltas", () => {
    const request = buildRepeatablePageRowPatchRequest({
      section: "DH",
      authority: "ICH",
      rowId: "dh-1",
      data: {
        patientInformation: {
          pastDrugHistory: [{ id: "dh-1", drugName: "Past Drug" }],
        },
      },
      dirty: {
        patientInformation: {
          pastDrugHistory: [{ drugName: true }],
        },
      },
    });

    expect(request).toEqual({
      profiles: ["ich"],
      changes: { drugName: { value: "Past Drug" } },
      rows: {},
    });
  });
});
```

- [ ] **Step 2: Run repeatable tests**

Run:

```bash
npm test -- --runTestsByPath __tests__/case-save/repeatable-row-delta.test.ts --runInBand
```

Expected: fail because repeatable builders still require full row payloads.

- [ ] **Step 3: Add repeatable row maps**

In `direct-page-patch.ts`, add:

```ts
const REPEATABLE_ROW_MAPS: Record<RepeatableEditorSectionCode, {
  collectionPath: readonly (string | number)[];
  rowKey: string;
  fields: readonly FieldDeltaMapping[];
}> = {
  AE: {
    collectionPath: ["reactions"],
    rowKey: "reaction",
    fields: [
      { patchKey: "reactionTerm", path: ["reactionTerm"] },
      { patchKey: "outcome", path: ["outcome"] },
      { patchKey: "meddraCode", path: ["meddraCode"] },
    ],
  },
  LB: {
    collectionPath: ["testResults"],
    rowKey: "testResult",
    fields: [
      { patchKey: "testName", path: ["testName"] },
      { patchKey: "resultValue", path: ["resultValue"] },
      { patchKey: "resultUnit", path: ["resultUnit"] },
    ],
  },
  DG: {
    collectionPath: ["drugs"],
    rowKey: "drug",
    fields: [
      { patchKey: "medicinalProduct", path: ["medicinalProduct"] },
      { patchKey: "drugCharacterization", path: ["drugCharacterization"] },
      { patchKey: "authorizationNumber", path: ["authorizationNumber"] },
    ],
  },
  DH: {
    collectionPath: ["patientInformation", "pastDrugHistory"],
    rowKey: "pastDrugHistory",
    fields: [
      { patchKey: "drugName", path: ["drugName"] },
      { patchKey: "startDate", path: ["startDate"] },
      { patchKey: "endDate", path: ["endDate"] },
    ],
  },
};
```

- [ ] **Step 4: Add row extraction helpers**

In `direct-page-patch.ts`, add:

```ts
function rowIndexById(rows: Record<string, unknown>[], rowId: string): number {
  return rows.findIndex((row) => row.id === rowId);
}

function prependPath(prefix: readonly (string | number)[], mapping: FieldDeltaMapping): FieldDeltaMapping {
  return {
    patchKey: mapping.patchKey,
    path: [...prefix, ...mapping.path],
  };
}
```

- [ ] **Step 5: Update repeatable builder signature and output**

Change `buildRepeatablePageRowPatchRequest` signature to:

```ts
export function buildRepeatablePageRowPatchRequest(args: {
  section: RepeatableEditorSectionCode;
  authority: CaseEditAuthority;
  rowId: string;
  data: Record<string, unknown>;
  dirty?: unknown;
}): CaseEditorPagePatchRequest {
```

At the top of the function, before legacy row fallback:

```ts
const map = REPEATABLE_ROW_MAPS[args.section];
const rows = arrayFrom(getValueAtPath(args.data, map.collectionPath));
const index = rowIndexById(rows, args.rowId);
if (args.dirty && index >= 0) {
  const rowPrefix = [...map.collectionPath, index];
  const changes = buildFieldDelta({
    data: args.data,
    dirty: args.dirty,
    fields: map.fields.map((field) => prependPath(rowPrefix, field)),
  });
  return {
    profiles: authorityToProfiles(args.authority),
    changes,
    rows: {},
  };
}
```

Keep the existing full-row fallback below this block until backend delta contracts are confirmed for all repeatables.

- [ ] **Step 6: Pass dirty into row patch saves**

In `CaseFormWizardNew.tsx`, update `saveRepeatablePageRowPatch` to accept `dirty?: unknown` and pass it into `buildRepeatablePageRowPatchRequest`.

In `saveSectionScopedPagePatch`, pass `dirty` into `saveRepeatablePageRowPatch`.

- [ ] **Step 7: Run repeatable and orchestration tests**

Run:

```bash
npm test -- --runTestsByPath __tests__/case-save/repeatable-row-delta.test.ts __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts --runInBand
```

Expected: pass.

- [ ] **Step 8: Commit**

```bash
git add lib/case-save/pages/direct-page-patch.ts components/case-form/CaseFormWizardNew.tsx __tests__/case-save/repeatable-row-delta.test.ts __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts
git commit -m "Use repeatable row field deltas for editor saves"
```

---

## Task 5: Backend Accepts `changes` for Every Direct Section

**Files:**
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/e2br3/crates/services/web-server/src/web/rest/case_editor_rest.rs`
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/e2br3/crates/services/web-server/tests/api/case_editor_contract_web.rs`

- [ ] **Step 1: Add failing backend contract tests**

In `case_editor_contract_web.rs`, add one direct-section parameterized-style test using existing test helpers:

```rust
#[serial]
#[tokio::test]
async fn editor_direct_pages_accept_field_delta_changes_with_profiles() -> Result<()> {
    let mm = init_test_mm().await?;
    let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
    let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
    let cookie = cookie_header(&token.to_string());
    let app = web_server::app(mm);
    let case_id = create_case_for_editor(&app, &cookie, "EDITOR-DIRECT-DELTAS", &["ich"]).await?;
    create_safety_report(&app, &cookie, &case_id, "1", false).await?;

    let requests = [
        ("RP", json!({ "reporterGivenName": { "value": "Jane" } })),
        ("SD", json!({ "senderOrganization": { "value": "Sender Org" } })),
        ("LR", json!({ "literatureReference": { "value": "PMID:1" } })),
        ("SI", json!({ "studyName": { "value": "Study A" } })),
        ("DM", json!({ "patientInitials": { "value": "AB" } })),
        ("NR", json!({ "caseNarrative": { "value": "Narrative" } })),
    ];

    for (section, changes) in requests {
        let (status, body) = patch_json(
            &app,
            &cookie,
            &format!("/api/cases/{case_id}/editor/pages/{section}"),
            json!({
                "profiles": ["fda", "mfds"],
                "changes": changes,
                "rows": {}
            }),
        )
        .await?;

        assert_eq!(status, StatusCode::OK, "{section}: {body}");
        assert_eq!(body["profiles"], json!(["fda", "mfds"]), "{section}: {body}");
    }

    Ok(())
}
```

- [ ] **Step 2: Run backend contract test**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/e2br3
cargo test -p web-server editor_direct_pages_accept_field_delta_changes_with_profiles --test api -- --nocapture
```

Expected: fail for non-CI sections if backend only applies `rows`.

- [ ] **Step 3: Add direct changes dispatcher**

In `case_editor_rest.rs`, add a dispatcher near `apply_direct_page_rows_patch`:

```rust
async fn apply_direct_page_changes_patch(
    ctx: &lib_core::ctx::Ctx,
    mm: &ModelManager,
    case_id: Uuid,
    page_id: &'static str,
    changes: &BTreeMap<String, CaseEditorFieldPatch>,
) -> Result<()> {
    if changes.is_empty() {
        return Ok(());
    }
    match page_id {
        "CI" => apply_ci_page_changes_patch(ctx, mm, case_id, changes).await,
        "RP" => apply_rp_page_changes_patch(ctx, mm, case_id, changes).await,
        "SD" => apply_sd_page_changes_patch(ctx, mm, case_id, changes).await,
        "LR" => apply_lr_page_changes_patch(ctx, mm, case_id, changes).await,
        "SI" => apply_si_page_changes_patch(ctx, mm, case_id, changes).await,
        "DM" => apply_dm_page_changes_patch(ctx, mm, case_id, changes).await,
        "NR" => apply_nr_page_changes_patch(ctx, mm, case_id, changes).await,
        _ => Err(Error::BadRequest {
            message: format!("unsupported direct page '{page_id}'"),
        }),
    }
}
```

- [ ] **Step 4: Call the dispatcher from direct PATCH**

In `patch_direct_page_projection`, before or instead of row application, add:

```rust
apply_direct_page_changes_patch(&ctx, &mm, case_id, page_id, &request.changes).await?;
if !request.rows.is_empty() {
    apply_direct_page_rows_patch(&ctx, &mm, case_id, page_id, &request.rows).await?;
}
```

Keep `CaseValidationSummaryBmc::mark_stale_for_case` after either changes or rows mutate.

- [ ] **Step 5: Implement minimal per-section change appliers**

Implement each applier by reusing the same update models and aliases already used by the corresponding `apply_*_page_rows_patch` function. The field keys must match the frontend maps from Task 2.

Example shape for RP:

```rust
async fn apply_rp_page_changes_patch(
    ctx: &lib_core::ctx::Ctx,
    mm: &ModelManager,
    case_id: Uuid,
    changes: &BTreeMap<String, CaseEditorFieldPatch>,
) -> Result<()> {
    let mut row = serde_json::Map::new();
    for (key, patch) in changes {
        row.insert(key.clone(), patch.value.clone().unwrap_or(Value::Null));
    }
    let rows = BTreeMap::from([("primarySources".to_string(), Value::Array(vec![Value::Object(row)]))]);
    apply_rp_page_rows_patch(ctx, mm, case_id, "RP", &rows).await
}
```

Use equivalent row keys for each section:

```rust
RP -> "primarySources"
SD -> "senderInformation" / "receiverInformation" as needed
LR -> "literatureReferences"
SI -> "studyInformation"
DM -> "patientInformation"
NR -> "narrative"
```

If an existing row applier requires arrays, wrap one object in an array. If it requires an object, pass the object directly.

- [ ] **Step 6: Run backend test**

Run:

```bash
cargo test -p web-server editor_direct_pages_accept_field_delta_changes_with_profiles --test api -- --nocapture
```

Expected: pass.

- [ ] **Step 7: Commit**

```bash
git add crates/services/web-server/src/web/rest/case_editor_rest.rs crates/services/web-server/tests/api/case_editor_contract_web.rs
git commit -m "Accept direct editor field deltas"
```

---

## Task 6: Backend Accepts `changes` for Repeatable Row PATCH

**Files:**
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/e2br3/crates/services/web-server/src/web/rest/case_editor_rest.rs`
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/e2br3/crates/services/web-server/tests/api/case_editor_contract_web.rs`

- [ ] **Step 1: Add failing repeatable contract tests**

In `case_editor_contract_web.rs`, add tests for row `changes` on `AE`, `LB`, `DG`, and `DH`, using existing row creation helpers in that file:

```rust
#[serial]
#[tokio::test]
async fn editor_repeatable_rows_accept_field_delta_changes_with_profiles() -> Result<()> {
    let mm = init_test_mm().await?;
    let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
    let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
    let cookie = cookie_header(&token.to_string());
    let app = web_server::app(mm);
    let case_id = create_case_for_editor(&app, &cookie, "EDITOR-ROW-DELTAS", &["ich"]).await?;
    create_safety_report(&app, &cookie, &case_id, "1", false).await?;

    let reaction_id = create_editor_reaction_row(&app, &cookie, &case_id).await?;
    let test_result_id = create_editor_test_result_row(&app, &cookie, &case_id).await?;
    let drug_id = create_editor_drug_row(&app, &cookie, &case_id).await?;
    let past_drug_id = create_editor_past_drug_row(&app, &cookie, &case_id).await?;

    let requests = [
        ("AE", reaction_id, json!({ "reactionTerm": { "value": "Headache" } })),
        ("LB", test_result_id, json!({ "resultValue": { "value": "10" } })),
        ("DG", drug_id, json!({ "medicinalProduct": { "value": "Drug A" } })),
        ("DH", past_drug_id, json!({ "drugName": { "value": "Past Drug" } })),
    ];

    for (section, row_id, changes) in requests {
        let (status, body) = patch_json(
            &app,
            &cookie,
            &format!("/api/cases/{case_id}/editor/pages/{section}/rows/{row_id}"),
            json!({
                "profiles": ["fda"],
                "changes": changes,
                "rows": {}
            }),
        )
        .await?;

        assert_eq!(status, StatusCode::OK, "{section}: {body}");
        assert_eq!(body["profiles"], json!(["fda"]), "{section}: {body}");
    }

    Ok(())
}
```

- [ ] **Step 2: Run repeatable backend test**

Run:

```bash
cargo test -p web-server editor_repeatable_rows_accept_field_delta_changes_with_profiles --test api -- --nocapture
```

Expected: fail if row handlers require `rows`.

- [ ] **Step 3: Add row changes helpers**

In `case_editor_rest.rs`, add:

```rust
fn changes_to_row_object(changes: &BTreeMap<String, CaseEditorFieldPatch>) -> serde_json::Map<String, Value> {
    let mut row = serde_json::Map::new();
    for (key, patch) in changes {
        row.insert(key.clone(), patch.value.clone().unwrap_or(Value::Null));
    }
    row
}

fn row_payload_from_changes(
    row_key: &str,
    changes: &BTreeMap<String, CaseEditorFieldPatch>,
) -> BTreeMap<String, Value> {
    BTreeMap::from([(row_key.to_string(), Value::Object(changes_to_row_object(changes)))])
}
```

- [ ] **Step 4: Use changes in each row PATCH handler**

In each row PATCH handler, select the row source like this:

```rust
let synthesized_rows;
let rows = if !request.changes.is_empty() {
    synthesized_rows = row_payload_from_changes("reaction", &request.changes);
    &synthesized_rows
} else {
    &request.rows
};
let row = required_row_object("AE", rows, "reaction")?;
```

Use row keys:

```rust
AE -> "reaction"
LB -> "testResult"
DG -> "drug"
DH -> "pastDrugHistory"
```

- [ ] **Step 5: Run repeatable backend test**

Run:

```bash
cargo test -p web-server editor_repeatable_rows_accept_field_delta_changes_with_profiles --test api -- --nocapture
```

Expected: pass.

- [ ] **Step 6: Commit**

```bash
git add crates/services/web-server/src/web/rest/case_editor_rest.rs crates/services/web-server/tests/api/case_editor_contract_web.rs
git commit -m "Accept repeatable editor row field deltas"
```

---

## Task 7: Remove Full-Row Fallbacks From Frontend Builders

**Files:**
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/case-save/pages/direct-page-patch.ts`
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/case-save/direct-page-delta.test.ts`
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/case-save/repeatable-row-delta.test.ts`

- [ ] **Step 1: Add tests that forbid row-shaped saves**

Add to both test files:

```ts
expect(request.rows).toEqual({});
expect(Object.keys(request.changes).length).toBeGreaterThan(0);
```

Ensure every direct and repeatable test contains these assertions.

- [ ] **Step 2: Remove legacy fallback from direct builder**

In `buildDirectPagePatchRequest`, remove all non-CI `rowsBySection` logic and the CI no-dirty fallback if all callers now pass `dirty`.

Final direct builder shape:

```ts
export function buildDirectPagePatchRequest(args: {
  section: DirectEditorSectionCode;
  authority: CaseEditAuthority;
  data: Record<string, unknown>;
  dirty: unknown;
}): CaseEditorPagePatchRequest {
  return {
    profiles: authorityToProfiles(args.authority),
    changes: buildFieldDelta({
      data: args.data,
      dirty: args.dirty,
      fields: DIRECT_FIELD_MAPS[args.section],
    }),
    rows: {},
  };
}
```

- [ ] **Step 3: Remove legacy fallback from repeatable builder**

Final repeatable builder shape:

```ts
export function buildRepeatablePageRowPatchRequest(args: {
  section: RepeatableEditorSectionCode;
  authority: CaseEditAuthority;
  rowId: string;
  data: Record<string, unknown>;
  dirty: unknown;
}): CaseEditorPagePatchRequest {
  const map = REPEATABLE_ROW_MAPS[args.section];
  const rows = arrayFrom(getValueAtPath(args.data, map.collectionPath));
  const index = rowIndexById(rows, args.rowId);
  if (index < 0) {
    throw new Error(`Cannot save ${args.section} row ${args.rowId}: row data was not loaded`);
  }
  const rowPrefix = [...map.collectionPath, index];
  return {
    profiles: authorityToProfiles(args.authority),
    changes: buildFieldDelta({
      data: args.data,
      dirty: args.dirty,
      fields: map.fields.map((field) => prependPath(rowPrefix, field)),
    }),
    rows: {},
  };
}
```

- [ ] **Step 4: Run frontend delta tests**

Run:

```bash
npm test -- --runTestsByPath __tests__/case-save/direct-page-delta.test.ts __tests__/case-save/repeatable-row-delta.test.ts __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts --runInBand
```

Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add lib/case-save/pages/direct-page-patch.ts __tests__/case-save/direct-page-delta.test.ts __tests__/case-save/repeatable-row-delta.test.ts __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts
git commit -m "Require field deltas for editor section saves"
```

---

## Task 8: Final Cross-Section Verification

**Files:**
- No source files expected unless verification finds defects.

- [ ] **Step 1: Run frontend typecheck**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
./node_modules/.bin/tsc -p tsconfig.json --noEmit --incremental false
```

Expected: exit code `0`.

- [ ] **Step 2: Run targeted frontend tests**

Run:

```bash
npm test -- --runTestsByPath \
  __tests__/case-save/direct-page-delta.test.ts \
  __tests__/case-save/repeatable-row-delta.test.ts \
  __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts \
  __tests__/case-form/case-editor-route-loading.test.tsx \
  __tests__/api/case-editor-api.test.ts \
  __tests__/case-editor-authority.test.ts \
  --runInBand
```

Expected: all listed suites pass.

- [ ] **Step 3: Run backend editor contract tests**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/e2br3
cargo test -p web-server case_editor_contract_web --test api -- --nocapture
```

Expected: all editor contract tests pass.

- [ ] **Step 4: Run legacy architecture scan**

Run:

```bash
rg -n "appendices_json|authorityProfiles|primaryProfile|activeProfiles|focusedAppendix|selectedAppendices|ProfileSelector|appendix-selector|\\.appendices\\.test" \
  /Users/hyundonghoon/projects/rust/e2br3/e2br3 \
  /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend \
  --glob '!target/**' \
  --glob '!node_modules/**' \
  --glob '!.next/**' \
  --glob '!docs/superpowers/**'
```

Expected: no matches.

- [ ] **Step 5: Run route shell check**

If the local frontend dev server is available:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npm run dev -- --port 3000
```

In another shell:

```bash
for url in \
  /US/case/1/detail/CI \
  /US/case/1/detail/RP \
  /US/case/1/detail/SD \
  /US/case/1/detail/LR \
  /US/case/1/detail/SI \
  /US/case/1/detail/DM \
  /US/case/1/detail/NR \
  /US/case/1/AE/list \
  /KR/case/1/detail/CI \
  /USKR/case/1/detail/CI
do
  printf '%s ' "$url"
  curl -o /dev/null -sS -w '%{http_code}\n' "http://127.0.0.1:3000${url}"
done
```

Expected: route shell responds with `200` for registered routes.

- [ ] **Step 6: Commit any verification fixes**

If verification required fixes:

```bash
git add <changed-files>
git commit -m "Fix editor delta save verification issues"
```

---

## Self-Review

**Spec coverage:** The plan keeps one authority/profile architecture across all sections, covers all direct sections (`CI`, `RP`, `SD`, `LR`, `SI`, `DM`, `NR`), covers all repeatable sections (`DH`, `AE`, `LB`, `DG`), and keeps backend validation/projection request-driven through `profiles`.

**Placeholder scan:** No task says “TBD”, “TODO”, or “similar to”; each implementation task includes concrete files, commands, and code shape.

**Type consistency:** The canonical request shape uses the existing frontend `CaseEditorPagePatchRequest`, existing `CaseEditAuthority`, existing `ValidationProfile`, and existing backend `CaseEditorPagePatchRequest`. The same `profiles + changes + rows` envelope is used throughout.

