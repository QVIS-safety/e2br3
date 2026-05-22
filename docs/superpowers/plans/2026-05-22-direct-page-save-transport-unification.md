# Direct Page Save Transport Unification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every direct editor section save through `PATCH /api/cases/{case_id}/editor/pages/{section}` with explicit appendix context.

**Architecture:** Backend adds page patch handlers for `RP`, `SD`, `LR`, `SI`, `DM`, and `NR` beside the existing `CI` handler. Frontend direct route saves call the page patch transport for direct sections and keep repeatable sections on row/list save APIs.

**Tech Stack:** Rust Axum web-server, existing BMC/service persistence models, Next.js/React frontend, Jest frontend tests, Rust API contract tests.

---

## File Structure

Backend:

- Modify `crates/services/web-server/src/web/rest/mod.rs` to attach `PATCH` handlers to direct page projection routes.
- Modify `crates/services/web-server/src/web/rest/case_editor_rest.rs` to add direct section patch handlers and shared direct patch helpers.
- Modify `crates/services/web-server/src/web/rest/case_editor_dto.rs` only if the patch request needs section-specific typed row helpers.
- Modify `crates/services/web-server/src/openapi.rs` to document all direct page patch endpoints.
- Modify `crates/services/web-server/tests/api/case_editor_contract_web.rs` for API contract coverage.

Frontend:

- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/api/endpoints/cases/core/editor.ts` so `patchEditorPageProjection` accepts every `DirectEditorSectionCode`.
- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/case-form/CaseFormWizardNew.tsx` to route direct section saves through page patch.
- Add or modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/case-save/pages/direct-page-patch.ts` as a focused adapter from current form data to page patch requests.
- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts`.
- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/api/case-editor-api.test.ts`.

## Task 1: Backend Contract Tests For Direct Page Patch Routes

**Files:**

- Modify `crates/services/web-server/tests/api/case_editor_contract_web.rs`

- [ ] **Step 1: Write the failing route coverage test**

Add a test that proves every direct page route accepts `PATCH` and returns projection context. Use the existing authenticated app/case helpers in this file.

```rust
#[serial]
#[tokio::test]
async fn editor_remaining_direct_pages_accept_page_patch_with_appendix() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case_with_appendices(
		&app,
		&cookie,
		"EDITOR-PAGES-PATCH",
		&["ich"],
	)
	.await?;

	for section in ["RP", "SD", "LR", "SI", "DM", "NR"] {
		let (status, body) = patch_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/editor/pages/{section}"),
			json!({
				"appendix": "fda",
				"changes": {}
			}),
		)
			.await?;

		assert_eq!(status, StatusCode::OK, "{section}: {body}");
		assert_eq!(body["focusedAppendix"], "fda", "{section}");
		assert!(body.get("appendices").is_none(), "{section}");
	}

	Ok(())
}
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```bash
cargo test -p web-server editor_remaining_direct_pages_accept_page_patch_with_appendix --test api -- --nocapture
```

Expected: fail with method-not-allowed or missing route for at least `RP`.

- [ ] **Step 3: Add the route handlers minimally**

In `crates/services/web-server/src/web/rest/mod.rs`, add `.patch(...)` handlers to each direct page route:

```rust
.route(
	"/cases/{case_id}/editor/pages/RP",
	get(case_editor_rest::get_editor_rp_page_projection)
		.patch(case_editor_rest::patch_editor_rp_page_projection),
)
.route(
	"/cases/{case_id}/editor/pages/SD",
	get(case_editor_rest::get_editor_sd_page_projection)
		.patch(case_editor_rest::patch_editor_sd_page_projection),
)
.route(
	"/cases/{case_id}/editor/pages/LR",
	get(case_editor_rest::get_editor_lr_page_projection)
		.patch(case_editor_rest::patch_editor_lr_page_projection),
)
.route(
	"/cases/{case_id}/editor/pages/SI",
	get(case_editor_rest::get_editor_si_page_projection)
		.patch(case_editor_rest::patch_editor_si_page_projection),
)
.route(
	"/cases/{case_id}/editor/pages/DM",
	get(case_editor_rest::get_editor_dm_page_projection)
		.patch(case_editor_rest::patch_editor_dm_page_projection),
)
.route(
	"/cases/{case_id}/editor/pages/NR",
	get(case_editor_rest::get_editor_nr_page_projection)
		.patch(case_editor_rest::patch_editor_nr_page_projection),
)
```

In `crates/services/web-server/src/web/rest/case_editor_rest.rs`, add minimal no-change handlers that require permissions and return the refreshed projection:

```rust
pub async fn patch_editor_rp_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorPageProjectionResponse>)> {
	patch_direct_page_projection(mm, ctx_w, case_id, "RP", request).await
}

pub async fn patch_editor_sd_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorPageProjectionResponse>)> {
	patch_direct_page_projection(mm, ctx_w, case_id, "SD", request).await
}

pub async fn patch_editor_lr_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorPageProjectionResponse>)> {
	patch_direct_page_projection(mm, ctx_w, case_id, "LR", request).await
}

pub async fn patch_editor_si_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorPageProjectionResponse>)> {
	patch_direct_page_projection(mm, ctx_w, case_id, "SI", request).await
}

pub async fn patch_editor_dm_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorPageProjectionResponse>)> {
	patch_direct_page_projection(mm, ctx_w, case_id, "DM", request).await
}

pub async fn patch_editor_nr_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorPageProjectionResponse>)> {
	patch_direct_page_projection(mm, ctx_w, case_id, "NR", request).await
}
```

Add the shared helper:

```rust
async fn patch_direct_page_projection(
	mm: ModelManager,
	ctx_w: CtxW,
	case_id: Uuid,
	page_id: &'static str,
	request: CaseEditorPagePatchRequest,
) -> Result<(axum::http::StatusCode, Json<CaseEditorPageProjectionResponse>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_UPDATE)?;
	require_permission(&ctx, SAFETY_REPORT_UPDATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	if !request.changes.is_empty() || !request.rows.is_empty() {
		return Err(Error::BadRequest {
			message: format!("{page_id} page patch fields are not implemented yet"),
		});
	}

	let appendix = parse_focused_appendix(request.appendix.as_deref())?;
	let projection = direct_page_projection_response(&mm, &ctx, case_id, page_id, appendix).await?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
}
```

- [ ] **Step 4: Run the test and verify GREEN**

Run:

```bash
cargo test -p web-server editor_remaining_direct_pages_accept_page_patch_with_appendix --test api -- --nocapture
```

Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add crates/services/web-server/src/web/rest/mod.rs crates/services/web-server/src/web/rest/case_editor_rest.rs crates/services/web-server/tests/api/case_editor_contract_web.rs
git commit -m "Add direct editor page patch routes"
```

## Task 2: Backend Field Rejection And Appendix Contract

**Files:**

- Modify `crates/services/web-server/tests/api/case_editor_contract_web.rs`
- Modify `crates/services/web-server/src/web/rest/case_editor_rest.rs`

- [ ] **Step 1: Write the failing contract tests**

Add tests:

```rust
#[serial]
#[tokio::test]
async fn editor_direct_page_patch_rejects_unknown_field() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_with_appendices(&app, &cookie, "EDITOR-PATCH-UNKNOWN", &["ich"])
			.await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/RP"),
		json!({
			"appendix": "fda",
			"changes": {
				"notAReporterField": { "value": "x" }
			}
		}),
	)
		.await?;

	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_direct_page_patch_rejects_unknown_appendix() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_with_appendices(&app, &cookie, "EDITOR-PATCH-BAD-APPENDIX", &["ich"])
			.await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/NR"),
		json!({
			"appendix": "unknown",
			"changes": {}
		}),
	)
		.await?;

	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	Ok(())
}
```

- [ ] **Step 2: Run the tests and verify RED/GREEN**

Run:

```bash
cargo test -p web-server editor_direct_page_patch_rejects_unknown --test api -- --nocapture
```

Expected after Task 1: appendix rejection already passes if `parse_focused_appendix` is used; unknown field rejection passes because non-empty changes are rejected. If either fails, adjust the shared helper before moving on.

- [ ] **Step 3: Commit**

```bash
git add crates/services/web-server/src/web/rest/case_editor_rest.rs crates/services/web-server/tests/api/case_editor_contract_web.rs
git commit -m "Lock direct page patch validation contract"
```

## Task 3: Frontend API Typing For All Direct Sections

**Files:**

- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/api/endpoints/cases/core/editor.ts`
- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/api/case-editor-api.test.ts`

- [ ] **Step 1: Write the failing frontend API test**

Add:

```ts
it("patches any direct editor page projection", async () => {
  mockedClient.patch.mockResolvedValueOnce({ caseId: "case-123" });

  await api.cases.patchEditorPageProjection("case-123", "NR", {
    appendix: "mfds",
    changes: {},
  });

  expect(mockedClient.patch).toHaveBeenCalledWith(
    "/api/cases/case-123/editor/pages/NR",
    {
      appendix: "mfds",
      changes: {},
    }
  );
});
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npm test -- --runTestsByPath __tests__/api/case-editor-api.test.ts --runInBand
```

Expected: TypeScript or test failure because `patchEditorPageProjection` only accepts `"CI"`.

- [ ] **Step 3: Implement minimal typing change**

Change:

```ts
patchEditorPageProjection: async (
  caseId: string,
  section: "CI",
  request: CaseEditorPagePatchRequest
) => {
```

to:

```ts
patchEditorPageProjection: async (
  caseId: string,
  section: DirectEditorSectionCode,
  request: CaseEditorPagePatchRequest
) => {
```

- [ ] **Step 4: Run the test and verify GREEN**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npm test -- --runTestsByPath __tests__/api/case-editor-api.test.ts --runInBand
```

Expected: pass.

- [ ] **Step 5: Commit**

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
git add lib/api/endpoints/cases/core/editor.ts __tests__/api/case-editor-api.test.ts
git commit -m "Allow direct editor page patch API for all sections"
```

## Task 4: Frontend Direct Section Save Adapter

**Files:**

- Create `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/case-save/pages/direct-page-patch.ts`
- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts`
- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/case-form/CaseFormWizardNew.tsx`

- [ ] **Step 1: Write the failing save orchestration test**

Add a test that mounts a direct route section such as `NR`, edits one field, saves, and asserts that the editor uses page patch:

```ts
it("saves direct section routes through page projection patch", async () => {
  mockUseCaseEditorRoute.mockReturnValue({
    caseId: "case-1",
    section: "NR",
    rowId: null,
    appendix: "fda",
    isDirectSection: true,
    isRepeatableSection: false,
  });
  mockedApi.cases.patchEditorPageProjection.mockResolvedValueOnce({
    caseId: "case-1",
    pageId: "NR",
    focusedAppendix: "fda",
    saved: true,
    requiredCount: 0,
    fields: {},
    rows: {},
    sectionSummaries: [],
  });

  render(<CaseFormWizardNew mode="edit" caseId="case-1" />);
  await userEvent.click(screen.getByRole("button", { name: /save/i }));

  expect(mockedApi.cases.patchEditorPageProjection).toHaveBeenCalledWith(
    "case-1",
    "NR",
    expect.objectContaining({
      appendix: "fda",
    })
  );
});
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npm test -- --runTestsByPath __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts --runInBand
```

Expected: fails because the save flow still builds legacy page save tasks.

- [ ] **Step 3: Add the direct page patch adapter**

Create `lib/case-save/pages/direct-page-patch.ts`:

```ts
import type { DirectEditorSectionCode } from "@/lib/case-editor/section-contracts";
import type { CaseEditorPagePatchRequest } from "@/lib/api/endpoints/cases/core/editor";

export type ValidationProfile = "ich" | "fda" | "mfds";

export function buildDirectPagePatchRequest(args: {
  section: DirectEditorSectionCode;
  focusedAppendix: ValidationProfile;
  data: Record<string, unknown>;
}): CaseEditorPagePatchRequest {
  const rowsBySection: Record<DirectEditorSectionCode, Record<string, unknown>> = {
    CI: {},
    RP: { primarySources: args.data.primarySources },
    SD: {
      messageHeader: args.data.messageHeader,
      senderInformation: args.data.senderInformation,
      receiverInformation: args.data.receiverInformation,
    },
    LR: { literatureReferences: args.data.literatureReferences },
    SI: {
      studyInformation: args.data.studyInformation,
      studyRegistrationNumbers: args.data.studyRegistrationNumbers,
    },
    DM: {
      patientInformation: args.data.patientInformation,
      patientMedicalHistory: args.data.patientMedicalHistory,
      patientDeath: args.data.patientDeath,
      parentInformation: args.data.parentInformation,
      parentMedicalHistory: args.data.parentMedicalHistory,
    },
    NR: {
      narrative: args.data.narrative,
      senderDiagnoses: args.data.senderDiagnoses,
      caseSummaries: args.data.caseSummaries,
    },
  };

  return {
    appendix: args.focusedAppendix,
    changes: {},
    rows: rowsBySection[args.section],
  };
}
```

This adapter sends only the section snapshot required by the focused direct page. It does not send the full case graph and does not send appendix metadata other than the explicit `appendix` request field.

- [ ] **Step 4: Route direct section saves through the patch API**

In `CaseFormWizardNew.tsx`, before legacy `pageSavePlans` are prepared, add a direct-route branch:

```ts
if (sectionScopedEditor?.isDirectSection && activeSection) {
  const sectionCode = sectionScopedEditor.section;
  const patchRequest = buildDirectPagePatchRequest({
    section: sectionCode,
    focusedAppendix,
    data: data as Record<string, unknown>,
  });

  await api.cases.patchEditorPageProjection(
    currentCaseId,
    sectionCode,
    patchRequest,
  );
  await refreshCaseAfterSave({ activeAppendix: focusedAppendix });
  return;
}
```

Import the adapter:

```ts
import { buildDirectPagePatchRequest } from "@/lib/case-save/pages/direct-page-patch";
```

- [ ] **Step 5: Run the test and verify GREEN**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npm test -- --runTestsByPath __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts --runInBand
```

Expected: pass.

- [ ] **Step 6: Commit**

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
git add components/case-form/CaseFormWizardNew.tsx lib/case-save/pages/direct-page-patch.ts __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts
git commit -m "Route direct editor saves through page patch"
```

## Task 5: Backend Implement Direct Page Persistence

**Files:**

- Modify `crates/services/web-server/src/web/rest/case_editor_rest.rs`
- Modify `crates/services/web-server/tests/api/case_editor_contract_web.rs`

- [ ] **Step 1: Write one failing persistence test per section**

Start with `NR` because it has a user-visible narrative field and clear readback behavior:

```rust
#[serial]
#[tokio::test]
async fn editor_nr_page_patch_persists_narrative_text() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_with_appendices(&app, &cookie, "EDITOR-NR-PATCH", &["ich"])
			.await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/NR"),
		json!({
			"appendix": "fda",
			"changes": {
				"narrativeText": { "value": "Narrative saved through page patch" }
			}
		}),
	)
		.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["fields"]["narrativeText"]["value"],
		"Narrative saved through page patch"
	);
	Ok(())
}
```

Add the remaining representative field tests as separate test functions:

```rust
("RP", "primarySourceQualification", "1")
("SD", "senderOrganization", "Sender Org")
("LR", "literatureReference", "Smith 2026")
("SI", "studyName", "Study 001")
("DM", "patientInitials", "ABC")
```

- [ ] **Step 2: Run the section persistence test and verify RED**

Run:

```bash
cargo test -p web-server editor_nr_page_patch_persists_narrative_text --test api -- --nocapture
```

Expected: fail with the not-implemented bad request from Task 1.

- [ ] **Step 3: Implement section dispatch**

Replace the non-empty rejection in `patch_direct_page_projection` with section dispatch:

```rust
match page_id {
	"RP" => apply_rp_page_patch(&mm, &ctx, case_id, &request).await?,
	"SD" => apply_sd_page_patch(&mm, &ctx, case_id, &request).await?,
	"LR" => apply_lr_page_patch(&mm, &ctx, case_id, &request).await?,
	"SI" => apply_si_page_patch(&mm, &ctx, case_id, &request).await?,
	"DM" => apply_dm_page_patch(&mm, &ctx, case_id, &request).await?,
	"NR" => apply_nr_page_patch(&mm, &ctx, case_id, &request).await?,
	_ => {
		return Err(Error::BadRequest {
			message: format!("unsupported direct page '{page_id}'"),
		});
	}
}
```

Each `apply_*_page_patch` function maps accepted field names to the existing section update model used by the legacy REST endpoint. Unknown fields return:

```rust
return Err(Error::BadRequest {
	message: format!("unknown {page_id} field '{field}'"),
});
```

- [ ] **Step 4: Implement and verify one section at a time**

For each section, repeat:

```bash
cargo test -p web-server editor_nr_page_patch_persists_narrative_text --test api -- --nocapture
cargo test -p web-server editor_rp_page_patch_persists_primary_source_qualification --test api -- --nocapture
cargo test -p web-server editor_sd_page_patch_persists_sender_organization --test api -- --nocapture
cargo test -p web-server editor_lr_page_patch_persists_literature_reference --test api -- --nocapture
cargo test -p web-server editor_si_page_patch_persists_study_name --test api -- --nocapture
cargo test -p web-server editor_dm_page_patch_persists_patient_initials --test api -- --nocapture
```

Expected: fail before mapping, pass after mapping.

- [ ] **Step 5: Commit after each section**

Example:

```bash
git add crates/services/web-server/src/web/rest/case_editor_rest.rs crates/services/web-server/tests/api/case_editor_contract_web.rs
git commit -m "Persist NR editor page patch"
```

Use equivalent commits for `RP`, `SD`, `LR`, `SI`, and `DM`.

## Task 6: OpenAPI Direct Page Patch Documentation

**Files:**

- Modify `crates/services/web-server/src/openapi.rs`

- [ ] **Step 1: Write or extend the OpenAPI test/check**

Run the existing OpenAPI generation/check command used by this repo:

```bash
cargo test -p web-server openapi -- --nocapture
```

If there is no dedicated OpenAPI test, use `cargo check -p web-server` as the compile-time contract.

- [ ] **Step 2: Add patch operation declarations**

Add `patch_editor_rp_page`, `patch_editor_sd_page`, `patch_editor_lr_page`, `patch_editor_si_page`, `patch_editor_dm_page`, and `patch_editor_nr_page` beside `patch_editor_ci_page`, all using `CaseEditorPagePatchRequestDoc` as request body and `CaseEditorPageProjectionResponseDoc` as response.

- [ ] **Step 3: Run verification**

Run:

```bash
cargo check -p web-server
```

Expected: pass.

- [ ] **Step 4: Commit**

```bash
git add crates/services/web-server/src/openapi.rs
git commit -m "Document direct editor page patch endpoints"
```

## Task 7: Frontend Repeatable Sections Stay On Row/List Save Path

**Files:**

- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts`

- [ ] **Step 1: Add regression test**

Add:

```ts
it("keeps repeatable section saves on the existing save coordinators", async () => {
  mockUseCaseEditorRoute.mockReturnValue({
    caseId: "case-1",
    section: "AE",
    rowId: "row-1",
    appendix: "fda",
    isDirectSection: false,
    isRepeatableSection: true,
  });

  render(<CaseFormWizardNew mode="edit" caseId="case-1" />);
  await userEvent.click(screen.getByRole("button", { name: /save/i }));

  expect(mockedApi.cases.patchEditorPageProjection).not.toHaveBeenCalled();
});
```

- [ ] **Step 2: Run test**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npm test -- --runTestsByPath __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts --runInBand
```

Expected: pass after Task 4.

- [ ] **Step 3: Commit**

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
git add __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts
git commit -m "Keep repeatable editor saves on row transport"
```

## Task 8: Full Verification

**Files:**

- No code changes unless verification exposes failures.

- [ ] **Step 1: Backend verification**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/e2br3
cargo check -p web-server
cargo test -p web-server editor_ --test api -- --nocapture
cargo test -p web-server explicit_profiles --test api -- --nocapture
git diff --check
```

Expected: all pass.

- [ ] **Step 2: Frontend verification**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
./node_modules/.bin/tsc -p tsconfig.json --noEmit --incremental false
npm test -- --runTestsByPath __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts __tests__/case-form/case-editor-route-loading.test.tsx __tests__/api/case-editor-api.test.ts __tests__/api.endpoints.test.ts --runInBand
git diff --check
```

Expected: all pass.

- [ ] **Step 3: Final architecture grep checks**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/e2br3
rg -n "appendices_json|appendicesJson" crates db --glob '!target'
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
rg -n "appendices_json|appendicesJson" app components lib __tests__
```

Expected: no runtime matches.

- [ ] **Step 4: Final commit**

If verification required fixes in backend files, commit them with the exact files changed by verification:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/e2br3
git add crates/services/web-server/src/web/rest/case_editor_rest.rs crates/services/web-server/tests/api/case_editor_contract_web.rs crates/services/web-server/src/openapi.rs
git commit -m "Verify direct editor save transport"
```

If verification required fixes in frontend files, commit them with the exact files changed by verification:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
git add components/case-form/CaseFormWizardNew.tsx lib/api/endpoints/cases/core/editor.ts lib/case-save/pages/direct-page-patch.ts __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts __tests__/api/case-editor-api.test.ts
git commit -m "Verify direct editor save transport"
```
