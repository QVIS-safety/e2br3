use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::Value;
use serial_test::serial;
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
	time::OffsetDateTime::parse(
		report["checked_at"].as_str().ok_or("missing checked_at")?,
		&time::format_description::well_known::Rfc3339,
	)?;

	let request = Request::builder()
		.method("GET")
		.uri("/api/audit-logs/verify-integrity")
		.header("cookie", cookie_header(&viewer_token.to_string()))
		.body(Body::empty())?;
	let response = app.oneshot(request).await?;
	assert_eq!(response.status(), StatusCode::FORBIDDEN);
	let body = to_bytes(response.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert!(
		value.get("data").is_none(),
		"denial must not expose report: {value}"
	);
	Ok(())
}
