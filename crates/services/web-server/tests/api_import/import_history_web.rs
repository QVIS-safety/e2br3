use super::support::{fixture_xml, get_json, get_response, import_xml_string};
use crate::common::{
	cookie_header, init_test_env, init_test_mm, seed_org_with_users, Result,
};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::store::{set_org_context, set_user_context};
use serde_json::{json, Value};
use serial_test::serial;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tower::ServiceExt;

#[serial]
#[tokio::test]
async fn test_import_history_uploaded_at_is_rfc3339() -> Result<()> {
	init_test_env().await?;

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let req = Request::builder()
		.method("POST")
		.uri("/api/presaves/senders")
		.header("cookie", &cookie)
		.header("content-type", "application/json")
		.body(Body::from(
			json!({"data": {"rows": {"sender": {
				"senderType": "2",
				"organizationName": "Import History Sender",
				"email": "import-history@example.test"
			}, "gateways": [], "responsiblePersons": []}}})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let sender: Value = serde_json::from_slice(&body)?;
	let sender_id = sender["data"]["rows"]["sender"]["id"]
		.as_str()
		.ok_or_else(|| format!("missing sender id: {sender}"))?;
	let req = Request::builder()
		.method("POST")
		.uri("/api/presaves/products")
		.header("cookie", &cookie)
		.header("content-type", "application/json")
		.body(Body::from(
			json!({"data": {"rows": {"product": {
				"senderPresaveId": sender_id,
				"productId": "IMPORT-HISTORY-PRODUCT",
				"medicinalProduct": "Import History Product"
			}, "activeSubstances": []}}})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let product: Value = serde_json::from_slice(&body)?;
	let product_id = product["data"]["rows"]["product"]["id"]
		.as_str()
		.ok_or_else(|| format!("missing product id: {product}"))?;

	let xml = fixture_xml("FAERS2022Scenario1.xml")?;
	import_xml_string(&app, &cookie, "FAERS2022Scenario1.xml", &xml, product_id)
		.await?;

	let (status, body) = get_json(&app, &cookie, "/api/import/xml/history").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let item = body["data"]["items"]
		.as_array()
		.and_then(|items| items.first())
		.cloned()
		.ok_or_else(|| format!("missing history items in response: {body}"))?;
	let uploaded_at = item["uploadedAt"]
		.as_str()
		.ok_or_else(|| format!("uploadedAt must be a string: {item}"))?;

	let parsed = OffsetDateTime::parse(uploaded_at, &Rfc3339).map_err(|err| {
		format!("uploadedAt was not RFC3339: {uploaded_at} ({err})")
	})?;
	assert!(parsed.unix_timestamp() > 0, "{item:?}");

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
			uploaded_by
		) VALUES ($1, $2, $3, $4, $5, $6)
		RETURNING id",
	)
	.bind("batch.zip")
	.bind("broken-case.xml")
	.bind("SR-IMPORT-ERR-1")
	.bind("error")
	.bind("schema validation failed on line 14")
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
	assert_eq!(
		disposition,
		format!(
			"attachment; filename=\"import-error-{history_id}-broken-case.xml.txt\""
		)
	);

	let body = to_bytes(response.into_body(), usize::MAX).await?;
	assert_eq!(
		std::str::from_utf8(&body)?,
		"schema validation failed on line 14"
	);

	Ok(())
}
