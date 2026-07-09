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
async fn test_viewer_cannot_create_user() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;

	let app = web_server::app(mm);
	let suffix = Uuid::new_v4();
	let body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": format!("rbac-viewer-create-{suffix}@example.com"),
			"username": format!("rbac_viewer_create_{suffix}"),
			"pwd_clear": "p@ssw0rd",
			"role": "case_reviewer"
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);
	let body = axum::body::to_bytes(res.into_body(), usize::MAX).await?;
	let json: serde_json::Value = serde_json::from_slice(&body)?;
	json["error"]["data"]["detail"]
		.as_str()
		.ok_or("expected string detail for PERMISSION_DENIED")?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_viewer_cannot_update_user() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;

	let app = web_server::app(mm);
	let body = json!({
		"data": {
			"role": "sponsor_admin_company",
			"active": false
		}
	});
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/users/{}", seed.admin.id))
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_viewer_cannot_delete_user() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;

	let app = web_server::app(mm);
	let req = Request::builder()
		.method("DELETE")
		.uri(format!("/api/users/{}", seed.admin.id))
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_non_admin_cannot_list_users() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let app = web_server::app(mm);

	let manager_token =
		generate_web_token(&seed.manager.email, seed.manager.token_salt)?;
	let user_token = generate_web_token(&seed.user.email, seed.user.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;

	for (role, token) in [
		("manager", manager_token),
		("user", user_token),
		("viewer", viewer_token),
	] {
		let req = Request::builder()
			.method("GET")
			.uri("/api/users")
			.header("cookie", cookie_header(&token.to_string()))
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(
			res.status(),
			StatusCode::FORBIDDEN,
			"{role} should be forbidden from listing users"
		);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_non_admin_can_list_lightweight_workflow_user_options() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let app = web_server::app(mm);
	let token = generate_web_token(&seed.user.email, seed.user.token_salt)?;

	let req = Request::builder()
		.method("GET")
		.uri("/api/users/workflow-options?limit=25")
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let json: serde_json::Value = serde_json::from_slice(&body)?;
	let users = json["data"]
		.as_array()
		.ok_or("expected workflow user option array")?;
	let expected_user_id = seed.user.id.to_string();
	assert!(
		users
			.iter()
			.any(|user| user["id"].as_str() == Some(expected_user_id.as_str())),
		"expected current org user in workflow options: {json:?}"
	);
	assert!(
		users.iter().all(|user| user.get("comments").is_none()
			&& user.get("scope").is_none()
			&& user.get("roleMeta").is_none()),
		"workflow options should stay lightweight: {json:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_non_admin_cannot_get_user() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let app = web_server::app(mm);

	let manager_token =
		generate_web_token(&seed.manager.email, seed.manager.token_salt)?;
	let user_token = generate_web_token(&seed.user.email, seed.user.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;

	for (role, token) in [
		("manager", manager_token),
		("user", user_token),
		("viewer", viewer_token),
	] {
		let req = Request::builder()
			.method("GET")
			.uri(format!("/api/users/{}", seed.admin.id))
			.header("cookie", cookie_header(&token.to_string()))
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(
			res.status(),
			StatusCode::FORBIDDEN,
			"{role} should be forbidden from getting user by id"
		);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_non_admin_cannot_create_user() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let app = web_server::app(mm);

	let manager_token =
		generate_web_token(&seed.manager.email, seed.manager.token_salt)?;
	let user_token = generate_web_token(&seed.user.email, seed.user.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;

	for (role, token) in [
		("manager", manager_token),
		("user", user_token),
		("viewer", viewer_token),
	] {
		let suffix = Uuid::new_v4();
		let body = json!({
			"data": {
				"organization_id": seed.org_id,
				"email": format!("rbac-{role}-create-{suffix}@example.com"),
				"username": format!("rbac_{role}_create_{suffix}"),
				"pwd_clear": "p@ssw0rd",
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
		assert_eq!(
			res.status(),
			StatusCode::FORBIDDEN,
			"{role} should be forbidden from creating users"
		);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_non_admin_cannot_update_user() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let app = web_server::app(mm);

	let manager_token =
		generate_web_token(&seed.manager.email, seed.manager.token_salt)?;
	let user_token = generate_web_token(&seed.user.email, seed.user.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;

	for (role, token) in [
		("manager", manager_token),
		("user", user_token),
		("viewer", viewer_token),
	] {
		let body = json!({
			"data": {
				"role": "sponsor_admin_company",
				"active": false
			}
		});
		let req = Request::builder()
			.method("PUT")
			.uri(format!("/api/users/{}", seed.admin.id))
			.header("cookie", cookie_header(&token.to_string()))
			.header("content-type", "application/json")
			.body(Body::from(body.to_string()))?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(
			res.status(),
			StatusCode::FORBIDDEN,
			"{role} should be forbidden from updating users"
		);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_non_admin_cannot_delete_user() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let app = web_server::app(mm);

	let manager_token =
		generate_web_token(&seed.manager.email, seed.manager.token_salt)?;
	let user_token = generate_web_token(&seed.user.email, seed.user.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;

	for (role, token) in [
		("manager", manager_token),
		("user", user_token),
		("viewer", viewer_token),
	] {
		let req = Request::builder()
			.method("DELETE")
			.uri(format!("/api/users/{}", seed.admin.id))
			.header("cookie", cookie_header(&token.to_string()))
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(
			res.status(),
			StatusCode::FORBIDDEN,
			"{role} should be forbidden from deleting users"
		);
	}

	Ok(())
}
