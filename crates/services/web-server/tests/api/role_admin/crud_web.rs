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

#[tokio::test]
async fn test_role_admin_api_allows_new_role_without_privileges() -> Result<()> {
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
				"name": "QA empty privilege role",
				"description": "Created before privileges are assigned",
				"privileges": []
			}
		})),
	)
	.await?;

	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	Uuid::parse_str(value["id"].as_str().ok_or("missing role id")?)?;
	assert_eq!(value["name"], "QA empty privilege role");
	assert_eq!(value["privileges"].as_array().map(Vec::len), Some(0));
	assert_eq!(value["can_view"].as_bool(), Some(false));
	assert_eq!(value["can_admin"].as_bool(), Some(false));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_role_admin_api_preserves_description_equal_to_name() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let role_name = "QA Same Description Role";

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"name": role_name,
				"description": role_name,
				"privileges": []
			}
		})),
	)
	.await?;

	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	Uuid::parse_str(value["id"].as_str().ok_or("missing role id")?)?;
	assert_eq!(value["name"], role_name);
	assert_eq!(value["description"], role_name);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_role_admin_api_persists_menu_privileges() -> Result<()> {
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
				"name": "QA Role",
				"description": "Can edit cases and read audit",
				"privileges": [
					{
						"menu_key": "case",
						"can_read": true,
						"can_edit": true,
						"can_review": false,
						"can_lock": false
					},
					{
						"menu_key": "audit",
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
	let profile_id = value["id"].as_str().ok_or("missing role id")?.to_string();
	assert_eq!(value["description"], "Can edit cases and read audit");
	assert_eq!(value["can_view"].as_bool(), Some(true));
	assert_eq!(value["can_admin"].as_bool(), Some(false));
	assert_eq!(
		value["privilege_map"]["case"]["can_edit"].as_bool(),
		Some(true)
	);
	assert_eq!(
		value["privilege_map"]["audit"]["can_read"].as_bool(),
		Some(true)
	);

	let (status, value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		format!("/api/admin/permission-profiles/{profile_id}"),
		Some(json!({
			"data": {
				"description": "Can lock cases",
				"privileges": [
					{
						"menu_key": "case",
						"can_read": true,
						"can_edit": true,
						"can_review": true,
						"can_lock": true
					}
				]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["description"], "Can lock cases");
	assert_eq!(value["can_review"].as_bool(), Some(true));
	assert_eq!(value["can_lock"].as_bool(), Some(true));
	assert_eq!(
		value["privilege_map"]["case"]["can_lock"].as_bool(),
		Some(true)
	);
	Ok(())
}
