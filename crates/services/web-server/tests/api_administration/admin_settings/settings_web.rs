#![allow(unused_imports)]

use crate::administration_support::request_json;
use crate::common::{
	cookie_header, init_test_mm, seed_org_with_users, system_user_id, Result,
};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_meddra_settings_reject_unavailable_release_without_changing_saved_settings(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let version_prefix = format!("91.{}", Uuid::new_v4().as_u128() % 100000);
	let statuses = ["validated", "approved", "active", "loading", "retired"];
	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	lib_core::model::store::set_full_context_from_ctx_dbx(
		dbx,
		&lib_core::ctx::Ctx::root_ctx(),
	)
	.await?;
	for (index, status) in statuses.iter().enumerate() {
		lib_core::model::terminology_import::upsert_release_header(
			&mm,
			"meddra",
			&format!("{version_prefix}{index}"),
			"en",
			status,
			"settings-regression",
			None,
			1,
			None,
			None,
			None,
		)
		.await?;
	}
	dbx.commit_txn().await?;
	let app = web_server::app(mm);
	for (index, release_status) in statuses.iter().enumerate() {
		let (status, value) = request_json(&app, "PUT", &cookie, "/api/admin/settings",
			Some(json!({"data": {"meddra_language": "English", "meddra_version": format!("{version_prefix}{index}")}})),
		).await?;
		assert_eq!(
			status,
			if index < 3 {
				StatusCode::OK
			} else {
				StatusCode::BAD_REQUEST
			},
			"{release_status}: {value:?}"
		);
	}
	for data in [
		json!({"meddra_language": "Korean", "meddra_version": format!("{version_prefix}2")}),
		json!({"meddra_language": "English", "meddra_version": format!("{version_prefix}9")}),
	] {
		let (status, value) = request_json(
			&app,
			"PUT",
			&cookie,
			"/api/admin/settings",
			Some(json!({"data": data})),
		)
		.await?;
		assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	}
	let (status, value) =
		request_json(&app, "GET", &cookie, "/api/admin/settings", None).await?;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(value["meddra_language"], "English");
	assert_eq!(value["meddra_version"], format!("{version_prefix}2"));
	let (status, value) = request_json(
		&app,
		"PUT",
		&cookie,
		"/api/admin/settings",
		Some(json!({"data": {"meddra_language": "en"}})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	Ok(())
}

#[tokio::test]
async fn test_idle_session_settings_are_org_scoped_and_validated() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let other_seed =
		seed_org_with_users(&mm, "otheradminpwd", "otherviewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let other_viewer_token =
		generate_web_token(&other_seed.viewer.email, other_seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let other_viewer_cookie = cookie_header(&other_viewer_token.to_string());
	let app = web_server::app(mm);

	let (status, value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"idle_session_minutes": 30,
				"session_warning_minutes": 10
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["idle_session_minutes"], 30);
	assert_eq!(value["session_warning_minutes"], 10);

	let (status, value) =
		request_json(&app, "GET", &viewer_cookie, "/api/admin/settings", None)
			.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) =
		request_json(&app, "GET", &viewer_cookie, "/api/settings/runtime", None)
			.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["idle_session_minutes"], 30);
	assert_eq!(value["session_warning_minutes"], 10);

	let (status, value) = request_json(
		&app,
		"GET",
		&other_viewer_cookie,
		"/api/settings/runtime",
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["idle_session_minutes"], 60);
	assert_eq!(value["session_warning_minutes"], 5);

	let (status, _value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"idle_session_minutes": 4,
				"session_warning_minutes": 1
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST);

	let (status, _value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"idle_session_minutes": 30,
				"session_warning_minutes": 30
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_settings_round_trips_alignment_fields() -> Result<()> {
	let mm = init_test_mm().await?;
	lib_core::model::terminology_import::stage_meddra_rows(
		&mm,
		system_user_id(),
		&[lib_core::model::terminology_import::MeddraRow {
			code: "10000001".to_string(),
			term: "Settings regression term".to_string(),
			level: "LLT".to_string(),
		}],
		"28.0",
		"en",
		"settings-regression",
	)
	.await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = request_json(
		&app,
		"PUT",
		&cookie,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"timezone": "Asia/Seoul",
				"meddra_language": "English",
				"meddra_version": "28.0",
				"idf_version": "3.0",
				"company_logo": "qvis.png",
				"orientation": "Landscape",
				"data_ordering": "Primary data will appear first",
				"upload_excel_template_without_element_label": true,
				"notation": false,
				"apply_comments_on_exported_xml": true,
				"apply_sender_info_to_imported_cases": true,
				"import_date_update": {
					"date_of_creation": true,
					"most_recent_info_date": true,
					"report_first_received_date": false
				},
				"appendices": ["ICH", "FDA", "MFDS"],
				"case_number_setting": "AE Row No.",
				"case_number_identifier": "SAFETY",
				"case_number_padding": 7,
				"case_number_sequence_condition": "Per sender",
				"case_number_format_fields": ["AE Row No."],
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"allowed_roles": ["pvs"],
							"due_days": 0,
							"description": "Default state"
						}
					]
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let data = &body;
	assert_eq!(data["meddra_version"].as_str(), Some("28.0"));
	assert_eq!(data["idf_version"].as_str(), Some("3.0"));
	assert_eq!(data["orientation"].as_str(), Some("Landscape"));
	assert_eq!(data["apply_comments_on_exported_xml"].as_bool(), Some(true));
	assert_eq!(
		data["import_date_update"]["most_recent_info_date"].as_bool(),
		Some(true)
	);
	assert_eq!(
		data["case_number_format_fields"][0].as_str(),
		Some("AE Row No.")
	);
	assert_eq!(
		data["workflow"]["statuses"][0]["due_days"].as_i64(),
		Some(0)
	);

	let (status, body) =
		request_json(&app, "GET", &cookie, "/api/admin/settings", None).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let data = &body;
	assert_eq!(data["company_logo"].as_str(), Some("qvis.png"));
	assert_eq!(
		data["case_number_sequence_condition"].as_str(),
		Some("Per sender")
	);

	let (status, body) = request_json(
		&app,
		"PUT",
		&cookie,
		"/api/admin/settings",
		Some(json!({"data": {"import_date_update": {
			"date_of_creation": true,
			"most_recent_info_date": false,
			"report_first_received_date": false
		}}})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, body) = request_json(
		&app,
		"PUT",
		&cookie,
		"/api/admin/settings",
		Some(json!({"data": {"import_date_update": {
			"date_of_creation": true,
			"most_recent_info_date": false,
			"report_first_received_date": true
		}}})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	Ok(())
}
