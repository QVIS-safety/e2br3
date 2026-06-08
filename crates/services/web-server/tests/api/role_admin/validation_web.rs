#![allow(unused_imports)]

use super::helpers::*;
use crate::common::{
	cookie_header, init_test_mm, insert_user, seed_org_with_users, system_user_id,
	Result, TEST_CUSTOM_MANAGER_ROLE,
};
use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use lib_auth::token::generate_web_token;
use lib_core::ctx::{
	ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN,
};
use lib_core::model::acs::{
	has_permission, CASE_APPROVE, CASE_CREATE, CASE_UPDATE, PRESAVE_TEMPLATE_CREATE,
	PRESAVE_TEMPLATE_DELETE, PRESAVE_TEMPLATE_LIST, PRESAVE_TEMPLATE_READ,
	PRESAVE_TEMPLATE_UPDATE, SETTINGS_READ, SETTINGS_UPDATE, TERMINOLOGY_APPROVE,
	TERMINOLOGY_IMPORT, USER_CREATE, USER_DELETE, USER_LIST, USER_READ, USER_UPDATE,
	XML_EXPORT, XML_EXPORT_READ, XML_IMPORT, XML_IMPORT_READ,
};
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::ModelManager;
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_role_admin_api_rejects_duplicate_role_name_in_same_org() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"name": "Duplicate Role",
				"description": "First duplicate role",
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	Uuid::parse_str(value["id"].as_str().ok_or("missing role id")?)?;

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"name": " duplicate role ",
				"description": "Second duplicate role",
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert_eq!(value["error"]["message"], "SERVICE_ERROR");
	assert_eq!(
		value["error"]["data"]["detail"],
		"role name already exists in this organization"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_role_admin_api_rejects_rename_to_duplicate_role_name_in_same_org(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let mut second_profile_id = None;

	for name in ["Original Role", "Other Role"] {
		let (status, value) = request_json(
			&app,
			"POST",
			&admin_cookie,
			"/api/admin/permission-profiles".to_string(),
			Some(json!({
				"data": {
					"name": name,
					"privileges": []
				}
			})),
		)
		.await?;
		assert_eq!(status, StatusCode::CREATED, "{value:?}");
		if name == "Other Role" {
			second_profile_id =
				Some(value["id"].as_str().ok_or("missing role id")?.to_string());
		}
	}
	let second_profile_id = second_profile_id.ok_or("missing second role id")?;

	let (status, value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		format!("/api/admin/permission-profiles/{second_profile_id}"),
		Some(json!({
			"data": {
				"name": " original role "
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert_eq!(value["error"]["message"], "SERVICE_ERROR");
	assert_eq!(
		value["error"]["data"]["detail"],
		"role name already exists in this organization"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_role_admin_api_rejects_overlong_name_and_description() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let overlong_name = "R".repeat(129);
	let overlong_description = "D".repeat(513);

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"name": overlong_name,
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert_eq!(
		value["error"]["data"]["detail"],
		"role name must be 128 characters or fewer"
	);

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"name": "Description Limit Role",
				"description": overlong_description,
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert_eq!(
		value["error"]["data"]["detail"],
		"role description must be 512 characters or fewer"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_role_admin_api_rejects_overlong_name_and_description_on_update(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"name": "Update Limit Role",
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let profile_id = value["id"].as_str().ok_or("missing role id")?.to_string();

	for (payload, expected_detail) in [
		(
			json!({ "data": { "name": "R".repeat(129) } }),
			"role name must be 128 characters or fewer",
		),
		(
			json!({ "data": { "description": "D".repeat(513) } }),
			"role description must be 512 characters or fewer",
		),
	] {
		let (status, value) = request_json(
			&app,
			"PUT",
			&admin_cookie,
			format!("/api/admin/permission-profiles/{profile_id}"),
			Some(payload),
		)
		.await?;
		assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
		assert_eq!(value["error"]["data"]["detail"], expected_detail);
	}

	Ok(())
}
