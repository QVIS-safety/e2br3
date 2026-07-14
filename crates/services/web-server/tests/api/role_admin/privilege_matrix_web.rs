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
	has_permission, AUDIT_LIST, AUDIT_READ, CASE_APPROVE, CASE_CREATE, CASE_LIST,
	CASE_READ, CASE_UPDATE, DASHBOARD_NOTICE_READ, DASHBOARD_NOTICE_UPDATE,
	EMAIL_NOTIFICATION_SEND, PRESAVE_TEMPLATE_CREATE, PRESAVE_TEMPLATE_DELETE,
	PRESAVE_TEMPLATE_LIST, PRESAVE_TEMPLATE_READ, PRESAVE_TEMPLATE_UPDATE,
	SETTINGS_READ, SETTINGS_UPDATE, TERMINOLOGY_APPROVE, TERMINOLOGY_IMPORT,
	USER_CREATE, USER_DELETE, USER_LIST, USER_READ, USER_UPDATE, XML_EXPORT,
	XML_EXPORT_READ, XML_IMPORT, XML_IMPORT_READ,
};
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::ModelManager;
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_role_admin_api_persists_privilege_matrix_menu_keys() -> Result<()> {
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
				"name": "QA matrix role",
				"description": "Created before privilege matrix toggles",
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let profile_id = value["id"].as_str().ok_or("missing role id")?.to_string();

	let matrix_privileges = json!([
		{
			"menu_key": "home_workflow",
			"can_read": true,
			"can_edit": false,
			"can_review": true,
			"can_lock": false
		},
		{
			"menu_key": "home_notice",
			"can_read": true,
			"can_edit": true,
			"can_review": false,
			"can_lock": false
		},
		{
			"menu_key": "case",
			"can_read": true,
			"can_edit": true,
			"can_review": true,
			"can_lock": true
		},
		{
			"menu_key": "info",
			"can_read": true,
			"can_edit": true,
			"can_review": false,
			"can_lock": false
		},
		{
			"menu_key": "import",
			"can_read": true,
			"can_edit": true,
			"can_review": false,
			"can_lock": false
		}
	]);

	let (status, value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		format!("/api/admin/permission-profiles/{profile_id}"),
		Some(json!({ "data": { "privileges": matrix_privileges } })),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{value:?}");

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
	let persisted_role = roles
		.iter()
		.find(|role| role["id"] == profile_id)
		.ok_or("missing persisted matrix role")?;
	let privileges = persisted_role["privileges"]
		.as_array()
		.ok_or("persisted role privileges should be an array")?;
	for (menu_key, can_read, can_edit, can_review, can_lock) in [
		("home_workflow", true, false, true, false),
		("home_notice", true, true, false, false),
		("case", true, true, true, true),
		("info", true, true, false, false),
		("import", true, true, false, false),
	] {
		let row = privileges
			.iter()
			.find(|row| row["menu_key"] == menu_key)
			.ok_or_else(|| format!("missing persisted privilege for {menu_key}"))?;
		assert_eq!(row["can_read"].as_bool(), Some(can_read), "{menu_key}");
		assert_eq!(row["can_edit"].as_bool(), Some(can_edit), "{menu_key}");
		assert_eq!(row["can_review"].as_bool(), Some(can_review), "{menu_key}");
		assert_eq!(row["can_lock"].as_bool(), Some(can_lock), "{menu_key}");
	}
	assert!(has_permission(&profile_id, DASHBOARD_NOTICE_READ));
	assert!(has_permission(&profile_id, DASHBOARD_NOTICE_UPDATE));
	for menu_key in [
		"report_due_mail",
		"monitoring",
		"sync",
		"sync_mapping",
		// Organization management is system-admin only, not a matrix privilege.
		"organization",
		"organizations",
	] {
		let invalid_privileges = json!([
			{
				"menu_key": menu_key,
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]);
		let (status, value) = request_json(
			&app,
			"PUT",
			&admin_cookie,
			format!("/api/admin/permission-profiles/{profile_id}"),
			Some(json!({ "data": { "privileges": invalid_privileges } })),
		)
		.await?;
		assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
		assert!(
			value
				.to_string()
				.contains(&format!("unknown role privilege menu '{menu_key}'")),
			"unexpected unsupported privilege body for {menu_key}: {value:?}"
		);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_home_notice_matrix_privileges_surface_in_current_user_capabilities(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let profile_id = format!("qa_home_notice_{}", Uuid::new_v4().simple());
	let profile_id =
		create_empty_custom_role(&app, &admin_cookie, &profile_id).await?;
	let (_custom_user_id, custom_cookie) =
		custom_role_user(&mm, seed.org_id, &profile_id).await?;

	assert_profile_capabilities(
		&app,
		&custom_cookie,
		&[
			("homeNotice", "read", false),
			("homeNotice", "update", false),
		],
	)
	.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "home_notice",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;

	assert_profile_capabilities(
		&app,
		&custom_cookie,
		&[
			("homeNotice", "read", true),
			("homeNotice", "update", true),
			("settings", "update", false),
		],
	)
	.await?;
	assert!(has_permission(&profile_id, DASHBOARD_NOTICE_READ));
	assert!(has_permission(&profile_id, DASHBOARD_NOTICE_UPDATE));
	assert!(!has_permission(&profile_id, SETTINGS_UPDATE));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_role_privilege_matrix_update_grants_effective_case_access(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"name": "QA effective role",
				"description": "Starts without effective case permissions",
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let profile_id = value["id"].as_str().ok_or("missing role id")?.to_string();

	let custom_user = insert_user(
		&mm,
		seed.org_id,
		&profile_id,
		system_user_id(),
		Some("custompwd"),
	)
	.await?;
	let custom_token =
		generate_web_token(&custom_user.email, custom_user.token_salt)?;
	let custom_cookie = cookie_header(&custom_token.to_string());

	let (status, _value) =
		request_json(&app, "GET", &custom_cookie, "/api/cases".to_string(), None)
			.await?;
	assert_eq!(status, StatusCode::FORBIDDEN);

	let (status, value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		format!("/api/admin/permission-profiles/{profile_id}"),
		Some(json!({
			"data": {
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
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let (status, value) =
		request_json(&app, "GET", &custom_cookie, "/api/cases".to_string(), None)
			.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_info_matrix_privileges_grant_effective_presave_permissions(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let profile_id = format!("qa_info_matrix_{}", Uuid::new_v4().simple());

	let profile_id =
		create_empty_custom_role(&app, &admin_cookie, &profile_id).await?;
	let (custom_user_id, custom_cookie) =
		custom_role_user(&mm, seed.org_id, &profile_id).await?;
	let seed_sender_name = format!("Info Matrix Seed {}", Uuid::new_v4().simple());
	let editable_sender_name =
		format!("Info Matrix Editable {}", Uuid::new_v4().simple());
	let deletable_sender_name =
		format!("Info Matrix Deletable {}", Uuid::new_v4().simple());
	let template_id = create_sender_presave(
		&app,
		&admin_cookie,
		&seed_sender_name,
		"INFO-MATRIX-SEED",
	)
	.await?;
	let editable_template_id = create_sender_presave(
		&app,
		&admin_cookie,
		&editable_sender_name,
		"INFO-MATRIX-EDITABLE",
	)
	.await?;
	let deletable_template_id = create_sender_presave(
		&app,
		&admin_cookie,
		&deletable_sender_name,
		"INFO-MATRIX-DELETABLE",
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		custom_user_id,
		json!({
			"access_sender_ids": [
				template_id.to_string(),
				editable_template_id.to_string(),
				deletable_template_id.to_string()
			]
		}),
	)
	.await?;

	assert_get_status(
		&app,
		&custom_cookie,
		"/api/presaves/senders",
		StatusCode::FORBIDDEN,
	)
	.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "info",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_profile_capabilities(
		&app,
		&custom_cookie,
		&[
			("info", "read", true),
			("info", "create", false),
			("info", "update", false),
			("info", "delete", false),
		],
	)
	.await?;
	assert!(has_permission(&profile_id, PRESAVE_TEMPLATE_READ));
	assert!(has_permission(&profile_id, PRESAVE_TEMPLATE_LIST));
	assert!(!has_permission(&profile_id, PRESAVE_TEMPLATE_CREATE));
	assert!(!has_permission(&profile_id, PRESAVE_TEMPLATE_UPDATE));
	assert!(!has_permission(&profile_id, PRESAVE_TEMPLATE_DELETE));
	assert_get_status(
		&app,
		&custom_cookie,
		"/api/presaves/senders",
		StatusCode::OK,
	)
	.await?;
	assert_get_status(
		&app,
		&custom_cookie,
		&format!("/api/presaves/senders/{template_id}"),
		StatusCode::OK,
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/presaves/senders".to_string(),
		Some(json!({
			"data": {
				"authority": "fda",
				"sender_type": "2",
				"organization_name": "Info Matrix Sender",
				"person_given_name": "Safety"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) = request_json(
		&app,
		"PATCH",
		&custom_cookie,
		format!("/api/presaves/senders/{editable_template_id}"),
		Some(json!({
			"data": {
				"organization_name": "Info Matrix Readonly Patch"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) = request_json(
		&app,
		"DELETE",
		&custom_cookie,
		format!("/api/presaves/senders/{deletable_template_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "info",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_profile_capabilities(
		&app,
		&custom_cookie,
		&[
			("info", "read", true),
			("info", "create", true),
			("info", "update", true),
			("info", "delete", true),
		],
	)
	.await?;
	assert!(has_permission(&profile_id, PRESAVE_TEMPLATE_CREATE));
	assert!(has_permission(&profile_id, PRESAVE_TEMPLATE_UPDATE));
	assert!(has_permission(&profile_id, PRESAVE_TEMPLATE_DELETE));

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/presaves/senders".to_string(),
		Some(json!({
			"data": {
				"authority": "fda",
				"sender_type": "2",
				"organization_name": "INFO-MATRIX-EDIT",
				"person_given_name": "Safety"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let created_template_id = extract_id(&value)?;

	let (status, value) = request_json(
		&app,
		"PATCH",
		&custom_cookie,
		format!("/api/presaves/senders/{editable_template_id}"),
		Some(json!({
			"data": {
				"sender_type": "2",
				"organization_name": editable_sender_name,
				"person_given_name": "Safety"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert!(value["data"].get("name").is_none(), "{value:?}");
	assert!(value["data"].get("comments").is_none(), "{value:?}");
	assert_eq!(
		value["data"]["organization_name"].as_str(),
		Some(editable_sender_name.as_str()),
		"{value:?}"
	);

	let (status, value) = request_json(
		&app,
		"DELETE",
		&custom_cookie,
		format!("/api/presaves/senders/{deletable_template_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::CONFLICT, "{value:?}");
	assert_get_status(
		&app,
		&custom_cookie,
		&format!("/api/presaves/senders/{created_template_id}"),
		StatusCode::FORBIDDEN,
	)
	.await?;

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_data_matrix_privileges_grant_effective_terminology_permissions(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let profile_id = format!("qa_data_matrix_{}", Uuid::new_v4().simple());

	let profile_id =
		create_empty_custom_role(&app, &admin_cookie, &profile_id).await?;
	let (_custom_user_id, custom_cookie) =
		custom_role_user(&mm, seed.org_id, &profile_id).await?;

	assert_get_status(
		&app,
		&custom_cookie,
		"/api/terminology/countries",
		StatusCode::FORBIDDEN,
	)
	.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "data",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_profile_capabilities(
		&app,
		&custom_cookie,
		&[
			("data", "read", true),
			("data", "import", false),
			("data", "approve", false),
		],
	)
	.await?;
	assert_get_status(
		&app,
		&custom_cookie,
		"/api/terminology/countries",
		StatusCode::OK,
	)
	.await?;
	assert!(
		!has_permission(&profile_id, TERMINOLOGY_IMPORT),
		"read-only DATA must not grant terminology import permission"
	);
	assert!(
		!has_permission(&profile_id, TERMINOLOGY_APPROVE),
		"read-only DATA must not grant terminology approve permission"
	);

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/import/meddra?version=27.1&language=en")
		.header("cookie", custom_cookie.clone())
		.header("content-type", "multipart/form-data; boundary=----boundary")
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(
		res.status(),
		StatusCode::FORBIDDEN,
		"read-only DATA must not import terminology"
	);

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/terminology/releases/meddra/TEST/approve".to_string(),
		None,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"read-only DATA must not approve terminology releases: {value:?}"
	);

	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "data",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_profile_capabilities(
		&app,
		&custom_cookie,
		&[
			("data", "read", true),
			("data", "import", true),
			("data", "approve", true),
		],
	)
	.await?;
	assert!(
		has_permission(&profile_id, TERMINOLOGY_IMPORT),
		"editable DATA must grant terminology import permission"
	);
	assert!(
		has_permission(&profile_id, TERMINOLOGY_APPROVE),
		"editable DATA must grant terminology approve permission"
	);

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/import/meddra?version=27.1&language=en")
		.header("cookie", custom_cookie.clone())
		.header("content-type", "multipart/form-data; boundary=----boundary")
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_ne!(
		res.status(),
		StatusCode::FORBIDDEN,
		"editable DATA should pass terminology import permission check"
	);

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/terminology/releases/meddra/TEST/approve".to_string(),
		None,
	)
	.await?;
	assert_ne!(
		status,
		StatusCode::FORBIDDEN,
		"editable DATA should pass terminology approve permission check: {value:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_export_submission_matrix_privileges_grant_effective_xml_export_permission(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let no_export_profile_name =
		format!("qa_export_none_{}", Uuid::new_v4().simple());
	let read_profile_name = format!("qa_export_read_{}", Uuid::new_v4().simple());
	let edit_profile_name = format!("qa_export_edit_{}", Uuid::new_v4().simple());

	let no_export_profile_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&no_export_profile_name,
	)
	.await?;
	let read_profile_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&read_profile_name,
	)
	.await?;
	let edit_profile_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&edit_profile_name,
	)
	.await?;
	let (_no_export_user_id, no_export_cookie) =
		custom_role_user(&mm, seed.org_id, &no_export_profile_id).await?;
	let (_read_user_id, read_cookie) =
		custom_role_user(&mm, seed.org_id, &read_profile_id).await?;
	let (_edit_user_id, edit_cookie) =
		custom_role_user(&mm, seed.org_id, &edit_profile_id).await?;

	assert!(
		!has_permission(&no_export_profile_id, XML_EXPORT),
		"empty custom role must not grant XML_EXPORT"
	);
	assert!(
		!has_permission(&no_export_profile_id, XML_EXPORT_READ),
		"empty custom role must not grant XML_EXPORT_READ"
	);
	assert_get_status(
		&app,
		&no_export_cookie,
		"/api/exports/history",
		StatusCode::FORBIDDEN,
	)
	.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&read_profile_id,
		json!([
			{
				"menu_key": "export_submission",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_profile_capabilities(
		&app,
		&read_cookie,
		&[
			("exportSubmission", "read", true),
			("exportSubmission", "execute", false),
		],
	)
	.await?;
	assert!(
		has_permission(&read_profile_id, XML_EXPORT_READ),
		"export_submission.can_read must grant XML_EXPORT_READ"
	);
	assert!(
		!has_permission(&read_profile_id, XML_EXPORT),
		"export_submission.can_read must not grant XML_EXPORT"
	);
	assert_get_not_status(
		&app,
		&read_cookie,
		"/api/exports/history",
		StatusCode::FORBIDDEN,
	)
	.await?;
	let (status, value) = request_json(
		&app,
		"POST",
		&read_cookie,
		"/api/cases/export/xml".to_string(),
		Some(json!({ "case_ids": [] })),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"export_submission.can_read must not execute XML export: {value:?}"
	);

	update_role_privileges(
		&app,
		&admin_cookie,
		&edit_profile_id,
		json!([
			{
				"menu_key": "export_submission",
				"can_read": false,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_profile_capabilities(
		&app,
		&edit_cookie,
		&[
			("exportSubmission", "read", false),
			("exportSubmission", "execute", true),
		],
	)
	.await?;
	assert!(
		has_permission(&edit_profile_id, XML_EXPORT),
		"export_submission.can_edit must independently grant XML_EXPORT"
	);
	assert!(
		!has_permission(&edit_profile_id, XML_EXPORT_READ),
		"export_submission.can_edit must not grant history read without can_read"
	);
	assert_get_status(
		&app,
		&edit_cookie,
		"/api/exports/history",
		StatusCode::FORBIDDEN,
	)
	.await?;
	let (status, value) = request_json(
		&app,
		"POST",
		&edit_cookie,
		"/api/cases/export/xml".to_string(),
		Some(json!({ "case_ids": [] })),
	)
	.await?;
	assert_ne!(
		status,
		StatusCode::FORBIDDEN,
		"export_submission.can_edit should pass XML export permission check: {value:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_import_matrix_privileges_split_files_edit_from_history_read(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let read_profile_name = format!("qa_import_read_{}", Uuid::new_v4().simple());
	let edit_profile_name = format!("qa_import_edit_{}", Uuid::new_v4().simple());

	let read_profile_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&read_profile_name,
	)
	.await?;
	let edit_profile_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&edit_profile_name,
	)
	.await?;
	let (_read_user_id, read_cookie) =
		custom_role_user(&mm, seed.org_id, &read_profile_id).await?;
	let (_edit_user_id, edit_cookie) =
		custom_role_user(&mm, seed.org_id, &edit_profile_id).await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&read_profile_id,
		json!([
			{
				"menu_key": "import",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_profile_capabilities(
		&app,
		&read_cookie,
		&[("import", "read", true), ("import", "execute", false)],
	)
	.await?;
	assert!(has_permission(&read_profile_id, XML_IMPORT_READ));
	assert!(!has_permission(&read_profile_id, XML_IMPORT));
	assert_get_status(
		&app,
		&read_cookie,
		"/api/import/xml/history",
		StatusCode::OK,
	)
	.await?;
	let req = Request::builder()
		.method("POST")
		.uri("/api/import/xml")
		.header("cookie", read_cookie.clone())
		.header("content-type", "multipart/form-data; boundary=----boundary")
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(
		res.status(),
		StatusCode::FORBIDDEN,
		"import.can_read must not execute XML import"
	);

	update_role_privileges(
		&app,
		&admin_cookie,
		&edit_profile_id,
		json!([
			{
				"menu_key": "import",
				"can_read": false,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_profile_capabilities(
		&app,
		&edit_cookie,
		&[("import", "read", false), ("import", "execute", true)],
	)
	.await?;
	assert!(has_permission(&edit_profile_id, XML_IMPORT));
	assert!(!has_permission(&edit_profile_id, XML_IMPORT_READ));
	assert_get_status(
		&app,
		&edit_cookie,
		"/api/import/xml/history",
		StatusCode::FORBIDDEN,
	)
	.await?;
	let req = Request::builder()
		.method("POST")
		.uri("/api/import/xml")
		.header("cookie", edit_cookie.clone())
		.header("content-type", "multipart/form-data; boundary=----boundary")
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_ne!(
		res.status(),
		StatusCode::FORBIDDEN,
		"import.can_edit should pass XML import permission check"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_users_and_roles_matrix_privileges_grant_effective_admin_permissions(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let profile_id = format!("qa_users_roles_matrix_{}", Uuid::new_v4().simple());

	let profile_id =
		create_empty_custom_role(&app, &admin_cookie, &profile_id).await?;
	let (_custom_user_id, custom_cookie) =
		custom_role_user(&mm, seed.org_id, &profile_id).await?;

	assert_profile_capabilities(
		&app,
		&custom_cookie,
		&[
			("admin", "read", false),
			("admin", "update", false),
			("users", "read", false),
			("users", "create", false),
			("users", "update", false),
			("users", "delete", false),
			("roles", "read", false),
			("roles", "create", false),
			("roles", "update", false),
			("roles", "delete", false),
		],
	)
	.await?;
	assert_get_status(&app, &custom_cookie, "/api/users", StatusCode::FORBIDDEN)
		.await?;
	assert_get_status(
		&app,
		&custom_cookie,
		"/api/admin/permission-profiles",
		StatusCode::FORBIDDEN,
	)
	.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "users",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert!(has_permission(&profile_id, USER_READ));
	assert!(has_permission(&profile_id, USER_LIST));
	assert!(!has_permission(&profile_id, USER_CREATE));
	assert!(!has_permission(&profile_id, USER_UPDATE));
	assert!(!has_permission(&profile_id, USER_DELETE));
	assert_profile_capabilities(
		&app,
		&custom_cookie,
		&[
			("admin", "read", false),
			("admin", "update", false),
			("users", "read", true),
			("users", "create", false),
			("users", "update", false),
			("users", "delete", false),
			("roles", "read", false),
			("roles", "create", false),
			("roles", "update", false),
			("roles", "delete", false),
		],
	)
	.await?;
	assert_get_status(&app, &custom_cookie, "/api/users", StatusCode::FORBIDDEN)
		.await?;
	let (status, value) = request_json(
		&app,
		"PUT",
		&custom_cookie,
		format!("/api/users/{}", seed.viewer.id),
		Some(json!({
			"data": {
				"comments": "users read must not update"
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"users.can_read must not update users: {value:?}"
	);

	let roles_profile_id = format!("qa_roles_matrix_{}", Uuid::new_v4().simple());
	let roles_profile_id =
		create_empty_custom_role(&app, &admin_cookie, &roles_profile_id).await?;
	let (_roles_user_id, roles_cookie) =
		custom_role_user(&mm, seed.org_id, &roles_profile_id).await?;
	update_role_privileges(
		&app,
		&admin_cookie,
		&roles_profile_id,
		json!([
			{
				"menu_key": "roles",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert!(has_permission(&roles_profile_id, USER_CREATE));
	assert!(has_permission(&roles_profile_id, USER_UPDATE));
	assert!(has_permission(&roles_profile_id, USER_DELETE));
	assert_profile_capabilities(
		&app,
		&roles_cookie,
		&[
			("admin", "read", true),
			("admin", "update", true),
			("users", "create", true),
			("users", "update", true),
			("users", "delete", true),
			("roles", "read", true),
			("roles", "create", true),
			("roles", "update", true),
			("roles", "delete", true),
		],
	)
	.await?;
	let (status, value) = request_json(
		&app,
		"POST",
		&roles_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"name": "Roles Matrix Child",
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"roles.can_edit should create permission profiles: {value:?}"
	);

	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "users",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert!(has_permission(&profile_id, USER_CREATE));
	assert!(has_permission(&profile_id, USER_UPDATE));
	assert!(has_permission(&profile_id, USER_DELETE));
	assert_profile_capabilities(
		&app,
		&custom_cookie,
		&[
			("admin", "read", true),
			("admin", "update", true),
			("users", "read", true),
			("users", "create", true),
			("users", "update", true),
			("users", "delete", true),
			("roles", "read", true),
			("roles", "create", true),
			("roles", "update", true),
			("roles", "delete", true),
		],
	)
	.await?;
	assert_get_status(&app, &custom_cookie, "/api/users", StatusCode::OK).await?;

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"name": "Users Roles Child",
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_settings_admin_matrix_grants_only_settings_route_access() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let profile_id = format!("qa_settings_admin_{}", Uuid::new_v4().simple());

	let profile_id =
		create_empty_custom_role(&app, &admin_cookie, &profile_id).await?;
	let (_custom_user_id, custom_cookie) =
		custom_role_user(&mm, seed.org_id, &profile_id).await?;

	assert_get_status(&app, &custom_cookie, "/api/users", StatusCode::FORBIDDEN)
		.await?;
	assert_get_status(
		&app,
		&custom_cookie,
		"/api/admin/settings",
		StatusCode::FORBIDDEN,
	)
	.await?;
	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/users".to_string(),
		Some(json!({
			"data": {
				"organization_id": seed.org_id,
				"email": format!("settings-admin-empty-{}@example.com", Uuid::new_v4()),
				"role": "viewer"
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"empty settings role must not create users: {value:?}"
	);

	let value = update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "settings",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_eq!(
		value["sponsor_admin_capable"].as_bool(),
		Some(false),
		"settings.can_read alone must not make the role Safety DB admin capable: {value:?}"
	);
	assert_profile_capabilities(
		&app,
		&custom_cookie,
		&[
			("admin", "read", false),
			("admin", "update", false),
			("users", "read", false),
			("users", "create", false),
			("roles", "read", false),
			("roles", "create", false),
		],
	)
	.await?;
	assert!(
		!has_permission(&profile_id, CASE_CREATE),
		"settings.can_read alone must not grant raw CASE_CREATE permission"
	);
	assert!(
		!has_permission(&profile_id, USER_CREATE),
		"settings.can_read alone must not grant raw USER_CREATE permission"
	);
	assert_get_status(&app, &custom_cookie, "/api/admin/settings", StatusCode::OK)
		.await?;
	let (status, value) = request_json(
		&app,
		"PUT",
		&custom_cookie,
		"/api/admin/settings".to_string(),
		Some(json!({
			"data": {
				"idle_session_minutes": 45,
				"session_warning_minutes": 5
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"settings.can_read alone must not update admin settings: {value:?}"
	);
	assert!(has_permission(&profile_id, SETTINGS_READ));
	assert!(!has_permission(&profile_id, SETTINGS_UPDATE));
	assert_get_status(&app, &custom_cookie, "/api/users", StatusCode::FORBIDDEN)
		.await?;
	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/cases".to_string(),
		Some(json!({
			"data": {
				"safetyReportIdentification": {
					"safetyReportId": format!("SETTINGS-READ-{}", Uuid::new_v4().simple())
				},
				"status": "draft"
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"settings.can_read alone must not create cases via raw permissions: {value:?}"
	);
	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/users".to_string(),
		Some(json!({
			"data": {
				"organization_id": seed.org_id,
				"email": format!("settings-admin-read-{}@example.com", Uuid::new_v4()),
				"role": "viewer"
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"settings.can_read alone must not create users: {value:?}"
	);

	let value = update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "settings",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_eq!(
		value["sponsor_admin_capable"].as_bool(),
		Some(false),
		"settings.can_edit alone must not make the role broadly admin capable: {value:?}"
	);
	assert_profile_capabilities(
		&app,
		&custom_cookie,
		&[
			("admin", "read", false),
			("admin", "update", false),
			("users", "read", false),
			("users", "create", false),
			("users", "update", false),
			("users", "delete", false),
			("roles", "read", false),
			("roles", "create", false),
			("roles", "update", false),
			("roles", "delete", false),
			("settings", "read", true),
			("settings", "update", true),
		],
	)
	.await?;
	assert!(has_permission(&profile_id, SETTINGS_READ));
	assert!(has_permission(&profile_id, SETTINGS_UPDATE));
	assert!(!has_permission(&profile_id, USER_CREATE));
	assert!(!has_permission(&profile_id, USER_UPDATE));
	assert!(!has_permission(&profile_id, USER_DELETE));
	assert_get_status(&app, &custom_cookie, "/api/users", StatusCode::FORBIDDEN)
		.await?;
	let (status, value) = request_json(
		&app,
		"PUT",
		&custom_cookie,
		"/api/admin/settings".to_string(),
		Some(json!({
			"data": {
				"idle_session_minutes": 45,
				"session_warning_minutes": 5
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::OK,
		"settings.can_edit should update admin settings: {value:?}"
	);
	let (status, value) = request_json(
		&app,
		"DELETE",
		&custom_cookie,
		format!("/api/admin/permission-profiles/{ROLE_SYSTEM_ADMIN}"),
		None,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"settings.can_edit must not delete roles: {value:?}"
	);
	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/users".to_string(),
		Some(json!({
			"data": {
				"organization_id": seed.org_id,
				"email": format!("settings-admin-edit-{}@example.com", Uuid::new_v4()),
				"role": "viewer"
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"settings.can_edit must not create users through POST /api/users: {value:?}"
	);

	Ok(())
}

// Gap coverage: home_workflow read privilege must grant effective case-list
// access (GET /api/cases/list-view is guarded by CASE_LIST).
#[serial]
#[tokio::test]
async fn test_home_workflow_matrix_privileges_grant_effective_case_list_access(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	let none_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&format!("qa_hw_none_{}", Uuid::new_v4().simple()),
	)
	.await?;
	let read_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&format!("qa_hw_read_{}", Uuid::new_v4().simple()),
	)
	.await?;
	let (_none_user, none_cookie) =
		custom_role_user(&mm, seed.org_id, &none_id).await?;
	let (_read_user, read_cookie) =
		custom_role_user(&mm, seed.org_id, &read_id).await?;

	// Unchecked: no case access.
	assert!(!has_permission(&none_id, CASE_LIST));
	assert_get_status(
		&app,
		&none_cookie,
		"/api/cases/list-view",
		StatusCode::FORBIDDEN,
	)
	.await?;

	// home_workflow read grants case view + list, but not write.
	update_role_privileges(
		&app,
		&admin_cookie,
		&read_id,
		json!([{
			"menu_key": "home_workflow",
			"can_read": true,
			"can_edit": false,
			"can_review": false,
			"can_lock": false
		}]),
	)
	.await?;
	assert!(has_permission(&read_id, CASE_READ));
	assert!(has_permission(&read_id, CASE_LIST));
	assert!(!has_permission(&read_id, CASE_CREATE));
	assert_get_not_status(
		&app,
		&read_cookie,
		"/api/cases/list-view",
		StatusCode::FORBIDDEN,
	)
	.await?;

	Ok(())
}

// Gap coverage: audit read (or review) privilege must grant effective audit-log
// access (GET /api/audit-logs is guarded by AUDIT_LIST).
#[serial]
#[tokio::test]
async fn test_audit_matrix_privileges_grant_effective_audit_log_access() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	let none_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&format!("qa_audit_none_{}", Uuid::new_v4().simple()),
	)
	.await?;
	let read_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&format!("qa_audit_read_{}", Uuid::new_v4().simple()),
	)
	.await?;
	let (_none_user, none_cookie) =
		custom_role_user(&mm, seed.org_id, &none_id).await?;
	let (_read_user, read_cookie) =
		custom_role_user(&mm, seed.org_id, &read_id).await?;

	// Unchecked: no audit access.
	assert!(!has_permission(&none_id, AUDIT_LIST));
	assert_get_status(&app, &none_cookie, "/api/audit-logs", StatusCode::FORBIDDEN)
		.await?;

	// audit read grants AUDIT_READ + AUDIT_LIST.
	update_role_privileges(
		&app,
		&admin_cookie,
		&read_id,
		json!([{
			"menu_key": "audit",
			"can_read": true,
			"can_edit": false,
			"can_review": false,
			"can_lock": false
		}]),
	)
	.await?;
	assert!(has_permission(&read_id, AUDIT_READ));
	assert!(has_permission(&read_id, AUDIT_LIST));
	assert_get_not_status(
		&app,
		&read_cookie,
		"/api/audit-logs",
		StatusCode::FORBIDDEN,
	)
	.await?;

	Ok(())
}

// Organization management is system-admin only (require_system_admin), and is
// intentionally not a profile-matrix privilege. A sponsor admin must be denied
// listing organizations. Locks the "org = system-admin only" contract.
#[serial]
#[tokio::test]
async fn test_organization_management_requires_system_admin() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	// seed.admin is a sponsor admin (ROLE_SPONSOR_ADMIN_CRO), not a system admin.
	assert_get_status(
		&app,
		&admin_cookie,
		"/api/organizations",
		StatusCode::FORBIDDEN,
	)
	.await?;

	Ok(())
}

// The frontend exposes a "home_email" (E-mail / Send) checkbox. The backend
// must accept it (not reject as unknown) and grant the reserved e-mail send
// permission so the checkbox persists. The e-mail feature itself is pending, so
// no endpoint enforces the permission yet.
#[serial]
#[tokio::test]
async fn test_home_email_matrix_privilege_persists_and_grants_send() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	let profile_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&format!("qa_email_{}", Uuid::new_v4().simple()),
	)
	.await?;
	assert!(!has_permission(&profile_id, EMAIL_NOTIFICATION_SEND));

	// home_email is accepted (200, not BAD_REQUEST) and its Send checkbox grants
	// the reserved permission.
	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([{
			"menu_key": "home_email",
			"can_read": false,
			"can_edit": true,
			"can_review": false,
			"can_lock": false
		}]),
	)
	.await?;
	assert!(has_permission(&profile_id, EMAIL_NOTIFICATION_SEND));

	Ok(())
}
