# Workflow Save/Delete Stabilization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stabilize case save/delete behavior so manual and imported cases share the same lifecycle semantics, normal save responses avoid import/batch noise, and delete becomes an auditable soft-delete lifecycle action.

**Architecture:** Keep the existing REST and `CaseBmc` model boundaries. Add focused API contract tests first, then route `DELETE /api/cases/{id}` through a status update to `deleted` instead of hard-deleting the row. Preserve existing `PUT /api/cases/{id}` update behavior unless a failing test proves a save parity gap.

**Tech Stack:** Rust, Axum, SQLx/PostgreSQL, existing `web-server` API test harness, `serial_test`, `serde_json`.

---

## Spec Reference

- Design spec: `docs/superpowers/specs/2026-04-28-workflow-save-delete-design.md`
- Primary handler: `crates/services/web-server/src/web/rest/case_rest.rs`
- Primary tests: `crates/services/web-server/tests/api/case_contract_web.rs`
- Import helper reference: `crates/services/web-server/tests/api/import_history_web.rs`
- Core model: `crates/libs/lib-core/src/model/case.rs`

## Current Findings

- `PUT /api/cases/{id}` already maps public updates through `to_internal_case_for_update`, validates status/profile/appendices, blocks non-status edits in locked/workflow-readonly states, and writes via `CaseBmc::update`.
- Imported cases are created by XML import with `raw_xml` and dirty flags, but later case-level saves use the same `PUT /api/cases/{id}` REST endpoint as manual cases.
- `DELETE /api/cases/{id}` currently calls `CaseBmc::delete`, which hard-deletes the row through the base model.
- `deleted` is already a valid lifecycle status and permitted transition for draft/reviewed/validated/locked/submitted cases.
- Compliance context can be attached to audit triggers through `ctx.with_compliance(Some(reason), Some(signature_id))` or `ctx.with_compliance(Some(reason), None)`.

## File Structure

- Modify `crates/services/web-server/tests/api/case_contract_web.rs`: add API helpers and regression tests for manual save, imported-case-shaped save, soft delete, delete reason, and list/lifecycle visibility.
- Modify `crates/services/web-server/src/web/rest/case_rest.rs`: add a delete request payload, validate deletion reason, and change `delete_case` from hard-delete to status update.
- No planned schema changes.
- No planned frontend changes in this slice.

---

### Task 1: Add Failing API Contract Tests For Save/Delete Lifecycle

**Files:**
- Modify: `crates/services/web-server/tests/api/case_contract_web.rs`

- [ ] **Step 1: Add missing imports and DELETE helper**

Add `Method` to the existing `axum::http` import and add this helper below `put_json`:

```rust
async fn delete_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method(Method::DELETE)
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice::<Value>(&body)?))
}
```

- [ ] **Step 2: Add test for manual case save continuing to use normal case errors only**

Append this test to `case_contract_web.rs`:

```rust
#[serial]
#[tokio::test]
async fn test_manual_case_save_updates_public_fields_without_import_noise() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let report_id = format!("SR-{}", Uuid::new_v4());
	let (create_status, create_body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safety_report_id": report_id,
				"status": "draft",
				"validation_profile": "fda"
			}
		}),
	)
	.await?;
	assert_eq!(create_status, StatusCode::CREATED, "{create_body:?}");
	let case_id = create_body["data"]["id"]
		.as_str()
		.ok_or("missing created case id")?
		.to_string();

	let (update_status, update_body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
				"report_year": "2026",
				"mfds_report_type": "spontaneous"
			}
		}),
	)
	.await?;
	assert_eq!(update_status, StatusCode::OK, "{update_body:?}");
	assert_eq!(update_body["data"]["report_year"].as_str(), Some("2026"));
	assert_eq!(
		update_body["data"]["mfds_report_type"].as_str(),
		Some("spontaneous")
	);
	let rendered = update_body.to_string().to_ascii_lowercase();
	assert!(!rendered.contains("batch"), "{update_body:?}");
	assert!(!rendered.contains("header"), "{update_body:?}");
	assert!(!rendered.contains("import"), "{update_body:?}");

	Ok(())
}
```

- [ ] **Step 3: Add test for imported-case-shaped save using the same update path**

Append this test to `case_contract_web.rs`:

```rust
#[serial]
#[tokio::test]
async fn test_imported_case_save_updates_public_fields_without_import_noise() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = Uuid::new_v4();
	let mut tx = mm.dbx().db().begin().await?;
	lib_core::model::store::set_user_context(&mut tx, seed.admin.id).await?;
	lib_core::model::store::set_org_context(
		&mut tx,
		seed.org_id,
		lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	sqlx::query(
		"INSERT INTO cases (
			id,
			organization_id,
			safety_report_id,
			version,
			status,
			validation_profile,
			raw_xml,
			dirty_c,
			dirty_d,
			dirty_e,
			dirty_f,
			dirty_g,
			dirty_h,
			created_by
		) VALUES ($1, $2, $3, 1, 'draft', 'fda', $4, false, false, false, false, false, false, $5)",
	)
	.bind(case_id)
	.bind(seed.org_id)
	.bind(format!("SR-IMPORT-SAVE-{}", Uuid::new_v4()))
	.bind(b"<imported/>".to_vec())
	.bind(seed.admin.id)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;

	let (update_status, update_body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
				"report_year": "2026",
				"source_document_name": "imported-followup.pdf"
			}
		}),
	)
	.await?;
	assert_eq!(update_status, StatusCode::OK, "{update_body:?}");
	assert_eq!(update_body["data"]["report_year"].as_str(), Some("2026"));
	assert_eq!(
		update_body["data"]["source_document_name"].as_str(),
		Some("imported-followup.pdf")
	);
	let rendered = update_body.to_string().to_ascii_lowercase();
	assert!(!rendered.contains("batch"), "{update_body:?}");
	assert!(!rendered.contains("header"), "{update_body:?}");

	Ok(())
}
```

- [ ] **Step 4: Add tests for delete reason and soft-delete visibility**

Append these tests to `case_contract_web.rs`:

```rust
#[serial]
#[tokio::test]
async fn test_delete_case_requires_reason_for_change() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (create_status, create_body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safety_report_id": format!("SR-{}", Uuid::new_v4()),
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(create_status, StatusCode::CREATED, "{create_body:?}");
	let case_id = create_body["data"]["id"]
		.as_str()
		.ok_or("missing created case id")?
		.to_string();

	let (delete_status, delete_body) =
		delete_json(&app, &cookie, &format!("/api/cases/{case_id}"), json!({}))
			.await?;
	assert_eq!(delete_status, StatusCode::BAD_REQUEST, "{delete_body:?}");
	assert!(
		delete_body.to_string().contains("reason_for_change is required"),
		"{delete_body:?}"
	);

	let (get_status, get_body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}")).await?;
	assert_eq!(get_status, StatusCode::OK, "{get_body:?}");
	assert_eq!(get_body["data"]["status"].as_str(), Some("draft"));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_delete_case_soft_deletes_and_keeps_case_visible() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let report_id = format!("SR-{}", Uuid::new_v4());
	let (create_status, create_body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safety_report_id": report_id,
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(create_status, StatusCode::CREATED, "{create_body:?}");
	let case_id = create_body["data"]["id"]
		.as_str()
		.ok_or("missing created case id")?
		.to_string();

	let (delete_status, delete_body) = delete_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({ "reason_for_change": "client requested soft delete" }),
	)
	.await?;
	assert_eq!(delete_status, StatusCode::OK, "{delete_body:?}");
	assert_eq!(delete_body["data"]["status"].as_str(), Some("deleted"));

	let (get_status, get_body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}")).await?;
	assert_eq!(get_status, StatusCode::OK, "{get_body:?}");
	assert_eq!(get_body["data"]["status"].as_str(), Some("deleted"));

	let (list_status, list_body) = get_json(&app, &cookie, "/api/cases").await?;
	assert_eq!(list_status, StatusCode::OK, "{list_body:?}");
	let list_contains_deleted = list_body["data"]
		.as_array()
		.ok_or("case list should be an array")?
		.iter()
		.any(|item| {
			item["case"]["id"].as_str() == Some(case_id.as_str())
				|| item["id"].as_str() == Some(case_id.as_str())
		});
	assert!(list_contains_deleted, "{list_body:?}");

	let (lifecycle_status, lifecycle_body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/lifecycle")).await?;
	assert_eq!(lifecycle_status, StatusCode::OK, "{lifecycle_body:?}");
	let lifecycle_contains_deleted = lifecycle_body["data"]["items"]
		.as_array()
		.ok_or("lifecycle items should be an array")?
		.iter()
		.any(|item| {
			item["case_id"].as_str() == Some(case_id.as_str())
				&& item["status"].as_str() == Some("deleted")
		});
	assert!(lifecycle_contains_deleted, "{lifecycle_body:?}");

	Ok(())
}
```

- [ ] **Step 5: Verify RED**

Run:

```bash
cargo test -p web-server case_contract_web --test api -- --nocapture --test-threads=1
```

Expected: the save tests pass or fail only for existing behavior details, and `test_delete_case_soft_deletes_and_keeps_case_visible` fails because `DELETE /api/cases/{id}` returns `204 No Content` and hard-deletes the row. If compile fails because `Method` is missing, fix the import and rerun until the delete test fails for the lifecycle behavior.

- [ ] **Step 6: Commit failing tests only**

```bash
git add crates/services/web-server/tests/api/case_contract_web.rs
git commit -m "test: cover case save delete lifecycle contracts"
```

---

### Task 2: Implement Soft Delete Lifecycle Handler

**Files:**
- Modify: `crates/services/web-server/src/web/rest/case_rest.rs`

- [ ] **Step 1: Add delete request/response support types**

Below `PublicCaseUpdateRequest`, add:

```rust
#[derive(Debug, Deserialize)]
pub struct PublicCaseDeleteRequest {
	pub reason_for_change: Option<String>,
}
```

- [ ] **Step 2: Add a helper for non-empty compliance reasons**

Below `case_status_update`, add:

```rust
fn required_reason_for_change(
	reason_for_change: Option<String>,
	action: &str,
) -> Result<String> {
	reason_for_change
		.and_then(|v| {
			let trimmed = v.trim().to_string();
			if trimmed.is_empty() { None } else { Some(trimmed) }
		})
		.ok_or_else(|| Error::BadRequest {
			message: format!("reason_for_change is required for {action}"),
		})
}
```

- [ ] **Step 3: Reuse the helper in `update_case_guarded`**

Replace the inline `reason_for_change` trimming block inside `if requires_compliance` with:

```rust
let reason = required_reason_for_change(
	reason_for_change,
	"submitted/nullified/deleted status transitions",
)?;
```

Keep the existing e-signature requirement for `PUT` status transitions.

- [ ] **Step 4: Change `delete_case` to accept JSON and soft-delete**

Replace the existing `delete_case` function with:

```rust
/// DELETE /api/cases/{id}
pub async fn delete_case(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<PublicCaseDeleteRequest>,
) -> Result<(axum::http::StatusCode, Json<DataRestResult<Case>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_DELETE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, id).await?;
	let current = CaseBmc::get(&ctx, &mm, id).await?;
	if !is_allowed_case_status_transition(&current.status, "deleted") {
		return Err(Error::BadRequest {
			message: format!(
				"illegal case status transition: '{}' -> 'deleted'",
				current.status
			),
		});
	}

	let reason =
		required_reason_for_change(params.reason_for_change, "case delete")?;
	let ctx_for_delete = ctx.with_compliance(Some(reason), None);
	CaseBmc::update(
		&ctx_for_delete,
		&mm,
		id,
		case_status_update("deleted".to_string()),
	)
	.await?;
	let entity = CaseBmc::get(&ctx, &mm, id).await?;

	Ok((axum::http::StatusCode::OK, Json(DataRestResult { data: entity })))
}
```

- [ ] **Step 5: Verify GREEN for case contract tests**

Run:

```bash
cargo test -p web-server case_contract_web --test api -- --nocapture --test-threads=1
```

Expected: all `case_contract_web` tests pass.

- [ ] **Step 6: Commit implementation**

```bash
git add crates/services/web-server/src/web/rest/case_rest.rs
git commit -m "fix: soft delete cases through lifecycle status"
```

---

### Task 3: Add Audit Regression For Delete Reason

**Files:**
- Modify: `crates/services/web-server/tests/api/case_contract_web.rs`

- [ ] **Step 1: Add audit assertion to soft-delete test**

Inside `test_delete_case_soft_deletes_and_keeps_case_visible`, initialize `mm` as cloneable:

```rust
let mm = init_test_mm().await?;
```

Keep `web_server::app(mm.clone())` so the test can query audit state after API calls.

After the lifecycle assertions, add:

```rust
let dbx = mm.dbx();
dbx.begin_txn().await?;
dbx.execute(sqlx::query("SET ROLE e2br3_auditor_role")).await?;
let audit_reason = dbx
	.fetch_optional(
		sqlx::query_as::<_, (Option<String>,)>(
			r#"
			SELECT reason_for_change
			FROM audit_logs
			WHERE table_name = 'cases'
			  AND record_id = $1
			  AND action = 'UPDATE'
			  AND changed_fields ? 'status'
			  AND changed_fields->'status'->>'new' = 'deleted'
			ORDER BY id DESC
			LIMIT 1
			"#,
		)
		.bind(Uuid::parse_str(&case_id)?),
	)
	.await?;
dbx.rollback_txn().await?;
assert_eq!(
	audit_reason.and_then(|(v,)| v).as_deref(),
	Some("client requested soft delete")
);
```

- [ ] **Step 2: Verify RED or GREEN**

Run:

```bash
cargo test -p web-server test_delete_case_soft_deletes_and_keeps_case_visible --test api -- --nocapture --test-threads=1
```

Expected: pass if the audit trigger accepts compliance context with no e-signature; fail with an audit-context error if the trigger path needs a small adjustment.

- [ ] **Step 3: Fix only if the audit assertion fails**

If the audit reason is missing because delete used the wrong context, update only `delete_case` so `CaseBmc::update` receives `ctx_for_delete`. Do not change audit schema or triggers in this task.

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p web-server test_delete_case_soft_deletes_and_keeps_case_visible --test api -- --nocapture --test-threads=1
```

Expected: the test passes and the audit reason equals `client requested soft delete`.

- [ ] **Step 5: Commit audit regression**

```bash
git add crates/services/web-server/tests/api/case_contract_web.rs crates/services/web-server/src/web/rest/case_rest.rs
git commit -m "test: assert case delete audit reason"
```

---

### Task 4: Run Broader Save/Delete Regression

**Files:**
- Modify only if tests expose a real regression in the save/delete lifecycle.

- [ ] **Step 1: Run focused API suites**

Run:

```bash
cargo test -p web-server case_contract_web --test api -- --nocapture --test-threads=1
cargo test -p web-server import_contract_web --test api -- --nocapture --test-threads=1
cargo test -p web-server import_history_web --test api -- --nocapture --test-threads=1
cargo test -p web-server case_validation_web --test api -- --nocapture --test-threads=1
```

Expected: all focused suites pass.

- [ ] **Step 2: Run compile verification**

Run:

```bash
cargo check -p web-server --tests --keep-going
```

Expected: command completes successfully with no compiler errors.

- [ ] **Step 3: Update requirements checklist**

In `docs/requirements/client_requirements_todo.md`, update only the save/delete lines proven by this slice:

```markdown
- [-] Fix page-level save so it works for both directly entered cases and imported cases. Backend case-level save parity is covered for manual and imported-case-shaped records; full page-level UI UAT remains open.
- [-] Remove irrelevant batch/header error messages shown during normal case save. Backend case save contract now guards against import/batch/header noise in normal case update responses; broader UI UAT remains open.
- [-] Require save/delete reason and comments for compliance-sensitive actions. Delete now requires `reason_for_change`; save comments/reasons outside status transitions remain open.
- [-] Remove password re-entry from delete if the client still wants delete confirmation without PW input. `DELETE /api/cases/{id}` now requires reason only and does not require e-signature password re-entry; final client confirmation copy remains open.
- [x] Keep deleted cases visible as soft-deleted rows with clear visual marking and history retention.
```

- [ ] **Step 4: Verify requirements diff is scoped**

Run:

```bash
git diff -- docs/requirements/client_requirements_todo.md
```

Expected: only the five save/delete checklist lines above changed.

- [ ] **Step 5: Commit verification/docs**

```bash
git add docs/requirements/client_requirements_todo.md
git commit -m "docs: update save delete requirement status"
```

---

## Final Verification

After all tasks pass and are reviewed, run:

```bash
cargo fmt --all
cargo check -p web-server --tests --keep-going
cargo test -p web-server case_contract_web --test api -- --nocapture --test-threads=1
cargo test -p web-server import_contract_web --test api -- --nocapture --test-threads=1
cargo test -p web-server import_history_web --test api -- --nocapture --test-threads=1
cargo test -p web-server case_validation_web --test api -- --nocapture --test-threads=1
```

Expected: all commands complete successfully.
