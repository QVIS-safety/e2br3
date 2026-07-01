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

#[serial]
#[tokio::test]
async fn test_sponsor_admin_can_set_and_persist_blind_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&token.to_string());

	let app = web_server::app(mm);
	let suffix = Uuid::new_v4();
	let role_id = create_empty_permission_profile(
		&app,
		&admin_cookie,
		format!("Blind Scope Role {suffix}"),
	)
	.await?;
	let create_body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": format!("blind-scope-{suffix}@example.com"),
			"username": format!("blind_scope_{suffix}"),
			"role": role_id,
			"access_blind_allowed": true
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
	let create_bytes =
		axum::body::to_bytes(create_res.into_body(), usize::MAX).await?;
	let created: serde_json::Value = serde_json::from_slice(&create_bytes)?;
	let created_id = created["data"]["id"]
		.as_str()
		.ok_or("missing created user id")?;
	assert_eq!(
		created["data"]["scope"]["accessBlindAllowed"].as_bool(),
		Some(true),
		"{created:?}"
	);

	let update_body = json!({
		"data": {
			"access_blind_allowed": false
		}
	});
	let update_req = Request::builder()
		.method("PUT")
		.uri(format!("/api/users/{created_id}"))
		.header("cookie", admin_cookie)
		.header("content-type", "application/json")
		.body(Body::from(update_body.to_string()))?;
	let update_res = app.clone().oneshot(update_req).await?;
	assert_eq!(update_res.status(), StatusCode::OK);
	let update_bytes =
		axum::body::to_bytes(update_res.into_body(), usize::MAX).await?;
	let updated: serde_json::Value = serde_json::from_slice(&update_bytes)?;
	assert_eq!(
		updated["data"]["scope"]["accessBlindAllowed"].as_bool(),
		Some(false),
		"{updated:?}"
	);
	Ok(())
}

#[tokio::test]
async fn test_admin_can_update_user() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;

	let app = web_server::app(mm);
	let body = json!({
		"data": {
			"role": ROLE_SPONSOR_ADMIN_CRO,
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

#[serial]
#[tokio::test]
async fn test_update_user_rejects_sponsor_admin_role_for_wrong_org_type(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;

	let app = web_server::app(mm);
	let body = json!({
		"data": {
			"role": ROLE_SPONSOR_ADMIN_COMPANY
		}
	});
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/users/{}", seed.viewer.id))
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::BAD_REQUEST);
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

#[serial]
#[tokio::test]
async fn test_admin_can_delete_user() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());

	let app = web_server::app(mm);
	let req = Request::builder()
		.method("DELETE")
		.uri(format!("/api/users/{}", seed.viewer.id))
		.header("cookie", cookie.as_str())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::NO_CONTENT);

	let get_req = Request::builder()
		.method("GET")
		.uri(format!("/api/users/{}", seed.viewer.id))
		.header("cookie", cookie.as_str())
		.body(Body::empty())?;
	let get_res = app.clone().oneshot(get_req).await?;
	assert_eq!(get_res.status(), StatusCode::OK);
	let get_bytes = to_bytes(get_res.into_body(), usize::MAX).await?;
	let deleted_user: serde_json::Value = serde_json::from_slice(&get_bytes)?;
	assert_eq!(
		deleted_user["data"]["active"].as_bool(),
		Some(false),
		"deleted user should be retained as inactive: {deleted_user}"
	);

	let restore_body = json!({ "data": { "active": true } });
	let restore_req = Request::builder()
		.method("PUT")
		.uri(format!("/api/users/{}", seed.viewer.id))
		.header("cookie", cookie.as_str())
		.header("content-type", "application/json")
		.body(Body::from(restore_body.to_string()))?;
	let restore_res = app.clone().oneshot(restore_req).await?;
	assert_eq!(restore_res.status(), StatusCode::OK);
	let restore_bytes = to_bytes(restore_res.into_body(), usize::MAX).await?;
	let restored_user: serde_json::Value = serde_json::from_slice(&restore_bytes)?;
	assert_eq!(
		restored_user["data"]["active"].as_bool(),
		Some(true),
		"restored user should be active again: {restored_user}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_cannot_delete_self() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;

	let app = web_server::app(mm);
	let req = Request::builder()
		.method("DELETE")
		.uri(format!("/api/users/{}", seed.admin.id))
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::BAD_REQUEST);
	Ok(())
}
