# Case List Validation Warn Count Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `/api/cases/list-view` return the number of failed validation rules in each row's `warn` field instead of the current hardcoded `"No"`.

**Architecture:** The frontend already renders `CaseListViewItem.warn`, so the fix belongs in the backend list-view API. Reuse the existing validation profile resolution and validation report generation used by `/api/cases/{case_id}/validation/all`, expose a small internal summary helper, and compute counts only for rows that survive user scope, offset, and limit.

**Tech Stack:** Rust, Axum, SQLx, lib-core validation engine, existing web-server API contract tests, Next.js frontend type contract.

---

## File Structure

- Modify: `crates/services/web-server/tests/api/case_contract_web.rs`
  - Add an API contract test proving list-view `warn` equals `blockingCount + nonBlockingCount` from `/api/cases/{id}/validation/all`.
- Modify: `crates/services/web-server/src/web/rest/case_validation_rest.rs`
  - Add an internal validation summary type and helper that centralizes profile resolution, report generation, and count aggregation.
  - Refactor `validate_case_all` to use the helper so list-view and detail validation cannot drift.
- Modify: `crates/services/web-server/src/web/rest/case_rest.rs`
  - Import the validation REST module.
  - Compute `item.warn` for the paged/scoped rows before returning them.
- Modify: `crates/libs/lib-core/src/model/case.rs`
  - Change the SQL placeholder from `'No' AS warn` to `'0' AS warn`; REST overwrites it for returned rows.
- Optional verify only: `frontend/E2BR3-frontend/lib/types/api.ts`
  - Keep `warn: string`; the backend returns a numeric string to preserve the current table/filter contract.

## Task 1: Backend Contract Test

**Files:**
- Modify: `crates/services/web-server/tests/api/case_contract_web.rs`

- [ ] **Step 1: Write the failing test**

Add this test after `test_case_list_view_projects_reference_grid_fields`:

```rust
#[serial]
#[tokio::test]
async fn test_case_list_view_warn_matches_validation_failure_count() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let suffix = Uuid::new_v4().simple().to_string();
	let case_no = format!("CASE-LIST-WARN-{suffix}");

	let (status, raw_body) = post_raw(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safety_report_id": case_no,
				"status": "draft",
				"appendices_json": "[\"ich\"]"
			}
		}),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&raw_body)
	);
	let body: Value = serde_json::from_slice(&raw_body)?;
	let case_id = body["data"]["id"]
		.as_str()
		.ok_or("missing created case id")?
		.to_string();

	let (status, validation_body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/validation/all"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{validation_body:?}");
	let blocking_count = validation_body["data"]["blockingCount"]
		.as_u64()
		.ok_or("missing blockingCount")?;
	let non_blocking_count = validation_body["data"]["nonBlockingCount"]
		.as_u64()
		.ok_or("missing nonBlockingCount")?;
	let expected_warn = (blocking_count + non_blocking_count).to_string();
	assert_ne!(
		expected_warn, "0",
		"test fixture must have at least one validation failure: {validation_body:?}"
	);

	let (status, raw_body) = get_raw(
		&app,
		&cookie,
		"/api/cases/list-view?list_options%5Blimit%5D=25&list_options%5Border_bys%5D=%21created_at",
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::OK,
		"{}",
		String::from_utf8_lossy(&raw_body)
	);
	let body: Value = serde_json::from_slice(&raw_body)?;
	let items = body["data"]["items"]
		.as_array()
		.ok_or("missing list-view items")?;
	let row = items
		.iter()
		.find(|item| item["caseNo"].as_str() == Some(case_no.as_str()))
		.ok_or("missing projected warning case row")?;

	assert_eq!(
		row["warn"].as_str(),
		Some(expected_warn.as_str()),
		"list-view warn should equal blocking + non-blocking validation failures; row={row:?}, validation={validation_body:?}"
	);

	Ok(())
}
```

- [ ] **Step 2: Run the test to verify RED**

Run:

```bash
cargo test -p web-server --test api test_case_list_view_warn_matches_validation_failure_count -- --nocapture
```

Expected: the test fails at the final assertion because `row["warn"]` is currently `"No"` while `expected_warn` is a positive numeric string.

- [ ] **Step 3: Commit the failing test only if this workflow uses micro-commits**

```bash
git add crates/services/web-server/tests/api/case_contract_web.rs
git commit -m "test: cover case list validation warning count"
```

Skip this commit step if the user wants one final commit only.

## Task 2: Shared Validation Count Helper

**Files:**
- Modify: `crates/services/web-server/src/web/rest/case_validation_rest.rs`

- [ ] **Step 1: Add the internal summary type and helper**

In `case_validation_rest.rs`, after `CaseValidationBundle`, add:

```rust
#[derive(Debug)]
pub(crate) struct CaseValidationSummary {
	pub case_id: Uuid,
	pub profiles: Vec<String>,
	pub ok: bool,
	pub blocking_count: usize,
	pub non_blocking_count: usize,
	pub reports: Vec<CaseValidationReport>,
}

impl CaseValidationSummary {
	pub(crate) fn total_count(&self) -> usize {
		self.blocking_count + self.non_blocking_count
	}
}
```

After `resolve_profiles`, add:

```rust
pub(crate) async fn validation_summary_for_case(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<CaseValidationSummary> {
	let profiles = resolve_profiles(ctx, mm, case_id).await?;
	let reports = validate_case_for_profiles(ctx, mm, case_id, &profiles).await?;
	let blocking_count: usize = reports.iter().map(|r| r.blocking_count).sum();
	let non_blocking_count: usize =
		reports.iter().map(|r| r.non_blocking_count).sum();
	let ok = reports.iter().all(|r| r.ok);
	let profiles = reports
		.iter()
		.map(|r| r.profile.clone())
		.collect::<Vec<_>>();

	Ok(CaseValidationSummary {
		case_id,
		profiles,
		ok,
		blocking_count,
		non_blocking_count,
		reports,
	})
}
```

- [ ] **Step 2: Refactor `/validation/all` to use the helper**

Replace the body after permission/scope checks in `validate_case_all`:

```rust
	let summary = validation_summary_for_case(&ctx, &mm, case_id).await?;

	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: CaseValidationBundle {
				case_id: summary.case_id,
				profiles: summary.profiles,
				ok: summary.ok,
				blocking_count: summary.blocking_count,
				non_blocking_count: summary.non_blocking_count,
				reports: summary.reports,
			},
		}),
	))
```

- [ ] **Step 3: Run validation endpoint regression tests**

Run:

```bash
cargo test -p web-server --test api case_validation_web -- --nocapture
```

Expected: all `case_validation_web` tests pass. If the command output says no tests matched because of module filtering, run:

```bash
cargo test -p web-server --test api validation -- --nocapture
```

Expected: validation API tests pass.

## Task 3: Populate Warn in List View

**Files:**
- Modify: `crates/services/web-server/src/web/rest/case_rest.rs`
- Modify: `crates/libs/lib-core/src/model/case.rs`

- [ ] **Step 1: Import the validation helper module**

At the top of `case_rest.rs`, add this beside the other local REST imports:

```rust
use crate::web::rest::case_validation_rest;
```

- [ ] **Step 2: Compute `warn` only for returned rows**

In `list_case_view_rows`, replace the scoped loop:

```rust
	for item in items {
		if lib_rest_core::case_matches_user_scope(&ctx, &mm, item.case_id).await? {
			if scoped_offset < offset {
				scoped_offset += 1;
				continue;
			}
			scoped.push(item);
			if scoped.len() >= limit {
				break;
			}
		}
	}
```

with:

```rust
	for mut item in items {
		if lib_rest_core::case_matches_user_scope(&ctx, &mm, item.case_id).await? {
			if scoped_offset < offset {
				scoped_offset += 1;
				continue;
			}

			let validation =
				case_validation_rest::validation_summary_for_case(
					&ctx,
					&mm,
					item.case_id,
				)
				.await?;
			item.warn = validation.total_count().to_string();

			scoped.push(item);
			if scoped.len() >= limit {
				break;
			}
		}
	}
```

This keeps the expensive validation work limited to the current response page after auth scope and pagination.

- [ ] **Step 3: Change the SQL placeholder**

In `crates/libs/lib-core/src/model/case.rs`, replace:

```rust
			       'No' AS warn,
```

with:

```rust
			       '0' AS warn,
```

The SQL projection remains valid for serialization and ordering, while the REST handler owns the dynamic validation count.

- [ ] **Step 4: Run the RED test again to verify GREEN**

Run:

```bash
cargo test -p web-server --test api test_case_list_view_warn_matches_validation_failure_count -- --nocapture
```

Expected: the test passes.

- [ ] **Step 5: Run related list-view regression tests**

Run:

```bash
cargo test -p web-server --test api case_list_view -- --nocapture
```

Expected: list-view projection and paging tests pass.

## Task 4: Contract and Manual Verification

**Files:**
- Verify: `frontend/E2BR3-frontend/lib/types/api.ts`
- Verify: running frontend at `http://localhost:3003/dashboard/cases`

- [ ] **Step 1: Confirm frontend type remains compatible**

Run:

```bash
rg -n "warn: string|warn" /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/lib/types/api.ts /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/app/dashboard/cases/page.tsx
```

Expected: `CaseListViewItem.warn` is still `string` and the case table still renders `item.warn` without assuming `"Yes"` or `"No"`.

- [ ] **Step 2: Type-check the frontend**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npx tsc --noEmit
```

Expected: TypeScript exits with code 0.

- [ ] **Step 3: Rebuild and restart the local backend**

Run:

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/e2br3
cargo build -p web-server
```

Expected: build exits with code 0.

If the currently running backend was started from `target/debug/web-server`, stop that session and start a new one:

```bash
set -a
source .env
set +a
target/debug/web-server
```

Expected: backend listens on `127.0.0.1:8080`.

- [ ] **Step 4: Verify the authenticated API response**

Run:

```bash
TOKEN=$(curl -s -X POST http://localhost:3003/auth/v1/login \
  -H 'content-type: application/json' \
  -d '{"email":"demo.user@example.com","password":"welcome"}' \
  | jq -r '.data.token // .token')

curl -s http://localhost:3003/api/cases/list-view \
  -H "cookie: token=$TOKEN" \
  | jq '.data.items[0:5] | map({caseNo, warn})'
```

Expected: `warn` values are numeric strings such as `"0"`, `"3"`, or `"12"`, not `"No"`.

- [ ] **Step 5: Browser-check the case list**

Open:

```text
http://localhost:3003/dashboard/cases
```

Expected: the `Warn` column displays failed-rule counts. The table layout remains at 25 rows per page with the taller row height requested earlier.

## Task 5: Final Regression Suite

**Files:**
- Verify only.

- [ ] **Step 1: Run focused backend tests**

```bash
cargo test -p web-server --test api test_case_list_view_warn_matches_validation_failure_count -- --nocapture
cargo test -p web-server --test api case_list_view -- --nocapture
cargo test -p web-server --test api validation -- --nocapture
```

Expected: all commands pass.

- [ ] **Step 2: Run frontend type-check**

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend
npx tsc --noEmit
```

Expected: exits with code 0.

- [ ] **Step 3: Inspect changed files**

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/e2br3
git diff -- crates/services/web-server/tests/api/case_contract_web.rs crates/services/web-server/src/web/rest/case_validation_rest.rs crates/services/web-server/src/web/rest/case_rest.rs crates/libs/lib-core/src/model/case.rs
```

Expected: diff contains only the warning-count test, shared validation summary helper, list-view warning assignment, and SQL placeholder change.

- [ ] **Step 4: Commit if requested**

```bash
git add crates/services/web-server/tests/api/case_contract_web.rs crates/services/web-server/src/web/rest/case_validation_rest.rs crates/services/web-server/src/web/rest/case_rest.rs crates/libs/lib-core/src/model/case.rs
git commit -m "fix: show validation warning counts in case list"
```

Expected: commit succeeds. Do not stage unrelated dirty frontend files or existing unrelated backend docs.

## Self-Review

- Spec coverage: The plan fixes the hardcoded `Warn = No` source, adds backend API support by populating list-view rows with validation failure counts, and keeps frontend display compatible.
- Placeholder scan: No `TBD`, `TODO`, vague “add tests,” or missing implementation steps remain.
- Type consistency: `CaseValidationSummary.total_count()` returns `usize`; list-view converts it to `String` because `CaseListViewRow.warn` and `CaseListViewItem.warn` are already string fields.
