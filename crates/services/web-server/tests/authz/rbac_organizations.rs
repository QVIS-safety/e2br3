use crate::common::{
	cookie_header, init_test_mm, insert_user, seed_org_with_all_roles,
	system_user_id, Result,
};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SYSTEM_ADMIN;
use serde_json::json;
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_non_admin_cannot_list_organizations() -> Result<()> {
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
			.uri("/api/organizations")
			.header("cookie", cookie_header(&token.to_string()))
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(
			res.status(),
			StatusCode::FORBIDDEN,
			"{role} should be forbidden from listing organizations"
		);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sponsor_admin_cannot_access_organization_admin_endpoints() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let app = web_server::app(mm);

	let create_body = json!({
		"data": {
			"name": format!("Sponsor Admin Blocked {}", Uuid::new_v4()),
			"org_type": "internal",
			"contact_email": format!("blocked-{}@example.com", Uuid::new_v4())
		}
	});
	let update_body = json!({
		"data": {
			"name": format!("Sponsor Admin Updated {}", Uuid::new_v4())
		}
	});

	let requests = [
		Request::builder()
			.method("GET")
			.uri("/api/organizations")
			.header("cookie", cookie_header(&token.to_string()))
			.body(Body::empty())?,
		Request::builder()
			.method("GET")
			.uri(format!("/api/organizations/{}", seed.org_id))
			.header("cookie", cookie_header(&token.to_string()))
			.body(Body::empty())?,
		Request::builder()
			.method("POST")
			.uri("/api/organizations")
			.header("cookie", cookie_header(&token.to_string()))
			.header("content-type", "application/json")
			.body(Body::from(create_body.to_string()))?,
		Request::builder()
			.method("PUT")
			.uri(format!("/api/organizations/{}", seed.org_id))
			.header("cookie", cookie_header(&token.to_string()))
			.header("content-type", "application/json")
			.body(Body::from(update_body.to_string()))?,
		Request::builder()
			.method("DELETE")
			.uri(format!("/api/organizations/{}", seed.org_id))
			.header("cookie", cookie_header(&token.to_string()))
			.body(Body::empty())?,
	];

	for req in requests {
		let res = app.clone().oneshot(req).await?;
		assert_eq!(res.status(), StatusCode::FORBIDDEN);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_system_admin_can_create_and_list_organizations() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
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
	let create_body = json!({
		"data": {
			"name": format!("System Admin Org {suffix}"),
			"type": "CRO",
			"contact_email": format!("system-admin-org-{suffix}@example.com")
		}
	});
	let create_req = Request::builder()
		.method("POST")
		.uri("/api/organizations")
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(create_body.to_string()))?;
	let create_res = app.clone().oneshot(create_req).await?;
	assert_eq!(create_res.status(), StatusCode::CREATED);

	let body = to_bytes(create_res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	let created_id = payload["data"]["id"]
		.as_str()
		.ok_or("expected created organization id")?;
	assert_eq!(payload["data"]["type"].as_str(), Some("cro"));

	let get_req = Request::builder()
		.method("GET")
		.uri(format!("/api/organizations/{created_id}"))
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let get_res = app.oneshot(get_req).await?;
	assert_eq!(get_res.status(), StatusCode::OK);
	let body = to_bytes(get_res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	assert_eq!(payload["data"]["id"].as_str(), Some(created_id));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_system_admin_cannot_create_organization_with_unknown_type(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
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
	let create_body = json!({
		"data": {
			"name": format!("Invalid Org Type {suffix}"),
			"type": "internal",
			"contact_email": format!("invalid-org-type-{suffix}@example.com")
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri("/api/organizations")
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(create_body.to_string()))?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::BAD_REQUEST);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_non_admin_cannot_get_organization() -> Result<()> {
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
			.uri(format!("/api/organizations/{}", seed.org_id))
			.header("cookie", cookie_header(&token.to_string()))
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(
			res.status(),
			StatusCode::FORBIDDEN,
			"{role} should be forbidden from reading organizations"
		);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_non_admin_cannot_create_organization() -> Result<()> {
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
				"name": format!("RBAC Org {role} {suffix}"),
				"org_type": "internal",
				"contact_email": format!("rbac-org-{role}-{suffix}@example.com")
			}
		});
		let req = Request::builder()
			.method("POST")
			.uri("/api/organizations")
			.header("cookie", cookie_header(&token.to_string()))
			.header("content-type", "application/json")
			.body(Body::from(body.to_string()))?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(
			res.status(),
			StatusCode::FORBIDDEN,
			"{role} should be forbidden from creating organizations"
		);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_non_admin_cannot_update_organization() -> Result<()> {
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
				"name": format!("Updated by {role}")
			}
		});
		let req = Request::builder()
			.method("PUT")
			.uri(format!("/api/organizations/{}", seed.org_id))
			.header("cookie", cookie_header(&token.to_string()))
			.header("content-type", "application/json")
			.body(Body::from(body.to_string()))?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(
			res.status(),
			StatusCode::FORBIDDEN,
			"{role} should be forbidden from updating organizations"
		);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_non_admin_cannot_delete_organization() -> Result<()> {
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
			.uri(format!("/api/organizations/{}", seed.org_id))
			.header("cookie", cookie_header(&token.to_string()))
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(
			res.status(),
			StatusCode::FORBIDDEN,
			"{role} should be forbidden from deleting organizations"
		);
	}

	Ok(())
}
