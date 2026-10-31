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
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_user_reads_are_scoped_to_current_org_for_system_and_sponsor_admins(
) -> Result<()> {
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
	let org1_user_id = seed.user1.id.to_string();
	let org2_user_id = seed.user2.id.to_string();
	let org2_email_query = format!(
		"/api/users?filters[email][%24eq]={}",
		seed.user2.email.replace('@', "%40")
	);
	let org1_email_query = format!(
		"/api/users?filters[email][%24eq]={}",
		seed.user1.email.replace('@', "%40")
	);
	let org2_organization_query = format!(
		"/api/users?filters[organization_id][%24eq]={}",
		seed.org2_id
	);
	let alternative_filters = format!(
		"/api/users?include_blinded=true&filters[0][email][%24eq]={}&filters[1][username][%24eq]={}",
		seed.user2.email.replace('@', "%40"),
		format!("rls_user_{}", seed.user2.id)
	);

	for (role, token) in [
		("system admin", system_token),
		("sponsor admin", sponsor_token),
	] {
		let cookie = cookie_header(&token.to_string());
		for uri in [
			"/api/users",
			org2_email_query.as_str(),
			org2_organization_query.as_str(),
			alternative_filters.as_str(),
		] {
			let req = Request::builder()
				.method("GET")
				.uri(uri)
				.header("cookie", cookie.as_str())
				.body(Body::empty())?;
			let res = app.clone().oneshot(req).await?;
			assert_eq!(res.status(), StatusCode::OK, "{role}: {uri}");
			let body = to_bytes(res.into_body(), usize::MAX).await?;
			let value: serde_json::Value = serde_json::from_slice(&body)?;
			let users = value["data"].as_array().ok_or("expected user list")?;
			assert!(
				users.iter().all(|user| {
					user["id"].as_str() != Some(org2_user_id.as_str())
				}),
				"{role} leaked an other-org user through {uri}: {value:?}"
			);
			if uri == "/api/users" {
				assert!(
					users.iter().any(|user| {
						user["id"].as_str() == Some(org1_user_id.as_str())
					}),
					"{role} could not list a current-org user: {value:?}"
				);
			} else {
				assert!(
					users.is_empty(),
					"{role} returned users for a foreign-org filter through {uri}: {value:?}"
				);
			}
		}

		let own_query_req = Request::builder()
			.method("GET")
			.uri(org1_email_query.as_str())
			.header("cookie", cookie.as_str())
			.body(Body::empty())?;
		let own_query_res = app.clone().oneshot(own_query_req).await?;
		assert_eq!(own_query_res.status(), StatusCode::OK, "{role}");
		let own_query_body = to_bytes(own_query_res.into_body(), usize::MAX).await?;
		let own_query_json: serde_json::Value =
			serde_json::from_slice(&own_query_body)?;
		let own_query_users = own_query_json["data"]
			.as_array()
			.ok_or("expected filtered user list")?;
		assert_eq!(own_query_users.len(), 1, "{role}: {own_query_json:?}");
		assert_eq!(
			own_query_users[0]["id"].as_str(),
			Some(org1_user_id.as_str()),
			"{role}: {own_query_json:?}"
		);

		let own_req = Request::builder()
			.method("GET")
			.uri(format!("/api/users/{org1_user_id}"))
			.header("cookie", cookie.as_str())
			.body(Body::empty())?;
		let own_res = app.clone().oneshot(own_req).await?;
		assert_eq!(own_res.status(), StatusCode::OK, "{role}");

		let other_req = Request::builder()
			.method("GET")
			.uri(format!("/api/users/{org2_user_id}"))
			.header("cookie", cookie.as_str())
			.body(Body::empty())?;
		let other_res = app.clone().oneshot(other_req).await?;
		assert_eq!(other_res.status(), StatusCode::NOT_FOUND, "{role}");
		let other_body = to_bytes(other_res.into_body(), usize::MAX).await?;
		let other_json: serde_json::Value = serde_json::from_slice(&other_body)?;
		assert!(other_json["data"].is_null(), "{role}: {other_json:?}");
		assert!(
			!other_json.to_string().contains(&seed.user2.email),
			"{role}: {other_json:?}"
		);
	}

	Ok(())
}
