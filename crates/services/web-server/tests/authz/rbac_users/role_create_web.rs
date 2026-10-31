use crate::common::{
	cookie_header, init_test_mm, insert_user, seed_org_with_users, system_user_id,
	Result,
};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::{ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN};
use serde_json::json;
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_admin_create_user_allows_plain_user_role() -> Result<()> {
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

	assert_eq!(status, StatusCode::CREATED, "{json:?}");
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

	assert_eq!(status, StatusCode::FORBIDDEN, "{json:?}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_system_admin_create_user_rejects_second_sponsor_admin_for_org(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let system_admin = insert_user(
		&mm,
		seed.org_id,
		ROLE_SYSTEM_ADMIN,
		system_user_id(),
		Some("systempwd"),
	)
	.await?;
	let token = generate_web_token(&system_admin.email, system_admin.token_salt)?;

	let app = web_server::app(mm);
	let suffix = Uuid::new_v4();
	let body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": format!("second-sponsor-admin-{suffix}@example.com"),
			"username": format!("second_sponsor_admin_{suffix}"),
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
	assert!(
		json.to_string().contains("Only one Sponsor Administrator"),
		"{json:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_create_user_rejects_role_when_org_has_no_permission_profile(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;

	let app = web_server::app(mm);
	let suffix = Uuid::new_v4();
	let body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": format!("rbac-admin-create-no-profile-{suffix}@example.com"),
			"username": format!("rbac_admin_create_no_profile_{suffix}"),
			"role": Uuid::new_v4().to_string()
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
	assert!(
		json.to_string()
			.contains("permission profile must be registered"),
		"{json:?}"
	);
	Ok(())
}
