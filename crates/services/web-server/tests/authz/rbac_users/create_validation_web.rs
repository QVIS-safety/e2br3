#![allow(unused_imports)]

use super::helpers::*;
use crate::common::{
	cookie_header, init_test_mm, insert_user, seed_org_with_all_roles,
	seed_org_with_users, seed_two_orgs_users_cases, system_user_id, Result,
};
use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::{
	ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN,
};
use serde_json::json;
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_admin_can_create_user() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&token.to_string());

	let app = web_server::app(mm);
	let suffix = Uuid::new_v4();
	let role_id = create_empty_permission_profile(
		&app,
		&admin_cookie,
		format!("Create User Role {suffix}"),
	)
	.await?;
	let body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": format!("rbac-admin-create-{suffix}@example.com"),
			"username": format!("rbac_admin_create_{suffix}"),
			"pwd_clear": "p@ssw0rd",
			"role": role_id,
			"access_blind_allowed": true
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", admin_cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.oneshot(req).await?;
	let status = res.status();
	if status != StatusCode::CREATED {
		let body = axum::body::to_bytes(res.into_body(), usize::MAX).await?;
		return Err(format!(
			"create user status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	Ok(())
}

#[tokio::test]
async fn test_admin_create_user_rejects_plain_user_role() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;

	let app = web_server::app(mm);
	let suffix = Uuid::new_v4();
	let body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": format!("rbac-admin-create-plain-user-{suffix}@example.com"),
			"username": format!("rbac_admin_create_plain_user_{suffix}"),
			"role": "user"
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.oneshot(req).await?;
	let status = res.status();
	let body = axum::body::to_bytes(res.into_body(), usize::MAX).await?;
	let json: serde_json::Value = serde_json::from_slice(&body)?;

	assert_eq!(status, StatusCode::BAD_REQUEST, "{json:?}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sponsor_admin_create_user_rejects_sponsor_admin_role() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;

	let app = web_server::app(mm);
	let suffix = Uuid::new_v4();
	let body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": format!("rbac-admin-create-sponsor-admin-{suffix}@example.com"),
			"username": format!("rbac_admin_create_sponsor_admin_{suffix}"),
			"role": ROLE_SPONSOR_ADMIN_CRO
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.oneshot(req).await?;
	let status = res.status();
	let body = axum::body::to_bytes(res.into_body(), usize::MAX).await?;
	let json: serde_json::Value = serde_json::from_slice(&body)?;

	assert_eq!(status, StatusCode::BAD_REQUEST, "{json:?}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_create_user_rejects_missing_role() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;

	let app = web_server::app(mm);
	let suffix = Uuid::new_v4();
	let body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": format!("rbac-admin-create-missing-role-{suffix}@example.com"),
			"username": format!("rbac_admin_create_missing_role_{suffix}")
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.oneshot(req).await?;
	let status = res.status();
	let body = axum::body::to_bytes(res.into_body(), usize::MAX).await?;
	let json: serde_json::Value = serde_json::from_slice(&body)?;

	assert_eq!(status, StatusCode::BAD_REQUEST, "{json:?}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_create_user_missing_optional_fields_uses_backend_defaults(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let suffix = Uuid::new_v4();

	let app = web_server::app(mm);
	let role_req = Request::builder()
		.method("POST")
		.uri("/api/admin/permission-profiles")
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(
			json!({
				"data": {
					"name": format!("Missing Fields Role {suffix}"),
					"privileges": []
				}
			})
			.to_string(),
		))?;
	let role_res = app.clone().oneshot(role_req).await?;
	assert_eq!(role_res.status(), StatusCode::CREATED);
	let role_body = to_bytes(role_res.into_body(), usize::MAX).await?;
	let role_json: serde_json::Value = serde_json::from_slice(&role_body)?;
	let role_id = role_json["id"].as_str().ok_or("missing role id")?;
	// Mirrors the currently documented frontend payload shape that omits
	// required fields for backend UserForCreate.
	let body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": format!("missing-fields-{suffix}@example.com"),
			"role": role_id
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.oneshot(req).await?;
	let status = res.status();
	let body = axum::body::to_bytes(res.into_body(), usize::MAX).await?;
	let json: serde_json::Value = serde_json::from_slice(&body)?;

	assert_eq!(status, StatusCode::CREATED, "{json:?}");
	assert_eq!(
		json["data"]["email"].as_str(),
		Some(format!("missing-fields-{suffix}@example.com").as_str())
	);
	assert_eq!(json["data"]["role"].as_str(), Some(role_id));
	assert!(
		json["data"]["username"]
			.as_str()
			.map(|value| !value.trim().is_empty())
			.unwrap_or(false),
		"{json:?}"
	);
	Ok(())
}

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
		"pwd": "welcome"
	});
	let login_req = Request::builder()
		.method("POST")
		.uri("/auth/v1/login")
		.header("content-type", "application/json")
		.body(Body::from(login_body.to_string()))?;
	let login_res = app.clone().oneshot(login_req).await?;
	assert_eq!(login_res.status(), StatusCode::OK);
	let auth_cookie = login_res
		.headers()
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

	let me_req = Request::builder()
		.method("GET")
		.uri("/api/users/me")
		.header("cookie", auth_cookie.as_str())
		.body(Body::empty())?;
	let me_res = app.clone().oneshot(me_req).await?;
	assert_eq!(me_res.status(), StatusCode::OK);
	let me_bytes = axum::body::to_bytes(me_res.into_body(), usize::MAX).await?;
	let me_json: serde_json::Value = serde_json::from_slice(&me_bytes)?;
	assert_eq!(me_json["data"]["mustChangePassword"].as_bool(), Some(true));

	let set_pwd_body = json!({ "data": { "new_password": "new_password_123" } });
	let set_pwd_req = Request::builder()
		.method("POST")
		.uri("/api/users/me/password")
		.header("cookie", auth_cookie.as_str())
		.header("content-type", "application/json")
		.body(Body::from(set_pwd_body.to_string()))?;
	let set_pwd_res = app.clone().oneshot(set_pwd_req).await?;
	assert_eq!(set_pwd_res.status(), StatusCode::NO_CONTENT);

	let old_login_body = json!({
		"email": email,
		"pwd": "welcome"
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
		"pwd": "new_password_123"
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

#[serial]
#[tokio::test]
async fn test_create_user_duplicate_email_returns_conflict_with_detail() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&token.to_string());

	let app = web_server::app(mm);
	let suffix = Uuid::new_v4();
	let email = format!("rbac-dup-email-{suffix}@example.com");
	let role_id = create_empty_permission_profile(
		&app,
		&admin_cookie,
		format!("Duplicate Email Role {suffix}"),
	)
	.await?;

	let body1 = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": email,
			"username": format!("rbac_dup_email_1_{suffix}"),
			"pwd_clear": "p@ssw0rd",
			"role": role_id
		}
	});
	let req1 = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", admin_cookie.as_str())
		.header("content-type", "application/json")
		.body(Body::from(body1.to_string()))?;
	let res1 = app.clone().oneshot(req1).await?;
	assert_eq!(res1.status(), StatusCode::CREATED);

	let body2 = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": email,
			"username": format!("rbac_dup_email_2_{suffix}"),
			"pwd_clear": "p@ssw0rd",
			"role": role_id
		}
	});
	let req2 = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", admin_cookie)
		.header("content-type", "application/json")
		.body(Body::from(body2.to_string()))?;
	let res2 = app.oneshot(req2).await?;
	assert_eq!(res2.status(), StatusCode::CONFLICT);

	let body = axum::body::to_bytes(res2.into_body(), usize::MAX).await?;
	let json: serde_json::Value = serde_json::from_slice(&body)?;
	assert!(
		json["error"]["data"]["detail"]
			.as_str()
			.unwrap_or_default()
			.to_ascii_lowercase()
			.contains("already exists")
			|| json["error"]["data"]["detail"]
				.as_str()
				.unwrap_or_default()
				.to_ascii_lowercase()
				.contains("duplicate"),
		"expected safe conflict detail, body={json}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_create_user_rejects_overlong_username_and_email() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let app = web_server::app(mm);
	let suffix = Uuid::new_v4();

	for (email, username, expected_detail) in [
		(
			format!("valid-limit-{suffix}@example.com"),
			"U".repeat(129),
			"username must be 128 characters or fewer",
		),
		(
			format!("{}@example.com", "e".repeat(244)),
			format!("valid_user_{suffix}"),
			"email must be 255 characters or fewer",
		),
	] {
		let body = json!({
			"data": {
				"email": email,
				"username": username,
				"role": "case_reviewer"
			}
		});
		let req = Request::builder()
			.method("POST")
			.uri("/api/users")
			.header("cookie", cookie_header(&token.to_string()))
			.header("content-type", "application/json")
			.body(Body::from(body.to_string()))?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(res.status(), StatusCode::BAD_REQUEST);
		let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await?;
		let value: serde_json::Value = serde_json::from_slice(&bytes)?;
		assert_eq!(value["error"]["data"]["detail"], expected_detail);
	}

	Ok(())
}
