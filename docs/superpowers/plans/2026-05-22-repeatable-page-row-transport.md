# Repeatable Page Row Transport Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move repeatable editor sections `DH/AE/LB/DG` under the page namespace with row-level save precision, then remove frontend editor dependence on old repeatable routes.

**Architecture:** Backend adds page namespace list, row detail, create, patch, and delete endpoints for repeatable sections. Frontend API and route loading switch to those endpoints, while direct sections keep direct page patch and full-case wizard save coordinators remain unchanged.

**Tech Stack:** Rust Axum web-server, existing lib-core BMC models, Next.js/React frontend, TypeScript API client, Jest, Rust API contract tests.

---

## File Structure

Backend:

- Modify `crates/services/web-server/src/web/rest/mod.rs` to add repeatable page namespace routes.
- Modify `crates/services/web-server/src/web/rest/case_editor_rest.rs` to add repeatable page list/row/save/delete handlers.
- Modify `crates/services/web-server/src/openapi.rs` to document the new repeatable page namespace endpoints.
- Modify `crates/services/web-server/tests/api/case_editor_contract_web.rs` for endpoint contracts and persistence coverage.

Frontend:

- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/api/endpoints/cases/core/editor.ts` to add repeatable page namespace methods.
- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/case-editor/route-loading.ts` if route unwrap logic needs to normalize list projection rows.
- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/case-form/CaseFormWizardNew.tsx` to refresh repeatable route saves through the page row namespace.
- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/api/case-editor-api.test.ts`.
- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/case-form/case-editor-route-loading.test.tsx`.
- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts`.

## Task 1: Backend Repeatable Page List Routes

**Files:**

- Modify `crates/services/web-server/tests/api/case_editor_contract_web.rs`
- Modify `crates/services/web-server/src/web/rest/mod.rs`
- Modify `crates/services/web-server/src/web/rest/case_editor_rest.rs`

- [ ] **Step 1: Write failing backend list projection test**

Add this test near existing editor repeatable list tests:

```rust
#[serial]
#[tokio::test]
async fn editor_repeatable_pages_have_list_projection_routes() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-REPEATABLE-PAGES").await?;

	for (section, expected_key) in [
		("DH", "rows"),
		("AE", "rows"),
		("LB", "rows"),
		("DG", "rows"),
	] {
		let (status, body) = get_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/editor/pages/{section}?appendix=fda"),
		)
		.await?;

		assert_eq!(status, StatusCode::OK, "{section}: {body}");
		assert_eq!(body["caseId"], case_id);
		assert_eq!(body["pageId"], section);
		assert_eq!(body["focusedAppendix"], "fda");
		assert!(body.get("appendices").is_none(), "{section}: {body}");
		assert!(body["rows"][expected_key].is_array(), "{section}: {body}");
	}

	Ok(())
}
```

- [ ] **Step 2: Run RED**

Run:

```bash
cargo test -p web-server editor_repeatable_pages_have_list_projection_routes --test api -- --nocapture
```

Expected: fail with `404` or route mismatch for `DH`.

- [ ] **Step 3: Add routes**

In `crates/services/web-server/src/web/rest/mod.rs`, add these before the legacy repeatable routes:

```rust
.route(
	"/cases/{case_id}/editor/pages/DH",
	get(case_editor_rest::get_editor_dh_page_projection),
)
.route(
	"/cases/{case_id}/editor/pages/AE",
	get(case_editor_rest::get_editor_ae_page_projection),
)
.route(
	"/cases/{case_id}/editor/pages/LB",
	get(case_editor_rest::get_editor_lb_page_projection),
)
.route(
	"/cases/{case_id}/editor/pages/DG",
	get(case_editor_rest::get_editor_dg_page_projection),
)
```

In `case_editor_rest.rs`, add handlers that call the existing list loaders and wrap them in `CaseEditorPageProjectionResponse`:

```rust
async fn repeatable_page_projection_response(
	case_id: Uuid,
	page_id: &'static str,
	focused_appendix: Option<String>,
	rows: Value,
) -> Result<CaseEditorPageProjectionResponse> {
	Ok(CaseEditorPageProjectionResponse {
		case_id,
		page_id,
		focused_appendix: normalize_appendix(focused_appendix)?,
		saved: rows
			.get("rows")
			.and_then(Value::as_array)
			.map(|items| !items.is_empty())
			.unwrap_or(false),
		required_count: 0,
		fields: BTreeMap::new(),
		rows: rows_from_direct_section(rows),
		section_summaries: Vec::new(),
	})
}
```

For each section handler, require the same permissions as the current `list_editor_*` handler, call the same list function or extracted helper, and return:

```rust
Ok((axum::http::StatusCode::OK, Json(projection)))
```

- [ ] **Step 4: Run GREEN**

Run:

```bash
cargo test -p web-server editor_repeatable_pages_have_list_projection_routes --test api -- --nocapture
```

Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add crates/services/web-server/src/web/rest/mod.rs crates/services/web-server/src/web/rest/case_editor_rest.rs crates/services/web-server/tests/api/case_editor_contract_web.rs
git commit -m "Add repeatable editor page list projections"
```

## Task 2: Backend Repeatable Page Row Detail Routes

**Files:**

- Modify `crates/services/web-server/tests/api/case_editor_contract_web.rs`
- Modify `crates/services/web-server/src/web/rest/mod.rs`
- Modify `crates/services/web-server/src/web/rest/case_editor_rest.rs`

- [ ] **Step 1: Write failing row detail route test**

Use one existing row fixture per section. Add:

```rust
#[serial]
#[tokio::test]
async fn editor_repeatable_page_rows_return_row_detail_by_uuid() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-REPEATABLE-ROWS").await?;

	let reaction_id = create_reaction_fixture(&app, &cookie, &case_id).await?;
	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/AE/rows/{reaction_id}?appendix=fda"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	assert_eq!(body["section"], "AE");
	assert_eq!(body["rowId"], reaction_id);
	assert!(body.get("appendices").is_none(), "{body}");

	Ok(())
}
```

Before adding the test, create this helper in the same test file by moving the reaction creation code from `editor_ae_detail_returns_one_reaction_by_uuid` into a reusable function:

```rust
async fn create_reaction_fixture(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<String> {
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/reactions"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"reaction_primary_source_native": "Initial reaction"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	Ok(body["data"]["id"]
		.as_str()
		.ok_or("missing reaction id")?
		.to_string())
}
```

- [ ] **Step 2: Run RED**

Run:

```bash
cargo test -p web-server editor_repeatable_page_rows_return_row_detail_by_uuid --test api -- --nocapture
```

Expected: fail with missing route.

- [ ] **Step 3: Add row routes**

In `mod.rs`, add:

```rust
.route(
	"/cases/{case_id}/editor/pages/DH/rows/{row_id}",
	get(case_editor_rest::get_editor_dh_page_row),
)
.route(
	"/cases/{case_id}/editor/pages/AE/rows/{row_id}",
	get(case_editor_rest::get_editor_ae_page_row),
)
.route(
	"/cases/{case_id}/editor/pages/LB/rows/{row_id}",
	get(case_editor_rest::get_editor_lb_page_row),
)
.route(
	"/cases/{case_id}/editor/pages/DG/rows/{row_id}",
	get(case_editor_rest::get_editor_dg_page_row),
)
```

In `case_editor_rest.rs`, extract the existing row detail body from `get_editor_ae`, `get_editor_dh`, `get_editor_lb`, and `get_editor_dg` into helpers with this shape:

```rust
async fn build_editor_ae_row_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
) -> Result<CaseEditorRowDetailResponse> {
	let reaction = ReactionBmc::get(ctx, mm, row_id).await?;
	Ok(CaseEditorRowDetailResponse {
		case_id,
		section: "AE".to_string(),
		row_id: row_id.to_string(),
		data: json!({ "reaction": reaction }),
	})
}
```

Use equivalent helpers named `build_editor_dh_row_detail`, `build_editor_lb_row_detail`, and `build_editor_dg_row_detail`. Both legacy row handlers and new page row handlers must call these helpers, so numeric row id rejection remains shared.

- [ ] **Step 4: Run GREEN**

Run:

```bash
cargo test -p web-server editor_repeatable_page_rows_return_row_detail_by_uuid --test api -- --nocapture
cargo test -p web-server editor_row_detail_rejects_numeric_row_position_as_identifier --test api -- --nocapture
```

Expected: both pass.

- [ ] **Step 5: Commit**

```bash
git add crates/services/web-server/src/web/rest/mod.rs crates/services/web-server/src/web/rest/case_editor_rest.rs crates/services/web-server/tests/api/case_editor_contract_web.rs
git commit -m "Add repeatable editor page row detail routes"
```

## Task 3: Backend Repeatable Page Row Save Routes

**Files:**

- Modify `crates/services/web-server/tests/api/case_editor_contract_web.rs`
- Modify `crates/services/web-server/src/web/rest/mod.rs`
- Modify `crates/services/web-server/src/web/rest/case_editor_rest.rs`

- [ ] **Step 1: Write failing row save test**

Start with `AE`, because reaction row creation/update is a representative repeatable row:

```rust
#[serial]
#[tokio::test]
async fn editor_repeatable_page_row_patch_updates_one_reaction() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-AE-ROW-PATCH").await?;
	let reaction_id = create_reaction_fixture(&app, &cookie, &case_id).await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/AE/rows/{reaction_id}"),
		json!({
			"appendix": "fda",
			"rows": {
				"reaction": {
					"reactionPrimarySourceNative": "Updated reaction"
				}
			}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["section"], "AE");
	assert_eq!(body["rowId"], reaction_id);
	assert_eq!(
		body["data"]["reaction"]["reaction_primary_source_native"],
		"Updated reaction"
	);

	Ok(())
}
```

- [ ] **Step 2: Run RED**

Run:

```bash
cargo test -p web-server editor_repeatable_page_row_patch_updates_one_reaction --test api -- --nocapture
```

Expected: fail with missing route or method not allowed.

- [ ] **Step 3: Add create/patch/delete routes**

In `mod.rs`, extend each repeatable page row namespace:

```rust
.route(
	"/cases/{case_id}/editor/pages/AE/rows",
	post(case_editor_rest::create_editor_ae_page_row),
)
.route(
	"/cases/{case_id}/editor/pages/AE/rows/{row_id}",
	get(case_editor_rest::get_editor_ae_page_row)
		.patch(case_editor_rest::patch_editor_ae_page_row)
		.delete(case_editor_rest::delete_editor_ae_page_row),
)
```

Add the remaining repeatable row routes explicitly:

```rust
.route(
	"/cases/{case_id}/editor/pages/DH/rows",
	post(case_editor_rest::create_editor_dh_page_row),
)
.route(
	"/cases/{case_id}/editor/pages/DH/rows/{row_id}",
	get(case_editor_rest::get_editor_dh_page_row)
		.patch(case_editor_rest::patch_editor_dh_page_row)
		.delete(case_editor_rest::delete_editor_dh_page_row),
)
.route(
	"/cases/{case_id}/editor/pages/LB/rows",
	post(case_editor_rest::create_editor_lb_page_row),
)
.route(
	"/cases/{case_id}/editor/pages/LB/rows/{row_id}",
	get(case_editor_rest::get_editor_lb_page_row)
		.patch(case_editor_rest::patch_editor_lb_page_row)
		.delete(case_editor_rest::delete_editor_lb_page_row),
)
.route(
	"/cases/{case_id}/editor/pages/DG/rows",
	post(case_editor_rest::create_editor_dg_page_row),
)
.route(
	"/cases/{case_id}/editor/pages/DG/rows/{row_id}",
	get(case_editor_rest::get_editor_dg_page_row)
		.patch(case_editor_rest::patch_editor_dg_page_row)
		.delete(case_editor_rest::delete_editor_dg_page_row),
)
```

The request body should reuse `CaseEditorPagePatchRequest`:

```rust
Json(request): Json<CaseEditorPagePatchRequest>
```

Handlers should:

- require write permissions matching existing create/update/delete endpoints.
- require case write allowed.
- map `rows` into the existing row create/update models.
- mark validation summaries stale for the case.
- return `CaseEditorRowDetailResponse` for create and patch.
- return `StatusCode::NO_CONTENT` for delete.

- [ ] **Step 4: Add one persistence test per repeatable section**

Add tests with names:

```rust
editor_dh_page_row_patch_updates_one_drug_history
editor_ae_page_row_patch_updates_one_reaction
editor_lb_page_row_patch_updates_one_test_result
editor_dg_page_row_patch_updates_one_drug
```

Use these scalar assertions for the remaining section tests:

```rust
assert_eq!(body["data"]["pastDrugHistory"]["drug_name"], "Updated prior drug");
assert_eq!(body["data"]["testResult"]["test_name"], "Updated lab");
assert_eq!(body["data"]["drug"]["medicinal_product"], "Updated product");
```

The helper setup should mirror existing row detail tests in this file: create the required parent entity, create exactly one row, patch through `/editor/pages/{section}/rows/{row_id}`, and assert the returned detail contains the updated scalar above.

- [ ] **Step 5: Run GREEN**

Run:

```bash
cargo test -p web-server page_row_patch_updates_one --test api -- --nocapture
```

Expected: all four tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/services/web-server/src/web/rest/mod.rs crates/services/web-server/src/web/rest/case_editor_rest.rs crates/services/web-server/tests/api/case_editor_contract_web.rs
git commit -m "Add repeatable editor page row save routes"
```

## Task 4: Frontend API Client For Repeatable Page Namespace

**Files:**

- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/api/endpoints/cases/core/editor.ts`
- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/api/case-editor-api.test.ts`

- [ ] **Step 1: Write failing API tests**

Add:

```ts
it("uses page namespace for repeatable editor list and row routes", async () => {
  await api.cases.getEditorRepeatablePageProjection("case-123", "AE", "fda");
  await api.cases.getEditorPageRow("case-123", "DG", "row/with spaces", "mfds");

  expect(mockedGet).toHaveBeenCalledWith(
    "/api/cases/case-123/editor/pages/AE",
    { appendix: "fda" }
  );
  expect(mockedGet).toHaveBeenCalledWith(
    "/api/cases/case-123/editor/pages/DG/rows/row%2Fwith%20spaces",
    { appendix: "mfds" }
  );
});

it("uses page namespace for repeatable editor row mutations", async () => {
  await api.cases.createEditorPageRow("case-123", "AE", {
    appendix: "fda",
    rows: { reaction: { reactionPrimarySourceNative: "Created" } },
  });
  await api.cases.patchEditorPageRow("case-123", "AE", "row-1", {
    appendix: "fda",
    rows: { reaction: { reactionPrimarySourceNative: "Updated" } },
  });
  await api.cases.deleteEditorPageRow("case-123", "AE", "row-1", "fda");

  expect(mockedPost).toHaveBeenCalledWith(
    "/api/cases/case-123/editor/pages/AE/rows",
    {
      appendix: "fda",
      rows: { reaction: { reactionPrimarySourceNative: "Created" } },
    }
  );
  expect(mockedPatch).toHaveBeenCalledWith(
    "/api/cases/case-123/editor/pages/AE/rows/row-1",
    {
      appendix: "fda",
      rows: { reaction: { reactionPrimarySourceNative: "Updated" } },
    }
  );
  expect(mockedDelete).toHaveBeenCalledWith(
    "/api/cases/case-123/editor/pages/AE/rows/row-1",
    { appendix: "fda" }
  );
});
```

Update the mock to include `post` and `delete` if this test file does not already mock them.

- [ ] **Step 2: Run RED**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npm test -- --runTestsByPath __tests__/api/case-editor-api.test.ts --runInBand
```

Expected: fail because new API methods do not exist.

- [ ] **Step 3: Implement API methods**

Add to `editorCasesCoreAPI`:

```ts
getEditorRepeatablePageProjection: async (
  caseId: string,
  section: RepeatableEditorSectionCode,
  appendix?: string
) => {
  const normalizedSection = normalizeEditorSectionCode(section);
  return apiClient.get<CaseEditorPageProjection>(
    `/api/cases/${caseId}/editor/pages/${normalizedSection}`,
    appendix ? { appendix } : undefined
  );
},

getEditorPageRow: async (
  caseId: string,
  section: RepeatableEditorSectionCode,
  rowId: string,
  appendix?: string
) => {
  const normalizedSection = normalizeEditorSectionCode(section);
  return apiClient.get(
    `/api/cases/${caseId}/editor/pages/${normalizedSection}/rows/${encodeURIComponent(rowId)}`,
    appendix ? { appendix } : undefined
  );
},

createEditorPageRow: async (
  caseId: string,
  section: RepeatableEditorSectionCode,
  request: CaseEditorPagePatchRequest
) => {
  const normalizedSection = normalizeEditorSectionCode(section);
  return apiClient.post(
    `/api/cases/${caseId}/editor/pages/${normalizedSection}/rows`,
    request
  );
},

patchEditorPageRow: async (
  caseId: string,
  section: RepeatableEditorSectionCode,
  rowId: string,
  request: CaseEditorPagePatchRequest
) => {
  const normalizedSection = normalizeEditorSectionCode(section);
  return apiClient.patch(
    `/api/cases/${caseId}/editor/pages/${normalizedSection}/rows/${encodeURIComponent(rowId)}`,
    request
  );
},

deleteEditorPageRow: async (
  caseId: string,
  section: RepeatableEditorSectionCode,
  rowId: string,
  appendix?: string
) => {
  const normalizedSection = normalizeEditorSectionCode(section);
  return apiClient.delete(
    `/api/cases/${caseId}/editor/pages/${normalizedSection}/rows/${encodeURIComponent(rowId)}`,
    appendix ? { appendix } : undefined
  );
},
```

- [ ] **Step 4: Run GREEN**

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
git commit -m "Add repeatable editor page row API client"
```

## Task 5: Frontend Route Loading Uses Repeatable Page Namespace

**Files:**

- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/case-form/case-editor-route-loading.test.tsx`
- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/case-form/CaseFormWizardNew.tsx`
- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/case-editor/route-loading.ts`

- [ ] **Step 1: Write failing route loading tests**

Update the repeatable route loading test to expect:

```ts
expect(mockedApi.cases.getEditorRepeatablePageProjection).toHaveBeenCalledWith(
  "case-1",
  "AE",
  "fda"
);
expect(mockedApi.cases.getEditorSectionList).not.toHaveBeenCalled();
```

Update the row route loading test to expect:

```ts
expect(mockedApi.cases.getEditorPageRow).toHaveBeenCalledWith(
  "case-1",
  "DG",
  "row-1",
  "fda"
);
expect(mockedApi.cases.getEditorSectionRow).not.toHaveBeenCalled();
```

- [ ] **Step 2: Run RED**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npm test -- --runTestsByPath __tests__/case-form/case-editor-route-loading.test.tsx --runInBand
```

Expected: fail because route loading still uses old repeatable methods.

- [ ] **Step 3: Update route loading**

Where repeatable list routes call:

```ts
api.cases.getEditorSectionList(caseId, sectionCode)
```

replace with:

```ts
api.cases.getEditorRepeatablePageProjection(caseId, sectionCode, activeAppendix)
```

Where repeatable row routes call:

```ts
api.cases.getEditorSectionRow(caseId, sectionCode, rowId)
```

replace with:

```ts
api.cases.getEditorPageRow(caseId, sectionCode, rowId, activeAppendix)
```

Ensure `unwrapEditorPayload` keeps accepting page projection `rows` and row detail `data`.

- [ ] **Step 4: Run GREEN**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npm test -- --runTestsByPath __tests__/case-form/case-editor-route-loading.test.tsx --runInBand
```

Expected: pass.

- [ ] **Step 5: Commit**

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
git add components/case-form/CaseFormWizardNew.tsx lib/case-editor/route-loading.ts __tests__/case-form/case-editor-route-loading.test.tsx
git commit -m "Load repeatable editor routes through page namespace"
```

## Task 6: Frontend Repeatable Save Refresh Uses Page Row Namespace

**Files:**

- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts`
- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/components/case-form/CaseFormWizardNew.tsx`

- [ ] **Step 1: Write failing refresh test**

Change the current section-scoped repeatable refresh expectation from old row endpoint to new page row endpoint:

```ts
expect(getEditorPageRow).toHaveBeenCalledWith("case-1", "AE", "rx-1", "fda");
expect(getEditorSectionRow).not.toHaveBeenCalled();
```

For repeatable list mode, add:

```ts
expect(getEditorRepeatablePageProjection).toHaveBeenCalledWith(
  "case-1",
  "AE",
  "fda"
);
expect(getEditorSectionList).not.toHaveBeenCalled();
```

- [ ] **Step 2: Run RED**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npm test -- --runTestsByPath __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts --runInBand
```

Expected: fail because `refreshCaseAfterSave` still calls old repeatable methods.

- [ ] **Step 3: Update refresh helper**

In `refreshCaseAfterSave`, replace repeatable row/list calls:

```ts
api.cases.getEditorSectionRow(caseId, "AE", "rx-1")
api.cases.getEditorSectionList(caseId, "AE")
```

with:

```ts
api.cases.getEditorPageRow(
  caseId,
  sectionScopedEditor.sectionCode as RepeatableEditorSectionCode,
  sectionScopedEditor.rowId,
  activeAppendix || sectionScopedEditor.activeAppendix,
)

api.cases.getEditorRepeatablePageProjection(
  caseId,
  sectionScopedEditor.sectionCode as RepeatableEditorSectionCode,
  activeAppendix || sectionScopedEditor.activeAppendix,
)
```

- [ ] **Step 4: Run GREEN**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npm test -- --runTestsByPath __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts --runInBand
```

Expected: pass.

- [ ] **Step 5: Commit**

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
git add components/case-form/CaseFormWizardNew.tsx __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts
git commit -m "Refresh repeatable editor saves through page namespace"
```

## Task 7: Phase 3 Frontend Legacy Editor Route Cleanup

**Files:**

- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/__tests__/api/case-editor-api.test.ts`
- Modify `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/api/endpoints/cases/core/editor.ts`

- [ ] **Step 1: Add legacy usage guard test**

Add:

```ts
it("does not expose legacy repeatable editor route helpers for editor loading", () => {
  expect("getEditorSectionList" in api.cases).toBe(false);
  expect("getEditorSectionRow" in api.cases).toBe(false);
});
```

- [ ] **Step 2: Run RED**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npm test -- --runTestsByPath __tests__/api/case-editor-api.test.ts --runInBand
```

Expected: fail because the legacy helpers still exist.

- [ ] **Step 3: Remove frontend legacy helpers**

Remove these methods from `editorCasesCoreAPI`:

```ts
getEditorSectionList
getEditorSectionRow
```

Update all frontend compile errors by replacing call sites with the page namespace methods from Task 4.

- [ ] **Step 4: Run GREEN**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npm test -- --runTestsByPath __tests__/api/case-editor-api.test.ts __tests__/case-form/case-editor-route-loading.test.tsx __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts --runInBand
./node_modules/.bin/tsc -p tsconfig.json --noEmit --incremental false
```

Expected: pass.

- [ ] **Step 5: Commit**

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
git add lib/api/endpoints/cases/core/editor.ts components/case-form/CaseFormWizardNew.tsx lib/case-editor/route-loading.ts __tests__/api/case-editor-api.test.ts __tests__/case-form/case-editor-route-loading.test.tsx __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts
git commit -m "Remove frontend legacy repeatable editor route helpers"
```

## Task 8: OpenAPI Documentation

**Files:**

- Modify `crates/services/web-server/src/openapi.rs`

- [ ] **Step 1: Add OpenAPI declarations**

Add path declarations for:

```http
GET    /api/cases/{case_id}/editor/pages/DH
GET    /api/cases/{case_id}/editor/pages/AE
GET    /api/cases/{case_id}/editor/pages/LB
GET    /api/cases/{case_id}/editor/pages/DG
GET    /api/cases/{case_id}/editor/pages/DH/rows/{row_id}
GET    /api/cases/{case_id}/editor/pages/AE/rows/{row_id}
GET    /api/cases/{case_id}/editor/pages/LB/rows/{row_id}
GET    /api/cases/{case_id}/editor/pages/DG/rows/{row_id}
POST   /api/cases/{case_id}/editor/pages/{section}/rows
PATCH  /api/cases/{case_id}/editor/pages/{section}/rows/{row_id}
DELETE /api/cases/{case_id}/editor/pages/{section}/rows/{row_id}
```

Use `CaseEditorPageProjectionResponseDoc`, `CaseEditorRowDetailResponseDoc`, and `CaseEditorPagePatchRequestDoc` where applicable.

- [ ] **Step 2: Run compile verification**

Run:

```bash
cargo check -p web-server
```

Expected: pass.

- [ ] **Step 3: Commit**

```bash
git add crates/services/web-server/src/openapi.rs
git commit -m "Document repeatable editor page row endpoints"
```

## Task 9: Full Verification

**Files:**

- No planned source changes.

- [ ] **Step 1: Backend verification**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/e2br3
cargo check -p web-server
cargo test -p web-server editor_ --test api -- --nocapture
cargo test -p web-server explicit_profiles --test api -- --nocapture
rg -n "appendices_json|appendicesJson" crates db --glob '!target'
git diff --check
```

Expected:

- `cargo check` passes.
- editor API tests pass.
- explicit profile tests pass.
- appendix metadata grep prints no runtime matches.
- diff check prints no whitespace errors.

- [ ] **Step 2: Frontend verification**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
./node_modules/.bin/tsc -p tsconfig.json --noEmit --incremental false
npm test -- --runTestsByPath __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts __tests__/case-form/case-editor-route-loading.test.tsx __tests__/api/case-editor-api.test.ts __tests__/api.endpoints.test.ts --runInBand
rg -n "appendices_json|appendicesJson" app components lib __tests__
rg -n "getEditorSectionList|getEditorSectionRow|/editor/\\$\\{normalizedSection\\}/list|/editor/\\$\\{normalizedSection\\}/\\$\\{encodeURIComponent\\(rowId\\)\\}" app components lib __tests__
git diff --check
```

Expected:

- TypeScript passes.
- targeted Jest suites pass.
- appendix metadata grep prints no runtime matches.
- legacy repeatable editor helper grep prints no frontend runtime matches.
- diff check prints no whitespace errors.

- [ ] **Step 3: Commit verification fixes**

If verification requires backend fixes:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/e2br3
git add crates/services/web-server/src/web/rest/mod.rs crates/services/web-server/src/web/rest/case_editor_rest.rs crates/services/web-server/src/openapi.rs crates/services/web-server/tests/api/case_editor_contract_web.rs
git commit -m "Verify repeatable editor page row transport"
```

If verification requires frontend fixes:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
git add lib/api/endpoints/cases/core/editor.ts lib/case-editor/route-loading.ts components/case-form/CaseFormWizardNew.tsx __tests__/api/case-editor-api.test.ts __tests__/case-form/case-editor-route-loading.test.tsx __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts
git commit -m "Verify repeatable editor page row transport"
```
