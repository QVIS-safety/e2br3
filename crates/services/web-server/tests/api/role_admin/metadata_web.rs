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
	let system = roles
		.iter()
		.find(|role| role["id"] == ROLE_SYSTEM_ADMIN)
		.ok_or("missing system permission profile")?;
	assert_eq!(system["is_operational"].as_bool(), Some(false));
	assert_eq!(system["is_editable"].as_bool(), Some(false));

	let sponsor = roles
		.iter()
		.find(|role| role["id"] == ROLE_SPONSOR_ADMIN_CRO)
		.ok_or("missing sponsor permission profile")?;
	assert_eq!(sponsor["is_builtin"].as_bool(), Some(true));
	assert_eq!(sponsor["is_sponsor_admin"].as_bool(), Some(true));
	assert_eq!(sponsor["is_editable"].as_bool(), Some(false));
	let sponsor_privileges = sponsor["privileges"]
		.as_array()
		.ok_or("sponsor privileges should be an array")?;
	for menu_key in [
		"case",
		"info",
		"import",
		"export_submission",
		"users",
		"roles",
		"settings",
		"audit",
		"data",
	] {
		let privilege = sponsor_privileges
			.iter()
			.find(|row| row["menu_key"] == menu_key)
			.ok_or_else(|| format!("missing sponsor privilege for {menu_key}"))?;
		assert_eq!(privilege["can_read"].as_bool(), Some(true), "{menu_key}");
		assert_eq!(privilege["can_edit"].as_bool(), Some(true), "{menu_key}");
	}

	let system_privileges = system["privileges"]
		.as_array()
		.ok_or("system privileges should be an array")?;
	assert!(
		system_privileges.is_empty(),
		"system admin should not receive Safety DB working menu privileges"
	);

	let (status, value) = request_json(
		&app,
		"GET",
		&admin_cookie,
		format!("/api/admin/permission-profiles/{ROLE_SPONSOR_ADMIN_CRO}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["id"], ROLE_SPONSOR_ADMIN_CRO);

	let (status, _value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		format!("/api/admin/permission-profiles/{ROLE_SPONSOR_ADMIN_CRO}"),
		Some(json!({ "data": { "name": "Should Not Change" } })),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_role_admin_api_filters_sponsor_built_ins_by_org_type() -> Result<()> {
	let mm = init_test_mm().await?;
	let cro_seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let cro_token =
		generate_web_token(&cro_seed.admin.email, cro_seed.admin.token_salt)?;
	let cro_cookie = cookie_header(&cro_token.to_string());

	let company_seed = seed_org_with_users(&mm, "companypwd", "companyview").await?;
	set_org_type(&mm, company_seed.org_id, "pharmaceutical_company").await?;
	let company_admin = insert_user(
		&mm,
		company_seed.org_id,
		ROLE_SPONSOR_ADMIN_COMPANY,
		system_user_id(),
		Some("companypwd"),
	)
	.await?;
	let company_token =
		generate_web_token(&company_admin.email, company_admin.token_salt)?;
	let company_cookie = cookie_header(&company_token.to_string());
	let app = web_server::app(mm);

	let (status, cro_value) = request_json(
		&app,
		"GET",
		&cro_cookie,
		"/api/admin/permission-profiles".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{cro_value:?}");
	let cro_roles = cro_value.as_array().ok_or("CRO roles should be array")?;
	assert!(cro_roles
		.iter()
		.any(|role| role["id"] == ROLE_SPONSOR_ADMIN_CRO));
	assert!(!cro_roles
		.iter()
		.any(|role| role["id"] == ROLE_SPONSOR_ADMIN_COMPANY));

	let (status, company_value) = request_json(
		&app,
		"GET",
		&company_cookie,
		"/api/admin/permission-profiles".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{company_value:?}");
	let company_roles = company_value
		.as_array()
		.ok_or("company roles should be array")?;
	assert!(company_roles
		.iter()
		.any(|role| role["id"] == ROLE_SPONSOR_ADMIN_COMPANY));
	assert!(!company_roles
		.iter()
		.any(|role| role["id"] == ROLE_SPONSOR_ADMIN_CRO));

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
