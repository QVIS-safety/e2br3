# Effective Role Privilege Tests Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add TDD-backed integration tests proving Role & Privilege matrix checkbox changes grant or withhold real backend authorization for matrix rows that already map to backend permissions.

**Architecture:** Keep the tests in the existing Rust web integration suite because effective permissions are enforced by backend middleware and dynamic role cache refresh. Add small reusable test helpers in `scope_visibility_web.rs` to create empty custom roles, update their matrix privileges, mint users assigned to those roles, and assert endpoint access before/after privilege updates. Do not touch frontend code for this plan.

**Tech Stack:** Rust, Axum integration tests, `cargo test -p web-server`, existing `request_json`, `insert_user`, `generate_web_token`, and `/api/admin/roles` endpoints.

---

## File Structure

- Modify: `crates/services/web-server/tests/api/scope_visibility_web.rs`
  - Add test helpers near existing `request_json`.
  - Add effective authorization tests for backend-mapped matrix rows.
- Modify only if a RED test proves a missing mapping: `crates/services/web-server/src/web/rest/admin_role_rest.rs`
  - Keep the accepted menu key list aligned with the frontend matrix keys that should be persisted.
- Modify only if a RED test proves a missing permission expansion: `crates/libs/lib-core/src/model/acs/permission.rs`
  - Keep `permissions_for_menu_key` as the source of truth for menu-key to backend-permission expansion.

## Scope

Backend-enforced rows to cover first:

- `case`: CASE Read/Edit/Workflow/Review/Lock rows map to `CASE_LIST`, `CASE_READ`, `CASE_CREATE`, `CASE_UPDATE`, and `CASE_APPROVE`.
- `info`: CASE INFO Read/Edit rows map to presave sender/template read/list/create/update/delete permissions.
- `data`: DATA Read/Edit rows map to terminology read/import/approve permissions.
- `export_submission`: SUBMISSION Read/Edit rows map to `XML_EXPORT`.
- `settings`: ADMIN Edit row should map to safety-db admin behavior through `sponsor_admin_capable` plus admin permissions.

Rows intentionally out of scope for this plan because they currently have no backend-enforced endpoint permission mapping:

- `home_notice`
- `home_workflow`
- `monitoring`
- `sync`
- `sync_mapping`
- `report_due_mail`

---

### Task 1: Add Effective Permission Test Helpers

**Files:**
- Modify: `crates/services/web-server/tests/api/scope_visibility_web.rs`

- [ ] **Step 1: Add helper functions**

Insert these helpers after the existing `request_json` function:

```rust
async fn create_empty_custom_role(
	app: &Router,
	admin_cookie: &str,
	role_name: &str,
) -> Result<()> {
	let (status, value) = request_json(
		app,
		"POST",
		admin_cookie,
		"/api/admin/roles".to_string(),
		Some(json!({
			"data": {
				"role_name": role_name,
				"display_name": role_name,
				"description": format!("Effective permission test role {role_name}"),
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	Ok(())
}

async fn update_role_privileges(
	app: &Router,
	admin_cookie: &str,
	role_name: &str,
	privileges: Value,
) -> Result<Value> {
	let (status, value) = request_json(
		app,
		"PUT",
		admin_cookie,
		format!("/api/admin/roles/{role_name}"),
		Some(json!({ "data": { "privileges": privileges } })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	Ok(value)
}

async fn custom_role_cookie(
	mm: &ModelManager,
	org_id: Uuid,
	role_name: &str,
) -> Result<String> {
	let user = insert_user(
		mm,
		org_id,
		role_name,
		system_user_id(),
		Some("custompwd"),
	)
	.await?;
	let token = generate_web_token(&user.email, user.token_salt)?;
	Ok(cookie_header(&token.to_string()))
}

async fn assert_get_status(
	app: &Router,
	cookie: &str,
	uri: &str,
	expected: StatusCode,
) -> Result<Value> {
	let (status, value) =
		request_json(app, "GET", cookie, uri.to_string(), None).await?;
	assert_eq!(status, expected, "{uri} body={value:?}");
	Ok(value)
}
```

- [ ] **Step 2: Run a no-op compile check**

Run:

```bash
cargo test -p web-server role_privilege_matrix_update_grants_effective_case_access -- --nocapture
```

Expected: PASS. This confirms helpers compile before adding more tests.

- [ ] **Step 3: Commit**

```bash
git add crates/services/web-server/tests/api/scope_visibility_web.rs
git commit -m "test: add role privilege test helpers"
```

---

### Task 2: CASE Matrix Rows Grant Effective Case Permissions

**Files:**
- Modify: `crates/services/web-server/tests/api/scope_visibility_web.rs`

- [ ] **Step 1: Write the failing test**

Add this test near the existing role admin API tests:

```rust
#[serial]
#[tokio::test]
async fn test_case_matrix_privileges_grant_effective_case_permissions(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let role_name = format!("qa_case_matrix_{}", Uuid::new_v4().simple());

	create_empty_custom_role(&app, &admin_cookie, &role_name).await?;
	let custom_cookie = custom_role_cookie(&mm, seed.org_id, &role_name).await?;

	assert_get_status(&app, &custom_cookie, "/api/cases", StatusCode::FORBIDDEN)
		.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&role_name,
		json!([
			{
				"menu_key": "case",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_get_status(&app, &custom_cookie, "/api/cases", StatusCode::OK).await?;

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/cases".to_string(),
		Some(json!({
			"data": {
				"safety_report_id": format!("CASE-MATRIX-{}", Uuid::new_v4().simple()),
				"status": "draft"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	update_role_privileges(
		&app,
		&admin_cookie,
		&role_name,
		json!([
			{
				"menu_key": "case",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/cases".to_string(),
		Some(json!({
			"data": {
				"safety_report_id": format!("CASE-MATRIX-{}", Uuid::new_v4().simple()),
				"status": "draft"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");

	update_role_privileges(
		&app,
		&admin_cookie,
		&role_name,
		json!([
			{
				"menu_key": "case",
				"can_read": true,
				"can_edit": false,
				"can_review": true,
				"can_lock": false
			}
		]),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/cases".to_string(),
		Some(json!({
			"data": {
				"safety_report_id": format!("CASE-MATRIX-{}", Uuid::new_v4().simple()),
				"status": "draft"
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"review alone should not grant case create: {value:?}"
	);

	Ok(())
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p web-server test_case_matrix_privileges_grant_effective_case_permissions -- --nocapture
```

Expected: FAIL if any CASE matrix field does not refresh dynamic permissions correctly. If it passes immediately, keep the test because it proves existing CASE behavior; proceed to Task 3.

- [ ] **Step 3: Minimal implementation if RED fails**

If the test fails because `case` privileges do not expand, update only the `case` arm in `crates/libs/lib-core/src/model/acs/permission.rs`:

```rust
"case" => {
	if can_read {
		push_unique(&mut permissions, viewer_permissions());
	}
	if can_edit {
		push_unique(&mut permissions, user_permissions());
	}
	if can_review || can_lock {
		push_unique(&mut permissions, &[CASE_APPROVE, CASE_UPDATE]);
	}
}
```

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p web-server test_case_matrix_privileges_grant_effective_case_permissions -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/services/web-server/tests/api/scope_visibility_web.rs crates/libs/lib-core/src/model/acs/permission.rs
git commit -m "test: cover effective case matrix permissions"
```

---

### Task 3: CASE INFO Matrix Rows Grant Effective Presave Template Permissions

**Files:**
- Modify: `crates/services/web-server/tests/api/scope_visibility_web.rs`

- [ ] **Step 1: Write the failing test**

Add:

```rust
#[serial]
#[tokio::test]
async fn test_info_matrix_privileges_grant_effective_presave_permissions(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let role_name = format!("qa_info_matrix_{}", Uuid::new_v4().simple());

	create_empty_custom_role(&app, &admin_cookie, &role_name).await?;
	let custom_cookie = custom_role_cookie(&mm, seed.org_id, &role_name).await?;

	assert_get_status(
		&app,
		&custom_cookie,
		"/api/presave-templates",
		StatusCode::FORBIDDEN,
	)
	.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&role_name,
		json!([
			{
				"menu_key": "info",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_get_status(&app, &custom_cookie, "/api/presave-templates", StatusCode::OK)
		.await?;

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/presave-templates".to_string(),
		Some(json!({
			"data": {
				"entity_type": "sender",
				"name": "Info Matrix Sender",
				"description": "Should require info edit",
				"data": {
					"senderType": "2",
					"senderIdentifier": "INFO-MATRIX",
					"senderOrganization": "Info Matrix Sender"
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	update_role_privileges(
		&app,
		&admin_cookie,
		&role_name,
		json!([
			{
				"menu_key": "info",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/presave-templates".to_string(),
		Some(json!({
			"data": {
				"entity_type": "sender",
				"name": format!("Info Matrix Sender {}", Uuid::new_v4().simple()),
				"description": "Info edit should allow creation",
				"data": {
					"senderType": "2",
					"senderIdentifier": "INFO-MATRIX-EDIT",
					"senderOrganization": "Info Matrix Sender Edit"
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");

	Ok(())
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p web-server test_info_matrix_privileges_grant_effective_presave_permissions -- --nocapture
```

Expected: FAIL if `info` matrix privileges do not expand to presave permissions. If it passes immediately, keep the test and continue.

- [ ] **Step 3: Minimal implementation if RED fails**

If required, update only the `"info"` arm in `permissions_for_menu_key`:

```rust
"info" => {
	if can_read {
		push_unique(
			&mut permissions,
			&[
				PRESAVE_TEMPLATE_READ,
				PRESAVE_TEMPLATE_LIST,
				SENDER_INFORMATION_READ,
				SENDER_INFORMATION_LIST,
				RECEIVER_READ,
				RECEIVER_LIST,
				STUDY_INFORMATION_READ,
				STUDY_INFORMATION_LIST,
				NARRATIVE_READ,
				NARRATIVE_LIST,
			],
		);
	}
	if can_edit {
		push_unique(
			&mut permissions,
			&[
				PRESAVE_TEMPLATE_CREATE,
				PRESAVE_TEMPLATE_UPDATE,
				PRESAVE_TEMPLATE_DELETE,
				SENDER_INFORMATION_CREATE,
				SENDER_INFORMATION_UPDATE,
				SENDER_INFORMATION_DELETE,
				RECEIVER_CREATE,
				RECEIVER_UPDATE,
				RECEIVER_DELETE,
				STUDY_INFORMATION_CREATE,
				STUDY_INFORMATION_UPDATE,
				STUDY_INFORMATION_DELETE,
				NARRATIVE_CREATE,
				NARRATIVE_UPDATE,
				NARRATIVE_DELETE,
			],
		);
	}
}
```

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p web-server test_info_matrix_privileges_grant_effective_presave_permissions -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/services/web-server/tests/api/scope_visibility_web.rs crates/libs/lib-core/src/model/acs/permission.rs
git commit -m "test: cover effective info matrix permissions"
```

---

### Task 4: DATA Matrix Rows Grant Effective Terminology Permissions

**Files:**
- Modify: `crates/services/web-server/tests/api/scope_visibility_web.rs`

- [ ] **Step 1: Write the failing test**

Add:

```rust
#[serial]
#[tokio::test]
async fn test_data_matrix_privileges_grant_effective_terminology_permissions(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let role_name = format!("qa_data_matrix_{}", Uuid::new_v4().simple());

	create_empty_custom_role(&app, &admin_cookie, &role_name).await?;
	let custom_cookie = custom_role_cookie(&mm, seed.org_id, &role_name).await?;

	assert_get_status(
		&app,
		&custom_cookie,
		"/api/terminology/countries",
		StatusCode::FORBIDDEN,
	)
	.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&role_name,
		json!([
			{
				"menu_key": "data",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_get_status(
		&app,
		&custom_cookie,
		"/api/terminology/countries",
		StatusCode::OK,
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/terminology/releases/meddra/TEST/approve".to_string(),
		None,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"read-only DATA must not approve terminology releases: {value:?}"
	);

	update_role_privileges(
		&app,
		&admin_cookie,
		&role_name,
		json!([
			{
				"menu_key": "data",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/terminology/releases/meddra/TEST/approve".to_string(),
		None,
	)
	.await?;
	assert_ne!(
		status,
		StatusCode::FORBIDDEN,
		"DATA edit should pass permission check and fail later if release is absent: {value:?}"
	);

	Ok(())
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p web-server test_data_matrix_privileges_grant_effective_terminology_permissions -- --nocapture
```

Expected: FAIL if `data` matrix privileges do not expand to terminology permissions. The final approve call should not be `403`; it may be `400 Bad Request` because the `TEST` release is intentionally absent.

- [ ] **Step 3: Minimal implementation if RED fails**

If needed, update only this arm in `permissions_for_menu_key`:

```rust
"data" | "terminology" => {
	if can_read {
		push_unique(&mut permissions, &[TERMINOLOGY_READ]);
	}
	if can_edit || can_review {
		push_unique(
			&mut permissions,
			&[TERMINOLOGY_IMPORT, TERMINOLOGY_APPROVE],
		);
	}
}
```

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p web-server test_data_matrix_privileges_grant_effective_terminology_permissions -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/services/web-server/tests/api/scope_visibility_web.rs crates/libs/lib-core/src/model/acs/permission.rs
git commit -m "test: cover effective data matrix permissions"
```

---

### Task 5: SUBMISSION Matrix Rows Grant Effective XML Export Permissions

**Files:**
- Modify: `crates/services/web-server/tests/api/scope_visibility_web.rs`

- [ ] **Step 1: Write the failing test**

Add:

```rust
#[serial]
#[tokio::test]
async fn test_submission_matrix_privileges_grant_effective_export_permissions(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let role_name = format!("qa_export_matrix_{}", Uuid::new_v4().simple());

	create_empty_custom_role(&app, &admin_cookie, &role_name).await?;
	let custom_cookie = custom_role_cookie(&mm, seed.org_id, &role_name).await?;

	assert_get_status(
		&app,
		&custom_cookie,
		"/api/exports/history",
		StatusCode::FORBIDDEN,
	)
	.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&role_name,
		json!([
			{
				"menu_key": "export_submission",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;

	assert_get_status(&app, &custom_cookie, "/api/exports/history", StatusCode::OK)
		.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&role_name,
		json!([
			{
				"menu_key": "export_submission",
				"can_read": false,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;

	assert_get_status(&app, &custom_cookie, "/api/exports/history", StatusCode::OK)
		.await?;

	Ok(())
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p web-server test_submission_matrix_privileges_grant_effective_export_permissions -- --nocapture
```

Expected: FAIL if `export_submission` matrix privileges do not expand to `XML_EXPORT`.

- [ ] **Step 3: Minimal implementation if RED fails**

If needed, update only this arm in `permissions_for_menu_key`:

```rust
"export_submission" | "submission" | "export" => {
	if can_read || can_edit {
		push_unique(&mut permissions, &[XML_EXPORT]);
	}
}
```

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p web-server test_submission_matrix_privileges_grant_effective_export_permissions -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/services/web-server/tests/api/scope_visibility_web.rs crates/libs/lib-core/src/model/acs/permission.rs
git commit -m "test: cover effective submission matrix permissions"
```

---

### Task 6: ADMIN Edit Matrix Row Grants Effective Safety DB Admin Access

**Files:**
- Modify: `crates/services/web-server/tests/api/scope_visibility_web.rs`

- [ ] **Step 1: Write the failing test**

Add:

```rust
#[serial]
#[tokio::test]
async fn test_admin_edit_matrix_privilege_grants_effective_admin_access(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let role_name = format!("qa_admin_matrix_{}", Uuid::new_v4().simple());

	create_empty_custom_role(&app, &admin_cookie, &role_name).await?;
	let custom_cookie = custom_role_cookie(&mm, seed.org_id, &role_name).await?;

	let (status, value) =
		request_json(&app, "GET", &custom_cookie, "/api/users".to_string(), None)
			.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	update_role_privileges(
		&app,
		&admin_cookie,
		&role_name,
		json!([
			{
				"menu_key": "settings",
				"can_read": false,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;

	let (status, value) =
		request_json(&app, "GET", &custom_cookie, "/api/users".to_string(), None)
			.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	Ok(())
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p web-server test_admin_edit_matrix_privilege_grants_effective_admin_access -- --nocapture
```

Expected: FAIL if `settings` Edit does not set `sponsor_admin_capable` and grant admin permissions. If it passes immediately, keep the test and continue.

- [ ] **Step 3: Minimal implementation if RED fails**

If failure shows the role has admin permissions but is not sponsor-admin-capable, update `role_summary_booleans` in `crates/services/web-server/src/web/rest/admin_role_rest.rs` so `settings` Edit remains admin-capable:

```rust
let can_admin = privileges.iter().any(|privilege| {
	matches!(
		privilege.menu_key.as_str(),
		"admin" | "settings" | "roles" | "users" | "user"
	) && (privilege.can_edit
		|| privilege.can_review
		|| privilege.can_lock
		|| privilege.can_read && privilege.menu_key == "admin")
});
```

Do not broaden `settings` Read to sponsor-admin-capable unless the product owner explicitly confirms that Read should allow Safety DB admin endpoints.

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p web-server test_admin_edit_matrix_privilege_grants_effective_admin_access -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/services/web-server/tests/api/scope_visibility_web.rs crates/services/web-server/src/web/rest/admin_role_rest.rs
git commit -m "test: cover effective admin matrix permissions"
```

---

### Task 7: Guard UI-Only Matrix Rows as Persistence-Only

**Files:**
- Modify: `crates/services/web-server/tests/api/scope_visibility_web.rs`

- [ ] **Step 1: Write the persistence-only regression test**

If not already present, add or keep this test:

```rust
#[serial]
#[tokio::test]
async fn test_role_admin_api_persists_privilege_matrix_menu_keys() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let role_name = format!("qa_matrix_{}", Uuid::new_v4().simple());

	create_empty_custom_role(&app, &admin_cookie, &role_name).await?;

	let value = update_role_privileges(
		&app,
		&admin_cookie,
		&role_name,
		json!([
			{
				"menu_key": "home_notice",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			},
			{
				"menu_key": "report_due_mail",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;

	assert_eq!(
		value["privilege_map"]["home_notice"]["can_edit"].as_bool(),
		Some(true)
	);
	assert_eq!(
		value["privilege_map"]["report_due_mail"]["can_read"].as_bool(),
		Some(true)
	);

	Ok(())
}
```

- [ ] **Step 2: Verify**

Run:

```bash
cargo test -p web-server test_role_admin_api_persists_privilege_matrix_menu_keys -- --nocapture
```

Expected: PASS. This confirms UI-only rows are persisted but does not claim effective backend permissions.

- [ ] **Step 3: Commit**

```bash
git add crates/services/web-server/tests/api/scope_visibility_web.rs
git commit -m "test: document persistence-only matrix privileges"
```

---

### Task 8: Final Verification

**Files:**
- No new files.

- [ ] **Step 1: Run focused role/permission tests**

Run:

```bash
cargo test -p web-server role_ -- --nocapture
```

Expected: PASS.

- [ ] **Step 2: Run formatting check**

Run:

```bash
cargo fmt --check
```

Expected: PASS.

- [ ] **Step 3: Inspect diff**

Run:

```bash
git diff -- crates/services/web-server/tests/api/scope_visibility_web.rs crates/services/web-server/src/web/rest/admin_role_rest.rs crates/libs/lib-core/src/model/acs/permission.rs
```

Expected: Diff contains only effective role privilege tests and the minimum backend mapping fixes required by RED tests.

- [ ] **Step 4: Commit final cleanup if needed**

```bash
git add crates/services/web-server/tests/api/scope_visibility_web.rs crates/services/web-server/src/web/rest/admin_role_rest.rs crates/libs/lib-core/src/model/acs/permission.rs
git commit -m "test: verify effective role privilege matrix permissions"
```

---

## Subagent Execution Notes

Use one implementer subagent per task. Each implementer must:

1. Follow TDD: add the test first and run it before production changes.
2. Report whether the test failed as expected or passed immediately because existing behavior was already correct.
3. Make only the minimum code change needed for that task.
4. Run the task-specific test and report the exact command/result.
5. Avoid touching frontend files.

After each implementer:

1. Dispatch a spec reviewer subagent to verify the task matches this plan and did not broaden scope.
2. Dispatch a code-quality reviewer subagent after spec approval.
3. If either reviewer finds issues, send the implementer back to fix them and re-review.

## Self-Review

**Spec coverage:** The plan covers all matrix rows that currently map to backend permissions: `case`, `info`, `data`, `export_submission`, and `settings` Edit. It explicitly excludes rows without backend mappings and keeps a persistence-only test for those rows.

**Placeholder scan:** No TBD/TODO/fill-in steps remain. Each task includes concrete tests, commands, expected outcomes, and minimal implementation snippets if RED fails.

**Type consistency:** Helper names and test code use existing types and imports already present in `scope_visibility_web.rs`: `Router`, `StatusCode`, `Value`, `Uuid`, `ModelManager`, `json!`, `request_json`, `insert_user`, `system_user_id`, `generate_web_token`, and `cookie_header`.
