#![allow(unused_imports)]

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
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_system_admin_profile_menu_privileges_match_eligible_actions(
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
	let request = Request::builder()
		.method("GET")
		.uri("/api/users/me/profile")
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let response = web_server::app(mm).oneshot(request).await?;
	assert_eq!(response.status(), StatusCode::OK);
	let body = to_bytes(response.into_body(), usize::MAX).await?;
	let profile: serde_json::Value = serde_json::from_slice(&body)?;
	let privileges = profile["data"]["privileges"]
		.as_array()
		.ok_or("profile privileges should be an array")?;
	assert!(privileges.iter().any(|row| {
		row["menu_key"] == "admin"
			&& row["can_read"] == true
			&& row["can_edit"] == true
	}));
	assert!(privileges
		.iter()
		.all(|row| row["menu_key"] != "home_notice"));

	let actions = profile["data"]["eligibleActions"]
		.as_array()
		.ok_or("eligible actions should be an array")?;
	assert!(actions.iter().any(|action| action == "settings.update"));
	assert!(actions.iter().all(|action| action != "notice.read"));
	assert!(actions.iter().all(|action| action != "notice.update"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_system_admin_permission_profiles_require_target_organization(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let source = seed_org_with_users(&mm, "sourcepwd", "sourceview").await?;
	let target = seed_org_with_users(&mm, "targetpwd", "targetview").await?;
	let system_admin = insert_user(
		&mm,
		source.org_id,
		ROLE_SYSTEM_ADMIN,
		system_user_id(),
		Some("systempwd"),
	)
	.await?;
	let token = generate_web_token(&system_admin.email, system_admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let source_role_name = format!("Source Role {}", Uuid::new_v4());
	let role_name = format!("Target Role {}", Uuid::new_v4());
	let role_body = |name: &str| {
		json!({
			"data": {
				"name": name,
				"privileges": [{
					"menu_key": "case",
					"can_read": true,
					"can_edit": false,
					"can_review": false,
					"can_lock": false
				}]
			}
		})
	};
	let body = role_body(&role_name);

	let missing_scope = Request::builder()
		.method("POST")
		.uri("/api/admin/permission-profiles")
		.header("cookie", &cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let missing_scope = app.clone().oneshot(missing_scope).await?;
	assert_eq!(missing_scope.status(), StatusCode::BAD_REQUEST);

	let source_uri = format!(
		"/api/admin/permission-profiles?organization_id={}",
		source.org_id
	);
	let source_role = Request::builder()
		.method("POST")
		.uri(&source_uri)
		.header("cookie", &cookie)
		.header("content-type", "application/json")
		.body(Body::from(role_body(&source_role_name).to_string()))?;
	let source_role = app.clone().oneshot(source_role).await?;
	assert_eq!(source_role.status(), StatusCode::CREATED);

	let target_uri = format!(
		"/api/admin/permission-profiles?organization_id={}",
		target.org_id
	);
	let scoped = Request::builder()
		.method("POST")
		.uri(&target_uri)
		.header("cookie", &cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let scoped = app.clone().oneshot(scoped).await?;
	assert_eq!(scoped.status(), StatusCode::CREATED);

	let list = Request::builder()
		.method("GET")
		.uri(&target_uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let list = app.oneshot(list).await?;
	assert_eq!(list.status(), StatusCode::OK);
	let list_body = to_bytes(list.into_body(), usize::MAX).await?;
	let rows: serde_json::Value = serde_json::from_slice(&list_body)?;
	assert!(rows
		.as_array()
		.ok_or("expected profile list")?
		.iter()
		.any(|row| row["name"] == role_name));
	assert!(rows
		.as_array()
		.ok_or("expected profile list")?
		.iter()
		.all(|row| row["name"] != source_role_name));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_system_admin_cross_org_profile_read_is_not_found() -> Result<()> {
	let mm = init_test_mm().await?;
	let source = seed_org_with_users(&mm, "sourcepwd", "sourceview").await?;
	let target = seed_org_with_users(&mm, "targetpwd", "targetview").await?;
	let system_admin = insert_user(
		&mm,
		source.org_id,
		ROLE_SYSTEM_ADMIN,
		system_user_id(),
		Some("systempwd"),
	)
	.await?;
	let token = generate_web_token(&system_admin.email, system_admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let create = Request::builder()
		.method("POST")
		.uri(format!(
			"/api/admin/permission-profiles?organization_id={}",
			source.org_id
		))
		.header("cookie", &cookie)
		.header("content-type", "application/json")
		.body(Body::from(
			json!({
				"data": {
					"name": format!("Cross-org source role {}", Uuid::new_v4()),
					"privileges": []
				}
			})
			.to_string(),
		))?;
	let created = app.clone().oneshot(create).await?;
	assert_eq!(created.status(), StatusCode::CREATED);
	let created_body = to_bytes(created.into_body(), usize::MAX).await?;
	let profile: serde_json::Value = serde_json::from_slice(&created_body)?;
	let profile_id = profile["id"].as_str().ok_or("profile id missing")?;

	let read = Request::builder()
		.method("GET")
		.uri(format!(
			"/api/admin/permission-profiles/{profile_id}?organization_id={}",
			target.org_id
		))
		.header("cookie", cookie)
		.body(Body::empty())?;
	let read = app.oneshot(read).await?;
	assert_eq!(read.status(), StatusCode::NOT_FOUND);
	Ok(())
}

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
	assert!(
		profiles
			.iter()
			.any(|profile| profile["id"] == ROLE_SPONSOR_ADMIN_CRO
				&& profile["built_in"] == true
				&& profile["editable"] == false),
		"CRO account should expose its assigned Sponsor Administrator as read-only: {rows:?}"
	);
	assert!(
		profiles.iter().all(|profile| {
			profile["id"] != ROLE_SPONSOR_ADMIN_COMPANY
				&& profile["id"] != ROLE_SYSTEM_ADMIN
		}),
		"CRO account should not expose unrelated built-in administrator roles: {rows:?}"
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

	assert_eq!(status, StatusCode::CONFLICT, "{json:?}");
	assert!(
		json.to_string().contains("20"),
		"limit error should mention the maximum role count: {json:?}"
	);

	Ok(())
}

async fn assert_profile_write_locks_revision_before_audit(
	app: axum::Router,
	control_pool: &sqlx::PgPool,
	organization_id: Uuid,
	request: Request<Body>,
	operation: &str,
) -> Result<axum::response::Response> {
	let mut controller = control_pool.begin().await?;
	let (controller_pid,): (i32,) = sqlx::query_as("SELECT pg_backend_pid()")
		.fetch_one(&mut *controller)
		.await?;
	sqlx::query("SELECT revision FROM organization_policy_state WHERE organization_id = $1 FOR UPDATE")
		.bind(organization_id)
		.execute(&mut *controller)
		.await?;
	let mut write = tokio::spawn(async move { app.oneshot(request).await });

	let blocked = tokio::time::timeout(Duration::from_secs(5), async {
		loop {
			let (blocked,): (bool,) = sqlx::query_as(
				"SELECT EXISTS (
					SELECT 1 FROM pg_stat_activity
					WHERE $1 = ANY(pg_blocking_pids(pid))
				)",
			)
			.bind(controller_pid)
			.fetch_one(control_pool)
			.await?;
			if blocked {
				return Ok::<(), sqlx::Error>(());
			}
			tokio::task::yield_now().await;
		}
	})
	.await;
	match blocked {
		Ok(Ok(())) => {}
		Ok(Err(error)) => {
			controller.rollback().await?;
			write.abort();
			let _ = write.await;
			return Err(error.into());
		}
		Err(_) => {
			controller.rollback().await?;
			write.abort();
			let _ = write.await;
			return Err(format!(
				"permission profile {operation} did not wait on revision lock"
			)
			.into());
		}
	}

	let (audit_lock_available,): (bool,) = sqlx::query_as(
		"SELECT pg_try_advisory_xact_lock(
			hashtextextended('e2br3.audit_logs.hash_chain', 0)
		)",
	)
	.fetch_one(&mut *controller)
	.await?;
	controller.commit().await?;

	let response = match tokio::time::timeout(Duration::from_secs(5), &mut write)
		.await
	{
		Ok(result) => result??,
		Err(_) => {
			write.abort();
			let _ = write.await;
			return Err(
				format!("permission profile {operation} did not complete").into()
			);
		}
	};
	assert!(
		audit_lock_available,
		"permission profile {operation} acquired the audit lock before the revision lock"
	);
	Ok(response)
}

#[serial]
#[tokio::test]
async fn test_permission_profile_writes_lock_revision_before_audit_chain(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let app = web_server::app(mm);
	let database_url = std::env::var("SERVICE_DB_URL")?;
	let control_pool = PgPoolOptions::new()
		.max_connections(2)
		.connect(&database_url)
		.await?;
	let role_name = format!("Lock Order Role {}", Uuid::new_v4());
	let create_body = json!({
		"data": {
			"name": role_name,
			"privileges": []
		}
	});
	let create_request = Request::builder()
		.method("POST")
		.uri("/api/admin/permission-profiles")
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(create_body.to_string()))?;
	let create_response = assert_profile_write_locks_revision_before_audit(
		app.clone(),
		&control_pool,
		seed.org_id,
		create_request,
		"create",
	)
	.await?;
	assert_eq!(create_response.status(), StatusCode::CREATED);
	let create_body = to_bytes(create_response.into_body(), usize::MAX).await?;
	let created: serde_json::Value = serde_json::from_slice(&create_body)?;
	let profile_id = created["id"]
		.as_str()
		.ok_or("created permission profile response is missing id")?;

	let update_body = json!({
		"data": {
			"name": format!("{role_name} Updated"),
			"privileges": []
		}
	});
	let update_request = Request::builder()
		.method("PUT")
		.uri(format!("/api/admin/permission-profiles/{profile_id}"))
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(update_body.to_string()))?;
	let update_response = assert_profile_write_locks_revision_before_audit(
		app,
		&control_pool,
		seed.org_id,
		update_request,
		"update",
	)
	.await?;
	assert_eq!(update_response.status(), StatusCode::OK);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_permission_profile_revision_lock_rejects_mismatched_request_context(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let database_url = std::env::var("SERVICE_DB_URL")?;
	let control_pool = PgPoolOptions::new()
		.max_connections(1)
		.connect(&database_url)
		.await?;
	let mut transaction = control_pool.begin().await?;
	sqlx::query(
		"SELECT set_config('app.current_user_id', $1, true),
		        set_config('app.current_organization_id', $2, true)",
	)
	.bind(seed.admin.id.to_string())
	.bind(seed.org_id.to_string())
	.execute(&mut *transaction)
	.await?;

	let mismatched_organization_id = Uuid::new_v4();
	let error = match sqlx::query_scalar::<_, i64>(
		"SELECT authz_lock_organization_policy_revision($1)",
	)
	.bind(mismatched_organization_id)
	.fetch_one(&mut *transaction)
	.await
	{
		Ok(_) => {
			transaction.rollback().await?;
			return Err(
				"mismatched organization revision lock unexpectedly succeeded"
					.into(),
			);
		}
		Err(error) => error,
	};
	let is_context_rejection = error
		.as_database_error()
		.and_then(|database_error| database_error.code())
		.is_some_and(|code| code == "42501");
	transaction.rollback().await?;
	assert!(is_context_rejection, "unexpected database error: {error}");
	Ok(())
}
