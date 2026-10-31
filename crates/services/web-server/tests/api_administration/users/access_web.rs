use crate::common::{
	cookie_header, create_empty_permission_profile, init_test_mm,
	seed_org_with_users, Result,
};
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_new_user_temp_password_and_first_login_reset_flow() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let suffix = Uuid::new_v4();
	let email = format!("first-login-{suffix}@example.com");
	let username = format!("first_login_{suffix}");
	let temporary_password = "welcome";
	let new_password = "new-password-456789";
	let role_id = create_empty_permission_profile(
		&app,
		&admin_cookie,
		format!("First Login Role {suffix}"),
	)
	.await?;

	let create_body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": email,
			"username": username,
			"role": role_id
		}
	});
	let create_req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", admin_cookie.as_str())
		.header("content-type", "application/json")
		.body(Body::from(create_body.to_string()))?;
	let create_res = app.clone().oneshot(create_req).await?;
	assert_eq!(create_res.status(), StatusCode::CREATED);

	let login_body = json!({
		"email": email,
		"pwd": temporary_password
	});
	let login_req = Request::builder()
		.method("POST")
		.uri("/auth/v1/login")
		.header("content-type", "application/json")
		.body(Body::from(login_body.to_string()))?;
	let login_res = app.clone().oneshot(login_req).await?;
	assert_eq!(login_res.status(), StatusCode::OK);
	let login_headers = login_res.headers().clone();
	let login_bytes =
		axum::body::to_bytes(login_res.into_body(), usize::MAX).await?;
	let login_json: serde_json::Value = serde_json::from_slice(&login_bytes)?;
	assert_eq!(
		login_json["result"]["must_change_password"].as_bool(),
		Some(true)
	);
	let auth_cookie = login_headers
		.get_all(header::SET_COOKIE)
		.iter()
		.filter_map(|val| val.to_str().ok())
		.find_map(|cookie| {
			if cookie.starts_with("auth-token=") {
				Some(cookie.split(';').next().unwrap_or_default().to_string())
			} else {
				None
			}
		})
		.ok_or("missing auth-token cookie after login")?;

	let profile_req = Request::builder()
		.method("GET")
		.uri("/api/users/me/profile")
		.header("cookie", auth_cookie.as_str())
		.body(Body::empty())?;
	let profile_res = app.clone().oneshot(profile_req).await?;
	assert_eq!(profile_res.status(), StatusCode::OK);
	let profile_bytes =
		axum::body::to_bytes(profile_res.into_body(), usize::MAX).await?;
	let profile_json: serde_json::Value = serde_json::from_slice(&profile_bytes)?;
	assert_eq!(
		profile_json["data"]["user"]["mustChangePassword"].as_bool(),
		Some(true)
	);

	let weak_password_body = json!({
		"data": {
			"new_password": "short"
		}
	});
	let weak_password_req = Request::builder()
		.method("POST")
		.uri("/api/users/me/password")
		.header("cookie", auth_cookie.as_str())
		.header("content-type", "application/json")
		.body(Body::from(weak_password_body.to_string()))?;
	let weak_password_res = app.clone().oneshot(weak_password_req).await?;
	assert_eq!(weak_password_res.status(), StatusCode::BAD_REQUEST);

	let set_pwd_body = json!({
		"data": {
			"new_password": new_password
		}
	});
	let set_pwd_req = Request::builder()
		.method("POST")
		.uri("/api/users/me/password")
		.header("cookie", auth_cookie.as_str())
		.header("content-type", "application/json")
		.body(Body::from(set_pwd_body.to_string()))?;
	let set_pwd_res = app.clone().oneshot(set_pwd_req).await?;
	assert_eq!(set_pwd_res.status(), StatusCode::NO_CONTENT);

	let stale_session_req = Request::builder()
		.method("GET")
		.uri("/api/users/me")
		.header("cookie", auth_cookie.as_str())
		.body(Body::empty())?;
	let stale_session_res = app.clone().oneshot(stale_session_req).await?;
	assert_eq!(stale_session_res.status(), StatusCode::FORBIDDEN);

	let old_login_body = json!({
		"email": email,
		"pwd": temporary_password
	});
	let old_login_req = Request::builder()
		.method("POST")
		.uri("/auth/v1/login")
		.header("content-type", "application/json")
		.body(Body::from(old_login_body.to_string()))?;
	let old_login_res = app.clone().oneshot(old_login_req).await?;
	assert_eq!(old_login_res.status(), StatusCode::FORBIDDEN);

	let new_login_body = json!({
		"email": email,
		"pwd": new_password
	});
	let new_login_req = Request::builder()
		.method("POST")
		.uri("/auth/v1/login")
		.header("content-type", "application/json")
		.body(Body::from(new_login_body.to_string()))?;
	let new_login_res = app.clone().oneshot(new_login_req).await?;
	assert_eq!(new_login_res.status(), StatusCode::OK);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_user_after_access_end_is_inactive_and_cannot_login() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let suffix = Uuid::new_v4();
	let email = format!("expired-user-{suffix}@example.com");
	let username = format!("expired_user_{suffix}");
	let role_id = create_empty_permission_profile(
		&app,
		&admin_cookie,
		format!("Expired User Role {suffix}"),
	)
	.await?;
	let create_body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": email,
			"username": username,
			"role": role_id,
			"access_end_at": "2000-01-01T00:00:00Z"
		}
	});
	let create_req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", admin_cookie)
		.header("content-type", "application/json")
		.body(Body::from(create_body.to_string()))?;
	let create_res = app.clone().oneshot(create_req).await?;
	let create_status = create_res.status();
	let create_bytes =
		axum::body::to_bytes(create_res.into_body(), usize::MAX).await?;
	assert_eq!(
		create_status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&create_bytes)
	);
	let created: serde_json::Value = serde_json::from_slice(&create_bytes)?;
	assert_eq!(
		created["data"]["active"].as_bool(),
		Some(false),
		"{created:?}"
	);

	let login_body = json!({
		"email": email,
		"pwd": "welcome"
	});
	let login_req = Request::builder()
		.method("POST")
		.uri("/auth/v1/login")
		.header("content-type", "application/json")
		.body(Body::from(login_body.to_string()))?;
	let login_res = app.oneshot(login_req).await?;
	assert_eq!(login_res.status(), StatusCode::FORBIDDEN);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_create_user_persists_access_start_and_end_dates() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let suffix = Uuid::new_v4();
	let email = format!("access-window-{suffix}@example.com");
	let username = format!("access_window_{suffix}");
	let role_id = create_empty_permission_profile(
		&app,
		&admin_cookie,
		format!("Access Window Role {suffix}"),
	)
	.await?;
	let create_body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": email,
			"username": username,
			"role": role_id,
			"access_start_at": "2026-01-01T00:00:00.000Z",
			"access_end_at": "2026-12-31T23:59:00.000Z"
		}
	});
	let create_req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", admin_cookie)
		.header("content-type", "application/json")
		.body(Body::from(create_body.to_string()))?;
	let create_res = app.oneshot(create_req).await?;
	let create_status = create_res.status();
	let create_bytes =
		axum::body::to_bytes(create_res.into_body(), usize::MAX).await?;
	assert_eq!(
		create_status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&create_bytes)
	);
	let created: serde_json::Value = serde_json::from_slice(&create_bytes)?;
	assert_eq!(
		created["data"]["scope"]["accessStartAt"].as_str(),
		Some("2026-01-01T00:00:00Z"),
		"{created:?}"
	);
	assert_eq!(
		created["data"]["scope"]["accessEndAt"].as_str(),
		Some("2026-12-31T23:59:00Z"),
		"{created:?}"
	);

	Ok(())
}
