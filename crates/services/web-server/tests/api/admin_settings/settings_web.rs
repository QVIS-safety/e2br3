#![allow(unused_imports)]

use super::helpers::*;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
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
		let (status, value) = request_json(
			&app, &cookie, Method::PUT, "/api/admin/settings",
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
			&cookie,
			Method::PUT,
			"/api/admin/settings",
			Some(json!({"data": data})),
		)
		.await?;
		assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	}
	let (status, value) =
		request_json(&app, &cookie, Method::GET, "/api/admin/settings", None)
			.await?;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(value["meddra_language"], "English");
	assert_eq!(value["meddra_version"], format!("{version_prefix}2"));
	let (status, value) = request_json(
		&app,
		&cookie,
		Method::PUT,
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
		&admin_cookie,
		Method::PUT,
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

	let (status, value) = request_json(
		&app,
		&viewer_cookie,
		Method::GET,
		"/api/admin/settings",
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) = request_json(
		&app,
		&viewer_cookie,
		Method::GET,
		"/api/settings/runtime",
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["idle_session_minutes"], 30);
	assert_eq!(value["session_warning_minutes"], 10);

	let (status, value) = request_json(
		&app,
		&other_viewer_cookie,
		Method::GET,
		"/api/settings/runtime",
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["idle_session_minutes"], 60);
	assert_eq!(value["session_warning_minutes"], 5);

	let (status, _value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
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
		&admin_cookie,
		Method::PUT,
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
