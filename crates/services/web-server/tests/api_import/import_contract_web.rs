use super::support::{
	create_product_presave, import_xml_fixture_with_product, post_json, put_json,
	scenario6,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::store::{set_org_context, set_user_context};
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;

async fn import_xml_without_filename_with_product(
	app: &axum::Router,
	cookie: &str,
	xml: &[u8],
	product_presave_id: &str,
) -> Result<(StatusCode, Value)> {
	let boundary = "X-BOUNDARY-IMPORT-MISSING-FILENAME";
	let mut multipart = format!(
		"--{boundary}\r\nContent-Disposition: form-data; name=\"productPresaveId\"\r\n\r\n{product_presave_id}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"\r\nContent-Type: application/xml\r\n\r\n"
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
async fn test_import_rejects_missing_upload_filename() -> Result<()> {
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
			"data": { "rows": {
				"sender": {
					"senderType": "2",
					"organizationName": "Missing Filename Sender",
					"email": "missing-filename@example.test"
				},
				"gateways": [], "responsiblePersons": []
			} }
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let sender_presave_id = body["data"]["rows"]["sender"]["id"]
		.as_str()
		.ok_or_else(|| format!("missing sender id in body {body:?}"))?;
	let product_presave_id = create_product_presave(
		&app,
		&cookie,
		sender_presave_id,
		"MISSING-FILENAME-PRODUCT",
	)
	.await?;

	let (status, body) = import_xml_without_filename_with_product(
		&app,
		&cookie,
		b"<xml />",
		&product_presave_id,
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string()
			.contains("uploaded file name is required for XML import"),
		"{body:?}"
	);

	Ok(())
}
#[serial]
#[tokio::test]
async fn test_import_selected_product_links_sender_without_overwriting_xml_sender(
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
			"data": { "rows": {
				"sender": {
					"senderType": "2",
					"organizationName": "Selected Product Sender",
					"email": "selected-product-sender@example.test"
				},
				"gateways": [], "responsiblePersons": []
			} }
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let sender_presave_id = body["data"]["rows"]["sender"]["id"]
		.as_str()
		.ok_or_else(|| format!("missing sender id in body {body:?}"))?;
	let (status, body) = put_json(
		&app,
		&cookie,
		"/api/admin/settings",
		json!({
			"data": {
				"apply_sender_info_to_imported_cases": false
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
			"data": { "rows": {
				"product": {
					"senderPresaveId": sender_presave_id,
					"productId": business_product_id,
					"medicinalProduct": "Selected Import Product"
				},
				"activeSubstances": []
			} }
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let product_presave_id = body["data"]["rows"]["product"]["id"]
		.as_str()
		.ok_or_else(|| format!("missing product id in body {body:?}"))?;

	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = scenario6(&root)?.replace(
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
		(
			Option<uuid::Uuid>,
			Option<String>,
			Option<uuid::Uuid>,
			Option<String>,
		),
	>(
		"SELECT d.source_product_presave_id, c.dg_prd_key,
		        sender.source_sender_presave_id, sender.organization_name
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
	assert_ne!(row.3.as_deref(), Some("Selected Product Sender"));

	Ok(())
}
