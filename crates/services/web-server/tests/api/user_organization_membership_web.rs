use crate::common::{
	cookie_header, init_test_mm, insert_user_organization_membership,
	seed_two_orgs_users_cases, Result,
};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;

async fn request_json(
	app: &axum::Router,
	method: &str,
	cookie: &str,
	uri: &str,
	body: Option<Value>,
) -> Result<(StatusCode, Value)> {
	let mut req = Request::builder().method(method).uri(uri);
	if !cookie.is_empty() {
		req = req.header("cookie", cookie);
	}
	if body.is_some() {
		req = req.header("content-type", "application/json");
	}
	let res = app
		.clone()
		.oneshot(req.body(match body {
			Some(body) => Body::from(body.to_string()),
			None => Body::empty(),
		})?)
		.await?;
	let status = res.status();
	let bytes = to_bytes(res.into_body(), usize::MAX).await?;
	let value = serde_json::from_slice(&bytes)
		.unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&bytes) }));
	Ok((status, value))
}

#[serial]
#[tokio::test]
async fn profile_lists_all_database_memberships_for_current_user() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_two_orgs_users_cases(&mm).await?;
	insert_user_organization_membership(&mm, seed.user1.id, seed.org2_id).await?;
	let token = generate_web_token(&seed.user1.email, seed.user1.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, profile) =
		request_json(&app, "GET", &cookie, "/api/users/me/profile", None).await?;

	assert_eq!(status, StatusCode::OK, "{profile:?}");
	let orgs = profile["data"]["availableOrganizations"]
		.as_array()
		.ok_or("missing availableOrganizations")?;
	let ids = orgs
		.iter()
		.map(|org| org["id"].as_str().unwrap_or_default())
		.collect::<Vec<_>>();
	assert!(
		ids.contains(&seed.org1_id.to_string().as_str()),
		"{profile:?}"
	);
	assert!(
		ids.contains(&seed.org2_id.to_string().as_str()),
		"{profile:?}"
	);
	assert_eq!(
		profile["data"]["activeOrganization"]["id"].as_str(),
		Some(seed.org1_id.to_string().as_str())
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn current_user_can_switch_active_database_to_member_org_only() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_two_orgs_users_cases(&mm).await?;
	insert_user_organization_membership(&mm, seed.user1.id, seed.org2_id).await?;
	let token = generate_web_token(&seed.user1.email, seed.user1.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let (status, switched) = request_json(
		&app,
		"PUT",
		&cookie,
		"/api/users/me/organization",
		Some(json!({ "data": { "organization_id": seed.org2_id } })),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{switched:?}");
	assert_eq!(
		switched["data"]["activeOrganization"]["id"].as_str(),
		Some(seed.org2_id.to_string().as_str())
	);

	let (status, rejected) = request_json(
		&app,
		"PUT",
		&cookie,
		"/api/users/me/organization",
		Some(json!({ "data": { "organization_id": seed.user2.id } })),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{rejected:?}");

	let (status, profile) =
		request_json(&app, "GET", &cookie, "/api/users/me/profile", None).await?;
	assert_eq!(status, StatusCode::OK, "{profile:?}");
	assert_eq!(
		profile["data"]["activeOrganization"]["id"].as_str(),
		Some(seed.org2_id.to_string().as_str())
	);

	Ok(())
}
