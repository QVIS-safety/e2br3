use crate::common::{
	cookie_header, init_test_mm, insert_user, seed_org_with_users, system_user_id,
	Result,
};
use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SYSTEM_ADMIN;
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;

async fn request_json(
	app: &Router,
	cookie: &str,
	method: Method,
	uri: &str,
	body: Option<Value>,
) -> Result<(StatusCode, Value)> {
	let mut builder = Request::builder()
		.method(method)
		.uri(uri)
		.header("cookie", cookie);
	if body.is_some() {
		builder = builder.header("content-type", "application/json");
	}
	let req =
		builder.body(Body::from(body.map(|v| v.to_string()).unwrap_or_default()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let bytes = to_bytes(res.into_body(), usize::MAX).await?;
	let value = serde_json::from_slice(&bytes)
		.unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&bytes) }));
	Ok((status, value))
}

#[serial]
#[tokio::test]
async fn test_idle_session_settings_are_system_level_and_validated() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"idle_session_minutes": 30,
				"session_warning_minutes": 10
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["idle_session_minutes"], 30);
	assert_eq!(value["session_warning_minutes"], 10);

	let (status, value) = request_json(
		&app,
		&viewer_cookie,
		Method::GET,
		"/api/admin/settings",
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["idle_session_minutes"], 30);
	assert_eq!(value["session_warning_minutes"], 10);

	let (status, _value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"idle_session_minutes": 4,
				"session_warning_minutes": 1
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST);

	let (status, _value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"idle_session_minutes": 30,
				"session_warning_minutes": 30
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_dashboard_notices_are_system_admin_written_and_user_readable(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let system_admin =
		insert_user(&mm, seed.org_id, ROLE_SYSTEM_ADMIN, system_user_id(), None)
			.await?;
	let sponsor_admin_token =
		generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let system_admin_token =
		generate_web_token(&system_admin.email, system_admin.token_salt)?;
	let sponsor_admin_cookie = cookie_header(&sponsor_admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let system_admin_cookie = cookie_header(&system_admin_token.to_string());
	let app = web_server::app(mm);

	let (status, _value) = request_json(
		&app,
		&sponsor_admin_cookie,
		Method::PUT,
		"/api/admin/notices",
		Some(json!({
			"data": {
				"notices": [{
					"id": "notice-1",
					"title": "Sponsor admin should not save",
					"body": "Blocked",
					"effective_date": "2026-05-14"
				}]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN);

	let (status, value) = request_json(
		&app,
		&system_admin_cookie,
		Method::PUT,
		"/api/admin/notices",
		Some(json!({
			"data": {
				"notices": [{
					"id": "notice-1",
					"title": "System maintenance",
					"body": "System maintenance at 18:00.",
					"effective_date": "2026-05-14",
					"expire_date": "2026-05-20"
				}]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["notices"][0]["title"], "System maintenance");

	let (status, value) = request_json(
		&app,
		&viewer_cookie,
		Method::GET,
		"/api/admin/settings",
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["notices"][0]["title"], "System maintenance");
	assert_eq!(value["notices"][0]["writer"], system_admin.email);

	Ok(())
}
