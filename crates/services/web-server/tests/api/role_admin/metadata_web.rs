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
async fn test_role_admin_api_exposes_client_role_metadata() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let (status, value) = request_json(
		&app,
		"GET",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let roles = value.as_array().ok_or("roles response should be array")?;
	let sponsor_cro = roles
		.iter()
		.find(|role| role["id"] == ROLE_SPONSOR_ADMIN_CRO)
		.ok_or("missing CRO sponsor permission profile")?;
	assert_eq!(sponsor_cro["is_operational"].as_bool(), Some(true));
	assert_eq!(sponsor_cro["is_editable"].as_bool(), Some(false));

	assert!(!roles.iter().any(|role| role["id"] == ROLE_SYSTEM_ADMIN));
	assert!(!roles
		.iter()
		.any(|role| role["id"] == ROLE_SPONSOR_ADMIN_COMPANY));

	let sponsor_privileges = sponsor_cro["privileges"]
		.as_array()
		.ok_or("sponsor privileges should be an array")?;
	assert!(
		!sponsor_privileges.is_empty(),
		"CRO sponsor admin should expose its fixed Safety DB privileges"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_restoring_twenty_first_active_custom_role_returns_conflict(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	for index in 0..20 {
		let (status, value) = request_json(
			&app,
			"POST",
			&admin_cookie,
			"/api/admin/permission-profiles".to_string(),
			Some(json!({
				"data": {
					"name": format!("Active PDF role {index}"),
					"active": true,
					"privileges": []
				}
			})),
		)
		.await?;
		assert_eq!(status, StatusCode::CREATED, "index={index} {value:?}");
	}

	let (status, inactive) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"name": "Inactive PDF role",
				"active": false,
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{inactive:?}");
	let inactive_id = inactive["id"].as_str().ok_or("missing inactive id")?;

	let (status, value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		format!("/api/admin/permission-profiles/{inactive_id}"),
		Some(json!({ "data": { "active": true } })),
	)
	.await?;
	assert_eq!(status, StatusCode::CONFLICT, "{value:?}");
	assert!(value.to_string().contains("active custom role limit is 20"));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_role_admin_api_defaults_visible_name_to_role_id() -> Result<()> {
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
				"description": "Role created with description only",
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
		})),
	)
	.await?;

	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	Uuid::parse_str(value["id"].as_str().ok_or("missing role id")?)?;
	assert_eq!(value["name"], "Custom Role");
	assert_eq!(value["description"], "Role created with description only");

	Ok(())
}
