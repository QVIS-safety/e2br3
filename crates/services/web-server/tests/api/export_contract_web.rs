use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SYSTEM_ADMIN;
use lib_core::model::store::{set_org_context, set_user_context};
use serde_json::Value;
use serial_test::serial;
use std::io::Cursor;
use tower::ServiceExt;
use uuid::Uuid;
use zip::ZipArchive;

async fn get_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("GET")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice::<Value>(&body)?))
}

async fn get_response(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
) -> Result<axum::response::Response> {
	let req = Request::builder()
		.method("GET")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	Ok(app.clone().oneshot(req).await?)
}

async fn post_json_response(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<axum::response::Response> {
	let req = Request::builder()
		.method("POST")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	Ok(app.clone().oneshot(req).await?)
}

async fn insert_validated_raw_case(
	mm: &lib_core::model::ModelManager,
	org_id: Uuid,
	user_id: Uuid,
	safety_report_id: &str,
	validation_profile: &str,
	appendices_json: &str,
) -> Result<Uuid> {
	let case_id = Uuid::new_v4();
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, user_id).await?;
	set_org_context(&mut tx, org_id, ROLE_SYSTEM_ADMIN).await?;
	sqlx::query(
		"INSERT INTO cases (
			id,
			organization_id,
			safety_report_id,
			status,
			validation_profile,
			appendices_json,
			raw_xml,
			created_by,
			updated_by
		) VALUES ($1, $2, $3, 'validated', $4, $5, $6, $7, $7)",
	)
	.bind(case_id)
	.bind(org_id)
	.bind(safety_report_id)
	.bind(validation_profile)
	.bind(appendices_json)
	.bind(br#"<?xml version="1.0" encoding="UTF-8"?><test/>"#.as_slice())
	.bind(user_id)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;
	Ok(case_id)
}

#[serial]
#[tokio::test]
async fn test_single_export_rejects_unselected_appendix_profile() -> Result<()> {
	std::env::set_var("E2BR3_EXPORT_VALIDATE_FDA", "0");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let safety_report_id = format!("SR-APPENDIX-FDA-{}", Uuid::new_v4());
	let case_id = insert_validated_raw_case(
		&mm,
		seed.org_id,
		seed.admin.id,
		&safety_report_id,
		"fda",
		r#"["fda"]"#,
	)
	.await?;

	let response = get_response(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/export/xml?profile=mfds"),
	)
	.await?;
	assert_eq!(response.status(), StatusCode::BAD_REQUEST);
	let body = to_bytes(response.into_body(), usize::MAX).await?;
	let body = std::str::from_utf8(&body)?;
	assert!(
		body.contains("profile 'mfds' is not selected on this case"),
		"{body}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_bulk_export_writes_one_xml_per_selected_appendix() -> Result<()> {
	std::env::set_var("E2BR3_EXPORT_VALIDATE_FDA", "0");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let safety_report_id = format!("SR-APPENDIX-MULTI-{}", Uuid::new_v4());
	let case_id = insert_validated_raw_case(
		&mm,
		seed.org_id,
		seed.admin.id,
		&safety_report_id,
		"fda",
		r#"["fda","mfds"]"#,
	)
	.await?;

	let response = post_json_response(
		&app,
		&cookie,
		"/api/cases/export/xml",
		serde_json::json!({ "case_ids": [case_id] }),
	)
	.await?;
	assert_eq!(response.status(), StatusCode::OK);
	let bytes = to_bytes(response.into_body(), usize::MAX).await?;
	let mut zip = ZipArchive::new(Cursor::new(bytes.to_vec()))?;
	let mut names = Vec::new();
	for index in 0..zip.len() {
		names.push(zip.by_index(index)?.name().to_string());
	}
	names.sort();

	assert_eq!(
		names,
		vec![
			format!("{safety_report_id}-{case_id}-fda.xml"),
			format!("{safety_report_id}-{case_id}-mfds.xml"),
		]
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_export_history_error_details_download_as_text() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = Uuid::new_v4();
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, seed.admin.id).await?;
	set_org_context(&mut tx, seed.org_id, ROLE_SYSTEM_ADMIN).await?;
	sqlx::query(
		"INSERT INTO cases (id, organization_id, safety_report_id, created_by, updated_by)
		 VALUES ($1, $2, $3, $4, $4)",
	)
	.bind(case_id)
	.bind(seed.org_id)
	.bind(format!("SR-EXPORT-{case_id}"))
	.bind(seed.admin.id)
	.execute(&mut *tx)
	.await?;
	let (history_id,): (Uuid,) = sqlx::query_as(
		"INSERT INTO xml_export_history (
					case_id,
					case_number,
					file_name,
					status,
					error_message,
					validation_profile,
					exported_by
				) VALUES ($1, $2, $3, $4, $5, $6, $7)
			RETURNING id",
	)
	.bind(case_id)
	.bind("SR-EXPORT-1")
	.bind("exported-case.xml")
	.bind("error")
	.bind("gateway rejected payload")
	.bind("fda")
	.bind(seed.admin.id)
	.fetch_one(&mut *tx)
	.await?;
	tx.commit().await?;

	let (status, body) = get_json(&app, &cookie, "/api/exports/history").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert!(
		body["data"]["items"]
			.as_array()
			.is_some_and(|items| !items.is_empty()),
		"{body:?}"
	);

	let response = get_response(
		&app,
		&cookie,
		&format!("/api/exports/history/{history_id}/error.txt"),
	)
	.await?;
	assert_eq!(response.status(), StatusCode::OK);
	assert_eq!(
		response
			.headers()
			.get("content-type")
			.and_then(|v| v.to_str().ok()),
		Some("text/plain; charset=utf-8")
	);
	let disposition = response
		.headers()
		.get("content-disposition")
		.and_then(|v| v.to_str().ok())
		.ok_or("missing content-disposition header")?;
	assert!(
		disposition.contains("attachment; filename="),
		"{disposition}"
	);
	assert!(
		disposition.contains("exported-case.xml.txt"),
		"{disposition}"
	);

	let body = to_bytes(response.into_body(), usize::MAX).await?;
	assert_eq!(std::str::from_utf8(&body)?, "gateway rejected payload");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_scoped_export_history_only_returns_case_rows() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = Uuid::new_v4();
	let other_case_id = Uuid::new_v4();
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, seed.admin.id).await?;
	set_org_context(&mut tx, seed.org_id, ROLE_SYSTEM_ADMIN).await?;
	for id in [case_id, other_case_id] {
		sqlx::query(
			"INSERT INTO cases (id, organization_id, safety_report_id, created_by, updated_by)
			 VALUES ($1, $2, $3, $4, $4)",
		)
		.bind(id)
		.bind(seed.org_id)
		.bind(format!("SR-EXPORT-{id}"))
		.bind(seed.admin.id)
		.execute(&mut *tx)
		.await?;
	}
	sqlx::query(
		"INSERT INTO xml_export_history (
			case_id,
			case_number,
			file_name,
			status,
			validation_profile,
			exported_by
		) VALUES ($1, $2, $3, $4, $5, $6)",
	)
	.bind(case_id)
	.bind("SR-EXPORT-ONE")
	.bind("one.xml")
	.bind("success")
	.bind("fda")
	.bind(seed.admin.id)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
		"INSERT INTO xml_export_history (
			case_id,
			case_number,
			file_name,
			status,
			validation_profile,
			exported_by
		) VALUES ($1, $2, $3, $4, $5, $6)",
	)
	.bind(other_case_id)
	.bind("SR-EXPORT-TWO")
	.bind("two.xml")
	.bind("success")
	.bind("fda")
	.bind(seed.admin.id)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/exports/history"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let items = body["data"]["items"]
		.as_array()
		.ok_or("missing case export history items")?;
	assert_eq!(items.len(), 1, "{body:?}");
	assert_eq!(
		items[0]["caseId"].as_str(),
		Some(case_id.to_string().as_str())
	);
	assert_eq!(items[0]["fileName"].as_str(), Some("one.xml"));

	Ok(())
}
