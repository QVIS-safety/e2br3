#![allow(unused_imports)]

use super::helpers::*;
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
