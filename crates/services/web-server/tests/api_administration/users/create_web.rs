use crate::common::{
	cookie_header, create_empty_permission_profile, init_test_mm,
	seed_org_with_users, Result,
};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
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
			"pwd_clear": "p@ssw0rd-long-enough",
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

#[serial]
#[tokio::test]
async fn test_admin_create_user_defaults_missing_role_to_plain_user() -> Result<()> {
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

	assert_eq!(status, StatusCode::CREATED, "{json:?}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_create_user_missing_optional_fields_uses_backend_defaults(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&token.to_string());
	let suffix = Uuid::new_v4();

	let app = web_server::app(mm);
	let role_id = create_empty_permission_profile(
		&app,
		&admin_cookie,
		format!("Missing Fields Role {suffix}"),
	)
	.await?;
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
		.header("cookie", admin_cookie)
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
	assert_eq!(json["data"]["role"].as_str(), Some(role_id.as_str()));
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
			"pwd_clear": "p@ssw0rd-long-enough",
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
			"pwd_clear": "p@ssw0rd-long-enough",
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
