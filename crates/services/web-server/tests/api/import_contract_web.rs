use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::store::{set_org_context, set_user_context};
use serde_json::Value;
use serial_test::serial;
use tower::ServiceExt;

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

#[serial]
#[tokio::test]
async fn test_import_history_uploaded_at_is_string_timestamp() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, seed.admin.id).await?;
	set_org_context(&mut tx, seed.org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	sqlx::query(
		"INSERT INTO xml_import_history (
			uploaded_file_name,
			source_file_name,
			case_number,
			status,
			validation_profile,
			uploaded_by
		) VALUES ($1, $2, $3, $4, $5, $6)",
	)
	.bind("batch.zip")
	.bind("case.xml")
	.bind("SR-IMPORT-1")
	.bind("success")
	.bind("fda")
	.bind(seed.admin.id)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;

	let (status, body) = get_json(&app, &cookie, "/api/import/xml/history").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let item = &body["data"]["items"][0];
	let uploaded_at = item["uploadedAt"]
		.as_str()
		.ok_or("uploadedAt should be a string")?;
	assert!(
		uploaded_at.contains('T') || uploaded_at.contains(' '),
		"expected a readable timestamp string, got {uploaded_at:?}"
	);
	assert_ne!(uploaded_at, "Invalid date");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_import_history_error_details_download_as_text() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, seed.admin.id).await?;
	set_org_context(&mut tx, seed.org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	let history_id: uuid::Uuid = sqlx::query_scalar(
		"INSERT INTO xml_import_history (
			uploaded_file_name,
			source_file_name,
			case_number,
			status,
			error_message,
			validation_profile,
			uploaded_by
		) VALUES ($1, $2, $3, $4, $5, $6, $7)
		RETURNING id",
	)
	.bind("batch.zip")
	.bind("broken-case.xml")
	.bind("SR-IMPORT-ERR-1")
	.bind("error")
	.bind("schema validation failed on line 14")
	.bind("fda")
	.bind(seed.admin.id)
	.fetch_one(&mut *tx)
	.await?;
	tx.commit().await?;

	let response = get_response(
		&app,
		&cookie,
		&format!("/api/import/xml/history/{history_id}/error.txt"),
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
	assert!(disposition.contains("broken-case.xml.txt"), "{disposition}");

	let body = to_bytes(response.into_body(), usize::MAX).await?;
	assert_eq!(
		std::str::from_utf8(&body)?,
		"schema validation failed on line 14"
	);

	Ok(())
}
