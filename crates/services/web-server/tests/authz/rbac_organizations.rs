use crate::common::{cookie_header, init_test_mm, seed_org_with_all_roles, Result};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
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
