# Workflow QC/Lock Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make QC/QCed, Lock, and workflow behavior match `docs/requirements/03.csv` rows 1, 3, 5, and 24: QCed cases stay read-only, QCed cases can still be locked, Review/Reviewed/Validated wording is removed from user-facing workflow surfaces, and export/submission filters expose QC, lock, and workflow axes separately.

**Architecture:** Keep the existing backend separation between legacy lifecycle status (`cases.status`) and configurable workflow status (`cases.workflow_status`). Enforce QCed/locked immutability before workflow editability, because QC/Lock are compliance locks independent of workflow. In the frontend, separate “content read-only” from “lock action allowed” so a QCed case cannot edit content but can transition to locked.

**Tech Stack:** Rust/Axum/sqlx backend in `/Users/hyundonghoon/projects/rust/e2br3/e2br3`; Next.js/React/Jest frontend in `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend`.

---

## Subagent Strategy

- Backend worker owns `/Users/hyundonghoon/projects/rust/e2br3/e2br3/crates/libs/lib-rest-core/src/lib.rs` and `/Users/hyundonghoon/projects/rust/e2br3/e2br3/crates/services/web-server/tests/api/case_validation_web.rs`.
- Frontend case worker owns `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/case-form/CaseFormLayout.tsx`, `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/case-form/CaseHeader.tsx`, `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/case-form/CaseFormWizardNew.tsx`, and case header tests.
- Frontend list/submission worker owns `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/app/dashboard/cases/page.tsx`, `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/app/dashboard/submission/page.tsx`, and focused UI/helper tests.
- Workers are not alone in the codebase. Do not revert changes made by other workers. Keep edits inside the owned files and adapt to any already-present changes.

## File Structure

- Modify `crates/libs/lib-rest-core/src/lib.rs`: move QCed legacy read-only enforcement before workflow enabled branch, preserving locked/deleted handling.
- Modify `crates/services/web-server/tests/api/case_validation_web.rs`: add a failing backend regression test proving QCed content edits are blocked while workflow is enabled and `Saved` is editable.
- Modify `components/case-form/CaseFormLayout.tsx`: pass separate lock disabled state instead of tying lock to `isReadOnly`.
- Modify `components/case-form/CaseHeader.tsx`: keep content read-only UI, but allow the lock button for QCed cases; ensure already locked/deleted cases still cannot lock.
- Modify `components/case-form/CaseFormWizardNew.tsx`: let `handleLockCase` lock a QCed case while still blocking already locked/deleted states; remove user-facing “Review/Reviewed/Validated” labels from the workflow panel where lifecycle status is displayed.
- Modify `app/dashboard/cases/page.tsx`: format reviewed/validated as `QCed`, update filters to `QCed`, and update badge coloring conditions.
- Modify `app/dashboard/submission/page.tsx`: replace the single legacy Status UI for export/submission queue with separate QC Status, Lock State, and Workflow Status controls using the fields already present on case read/list objects; preserve the current legacy status filter only where still needed for draft/submitted/deleted lifecycle filtering.
- Add/update focused Jest tests under `__tests__/case-form` and `__tests__/dashboard` for labels and lock action enablement.

---

### Task 1: Backend QCed Read-Only Enforcement

**Files:**
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/e2br3/crates/libs/lib-rest-core/src/lib.rs:725-790`
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/e2br3/crates/services/web-server/tests/api/case_validation_web.rs`

- [ ] **Step 1: Write the failing backend regression test**

Add this test near the existing workflow tests in `case_validation_web.rs`, after `test_non_editable_workflow_status_blocks_subresource_write`:

```rust
#[serial]
#[tokio::test]
async fn test_qced_case_blocks_content_updates_even_when_workflow_saved_is_editable(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"description": "Default authoring state",
							"allowed_roles": ["PVS", "PVM"]
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let (status, body) =
		update_case_status(&app, &cookie, case_id, "reviewed").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, body) = update_safety_report(
		&app,
		&cookie,
		case_id,
		json!({
			"data": {
				"report_type": "2"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string().contains("QCed cases are read-only"),
		"{body:?}"
	);

	Ok(())
}
```

- [ ] **Step 2: Run the test and confirm it fails**

Run:

```bash
cargo test -p web-server test_qced_case_blocks_content_updates_even_when_workflow_saved_is_editable --test api -- --nocapture
```

Expected: FAIL because the current code allows the update when workflow is enabled and `Saved` is editable.

- [ ] **Step 3: Implement the minimal backend fix**

In `case_write_block_reason_for_case`, move the `reviewed` guard before `load_workflow_runtime_settings`:

```rust
pub async fn case_write_block_reason_for_case(
	ctx: &Ctx,
	mm: &ModelManager,
	case: &Case,
) -> Result<Option<WorkflowBlockReason>> {
	let legacy_status = case.status.trim();
	if legacy_status.eq_ignore_ascii_case("deleted") {
		return Ok(Some(WorkflowBlockReason {
			code: "case_deleted",
			message: "deleted cases are read-only".to_string(),
		}));
	}
	if legacy_status.eq_ignore_ascii_case("locked") {
		return Ok(Some(WorkflowBlockReason {
			code: "case_locked",
			message: "locked cases are read-only".to_string(),
		}));
	}
	if legacy_status.eq_ignore_ascii_case("reviewed") {
		return Ok(Some(WorkflowBlockReason {
			code: "case_qced",
			message: "QCed cases are read-only".to_string(),
		}));
	}

	let workflow = load_workflow_runtime_settings(mm).await?;
	if workflow.enabled {
		let Some(rule) = workflow.find_status(&case.workflow_status) else {
			return Ok(Some(WorkflowBlockReason {
				code: "workflow_status_not_configured",
				message: format!(
					"workflow status '{}' is not configured",
					case.workflow_status
				),
			}));
		};
		let ownership = workflow_ownership_for_case(ctx, mm, case, rule).await?;
		if !ownership.role_match && !ownership.admin_override_allowed {
			return Ok(Some(WorkflowBlockReason {
				code: "workflow_role_mismatch",
				message: format!(
					"workflow status '{}' is assigned to a different role",
					rule.name
				),
			}));
		}
		if !ownership.user_match && !ownership.admin_override_allowed {
			return Ok(Some(WorkflowBlockReason {
				code: "workflow_user_mismatch",
				message: format!(
					"workflow status '{}' is assigned to a different user",
					rule.name
				),
			}));
		}
		if !rule.editable {
			return Ok(Some(WorkflowBlockReason {
				code: "workflow_status_read_only",
				message: format!("workflow status '{}' is read-only", rule.name),
			}));
		}
		return Ok(None);
	}

	Ok(None)
}
```

- [ ] **Step 4: Run targeted backend tests**

Run:

```bash
cargo test -p web-server test_qced_case_blocks_content_updates_even_when_workflow_saved_is_editable --test api -- --nocapture
cargo test -p web-server test_non_editable_workflow_status_blocks_subresource_write --test api -- --nocapture
cargo test -p web-server test_locked_case_rejects_content_updates --test api -- --nocapture
```

Expected: PASS for all three.

- [ ] **Step 5: Commit backend fix**

```bash
git add /Users/hyundonghoon/projects/rust/e2br3/e2br3/crates/libs/lib-rest-core/src/lib.rs /Users/hyundonghoon/projects/rust/e2br3/e2br3/crates/services/web-server/tests/api/case_validation_web.rs
git commit -m "fix: keep qced cases read-only with workflow enabled"
```

---

### Task 2: Frontend Lock Action for QCed Cases

**Files:**
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/case-form/CaseFormLayout.tsx`
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/case-form/CaseHeader.tsx`
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/case-form/CaseFormWizardNew.tsx`
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/case-form/CaseHeader.appendix-selector.test.ts`

- [ ] **Step 1: Add a failing CaseHeader test**

Append this test to `CaseHeader.appendix-selector.test.ts`:

```ts
it("keeps lock enabled for QCed read-only cases", async () => {
  const onLock = jest.fn();

  await act(async () => {
    root.render(
      React.createElement(CaseHeader, {
        caseId: "CASE-1",
        caseStatus: "reviewed",
        isReadOnly: true,
        onLock,
        validateDisabled: true,
        lockDisabled: false,
      })
    );
  });

  const lockButton = container.querySelector('[title="Lock case"]') as HTMLButtonElement | null;
  expect(lockButton).not.toBeNull();
  expect(lockButton!.disabled).toBe(false);

  await act(async () => {
    lockButton!.click();
  });

  expect(container.textContent).toContain("Lock this case and switch it to read-only mode?");
});
```

- [ ] **Step 2: Run the frontend test and confirm it fails**

Run from `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend`:

```bash
npm test -- __tests__/case-form/CaseHeader.appendix-selector.test.ts --runInBand
```

Expected: FAIL because the current layout passes `lockDisabled={isReadOnly || isSaving}` for QCed cases in normal app rendering.

- [ ] **Step 3: Split read-only from lock-disabled in the layout**

In `CaseFormLayout.tsx`, change the lock disabled prop:

```tsx
const lockDisabled =
  isSaving ||
  caseStatus === "locked" ||
  caseStatus === "submitted" ||
  caseStatus === "deleted";
```

Then pass it:

```tsx
<CaseHeader
  caseId={caseId}
  versionLabel={versionLabel}
  saveStatus={saveStatus}
  semanticStatus={semanticStatus}
  onValidate={onValidate}
  onLock={onLock}
  caseStatus={caseStatus}
  validateDisabled={isReadOnly || isSaving}
  lockDisabled={lockDisabled}
  appendixSelectionDisabled={isReadOnly || isSaving}
  isReadOnly={isReadOnly}
  focusedAppendix={focusedAppendix}
  selectedAppendices={selectedAppendices}
  onFocusedAppendixChange={onFocusedAppendixChange}
  onAppendicesChange={onAppendicesChange}
/>
```

- [ ] **Step 4: Allow locking reviewed/QCed in the wizard handler**

In `CaseFormWizardNew.tsx`, replace the start of `handleLockCase` with:

```ts
const handleLockCase = useCallback(async () => {
  if (!persistedCaseId) {
    toast.error("Save the case before locking it.");
    return;
  }
  if (normalizedCurrentCaseStatus === "locked") {
    toast("Case is already locked.");
    return;
  }
  if (normalizedCurrentCaseStatus === "deleted") {
    toast("Deleted cases cannot be locked.");
    return;
  }
  if (normalizedCurrentCaseStatus === "submitted") {
    toast("Submitted cases cannot be locked from this screen.");
    return;
  }
  const previousStatus = getValues("status");
  setValue("status", "locked", { shouldDirty: true });
  const saved = await handleSave();
  if (saved) {
    toast.success("Case locked successfully.");
    return;
  }
  setValue(
    "status",
    (previousStatus as E2BR3Case["status"] | undefined) ?? "draft",
    { shouldDirty: false }
  );
}, [
  getValues,
  handleSave,
  normalizedCurrentCaseStatus,
  persistedCaseId,
  setValue,
]);
```

- [ ] **Step 5: Run targeted frontend test**

Run:

```bash
npm test -- __tests__/case-form/CaseHeader.appendix-selector.test.ts --runInBand
```

Expected: PASS.

- [ ] **Step 6: Commit frontend lock fix**

```bash
git add /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/case-form/CaseFormLayout.tsx /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/case-form/CaseHeader.tsx /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/case-form/CaseFormWizardNew.tsx /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/case-form/CaseHeader.appendix-selector.test.ts
git commit -m "fix: allow locking qced cases from case header"
```

---

### Task 3: QC/QCed Terminology Cleanup in Case List

**Files:**
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/app/dashboard/cases/page.tsx`
- Add: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/dashboard/case-status-labels.test.ts`

- [ ] **Step 1: Extract and test status label helpers**

Move `formatStatus` to an exported helper near the top of `app/dashboard/cases/page.tsx`:

```ts
export const formatCaseLifecycleStatus = (value?: string) => {
  if (!value) return "Draft";
  const trimmed = value.trim();
  if (!trimmed) return "Draft";
  const normalized = trimmed.toLowerCase();
  if (normalized === "reviewed" || normalized === "validated") return "QCed";
  if (normalized === "locked") return "Locked";
  return trimmed
    .split(/[_\s-]+/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1).toLowerCase())
    .join(" ");
};
```

Add `__tests__/dashboard/case-status-labels.test.ts`:

```ts
import { formatCaseLifecycleStatus } from "@/app/dashboard/cases/page";

describe("case lifecycle status labels", () => {
  it("uses QCed instead of Reviewed or Validated", () => {
    expect(formatCaseLifecycleStatus("reviewed")).toBe("QCed");
    expect(formatCaseLifecycleStatus("validated")).toBe("QCed");
  });

  it("keeps lock and draft labels stable", () => {
    expect(formatCaseLifecycleStatus("locked")).toBe("Locked");
    expect(formatCaseLifecycleStatus("draft")).toBe("Draft");
  });
});
```

- [ ] **Step 2: Run the test and confirm it fails before helper update**

Run:

```bash
npm test -- __tests__/dashboard/case-status-labels.test.ts --runInBand
```

Expected: FAIL if the old formatter still returns `Reviewed`.

- [ ] **Step 3: Update case page labels and filters**

Replace calls to the local `formatStatus` with `formatCaseLifecycleStatus`.

Change the status filter options:

```tsx
<option value="draft">Draft</option>
<option value="reviewed">QCed</option>
<option value="locked">Locked</option>
<option value="submitted">Submitted</option>
<option value="deleted">Deleted</option>
<option value="archived">Archived</option>
```

Remove the visible `Validated` filter option unless backend/product explicitly needs users to search the raw legacy value. The formatter already maps legacy `validated` rows to `QCed`.

Update badge coloring checks:

```tsx
caseItem.status === "QCed"
  ? "bg-blue-100 text-blue-800"
```

- [ ] **Step 4: Run targeted frontend status tests**

Run:

```bash
npm test -- __tests__/dashboard/case-status-labels.test.ts --runInBand
npm test -- __tests__/case-form/CaseHeader.appendix-selector.test.ts --runInBand
```

Expected: PASS.

- [ ] **Step 5: Commit terminology cleanup**

```bash
git add /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/app/dashboard/cases/page.tsx /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/dashboard/case-status-labels.test.ts
git commit -m "fix: use qced terminology in case list"
```

---

### Task 4: Separate Export Queue Filters

**Files:**
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/app/dashboard/submission/page.tsx`
- Add: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/dashboard/submission-filters.test.ts`

- [ ] **Step 1: Extract filter predicate helpers**

Add exported helpers near the top of `submission/page.tsx`:

```ts
export type SubmissionCaseFilterInput = {
  status?: string | null;
  qcState?: string | null;
  isLocked?: boolean | null;
  workflowStatus?: string | null;
};

export type SubmissionQueueFilters = {
  lifecycleStatus: string;
  qcStatus: string;
  lockState: string;
  workflowStatus: string;
};

export function caseMatchesSubmissionQueueFilters(
  item: SubmissionCaseFilterInput,
  filters: SubmissionQueueFilters
) {
  const lifecycle = item.status?.trim().toLowerCase() || "draft";
  const qc = item.qcState?.trim().toLowerCase() || "pending";
  const workflow = item.workflowStatus?.trim() || "";

  if (filters.lifecycleStatus !== "all" && lifecycle !== filters.lifecycleStatus) {
    return false;
  }
  if (filters.qcStatus === "pending" && qc !== "pending") {
    return false;
  }
  if (filters.qcStatus === "qced" && qc !== "qced") {
    return false;
  }
  if (filters.lockState === "locked" && item.isLocked !== true) {
    return false;
  }
  if (filters.lockState === "unlocked" && item.isLocked === true) {
    return false;
  }
  if (filters.workflowStatus !== "all" && workflow !== filters.workflowStatus) {
    return false;
  }
  return true;
}
```

Add `__tests__/dashboard/submission-filters.test.ts`:

```ts
import {
  caseMatchesSubmissionQueueFilters,
  SubmissionQueueFilters,
} from "@/app/dashboard/submission/page";

const baseFilters: SubmissionQueueFilters = {
  lifecycleStatus: "all",
  qcStatus: "all",
  lockState: "all",
  workflowStatus: "all",
};

describe("submission queue filters", () => {
  it("filters QCed cases independently from lock state", () => {
    expect(
      caseMatchesSubmissionQueueFilters(
        { status: "reviewed", qcState: "QCed", isLocked: false, workflowStatus: "Saved" },
        { ...baseFilters, qcStatus: "qced" }
      )
    ).toBe(true);

    expect(
      caseMatchesSubmissionQueueFilters(
        { status: "draft", qcState: "Pending", isLocked: false, workflowStatus: "Saved" },
        { ...baseFilters, qcStatus: "qced" }
      )
    ).toBe(false);
  });

  it("filters locked cases independently from QC state", () => {
    expect(
      caseMatchesSubmissionQueueFilters(
        { status: "locked", qcState: "Pending", isLocked: true, workflowStatus: "Saved" },
        { ...baseFilters, lockState: "locked" }
      )
    ).toBe(true);

    expect(
      caseMatchesSubmissionQueueFilters(
        { status: "reviewed", qcState: "QCed", isLocked: false, workflowStatus: "Saved" },
        { ...baseFilters, lockState: "locked" }
      )
    ).toBe(false);
  });

  it("filters workflow status independently from legacy lifecycle status", () => {
    expect(
      caseMatchesSubmissionQueueFilters(
        { status: "draft", qcState: "Pending", isLocked: false, workflowStatus: "To be reviewed" },
        { ...baseFilters, workflowStatus: "To be reviewed" }
      )
    ).toBe(true);

    expect(
      caseMatchesSubmissionQueueFilters(
        { status: "draft", qcState: "Pending", isLocked: false, workflowStatus: "Saved" },
        { ...baseFilters, workflowStatus: "To be reviewed" }
      )
    ).toBe(false);
  });
});
```

- [ ] **Step 2: Run the test and confirm it fails before wiring**

Run:

```bash
npm test -- __tests__/dashboard/submission-filters.test.ts --runInBand
```

Expected: FAIL until the helpers are exported and wired.

- [ ] **Step 3: Add independent state variables**

Replace the single queue status state with:

```ts
const [lifecycleStatusFilter, setLifecycleStatusFilter] = useState("all");
const [qcStatusFilter, setQcStatusFilter] = useState("all");
const [lockStateFilter, setLockStateFilter] = useState("all");
const [workflowStatusFilter, setWorkflowStatusFilter] = useState("all");
```

Keep existing search/case-number filters unchanged.

- [ ] **Step 4: Wire the predicate into queue filtering**

Where the submission queue currently checks `statusFilter`, use:

```ts
const queueFilters: SubmissionQueueFilters = {
  lifecycleStatus: lifecycleStatusFilter,
  qcStatus: qcStatusFilter,
  lockState: lockStateFilter,
  workflowStatus: workflowStatusFilter,
};

const matchesQueueAxes = caseMatchesSubmissionQueueFilters(caseItem, queueFilters);
```

Then combine `matchesQueueAxes` with existing sender/case/search/export eligibility checks.

- [ ] **Step 5: Replace the visible filter UI**

Replace the single Status dropdown with four controls:

```tsx
<div className="space-y-1.5">
  <label className="text-xs font-semibold uppercase tracking-[0.12em] text-gray-500">Lifecycle</label>
  <select
    value={lifecycleStatusFilter}
    onChange={(e) => setLifecycleStatusFilter(e.target.value)}
    className="rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900"
  >
    <option value="all">All lifecycle states</option>
    <option value="draft">Draft</option>
    <option value="submitted">Submitted</option>
    <option value="deleted">Deleted</option>
    <option value="archived">Archived</option>
    <option value="nullified">Nullified</option>
  </select>
</div>
<div className="space-y-1.5">
  <label className="text-xs font-semibold uppercase tracking-[0.12em] text-gray-500">QC Status</label>
  <select
    value={qcStatusFilter}
    onChange={(e) => setQcStatusFilter(e.target.value)}
    className="rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900"
  >
    <option value="all">All QC states</option>
    <option value="pending">Pending</option>
    <option value="qced">QCed</option>
  </select>
</div>
<div className="space-y-1.5">
  <label className="text-xs font-semibold uppercase tracking-[0.12em] text-gray-500">Lock State</label>
  <select
    value={lockStateFilter}
    onChange={(e) => setLockStateFilter(e.target.value)}
    className="rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900"
  >
    <option value="all">All lock states</option>
    <option value="locked">Locked</option>
    <option value="unlocked">Unlocked</option>
  </select>
</div>
<div className="space-y-1.5">
  <label className="text-xs font-semibold uppercase tracking-[0.12em] text-gray-500">Workflow Status</label>
  <select
    value={workflowStatusFilter}
    onChange={(e) => setWorkflowStatusFilter(e.target.value)}
    className="rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900"
  >
    <option value="all">All workflow statuses</option>
    {Array.from(new Set(queueCases.map((item) => item.workflowStatus).filter(Boolean))).map((status) => (
      <option key={status} value={status as string}>{status}</option>
    ))}
  </select>
</div>
```

- [ ] **Step 6: Run targeted submission tests**

Run:

```bash
npm test -- __tests__/dashboard/submission-filters.test.ts --runInBand
```

Expected: PASS.

- [ ] **Step 7: Commit submission filters**

```bash
git add /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/app/dashboard/submission/page.tsx /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/dashboard/submission-filters.test.ts
git commit -m "fix: split submission queue qc lock workflow filters"
```

---

### Task 5: Cross-Repo Verification and Tracker Update

**Files:**
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/requirements/client_requirements_todo.md`

- [ ] **Step 1: Run backend verification**

Run from `/Users/hyundonghoon/projects/rust/e2br3/e2br3`:

```bash
cargo fmt --all
cargo test -p web-server test_qced_case_blocks_content_updates_even_when_workflow_saved_is_editable --test api -- --nocapture
cargo test -p web-server test_locked_case_rejects_content_updates --test api -- --nocapture
cargo test -p web-server test_locked_case_blocks_workflow_transition_even_for_admin_override --test api -- --nocapture
```

Expected: all tests pass.

- [ ] **Step 2: Run frontend verification**

Run from `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend`:

```bash
npm test -- __tests__/case-form/CaseHeader.appendix-selector.test.ts --runInBand
npm test -- __tests__/dashboard/case-status-labels.test.ts --runInBand
npm test -- __tests__/dashboard/submission-filters.test.ts --runInBand
npm run build
```

Expected: all tests and build pass.

- [ ] **Step 3: Update requirements tracker with evidence**

Update these entries in `docs/requirements/client_requirements_todo.md`:

```markdown
- [-] Global `QC` / `QCed` terminology cleanup outside already converted workflow areas. Case header, case list, and submission queue lifecycle filters now use `QC`/`QCed`; remaining scope is a broader scan of admin/dashboard copy that is not part of row 5 lock behavior.
- [x] Ensure QC/lock actions behave consistently for manual cases and imported cases. Backend blocks content edits for QCed and locked cases even when workflow is enabled; frontend allows QCed cases to proceed to Lock while keeping content read-only.
- [-] Replace ad hoc review state wording with explicit workflow-aware status where the client expects workflow status instead of a generic checked state. Backend workflow status remains separate from QC and Lock; case list and submission filters no longer collapse QC, lock, and workflow into one visible status. Remaining cleanup is non-blocking copy outside the reviewed workflow surfaces.
```

Update the submission filter checklist:

```markdown
- [x] QC status
- [x] lock status
- [x] workflow status
```

Leave `ack accept status if still required` open unless this task also implements ACK filtering.

- [ ] **Step 4: Commit tracker update**

```bash
git add /Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/requirements/client_requirements_todo.md
git commit -m "docs: update qc lock workflow requirement status"
```

---

## Final Review Checklist

- [ ] QCed content edits are blocked by backend regardless of workflow configuration.
- [ ] Locked cases remain read-only and cannot transition workflow.
- [ ] QCed cases can be locked from the frontend.
- [ ] Case list no longer displays `Reviewed` or `Validated` for QC state.
- [ ] Submission/export queue has independent QC Status, Lock State, and Workflow Status filters.
- [ ] Tracker only marks ACK filtering open if ACK filtering remains unimplemented.

