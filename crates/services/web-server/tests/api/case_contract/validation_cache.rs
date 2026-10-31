use super::support::{get_json, get_raw, post_json, post_raw, put_json};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::store::set_full_context_dbx;
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_case_list_view_warn_matches_validation_failure_count() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let suffix = Uuid::new_v4().simple().to_string();
	let case_no = format!("CASE-LIST-WARN-{suffix}");

	let (status, raw_body) = post_raw(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safetyReportIdentification": {"safetyReportId": case_no},
				"status": "draft"
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
		&format!("/api/cases/{case_id}/validation?authority=ich"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{validation_body:?}");
	let issue_count = validation_body["data"]["issue_count"]
		.as_u64()
		.ok_or("missing issue_count")?;
	let expected_warn = issue_count.to_string();
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
		"list-view warn should equal all validation failures; row={row:?}, validation={validation_body:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_list_view_warn_uses_cached_validation_summary() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let suffix = Uuid::new_v4().simple().to_string();
	let case_no = format!("CASE-LIST-CACHED-WARN-{suffix}");

	let (status, raw_body) = post_raw(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safetyReportIdentification": {"safetyReportId": case_no},
				"status": "draft"
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
		.ok_or("missing projected warning case row before validation")?;
	assert_eq!(
		row["warn"].as_str(),
		Some("Not checked"),
		"list-view should not live-validate uncached rows; row={row:?}"
	);

	let (status, validation_body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/validation?authority=ich"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{validation_body:?}");
	let issue_count = validation_body["data"]["issue_count"]
		.as_u64()
		.ok_or("missing issue_count")?;
	let expected_warn = issue_count.to_string();
	assert_ne!(expected_warn, "0", "{validation_body:?}");
	let case_uuid = Uuid::parse_str(&case_id)?;
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	let summary_rows = mm
		.dbx()
		.fetch_all(
			sqlx::query_as::<_, (String, String, i32)>(
				"SELECT appendix, page_id, issue_count
			   FROM case_validation_summaries
			  WHERE case_id = $1
			  ORDER BY appendix, page_id",
			)
			.bind(case_uuid),
		)
		.await?;
	mm.dbx().commit_txn().await?;
	assert!(
		summary_rows.iter().any(
			|(appendix, page_id, count)| appendix == "ich"
				&& page_id == "ALL"
				&& *count as u64 == issue_count
		),
		"single-authority validation should cache an appendix aggregate row; rows={summary_rows:?}"
	);
	assert!(
		summary_rows
			.iter()
			.any(|(appendix, page_id, _)| appendix == "ich" && page_id != "ALL"),
		"single-authority validation should cache appendix page rows; rows={summary_rows:?}"
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
		.ok_or("missing projected warning case row after validation")?;
	assert_eq!(
		row["warn"].as_str(),
		Some(expected_warn.as_str()),
		"list-view should use cached validation summary after validation; row={row:?}"
	);

	let (status, update_body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
				"dg_prd_key": "DG-CACHE-STALE"
			},
			"reason_for_change": "verify validation cache invalidation"
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{update_body:?}");

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
		.ok_or("missing list-view items after cache invalidation")?;
	let row = items
		.iter()
		.find(|item| item["caseNo"].as_str() == Some(case_no.as_str()))
		.ok_or("missing projected warning case row after cache invalidation")?;
	assert_eq!(
		row["warn"].as_str(),
		Some("Not checked"),
		"list-view should hide stale validation cache until validation refreshes it; row={row:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_single_profile_validation_caches_each_profile() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_no = format!("CASE-EXPLICIT-PROFILES-{}", Uuid::new_v4());

	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safetyReportIdentification": {"safetyReportId": case_no},
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let case_id = body["data"]["id"]
		.as_str()
		.ok_or("missing created case id")?;

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/validation?authority=fda"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["authority"], json!("fda"));
	let fda_count = body["data"]["issue_count"].as_u64().unwrap();
	let mut unique_issues = body["data"]["issues"]
		.as_array()
		.unwrap()
		.iter()
		.map(|issue| (issue["code"].to_string(), issue["path"].to_string()))
		.collect::<std::collections::HashSet<_>>();

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/validation?authority=mfds"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["authority"], json!("mfds"));
	let mfds_count = body["data"]["issue_count"].as_u64().unwrap();
	unique_issues.extend(
		body["data"]["issues"]
			.as_array()
			.unwrap()
			.iter()
			.map(|issue| (issue["code"].to_string(), issue["path"].to_string())),
	);
	assert!(
		unique_issues.len() < (fda_count + mfds_count) as usize,
		"fixture must exercise shared ICH errors"
	);

	for (authorities, expected) in [
		("ich", "Not checked".to_string()),
		("fda", fda_count.to_string()),
		("mfds", mfds_count.to_string()),
		("fda,mfds", unique_issues.len().to_string()),
		("fda,fda", fda_count.to_string()),
		("ich,fda", "Not checked".to_string()),
	] {
		let (status, list) = get_json(
			&app,
			&cookie,
			&format!("/api/cases/list-view?authorities={authorities}"),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{list:?}");
		let row = list["data"]["items"]
			.as_array()
			.unwrap()
			.iter()
			.find(|item| item["caseId"].as_str() == Some(case_id))
			.expect("case in list");
		assert_eq!(
			row["warn"].as_str(),
			Some(expected.as_str()),
			"{authorities}: {row:?}"
		);
	}

	let case_uuid = Uuid::parse_str(case_id)?;
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	let cached_appendices = mm
		.dbx()
		.fetch_all(
			sqlx::query_as::<_, (String,)>(
				"SELECT appendix
				   FROM case_validation_summaries
				  WHERE case_id = $1
				    AND page_id = 'ALL'
				  ORDER BY appendix",
			)
			.bind(case_uuid),
		)
		.await?;
	mm.dbx().commit_txn().await?;
	let cached_appendices = cached_appendices
		.into_iter()
		.map(|(appendix,)| appendix)
		.collect::<Vec<_>>();
	assert_eq!(cached_appendices, vec!["fda", "mfds"]);

	// A real editor save invalidates every authority, including those not refreshed next.
	let request = Request::builder()
		.method("PATCH")
		.uri(format!("/api/cases/{case_id}/editor/pages/RP"))
		.header("cookie", &cookie)
		.header("content-type", "application/json")
		.body(Body::from(json!({"authorities": ["fda", "mfds"], "rows": {
			"primarySources": [{"reporterGivenName": "Cache integration reporter", "reporterEmail": "validation@example.com", "qualification": "1", "primarySourceForRegulatoryPurposes": "1", "reporterCountry": "US"}]
		}}).to_string()))?;
	let saved = app.clone().oneshot(request).await?;
	let status = saved.status();
	let saved = to_bytes(saved.into_body(), usize::MAX).await?;
	assert_eq!(
		status,
		StatusCode::OK,
		"{}",
		String::from_utf8_lossy(&saved)
	);

	let mut refreshed = std::collections::HashMap::new();
	for refresh in [None, Some("fda"), Some("mfds")] {
		if let Some(authority) = refresh {
			let (status, response) = get_json(
				&app,
				&cookie,
				&format!("/api/cases/{case_id}/validation?authority={authority}"),
			)
			.await?;
			assert_eq!(status, StatusCode::OK, "{response:?}");
			let report = &response["data"];
			assert_eq!(report["authority"], authority);
			let issues = report["issues"].as_array().ok_or("missing issues")?;
			assert_eq!(report["issue_count"].as_u64(), Some(issues.len() as u64));
			assert_eq!(report["ok"].as_bool(), Some(issues.is_empty()));
			let sections = report["section_summaries"]
				.as_array()
				.ok_or("missing section summaries")?;
			for section in sections {
				let count = issues
					.iter()
					.filter(|issue| issue["section"] == section["section"])
					.count();
				assert_eq!(section["issue_count"].as_u64(), Some(count as u64));
			}
			assert_eq!(
				sections
					.iter()
					.map(|section| section["issue_count"].as_u64().unwrap())
					.sum::<u64>(),
				issues.len() as u64
			);
			let keys = issues
				.iter()
				.map(|issue| (issue["code"].to_string(), issue["path"].to_string()))
				.collect::<std::collections::HashSet<_>>();
			assert_eq!(
				keys.len(),
				issues.len(),
				"duplicate issue identity in {authority}"
			);
			refreshed.insert(authority, keys);
		}

		for authorities in ["fda", "mfds", "fda,mfds"] {
			let selected = authorities.split(',').collect::<Vec<_>>();
			let expected = if selected
				.iter()
				.all(|authority| refreshed.contains_key(authority))
			{
				selected
					.iter()
					.flat_map(|authority| refreshed[authority].iter().cloned())
					.collect::<std::collections::HashSet<_>>()
					.len()
					.to_string()
			} else {
				"Not checked".to_string()
			};
			let (status, list) = get_json(
				&app,
				&cookie,
				&format!("/api/cases/list-view?authorities={authorities}"),
			)
			.await?;
			assert_eq!(status, StatusCode::OK, "{list:?}");
			let row = list["data"]["items"]
				.as_array()
				.ok_or("missing items")?
				.iter()
				.find(|item| item["caseId"].as_str() == Some(case_id))
				.ok_or("missing case")?;
			assert_eq!(
				row["warn"].as_str(),
				Some(expected.as_str()),
				"refresh={refresh:?} authorities={authorities}"
			);
		}
	}
	assert!(
		refreshed["fda"].len() < fda_count as usize,
		"saved reporter must change the cached result"
	);
	assert!(
		refreshed["mfds"].len() < mfds_count as usize,
		"saved reporter must change both authorities"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_validation_cache_waits_for_case_mutation() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let (status, body) = post_json(&app, &cookie, "/api/cases", json!({"data": {
		"safetyReportIdentification": {"safetyReportId": format!("CACHE-RACE-{}", Uuid::new_v4())},
		"status": "draft"
	}})).await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let case_id = Uuid::parse_str(body["data"]["id"].as_str().unwrap())?;

	// The editing user's transaction holds the same row lock as authorized mutations.
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	let (pid,) = mm
		.dbx()
		.fetch_one(sqlx::query_as::<_, (i32,)>("SELECT pg_backend_pid()"))
		.await?;
	mm.dbx()
		.fetch_one(
			sqlx::query_as::<_, (Uuid,)>(
				"SELECT id FROM cases WHERE id = $1 FOR UPDATE",
			)
			.bind(case_id),
		)
		.await?;
	let validation = tokio::spawn(async move {
		get_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/validation?authority=ich"),
		)
		.await
	});
	let waiting = tokio::time::timeout(std::time::Duration::from_secs(5), async {
		loop {
            let blocked: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_stat_activity WHERE $1 = ANY(pg_blocking_pids(pid)))")
				.bind(pid).fetch_one(mm.dbx().db()).await?;
			if blocked { return Result::Ok(()); }
			tokio::time::sleep(std::time::Duration::from_millis(10)).await;
		}
	}).await;
	mm.dbx().commit_txn().await?;
	let (status, report) =
		tokio::time::timeout(std::time::Duration::from_secs(10), validation)
			.await???;
	assert_eq!(status, StatusCode::OK, "{report:?}");
	assert!(
		waiting.is_ok(),
		"validation must wait before loading Case data"
	);
	waiting??;
	assert!(report["data"]["issue_count"].as_u64().unwrap() > 0);
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let (status, report) = get_json(
		&web_server::app(mm.clone()),
		&cookie_header(&viewer_token.to_string()),
		&format!("/api/cases/{case_id}/validation?authority=ich"),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::OK,
		"read-only users must still validate: {report:?}"
	);
	Ok(())
}
