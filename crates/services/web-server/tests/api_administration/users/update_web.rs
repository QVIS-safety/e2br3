use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;
use tower::ServiceExt;

#[tokio::test]
async fn test_admin_can_update_user() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;

	let app = web_server::app(mm);
	let body = json!({
		"data": {
			"active": false
		}
	});
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/users/{}", seed.viewer.id))
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	Ok(())
}

#[tokio::test]
async fn test_admin_user_update_rejects_blank_name_and_clears_start_date(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let blank_name = Request::builder()
		.method("PUT")
		.uri(format!("/api/users/{}", seed.viewer.id))
		.header("cookie", cookie.as_str())
		.header("content-type", "application/json")
		.body(Body::from(
			json!({ "data": { "username": "   " } }).to_string(),
		))?;
	assert_eq!(
		app.clone().oneshot(blank_name).await?.status(),
		StatusCode::BAD_REQUEST
	);

	let set_date = Request::builder()
		.method("PUT")
		.uri(format!("/api/users/{}", seed.viewer.id))
		.header("cookie", cookie.as_str())
		.header("content-type", "application/json")
		.body(Body::from(
			json!({ "data": { "access_start_at": "2026-03-02T00:00:00Z" } })
				.to_string(),
		))?;
	assert_eq!(
		app.clone().oneshot(set_date).await?.status(),
		StatusCode::OK
	);

	let clear_date = Request::builder()
		.method("PUT")
		.uri(format!("/api/users/{}", seed.viewer.id))
		.header("cookie", cookie.as_str())
		.header("content-type", "application/json")
		.body(Body::from(
			json!({ "data": { "access_start_at": null } }).to_string(),
		))?;
	let response = app.oneshot(clear_date).await?;
	assert_eq!(response.status(), StatusCode::OK);
	let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
	let value: serde_json::Value = serde_json::from_slice(&body)?;
	assert!(
		value["data"]["scope"]["accessStartAt"].is_null(),
		"{value:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_update_user_rejects_overlong_username_and_email() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let app = web_server::app(mm);

	for (payload, expected_detail) in [
		(
			json!({ "data": { "username": "U".repeat(129) } }),
			"username must be 128 characters or fewer",
		),
		(
			json!({ "data": { "email": format!("{}@example.com", "e".repeat(244)) } }),
			"email must be 255 characters or fewer",
		),
	] {
		let req = Request::builder()
			.method("PUT")
			.uri(format!("/api/users/{}", seed.viewer.id))
			.header("cookie", cookie_header(&token.to_string()))
			.header("content-type", "application/json")
			.body(Body::from(payload.to_string()))?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(res.status(), StatusCode::BAD_REQUEST);
		let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await?;
		let value: serde_json::Value = serde_json::from_slice(&bytes)?;
		assert_eq!(value["error"]["data"]["detail"], expected_detail);
	}

	Ok(())
}
