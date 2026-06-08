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
	let org2_user_filter_uri = format!(
		"/api/users?filters[email][%24eq]={}",
		seed.user2.email.replace('@', "%40")
	);
	let org2_user_id = seed.user2.id.to_string();

	let system_req = Request::builder()
		.method("GET")
		.uri(org2_user_filter_uri.as_str())
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
			.any(|user| user["id"].as_str() == Some(org2_user_id.as_str())),
		"system admin should see filtered users in other organizations: {system_json:?}"
	);

	let sponsor_req = Request::builder()
		.method("GET")
		.uri(org2_user_filter_uri.as_str())
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
			.all(|user| user["id"].as_str() != Some(org2_user_id.as_str())),
		"sponsor admin should not see filtered org2 user after system-admin read: {sponsor_json:?}"
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
async fn test_permission_profiles_reject_twenty_first_custom_role_per_org(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let app = web_server::app(mm);

	for index in 1..=20 {
		let body = json!({
			"data": {
				"name": format!("Limited Role {index:02} {}", Uuid::new_v4()),
				"description": "Counts toward the organization role limit",
				"privileges": []
			}
		});
		let req = Request::builder()
			.method("POST")
			.uri("/api/admin/permission-profiles")
			.header("cookie", cookie_header(&token.to_string()))
			.header("content-type", "application/json")
			.body(Body::from(body.to_string()))?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(
			res.status(),
			StatusCode::CREATED,
			"custom role {index} should still be creatable"
		);
	}

	let body = json!({
		"data": {
			"name": format!("Limited Role 21 {}", Uuid::new_v4()),
			"description": "This role exceeds the organization limit",
			"privileges": []
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri("/api/admin/permission-profiles")
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let json: serde_json::Value = serde_json::from_slice(&body)?;

	assert_eq!(status, StatusCode::BAD_REQUEST, "{json:?}");
	assert!(
		json.to_string().contains("20"),
		"limit error should mention the maximum role count: {json:?}"
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

#[serial]
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
