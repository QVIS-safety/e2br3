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
			.all(|profile| profile["id"] != ROLE_SPONSOR_ADMIN_CRO
				&& profile["id"] != ROLE_SPONSOR_ADMIN_COMPANY),
		"Sponsor Administrator built-in roles should not be exposed: {rows:?}"
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
