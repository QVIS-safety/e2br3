use crate::common::{
	cookie_header, init_test_mm, insert_user, seed_org_with_all_roles,
	seed_org_with_users, seed_two_orgs_users_cases, system_user_id, Result,
};
use axum::body::{to_bytes, Body};
use axum::http::header;
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
			"role": "user",
			"access_blind_allowed": true
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
async fn test_sponsor_admin_can_set_and_persist_blind_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;

	let app = web_server::app(mm);
	let suffix = Uuid::new_v4();
	let create_body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": format!("blind-scope-{suffix}@example.com"),
			"username": format!("blind_scope_{suffix}"),
			"role": "user",
			"access_blind_allowed": true
		}
	});
	let create_req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", cookie_header(&token.to_string()))
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
		.header("cookie", cookie_header(&token.to_string()))
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
	let app = web_server::app(mm);

	let suffix = Uuid::new_v4();
	let email = format!("expired-user-{suffix}@example.com");
	let username = format!("expired_user_{suffix}");
	let create_body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": email,
			"username": username,
			"role": "user",
			"access_end_at": "2000-01-01T00:00:00Z"
		}
	});
	let create_req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", cookie_header(&admin_token.to_string()))
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
	let app = web_server::app(mm);

	let suffix = Uuid::new_v4();
	let email = format!("access-window-{suffix}@example.com");
	let username = format!("access_window_{suffix}");
	let create_body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": email,
			"username": username,
			"role": "user",
			"access_start_at": "2026-01-01T00:00:00.000Z",
			"access_end_at": "2026-12-31T23:59:00.000Z"
		}
	});
	let create_req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", cookie_header(&admin_token.to_string()))
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
async fn test_create_user_accepts_datetime_local_access_dates() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let app = web_server::app(mm);

	let suffix = Uuid::new_v4();
	let email = format!("datetime-local-window-{suffix}@example.com");
	let username = format!("datetime_local_window_{suffix}");
	let create_body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": email,
			"username": username,
			"role": "user",
			"access_start_at": "2026-01-01T00:00",
			"access_end_at": "2026-12-31T23:59"
		}
	});
	let create_req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", cookie_header(&admin_token.to_string()))
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
async fn test_user_list_sets_rls_context_for_system_and_sponsor_admins() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_two_orgs_users_cases(&mm).await?;
	let sponsor_admin = insert_user(
		&mm,
		seed.org1_id,
		ROLE_SPONSOR_ADMIN_CRO,
		system_user_id(),
		Some("adminpwd"),
	)
	.await?;
	let system_admin = insert_user(
		&mm,
		seed.org1_id,
		ROLE_SYSTEM_ADMIN,
		system_user_id(),
		Some("systempwd"),
	)
	.await?;
	let system_token =
		generate_web_token(&system_admin.email, system_admin.token_salt)?;
	let sponsor_token =
		generate_web_token(&sponsor_admin.email, sponsor_admin.token_salt)?;
	let app = web_server::app(mm);

	let system_req = Request::builder()
		.method("GET")
		.uri("/api/users")
		.header("cookie", cookie_header(&system_token.to_string()))
		.body(Body::empty())?;
	let system_res = app.clone().oneshot(system_req).await?;
	assert_eq!(system_res.status(), StatusCode::OK);
	let system_body = to_bytes(system_res.into_body(), usize::MAX).await?;
	let system_json: serde_json::Value = serde_json::from_slice(&system_body)?;
	let system_users = system_json["data"]
		.as_array()
		.ok_or("expected system user list")?;
	assert!(
		system_users
			.iter()
			.any(|user| user["organizationId"] == seed.org2_id.to_string()),
		"system admin should see users in other organizations: {system_json:?}"
	);

	let sponsor_req = Request::builder()
		.method("GET")
		.uri("/api/users")
		.header("cookie", cookie_header(&sponsor_token.to_string()))
		.body(Body::empty())?;
	let sponsor_res = app.oneshot(sponsor_req).await?;
	assert_eq!(sponsor_res.status(), StatusCode::OK);
	let sponsor_body = to_bytes(sponsor_res.into_body(), usize::MAX).await?;
	let sponsor_json: serde_json::Value = serde_json::from_slice(&sponsor_body)?;
	let sponsor_users = sponsor_json["data"]
		.as_array()
		.ok_or("expected sponsor user list")?;
	assert!(
		sponsor_users
			.iter()
			.all(|user| user["organizationId"] == seed.org1_id.to_string()),
		"sponsor admin should only see own-org users after system-admin read: {sponsor_json:?}"
	);
	assert!(
		sponsor_users
			.iter()
			.all(|user| user["id"] != seed.user2.id.to_string()),
		"sponsor admin should not see org2 user after system-admin read"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_permission_profiles_are_scoped_by_organization_for_sponsor_admins(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let org1 = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let org2 = seed_org_with_users(&mm, "otheradminpwd", "otherviewpwd").await?;
	let org1_token = generate_web_token(&org1.admin.email, org1.admin.token_salt)?;
	let org2_token = generate_web_token(&org2.admin.email, org2.admin.token_salt)?;
	let app = web_server::app(mm);

	let create_profile = |name: String| {
		json!({
			"data": {
				"name": name,
				"description": "Org-scoped custom role",
				"privileges": [
					{
						"menu_key": "case",
						"can_read": true,
						"can_edit": false,
						"can_review": false,
						"can_lock": false
					}
				]
			}
		})
	};

	let org1_name = format!("Org1 Role {}", Uuid::new_v4());
	let org1_req = Request::builder()
		.method("POST")
		.uri("/api/admin/permission-profiles")
		.header("cookie", cookie_header(&org1_token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(create_profile(org1_name.clone()).to_string()))?;
	let org1_res = app.clone().oneshot(org1_req).await?;
	assert_eq!(org1_res.status(), StatusCode::CREATED);

	let org2_name = format!("Org2 Role {}", Uuid::new_v4());
	let org2_req = Request::builder()
		.method("POST")
		.uri("/api/admin/permission-profiles")
		.header("cookie", cookie_header(&org2_token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(create_profile(org2_name.clone()).to_string()))?;
	let org2_res = app.clone().oneshot(org2_req).await?;
	assert_eq!(org2_res.status(), StatusCode::CREATED);

	let list_req = Request::builder()
		.method("GET")
		.uri("/api/admin/permission-profiles")
		.header("cookie", cookie_header(&org1_token.to_string()))
		.body(Body::empty())?;
	let list_res = app.oneshot(list_req).await?;
	assert_eq!(list_res.status(), StatusCode::OK);
	let body = to_bytes(list_res.into_body(), usize::MAX).await?;
	let rows: serde_json::Value = serde_json::from_slice(&body)?;
	let profiles = rows.as_array().ok_or("expected profile list")?;
	assert!(
		profiles.iter().any(|profile| profile["name"] == org1_name),
		"org1 sponsor admin should see own custom profile: {rows:?}"
	);
	assert!(
		profiles.iter().all(|profile| profile["name"] != org2_name),
		"org1 sponsor admin should not see org2 custom profile: {rows:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_system_admin_can_manage_admin_console_users_and_roles() -> Result<()> {
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
	let regular_suffix = Uuid::new_v4();
	let sponsor_suffix = Uuid::new_v4();

	let regular_body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": format!("system-admin-regular-{regular_suffix}@example.com"),
			"username": format!("System Admin Regular {regular_suffix}"),
			"role": "user"
		}
	});
	let regular_req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(regular_body.to_string()))?;
	let regular_res = app.clone().oneshot(regular_req).await?;
	assert_eq!(regular_res.status(), StatusCode::CREATED);

	let sponsor_body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": format!("system-admin-sponsor-{sponsor_suffix}@example.com"),
			"username": format!("System Admin Sponsor {sponsor_suffix}"),
			"role": ROLE_SPONSOR_ADMIN_COMPANY
		}
	});
	let sponsor_req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(sponsor_body.to_string()))?;
	let sponsor_res = app.clone().oneshot(sponsor_req).await?;
	assert_eq!(sponsor_res.status(), StatusCode::CREATED);

	let role_req = Request::builder()
		.method("POST")
		.uri("/api/admin/permission-profiles")
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(
			json!({
				"data": {
					"name": "Blocked By System",
					"description": "Created without client-provided profile id",
					"privileges": [
						{
							"menu_key": "case",
							"can_read": true,
							"can_edit": false,
							"can_review": false,
							"can_lock": false
						}
					]
				}
			})
			.to_string(),
		))?;
	let role_res = app.oneshot(role_req).await?;
	assert_eq!(role_res.status(), StatusCode::CREATED);
	let role_body = to_bytes(role_res.into_body(), usize::MAX).await?;
	let role_json: serde_json::Value = serde_json::from_slice(&role_body)?;
	let profile_id = role_json["profile_id"]
		.as_str()
		.ok_or("expected generated profile_id")?;
	Uuid::parse_str(profile_id)?;
	assert_eq!(role_json["name"], "Blocked By System");
	assert_eq!(
		role_json["description"],
		"Created without client-provided profile id"
	);
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
		assert_eq!(res.status(), StatusCode::BAD_REQUEST);
		let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await?;
		let value: serde_json::Value = serde_json::from_slice(&bytes)?;
		assert_eq!(value["error"]["data"]["detail"], expected_detail);
	}

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
	let created_org = json["data"]["organizationId"]
		.as_str()
		.ok_or("missing created user organizationId")?;
	assert_eq!(created_org, seed.org_id.to_string());
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_create_user_ignores_payload_org_id() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let other_seed =
		seed_org_with_users(&mm, "otheradminpwd", "otherviewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;

	let app = web_server::app(mm);
	let suffix = Uuid::new_v4();
	let body = json!({
		"data": {
			"organization_id": other_seed.org_id,
			"email": format!("rbac-foreign-org-{suffix}@example.com"),
			"username": format!("rbac_foreign_org_{suffix}"),
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
	let created_org = json["data"]["organizationId"]
		.as_str()
		.ok_or("missing created user organizationId")?;
	assert_eq!(created_org, seed.org_id.to_string());
	assert_ne!(created_org, other_seed.org_id.to_string());
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
			"role": "sponsor_admin_company",
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
