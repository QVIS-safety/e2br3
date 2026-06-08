#![allow(unused_imports)]

use super::helpers::*;
use crate::common::{
	cookie_header, init_test_mm, insert_user, seed_org_with_all_roles,
	seed_org_with_users, seed_two_orgs_users_cases, system_user_id, Result,
};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::{
	ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN,
};
use serde_json::json;
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

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
