use crate::common::{
	cookie_header, init_test_mm, insert_user_organization_membership,
	seed_two_orgs_users_cases, Result,
};
use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
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
async fn one_account_lists_and_switches_membership_organizations() -> Result<()> {
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
	assert!(
		orgs.iter().any(|org| org["id"] == seed.org1_id.to_string())
			&& orgs.iter().any(|org| org["id"] == seed.org2_id.to_string()),
		"{profile:?}"
	);

	let switch_req = Request::builder()
		.method("PUT")
		.uri("/api/users/me/organization")
		.header("cookie", &cookie)
		.header("content-type", "application/json")
		.body(Body::from(
			json!({ "data": { "organization_id": seed.org2_id } }).to_string(),
		))?;
	let switch_res = app.clone().oneshot(switch_req).await?;
	assert_eq!(switch_res.status(), StatusCode::OK);
	let switched_token = switch_res
		.headers()
		.get_all(header::SET_COOKIE)
		.iter()
		.filter_map(|value| value.to_str().ok())
		.find_map(|value| value.strip_prefix("auth-token=")?.split(';').next())
		.ok_or("missing switched auth-token")?;
	let (status, selected) = request_json(
		&app,
		"GET",
		&cookie_header(switched_token),
		"/api/users/me/profile",
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{selected:?}");
	assert_eq!(selected["data"]["user"]["id"], seed.user1.id.to_string());
	assert_eq!(
		selected["data"]["activeOrganization"]["id"],
		seed.org2_id.to_string()
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn account_cannot_switch_without_membership() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_two_orgs_users_cases(&mm).await?;
	let token = generate_web_token(&seed.user1.email, seed.user1.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, rejected) = request_json(
		&app,
		"PUT",
		&cookie,
		"/api/users/me/organization",
		Some(json!({ "data": { "organization_id": seed.org2_id } })),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{rejected:?}");
	Ok(())
}
