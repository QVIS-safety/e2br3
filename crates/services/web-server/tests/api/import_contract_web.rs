#![allow(dead_code, unused_imports)]

use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::store::{set_org_context, set_user_context};
use serde_json::{json, Value};
use serial_test::serial;
use sqlx::types::time::Date;
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

async fn put_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("PUT")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice::<Value>(&body)?))
}

async fn post_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("POST")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice::<Value>(&body)?))
}

async fn import_xml_fixture(
	app: &axum::Router,
	cookie: &str,
	filename: &str,
	xml: &[u8],
) -> Result<(StatusCode, Value)> {
	let boundary = "X-BOUNDARY-IMPORT-SETTINGS";
	let mut multipart = format!(
		"--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: application/xml\r\n\r\n"
	)
	.into_bytes();
	multipart.extend_from_slice(xml);
	multipart.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
	let req = Request::builder()
		.method("POST")
		.uri("/api/import/xml")
		.header("cookie", cookie)
		.header(
			"content-type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.body(Body::from(multipart))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice::<Value>(&body)?))
}

async fn import_xml_fixture_with_product(
	app: &axum::Router,
	cookie: &str,
	filename: &str,
	xml: &[u8],
	product_presave_id: &str,
) -> Result<(StatusCode, Value)> {
	let boundary = "X-BOUNDARY-IMPORT-PRODUCT";
	let mut multipart = format!(
		"--{boundary}\r\nContent-Disposition: form-data; name=\"productPresaveId\"\r\n\r\n{product_presave_id}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: application/xml\r\n\r\n"
	)
	.into_bytes();
	multipart.extend_from_slice(xml);
	multipart.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
	let req = Request::builder()
		.method("POST")
		.uri("/api/import/xml")
		.header("cookie", cookie)
		.header(
			"content-type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.body(Body::from(multipart))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice::<Value>(&body)?))
}

#[serial]
#[tokio::test]
async fn test_import_selected_product_links_first_drug_and_case_product_id(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/presaves/senders",
		json!({
			"data": {
				"authority": "fda",
				"name": "Selected product sender",
				"sender_type": "2",
				"organization_name": "Selected Product Sender",
				"email": "selected-product-sender@example.test"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let sender_presave_id = body["data"]["id"]
		.as_str()
		.ok_or_else(|| format!("missing sender id in body {body:?}"))?;
	let (status, body) = put_json(
		&app,
		&cookie,
		"/api/admin/settings",
		json!({
			"data": {
				"apply_sender_info_to_imported_cases": true
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let business_product_id = format!("IMPORT-PRODUCT-{}", uuid::Uuid::new_v4());
	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/presaves/products",
		json!({
			"data": {
				"name": "Selected import product",
				"sender_presave_id": sender_presave_id,
				"product_id": business_product_id,
				"medicinal_product": "Selected Import Product"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let product_presave_id = body["data"]["id"]
		.as_str()
		.ok_or_else(|| format!("missing product id in body {body:?}"))?;

	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = std::fs::read(
		root.join("crates/libs/lib-core/src/xml/fixtures/base_export_skeleton.xml"),
	)?;
	let xml = String::from_utf8(xml)?.replace(
		"US-APHARMA-8744554B",
		&format!("US-PRODUCT-LINK-{}", uuid::Uuid::new_v4()),
	);
	let (status, body) = import_xml_fixture_with_product(
		&app,
		&cookie,
		"FAERS2022Scenario6.xml",
		xml.as_bytes(),
		product_presave_id,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let case_id = uuid::Uuid::parse_str(
		body["data"]["importedCases"][0]["caseId"]
			.as_str()
			.ok_or_else(|| format!("missing imported case id in body {body:?}"))?,
	)?;

	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, seed.admin.id).await?;
	set_org_context(&mut tx, seed.org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	let row = sqlx::query_as::<
		_,
		(Option<uuid::Uuid>, Option<String>, Option<uuid::Uuid>),
	>(
		"SELECT d.source_product_presave_id, c.dg_prd_key,
		        sender.source_sender_presave_id
		 FROM cases c
		 JOIN drug_information d ON d.case_id = c.id
		 LEFT JOIN sender_information sender ON sender.case_id = c.id
		 WHERE c.id = $1
		 ORDER BY d.sequence_number
		 LIMIT 1",
	)
	.bind(case_id)
	.fetch_one(&mut *tx)
	.await?;
	let linked_drug_count = sqlx::query_scalar::<_, i64>(
		"SELECT COUNT(*) FROM drug_information
		 WHERE case_id = $1 AND source_product_presave_id = $2",
	)
	.bind(case_id)
	.bind(uuid::Uuid::parse_str(product_presave_id)?)
	.fetch_one(&mut *tx)
	.await?;
	tx.commit().await?;
	assert_eq!(
		row.0.map(|id| id.to_string()).as_deref(),
		Some(product_presave_id)
	);
	assert_eq!(row.1.as_deref(), Some(business_product_id.as_str()));
	assert_eq!(linked_drug_count, 1, "only the first G.k may be linked");
	assert_eq!(
		row.2.map(|id| id.to_string()).as_deref(),
		Some(sender_presave_id)
	);

	Ok(())
}

#[cfg(any())]
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
			uploaded_by
		) VALUES ($1, $2, $3, $4, $5)",
	)
	.bind("batch.zip")
	.bind("case.xml")
	.bind("SR-IMPORT-1")
	.bind("success")
	.bind(seed.admin.id)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;

	let (status, body) = get_json(&app, &cookie, "/api/import/xml/history").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let item = &body["data"]["items"][0];
	assert!(
		item.get("validationAuthority").is_none(),
		"import history must not expose legacy validationAuthority: {item:?}"
	);
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

#[cfg(any())]
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

#[cfg(any())]
#[serial]
#[tokio::test]
async fn test_import_settings_update_enabled_c1_dates() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let (status, body) = put_json(
		&app,
		&cookie,
		"/api/admin/settings",
		json!({
			"data": {
				"import_date_update": {
					"date_of_creation": true,
					"most_recent_info_date": false,
					"report_first_received_date": true
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = std::fs::read(root.join("docs/exporter/fda/FAERS2022Scenario6.xml"))?;
	let xml = String::from_utf8(xml)?.replace(
		"US-APHARMA-8744554B",
		&format!("US-TEST-{}", uuid::Uuid::new_v4()),
	);
	let (status, body) =
		import_xml_fixture(&app, &cookie, "FAERS2022Scenario6.xml", xml.as_bytes())
			.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let case_id = body["data"]["importedCases"][0]["caseId"]
		.as_str()
		.ok_or_else(|| format!("missing imported case id in body {body:?}"))?;
	let case_id = uuid::Uuid::parse_str(case_id)?;

	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, seed.admin.id).await?;
	set_org_context(&mut tx, seed.org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	let row = sqlx::query_as::<_, (Date, Date, Date)>(
		"SELECT transmission_date, date_first_received_from_source, date_of_most_recent_information
		 FROM safety_report_identification WHERE case_id = $1",
	)
	.bind(case_id)
	.fetch_one(&mut *tx)
	.await?;
	tx.commit().await?;

	let import_date = time::OffsetDateTime::now_utc().date();
	assert_eq!(row.0, import_date);
	assert_eq!(row.1, import_date);
	assert_ne!(row.2, import_date);

	Ok(())
}

#[cfg(any())]
#[serial]
#[tokio::test]
async fn test_import_settings_apply_default_sender_only_when_enabled() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/presaves/senders",
		json!({
			"data": {
				"authority": "fda",
				"name": "Default import sender",
				"comments": "sender used for import defaults",
				"sender_type": "2",
				"organization_name": "Admin Default Sender",
				"department": "Import Ops",
				"street_address": "10 Default Road",
				"city": "Seoul",
				"country_code": "KR",
				"email": "default-sender@example.test",
				"is_default": true
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");

	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let source_xml =
		std::fs::read(root.join("docs/exporter/fda/FAERS2022Scenario6.xml"))?;

	let disabled_xml = String::from_utf8(source_xml.clone())?.replace(
		"US-APHARMA-8744554B",
		&format!("US-SENDER-DISABLED-{}", uuid::Uuid::new_v4()),
	);
	let (status, body) = import_xml_fixture(
		&app,
		&cookie,
		"FAERS2022Scenario6.xml",
		disabled_xml.as_bytes(),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let disabled_case_id = body["data"]["importedCases"][0]["caseId"]
		.as_str()
		.ok_or_else(|| format!("missing imported case id in body {body:?}"))?;
	let disabled_case_id = uuid::Uuid::parse_str(disabled_case_id)?;

	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, seed.admin.id).await?;
	set_org_context(&mut tx, seed.org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	let disabled_sender =
		sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>)>(
			"SELECT sender_type, organization_name, email
		 FROM sender_information WHERE case_id = $1 LIMIT 1",
		)
		.bind(disabled_case_id)
		.fetch_one(&mut *tx)
		.await?;
	tx.commit().await?;
	assert_eq!(disabled_sender.0.as_deref(), Some("1"));
	assert_eq!(disabled_sender.1.as_deref(), Some("Big Pharma"));
	assert_eq!(
		disabled_sender.2.as_deref(),
		Some("emailAddress@company.com")
	);

	let (status, body) = put_json(
		&app,
		&cookie,
		"/api/admin/settings",
		json!({
			"data": {
				"apply_sender_info_to_imported_cases": true
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let xml = String::from_utf8(source_xml)?.replace(
		"US-APHARMA-8744554B",
		&format!("US-SENDER-{}", uuid::Uuid::new_v4()),
	);
	let (status, body) =
		import_xml_fixture(&app, &cookie, "FAERS2022Scenario6.xml", xml.as_bytes())
			.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let case_id = body["data"]["importedCases"][0]["caseId"]
		.as_str()
		.ok_or_else(|| format!("missing imported case id in body {body:?}"))?;
	let case_id = uuid::Uuid::parse_str(case_id)?;

	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, seed.admin.id).await?;
	set_org_context(&mut tx, seed.org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	let sender = sqlx::query_as::<
		_,
		(
			Option<String>,
			Option<String>,
			Option<String>,
			Option<String>,
			Option<String>,
			Option<String>,
			Option<String>,
		),
	>(
		"SELECT sender_type, organization_name, email, person_title, person_given_name, person_family_name, fax
		 FROM sender_information WHERE case_id = $1 LIMIT 1",
	)
	.bind(case_id)
	.fetch_one(&mut *tx)
	.await?;
	tx.commit().await?;

	assert_eq!(sender.0.as_deref(), Some("2"));
	assert_eq!(sender.1.as_deref(), Some("Admin Default Sender"));
	assert_eq!(sender.2.as_deref(), Some("default-sender@example.test"));
	assert_eq!(sender.3.as_deref(), Some("Mr"));
	assert_eq!(sender.4.as_deref(), Some("Charles"));
	assert_eq!(sender.5.as_deref(), Some("Conner"));
	assert_eq!(sender.6.as_deref(), Some("6109991122"));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_import_settings_apply_product_linked_sender_by_imported_product_id(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/presaves/senders",
		json!({
			"data": {
				"authority": "fda",
				"name": "Fallback default sender",
				"sender_type": "2",
				"organization_name": "Fallback Default Sender",
				"email": "fallback-default@example.test",
				"is_default": true
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");

	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/presaves/senders",
		json!({
			"data": {
				"authority": "fda",
				"name": "Product-linked sender",
				"sender_type": "3",
				"organization_name": "Product Linked Sender",
				"email": "product-linked@example.test"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let linked_sender_id = body["data"]["id"]
		.as_str()
		.ok_or_else(|| format!("missing linked sender id in body {body:?}"))?;

	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/presaves/products",
		json!({
			"data": {
				"name": "Drug A product",
				"sender_presave_id": linked_sender_id,
				"product_id": "Drug A",
				"medicinal_product": "Drug A"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");

	let (status, body) = put_json(
		&app,
		&cookie,
		"/api/admin/settings",
		json!({
			"data": {
				"apply_sender_info_to_imported_cases": true
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let source_xml =
		std::fs::read(root.join("docs/exporter/fda/FAERS2022Scenario6.xml"))?;
	let xml = String::from_utf8(source_xml)?.replace(
		"US-APHARMA-8744554B",
		&format!("US-PRODUCT-SENDER-{}", uuid::Uuid::new_v4()),
	);
	let (status, body) =
		import_xml_fixture(&app, &cookie, "FAERS2022Scenario6.xml", xml.as_bytes())
			.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let case_id = body["data"]["importedCases"][0]["caseId"]
		.as_str()
		.ok_or_else(|| format!("missing imported case id in body {body:?}"))?;
	let case_id = uuid::Uuid::parse_str(case_id)?;

	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, seed.admin.id).await?;
	set_org_context(&mut tx, seed.org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	let sender =
		sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>)>(
			"SELECT sender_type, organization_name, email
		 FROM sender_information WHERE case_id = $1 LIMIT 1",
		)
		.bind(case_id)
		.fetch_one(&mut *tx)
		.await?;
	tx.commit().await?;

	assert_eq!(sender.0.as_deref(), Some("3"));
	assert_eq!(sender.1.as_deref(), Some("Product Linked Sender"));
	assert_eq!(sender.2.as_deref(), Some("product-linked@example.test"));

	Ok(())
}
