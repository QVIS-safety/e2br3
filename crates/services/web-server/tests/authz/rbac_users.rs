use crate::common::{
	cookie_header, init_test_mm, seed_org_with_all_roles, seed_org_with_users,
	Result,
};
use axum::body::Body;
use axum::http::header;
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

	let app = web_server::app(mm);
	let suffix = Uuid::new_v4();
	let body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": format!("rbac-admin-create-{suffix}@example.com"),
			"username": format!("rbac_admin_create_{suffix}"),
			"pwd_clear": "p@ssw0rd",
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
async fn test_new_user_temp_password_and_first_login_reset_flow() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let app = web_server::app(mm);

	let suffix = Uuid::new_v4();
	let email = format!("first-login-{suffix}@example.com");
	let username = format!("first_login_{suffix}");

	let create_body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": email,
			"username": username,
			"role": "user"
		}
	});
	let create_req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", cookie_header(&admin_token.to_string()))
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
	assert_eq!(
		me_json["data"]["must_change_password"].as_bool(),
		Some(true)
	);

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
async fn test_create_user_missing_optional_fields_uses_backend_defaults(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let suffix = Uuid::new_v4();

	let app = web_server::app(mm);
	// Mirrors the currently documented frontend payload shape that omits
	// required fields for backend UserForCreate.
	let body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": format!("missing-fields-{suffix}@example.com"),
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
	assert_eq!(
		json["data"]["email"].as_str(),
		Some(format!("missing-fields-{suffix}@example.com").as_str())
	);
	assert_eq!(json["data"]["role"].as_str(), Some("user"));
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

	let app = web_server::app(mm);
	let suffix = Uuid::new_v4();
	let email = format!("rbac-dup-email-{suffix}@example.com");

	let body1 = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": email,
			"username": format!("rbac_dup_email_1_{suffix}"),
			"pwd_clear": "p@ssw0rd",
			"role": "user"
		}
	});
	let req1 = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", cookie_header(&token.to_string()))
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
			"role": "user"
		}
	});
	let req2 = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", cookie_header(&token.to_string()))
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
async fn test_admin_create_user_nil_org_id_uses_request_context_org() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;

	let app = web_server::app(mm);
	let suffix = Uuid::new_v4();
	let body = json!({
		"data": {
			"organization_id": Uuid::nil(),
			"email": format!("rbac-nil-org-{suffix}@example.com"),
			"username": format!("rbac_nil_org_{suffix}"),
			"pwd_clear": "p@ssw0rd",
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
	assert_eq!(res.status(), StatusCode::CREATED);

	let body = axum::body::to_bytes(res.into_body(), usize::MAX).await?;
	let json: serde_json::Value = serde_json::from_slice(&body)?;
	let created_org = json["data"]["organization_id"]
		.as_str()
		.ok_or("missing created user organization_id")?;
	assert_eq!(created_org, seed.org_id.to_string());
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_can_update_user() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;

	let app = web_server::app(mm);
	let body = json!({
		"data": {
			"role": "admin",
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
async fn test_viewer_cannot_update_user() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;

	let app = web_server::app(mm);
	let body = json!({
		"data": {
			"role": "admin",
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
async fn test_admin_can_delete_user() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;

	let app = web_server::app(mm);
	let req = Request::builder()
		.method("DELETE")
		.uri(format!("/api/users/{}", seed.viewer.id))
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::NO_CONTENT);
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
				"role": "user"
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
				"role": "admin",
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
