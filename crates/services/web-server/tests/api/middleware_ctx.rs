use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::Value;
use serial_test::serial;
use tower::ServiceExt;

#[serial]
#[tokio::test]
async fn test_ctx_resolve_sets_user() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;

	let app = web_server::app(mm);
	let req = Request::builder()
		.method("GET")
		.uri("/api/users/me")
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let body = axum::body::to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	let user_id = value
		.get("data")
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing data.id")?;
	assert_eq!(user_id, seed.admin.id.to_string());

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_ctx_no_leakage_between_requests() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;

	let app = web_server::app(mm);
	let req = Request::builder()
		.method("GET")
		.uri("/api/users/me")
		.header("cookie", cookie_header(&admin_token.to_string()))
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = axum::body::to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	let admin_id = value
		.get("data")
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing data.id")?;
	assert_eq!(admin_id, seed.admin.id.to_string());

	let req = Request::builder()
		.method("GET")
		.uri("/api/users/me")
		.header("cookie", cookie_header(&viewer_token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = axum::body::to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	let viewer_id = value
		.get("data")
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing data.id")?;
	assert_eq!(viewer_id, seed.viewer.id.to_string());

	Ok(())
}
