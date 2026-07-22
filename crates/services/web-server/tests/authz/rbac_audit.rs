use crate::common::{
	cookie_header, init_test_mm, insert_user, seed_org_with_all_roles,
	seed_org_with_users, seed_two_orgs_manager_cases, system_user_id, Result,
};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::{ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN};
use serde_json::json;
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_admin_can_list_audit_logs() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let app = web_server::app(mm);

	let req = Request::builder()
		.method("GET")
		.uri("/api/audit-logs")
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_viewer_cannot_list_audit_logs() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let app = web_server::app(mm);

	let req = Request::builder()
		.method("GET")
		.uri("/api/audit-logs")
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_custom_manager_without_admin_read_cannot_list_audit_logs() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let app = web_server::app(mm);

	let manager_token =
		generate_web_token(&seed.manager.email, seed.manager.token_salt)?;
	let req = Request::builder()
		.method("GET")
		.uri("/api/audit-logs")
		.header("cookie", cookie_header(&manager_token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_can_see_user_create_in_audit_logs() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let app = web_server::app(mm);
	let suffix = Uuid::new_v4();
	let email = format!("audit-created-user-{suffix}@example.com");
	let role_req = Request::builder()
		.method("POST")
		.uri("/api/admin/permission-profiles")
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(
			json!({
				"data": {
					"name": format!("Audit User Role {suffix}"),
					"privileges": [],
					"active": true
				}
			})
			.to_string(),
		))?;
	let role_res = app.clone().oneshot(role_req).await?;
	assert_eq!(role_res.status(), StatusCode::CREATED);
	let role_body = to_bytes(role_res.into_body(), usize::MAX).await?;
	let role: serde_json::Value = serde_json::from_slice(&role_body)?;
	let role_id = role["id"]
		.as_str()
		.ok_or("created role should include id")?;

	let create_req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(
			json!({
				"data": {
					"organization_id": seed.org_id,
					"email": email,
					"username": format!("audit_created_user_{suffix}"),
					"role": role_id
				}
			})
			.to_string(),
		))?;
	let create_res = app.clone().oneshot(create_req).await?;
	assert_eq!(create_res.status(), StatusCode::CREATED);
	let create_body = to_bytes(create_res.into_body(), usize::MAX).await?;
	let created: serde_json::Value = serde_json::from_slice(&create_body)?;
	let created_id = created["data"]["id"]
		.as_str()
		.ok_or("created user response should include id")?;

	let audit_req = Request::builder()
		.method("GET")
		.uri(format!("/api/audit-logs/by-record/users/{created_id}"))
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let audit_res = app.oneshot(audit_req).await?;
	assert_eq!(audit_res.status(), StatusCode::OK);
	let audit_body = to_bytes(audit_res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&audit_body)?;
	let logs = payload["data"]
		.as_array()
		.ok_or("expected audit logs array")?;
	assert!(
		logs.iter().any(|log| {
			log["table_name"] == "users"
				&& log["action"] == "CREATE"
				&& log["new_values"]["email"] == email
		}),
		"user create audit log should be visible: {payload:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_can_see_role_create_update_delete_in_audit_logs() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let app = web_server::app(mm);
	let suffix = Uuid::new_v4();

	let create_req = Request::builder()
		.method("POST")
		.uri("/api/admin/permission-profiles")
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(
			json!({
				"data": {
					"name": format!("Audit Role {suffix}"),
					"description": "Initial role description",
					"privileges": [],
					"active": true
				}
			})
			.to_string(),
		))?;
	let create_res = app.clone().oneshot(create_req).await?;
	assert_eq!(create_res.status(), StatusCode::CREATED);
	let create_body = to_bytes(create_res.into_body(), usize::MAX).await?;
	let created: serde_json::Value = serde_json::from_slice(&create_body)?;
	let role_id = created["id"]
		.as_str()
		.ok_or("created role response should include id")?;
	Uuid::parse_str(role_id)?;

	let update_req = Request::builder()
		.method("PUT")
		.uri(format!("/api/admin/permission-profiles/{role_id}"))
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(
			json!({
				"data": {
					"name": format!("Updated Audit Role {suffix}"),
					"description": "Updated role description",
					"active": true
				}
			})
			.to_string(),
		))?;
	let update_res = app.clone().oneshot(update_req).await?;
	assert_eq!(update_res.status(), StatusCode::OK);

	let delete_req = Request::builder()
		.method("DELETE")
		.uri(format!("/api/admin/permission-profiles/{role_id}"))
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let delete_res = app.clone().oneshot(delete_req).await?;
	assert_eq!(delete_res.status(), StatusCode::NO_CONTENT);

	let audit_req = Request::builder()
		.method("GET")
		.uri("/api/audit-logs")
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let audit_res = app.oneshot(audit_req).await?;
	assert_eq!(audit_res.status(), StatusCode::OK);
	let audit_body = to_bytes(audit_res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&audit_body)?;
	let logs = payload["data"]
		.as_array()
		.ok_or("expected audit logs array")?;
	let matching = logs
		.iter()
		.filter(|log| {
			log["table_name"] == "permission_profiles" && log["record_id"] == role_id
		})
		.collect::<Vec<_>>();
	assert!(
		matching.iter().any(|log| log["action"] == "CREATE"),
		"role create audit log should be visible: {payload:?}"
	);
	assert!(
		matching.iter().any(|log| log["action"] == "UPDATE"),
		"role update audit log should be visible: {payload:?}"
	);
	assert!(
		matching.iter().any(|log| {
			log["action"] == "UPDATE"
				&& log["changed_fields"]["active"]["old"] == true
				&& log["changed_fields"]["active"]["new"] == false
		}),
		"role soft-delete audit transition should be visible: {payload:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_audit_log_list_is_limited_to_own_org() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_two_orgs_manager_cases(&mm).await?;
	let admin = insert_user(
		&mm,
		seed.org1_id,
		ROLE_SPONSOR_ADMIN_CRO,
		system_user_id(),
		Some("adminpwd"),
	)
	.await?;
	let app = web_server::app(mm);

	let admin_token = generate_web_token(&admin.email, admin.token_salt)?;
	let req = Request::builder()
		.method("GET")
		.uri("/api/audit-logs")
		.header("cookie", cookie_header(&admin_token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	let logs = payload["data"]
		.as_array()
		.ok_or("expected audit log array")?;
	assert!(
		!logs.is_empty(),
		"expected admin to see own-organization audit logs"
	);
	assert!(
		logs.iter().all(|log| {
			log["organization_id"] == seed.org1_id.to_string()
				&& log["record_id"] != seed.org2_id.to_string()
				&& log["record_id"] != seed.user2.id.to_string()
				&& log["record_id"] != seed.case_org2.to_string()
		}),
		"admin should not see audit logs from another organization: {logs:#?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_system_admin_audit_log_list_can_cross_orgs() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_two_orgs_manager_cases(&mm).await?;
	let system_admin = insert_user(
		&mm,
		seed.org1_id,
		ROLE_SYSTEM_ADMIN,
		system_user_id(),
		Some("systempwd"),
	)
	.await?;
	let app = web_server::app(mm);

	let token = generate_web_token(&system_admin.email, system_admin.token_salt)?;
	let req = Request::builder()
		.method("GET")
		.uri("/api/audit-logs")
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	let logs = payload["data"]
		.as_array()
		.ok_or("expected audit log array")?;
	assert!(
		logs.iter()
			.any(|log| log["organization_id"] == seed.org2_id.to_string()),
		"system admin should be able to see another organization's audit logs"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_user_and_viewer_cannot_list_audit_logs() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let app = web_server::app(mm);

	let user_token = generate_web_token(&seed.user.email, seed.user.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;

	for (role, token) in [("user", user_token), ("viewer", viewer_token)] {
		let req = Request::builder()
			.method("GET")
			.uri("/api/audit-logs")
			.header("cookie", cookie_header(&token.to_string()))
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(
			res.status(),
			StatusCode::FORBIDDEN,
			"{role} should be forbidden from listing audit logs"
		);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_custom_manager_without_admin_read_cannot_list_audit_logs_by_record(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let app = web_server::app(mm);

	let manager_token =
		generate_web_token(&seed.manager.email, seed.manager.token_salt)?;
	let req = Request::builder()
		.method("GET")
		.uri(format!(
			"/api/audit-logs/by-record/organizations/{}",
			seed.org_id
		))
		.header("cookie", cookie_header(&manager_token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_system_admin_can_list_audit_logs_by_record() -> Result<()> {
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

	let req = Request::builder()
		.method("GET")
		.uri(format!(
			"/api/audit-logs/by-record/users/{}",
			seed.viewer.id
		))
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_user_and_viewer_cannot_list_audit_logs_by_record() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let app = web_server::app(mm);

	let user_token = generate_web_token(&seed.user.email, seed.user.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;

	for (role, token) in [("user", user_token), ("viewer", viewer_token)] {
		let req = Request::builder()
			.method("GET")
			.uri(format!(
				"/api/audit-logs/by-record/organizations/{}",
				seed.org_id
			))
			.header("cookie", cookie_header(&token.to_string()))
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(
			res.status(),
			StatusCode::FORBIDDEN,
			"{role} should be forbidden from reading audit logs by record"
		);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_can_filter_user_audit_logs_by_field() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let app = web_server::app(mm);

	let email_update = Request::builder()
		.method("PUT")
		.uri(format!("/api/users/{}", seed.viewer.id))
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(
			json!({
				"data": {
					"email": format!("field-filter-{}@example.com", seed.viewer.id)
				}
			})
			.to_string(),
		))?;
	let email_res = app.clone().oneshot(email_update).await?;
	assert_eq!(email_res.status(), StatusCode::OK);

	let username = format!("Username Only Audit Change {}", seed.viewer.id);
	let username_update = Request::builder()
		.method("PUT")
		.uri(format!("/api/users/{}", seed.viewer.id))
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(
			json!({
				"data": {
					"username": username
				}
			})
			.to_string(),
		))?;
	let username_res = app.clone().oneshot(username_update).await?;
	assert_eq!(username_res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("GET")
		.uri(format!(
			"/api/audit-logs/by-record/users/{}?field=email",
			seed.viewer.id
		))
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	let logs = payload["data"]
		.as_array()
		.ok_or("expected audit log array")?;
	assert!(!logs.is_empty(), "expected at least one email audit log");
	assert!(
		logs.iter().all(|log| {
			let has_email = log["changed_fields"].get("email").is_some()
				|| log["old_values"].get("email").is_some()
				|| log["new_values"].get("email").is_some();
			let username_only = log["action"] == "UPDATE"
				&& log["changed_fields"].get("username").is_some()
				&& log["changed_fields"].get("email").is_none();
			has_email && !username_only
		}),
		"field=email should only return logs touching email"
	);

	Ok(())
}
