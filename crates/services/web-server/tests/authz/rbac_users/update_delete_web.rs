#![allow(unused_imports)]

use super::helpers::*;
use crate::common::{
	cookie_header, init_test_mm, insert_user, seed_org_with_all_roles,
	seed_org_with_users, seed_two_orgs_users_cases, system_user_id, Result,
};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::{
	Ctx, ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN,
};
use lib_core::model::user::{User, UserBmc, UserForUpdate};
use serde_json::json;
use serial_test::serial;
use time::{Duration, OffsetDateTime};
use tower::ServiceExt;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_sponsor_admin_can_hide_and_restore_blind_user() -> Result<()> {
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
	let default_list = Request::builder()
		.method("GET")
		.uri("/api/users")
		.header("cookie", admin_cookie.as_str())
		.body(Body::empty())?;
	let default_res = app.clone().oneshot(default_list).await?;
	let default_status = default_res.status();
	let default_body =
		axum::body::to_bytes(default_res.into_body(), usize::MAX).await?;
	assert_eq!(
		default_status,
		StatusCode::OK,
		"{}",
		String::from_utf8_lossy(&default_body)
	);
	let default_users: serde_json::Value = serde_json::from_slice(&default_body)?;
	assert!(!default_users["data"]
		.as_array()
		.unwrap()
		.iter()
		.any(|user| { user["id"].as_str() == Some(created_id) }));

	let include_list = Request::builder()
		.method("GET")
		.uri("/api/users?include_blinded=true")
		.header("cookie", admin_cookie.as_str())
		.body(Body::empty())?;
	let include_res = app.clone().oneshot(include_list).await?;
	let include_body =
		axum::body::to_bytes(include_res.into_body(), usize::MAX).await?;
	let included_users: serde_json::Value = serde_json::from_slice(&include_body)?;
	assert!(included_users["data"]
		.as_array()
		.unwrap()
		.iter()
		.any(|user| { user["id"].as_str() == Some(created_id) }));

	let update_body = json!({
		"data": {
			"access_blind_allowed": false
		}
	});
	let update_req = Request::builder()
		.method("PUT")
		.uri(format!("/api/users/{created_id}"))
		.header("cookie", admin_cookie.as_str())
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
	let restored_list = Request::builder()
		.method("GET")
		.uri("/api/users")
		.header("cookie", admin_cookie)
		.body(Body::empty())?;
	let restored_res = app.clone().oneshot(restored_list).await?;
	let restored_body =
		axum::body::to_bytes(restored_res.into_body(), usize::MAX).await?;
	let restored_users: serde_json::Value = serde_json::from_slice(&restored_body)?;
	assert!(restored_users["data"]
		.as_array()
		.unwrap()
		.iter()
		.any(|user| { user["id"].as_str() == Some(created_id) }));
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
async fn test_user_role_update_changes_the_normalized_assignment_atomically(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let role_id = create_empty_permission_profile(
		&app,
		&admin_cookie,
		format!("Assignment update {}", Uuid::new_v4()),
	)
	.await?;

	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/users/{}", seed.viewer.id))
		.header("cookie", admin_cookie)
		.header("content-type", "application/json")
		.body(Body::from(
			json!({ "data": { "role": role_id } }).to_string(),
		))?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let assigned_role = sqlx::query_scalar::<_, Uuid>(
		"SELECT role_id FROM user_role_assignments WHERE user_id = $1 AND organization_id = $2",
	)
	.bind(seed.viewer.id)
	.bind(seed.org_id)
	.fetch_one(mm.dbx().db())
	.await?;
	assert_eq!(assigned_role.to_string(), role_id);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sponsor_admin_cannot_assign_sponsor_admin_role() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;

	let app = web_server::app(mm);
	let body = json!({
		"data": {
			"role": ROLE_SPONSOR_ADMIN_CRO
		}
	});
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/users/{}", seed.viewer.id))
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sponsor_admin_cannot_update_existing_sponsor_admin_user() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;

	let app = web_server::app(mm);
	let body = json!({
		"data": {
			"comments": "attempted sponsor admin edit"
		}
	});
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/users/{}", seed.admin.id))
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.oneshot(req).await?;
	let status = res.status();
	let body = axum::body::to_bytes(res.into_body(), usize::MAX).await?;
	let json: serde_json::Value = serde_json::from_slice(&body)?;

	assert_eq!(status, StatusCode::FORBIDDEN, "{json:?}");
	assert!(json.to_string().contains("PERMISSION_DENIED"), "{json:?}");
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
	assert_eq!(res.status(), StatusCode::FORBIDDEN);
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
async fn test_expired_user_is_soft_deactivated() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	UserBmc::update(
		&admin_ctx,
		&mm,
		seed.viewer.id,
		UserForUpdate {
			organization_id: None,
			email: None,
			username: None,
			role: None,
			comments: None,
			other_information: None,
			access_start_at: None,
			access_end_at: Some(OffsetDateTime::now_utc() - Duration::seconds(1)),
			access_sender_ids: None,
			access_product_ids: None,
			access_study_ids: None,
			access_blind_allowed: None,
			active_sender_identifier: None,
			active: Some(true),
			last_login_at: None,
		},
	)
	.await?;

	assert!(UserBmc::deactivate_expired(&Ctx::root_ctx(), &mm).await? >= 1);
	let user: User = UserBmc::get(&admin_ctx, &mm, seed.viewer.id).await?;
	assert!(!user.active);
	Ok(())
}
