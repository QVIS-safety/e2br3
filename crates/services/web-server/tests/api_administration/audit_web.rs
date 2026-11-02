use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::Value;
use serial_test::serial;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

#[serial]
#[tokio::test]
async fn test_audit_list_requires_permission() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;

	let app = web_server::app(mm);

	let req = Request::builder()
		.method("GET")
		.uri("/api/audit-logs")
		.header("cookie", cookie_header(&admin_token.to_string()))
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	let rows = value["data"].as_array().ok_or("missing audit rows")?;
	assert!(!rows.is_empty());
	assert!(rows.iter().all(|row| row["hash_version"].is_i64()));

	let req = Request::builder()
		.method("GET")
		.uri("/api/audit-logs")
		.header("cookie", cookie_header(&viewer_token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_audit_integrity_reports_first_broken_row_and_reason() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let database_url = std::env::var("SERVICE_MIGRATION_DB_URL")?;
	let pool = PgPoolOptions::new()
		.max_connections(1)
		.connect(&database_url)
		.await?;
	let database_name: String = sqlx::query_scalar("SELECT current_database()")
		.fetch_one(&pool)
		.await?;
	if !database_name.starts_with("e2br3_test_") {
		return Err(format!(
			"audit corruption test requires an isolated e2br3_test_ database, got {database_name}"
		)
		.into());
	}
	let (broken_id, original_hash): (i64, String) = sqlx::query_as(
		"SELECT id, entry_hash FROM audit_logs ORDER BY id DESC LIMIT 1",
	)
	.fetch_one(&pool)
	.await?;

	sqlx::query(
		"ALTER TABLE audit_logs DISABLE TRIGGER audit_logs_reject_update_delete",
	)
	.execute(&pool)
	.await?;
	sqlx::query("UPDATE audit_logs SET entry_hash = repeat('0', 64) WHERE id = $1")
		.bind(broken_id)
		.execute(&pool)
		.await?;
	sqlx::query(
		"ALTER TABLE audit_logs ENABLE TRIGGER audit_logs_reject_update_delete",
	)
	.execute(&pool)
	.await?;

	let request = Request::builder()
		.method("GET")
		.uri(format!(
			"/api/audit-logs/verify-integrity?since_id={broken_id}&until_id={broken_id}"
		))
		.header("cookie", cookie_header(&admin_token.to_string()))
		.body(Body::empty())?;
	let response = web_server::app(mm).oneshot(request).await;

	sqlx::query(
		"ALTER TABLE audit_logs DISABLE TRIGGER audit_logs_reject_update_delete",
	)
	.execute(&pool)
	.await?;
	sqlx::query("UPDATE audit_logs SET entry_hash = $1 WHERE id = $2")
		.bind(original_hash)
		.bind(broken_id)
		.execute(&pool)
		.await?;
	sqlx::query(
		"ALTER TABLE audit_logs ENABLE TRIGGER audit_logs_reject_update_delete",
	)
	.execute(&pool)
	.await?;

	let response = response?;
	let status = response.status();
	let body = to_bytes(response.into_body(), usize::MAX).await?;
	assert_eq!(status, StatusCode::OK);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["broken_rows"].as_i64(), Some(1));
	assert_eq!(value["data"]["first_broken_id"].as_i64(), Some(broken_id));
	assert_eq!(
		value["data"]["first_broken_reason"].as_str(),
		Some("entry_hash does not match recomputed payload hash")
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_audit_integrity_reports_healthy_chain_and_requires_permission(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let app = web_server::app(mm);

	let request = Request::builder()
		.method("GET")
		.uri("/api/audit-logs/verify-integrity")
		.header("cookie", cookie_header(&admin_token.to_string()))
		.body(Body::empty())?;
	let response = app.clone().oneshot(request).await?;
	assert_eq!(response.status(), StatusCode::OK);
	let body = to_bytes(response.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	let report = &value["data"];
	let total_rows = report["total_rows"].as_i64().ok_or("missing total_rows")?;
	assert!(total_rows > 0, "seeded writes must be audited: {value}");
	assert_eq!(report["verified_ok_rows"].as_i64(), Some(total_rows));
	assert_eq!(report["broken_rows"].as_i64(), Some(0));
	assert_eq!(report.get("first_broken_id"), Some(&Value::Null));
	assert_eq!(report.get("first_broken_reason"), Some(&Value::Null));
	let checked_from_id = report["checked_from_id"]
		.as_i64()
		.ok_or("missing checked_from_id")?;
	let checked_through_id = report["checked_through_id"]
		.as_i64()
		.ok_or("missing checked_through_id")?;
	let snapshot_head_id = report["snapshot_head_id"]
		.as_i64()
		.ok_or("missing snapshot_head_id")?;
	assert!(checked_from_id <= checked_through_id);
	assert!(checked_through_id <= snapshot_head_id);
	let has_more = report["has_more"].as_bool().ok_or("missing has_more")?;
	assert_eq!(report["next_since_id"].is_i64(), has_more);
	if !has_more {
		assert_eq!(checked_through_id, snapshot_head_id);
	}
	assert_eq!(
		report["snapshot_head_hash"].as_str().map(str::len),
		Some(64)
	);
	assert!(report["legacy_rows"].as_i64().is_some());
	assert_eq!(report["checkpoint_status"].as_str(), Some("not_checked"));
	time::OffsetDateTime::parse(
		report["checked_at"].as_str().ok_or("missing checked_at")?,
		&time::format_description::well_known::Rfc3339,
	)?;

	let request = Request::builder()
		.method("GET")
		.uri("/api/audit-logs/verify-integrity")
		.header("cookie", cookie_header(&viewer_token.to_string()))
		.body(Body::empty())?;
	let response = app.clone().oneshot(request).await?;
	assert_eq!(response.status(), StatusCode::FORBIDDEN);
	let body = to_bytes(response.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert!(
		value.get("data").is_none(),
		"denial must not expose report: {value}"
	);
	let request = Request::builder()
		.method("GET")
		.uri("/api/audit-logs/verify-integrity?limit=0")
		.header("cookie", cookie_header(&viewer_token.to_string()))
		.body(Body::empty())?;
	let response = app.clone().oneshot(request).await?;
	assert_eq!(response.status(), StatusCode::FORBIDDEN);

	let request = Request::builder()
		.method("GET")
		.uri("/api/audit-logs/verify-integrity?limit=1")
		.header("cookie", cookie_header(&admin_token.to_string()))
		.body(Body::empty())?;
	let response = app.clone().oneshot(request).await?;
	assert_eq!(response.status(), StatusCode::OK);
	let body = to_bytes(response.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	let page = &value["data"];
	assert_eq!(page["total_rows"].as_i64(), Some(1));
	assert_eq!(page["has_more"].as_bool(), Some(total_rows > 1));
	assert_eq!(page["checkpoint_status"].as_str(), Some("not_checked"));
	assert_eq!(
		page["snapshot_head_id"].as_i64(),
		Some(snapshot_head_id),
		"pagination must keep the original snapshot boundary"
	);
	if total_rows > 1 {
		assert!(page["next_since_id"].as_i64().is_some());
		assert!(
			page["checked_through_id"]
				.as_i64()
				.ok_or("missing page checked_through_id")?
				< snapshot_head_id
		);
	}

	let request = Request::builder()
		.method("GET")
		.uri(format!(
			"/api/audit-logs/verify-integrity?since_id={snapshot_head_id}&until_id={snapshot_head_id}"
		))
		.header("cookie", cookie_header(&admin_token.to_string()))
		.body(Body::empty())?;
	let response = app.clone().oneshot(request).await?;
	assert_eq!(response.status(), StatusCode::OK);
	let body = to_bytes(response.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["total_rows"].as_i64(), Some(1));
	assert_eq!(
		value["data"]["checked_from_id"].as_i64(),
		Some(snapshot_head_id)
	);
	assert_eq!(
		value["data"]["checked_through_id"].as_i64(),
		Some(snapshot_head_id)
	);

	let request = Request::builder()
		.method("GET")
		.uri(format!(
			"/api/audit-logs/verify-integrity?since_id={}",
			snapshot_head_id + 1
		))
		.header("cookie", cookie_header(&admin_token.to_string()))
		.body(Body::empty())?;
	let response = app.clone().oneshot(request).await?;
	assert_eq!(response.status(), StatusCode::OK);
	let body = to_bytes(response.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["total_rows"].as_i64(), Some(0));
	assert_eq!(value["data"]["checked_from_id"], Value::Null);
	assert_eq!(value["data"]["checked_through_id"], Value::Null);
	assert_eq!(
		value["data"]["snapshot_head_id"].as_i64(),
		Some(snapshot_head_id)
	);
	assert_eq!(value["data"]["has_more"].as_bool(), Some(false));

	for query in [
		"limit=0",
		"limit=10001",
		"since_id=-1",
		"until_id=-1",
		"since_id=2&until_id=1",
	] {
		let request = Request::builder()
			.method("GET")
			.uri(format!("/api/audit-logs/verify-integrity?{query}"))
			.header("cookie", cookie_header(&admin_token.to_string()))
			.body(Body::empty())?;
		let response = app.clone().oneshot(request).await?;
		assert_eq!(response.status(), StatusCode::BAD_REQUEST, "query={query}");
	}
	let request = Request::builder()
		.method("GET")
		.uri("/api/audit-logs/verify-integrity?until_id=9223372036854775807")
		.header("cookie", cookie_header(&admin_token.to_string()))
		.body(Body::empty())?;
	let response = app.oneshot(request).await?;
	assert_eq!(
		response.status(),
		StatusCode::BAD_REQUEST,
		"missing until_id must map SQLSTATE 22023 to HTTP 400"
	);
	Ok(())
}
