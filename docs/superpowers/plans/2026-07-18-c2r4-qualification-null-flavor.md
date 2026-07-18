# C.2.r.4 Qualification NullFlavor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the existing `UNK` nullFlavor control to Case Edit `C.2.r.4 Qualification` and register the resulting frontend binding.

**Architecture:** Reuse `NullFlavorSelect` through the existing `E2BRadioField.trailingSlot`. Keep `qualification` and `qualificationNullFlavor` mutually exclusive and clear MFDS `qualificationKr1` whenever `UNK` is selected.

**Tech Stack:** React, React Hook Form, Jest, Python registry validator, JSON registry rows.

## Global Constraints

- Reuse the existing `NullFlavorSelect`; do not create another nullFlavor component.
- `C.2.r.4` accepts only `UNK` as nullFlavor.
- Selecting `UNK` clears both `qualification` and `qualificationKr1`.
- Selecting a Qualification value clears `qualificationNullFlavor`.
- Do not modify reporter name or address nullFlavor behavior in this change.

---

### Task 1: Add and register the C.2.r.4 UNK control

**Files:**
- Create: `../frontend/E2BR3-frontend/__tests__/case-form/ReporterSection.qualification-null-flavor.test.tsx`
- Modify: `../frontend/E2BR3-frontend/app/(protected)/[authority]/case/[id]/detail/RP/RPPage.tsx`
- Modify: `../frontend/E2BR3-frontend/app/(protected)/[authority]/case/[id]/detail/RP/components/ReporterEditorPanel.tsx`
- Modify: `registry/sections/c-safety-report.json`

**Interfaces:**
- Consumes: `NullFlavorSelect`, `E2BRadioField.trailingSlot`, and React Hook Form paths under `primarySources[index]`.
- Produces: a visible `primarySources[index].qualificationNullFlavor` binding accepting only `UNK`.

- [ ] **Step 1: Write a failing Case Edit component test**

Render `ReporterSection` with an MFDS reporter whose `qualification` is `3` and
`qualificationKr1` is `1`. Assert a select labelled
`Null flavor for Qualification` exists with only blank and `UNK` options. Change
it to `UNK` and assert the three form values become:

```ts
expect(methods.getValues("primarySources.0.qualificationNullFlavor")).toBe("UNK");
expect(methods.getValues("primarySources.0.qualification")).toBe("");
expect(methods.getValues("primarySources.0.qualificationKr1")).toBe("");
```

Then click Qualification `1` and assert:

```ts
expect(methods.getValues("primarySources.0.qualification")).toBe("1");
expect(methods.getValues("primarySources.0.qualificationNullFlavor")).toBe("");
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```sh
npx jest --runInBand __tests__/case-form/ReporterSection.qualification-null-flavor.test.tsx
```

Expected: FAIL because Case Edit does not render the Qualification nullFlavor select.

- [ ] **Step 3: Implement the minimal shared-control binding**

Watch `primarySources[activeIndex].qualificationNullFlavor` in `RPPage.tsx` and
pass it into `ReporterEditorPanel`. Add this trailing slot to `E2BRadioField`:

```tsx
<NullFlavorSelect
  name={`primarySources.${activeIndex}.qualificationNullFlavor`}
  label="Qualification"
  options={["UNK"]}
  clearFieldsOnSet={[
    { name: `primarySources.${activeIndex}.qualification`, value: "" },
    { name: `primarySources.${activeIndex}.qualificationKr1`, value: "" },
  ]}
/>
```

Disable the radio field while the watched value is `UNK`, hide KR.1 in that
state, and clear `qualificationNullFlavor` inside the Qualification `onChange`.

- [ ] **Step 4: Verify GREEN and update registry evidence**

Run the focused Jest test. Change `C.2.r.local.qualificationNullFlavor` in the
case registry to `status: complete` and `frontend.status: mapped`, pointing its
evidence at `ReporterEditorPanel.tsx`.

- [ ] **Step 5: Run regression and strict validation**

```sh
npx jest --runInBand __tests__/case-form/ReporterSection.qualification-null-flavor.test.tsx __tests__/dashboard/presave-minimal-form-validation.test.ts
python3 registry/tools/validate.py --strict-frontend-inventory
python3 registry/tools/validate.py --strict-presave-inventory
```

Expected: all commands PASS.

- [ ] **Step 6: Commit in both repositories**

```sh
git -C ../frontend/E2BR3-frontend add app __tests__/case-form/ReporterSection.qualification-null-flavor.test.tsx
git -C ../frontend/E2BR3-frontend commit -m "fix: add qualification null flavor to case edit"
git add registry/sections/c-safety-report.json
git commit -m "fix: register qualification null flavor case binding"
```
