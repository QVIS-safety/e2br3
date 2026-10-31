use super::support::{post_json, post_raw};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_case_query_returns_lightweight_list_view_items() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let suffix = Uuid::new_v4().simple().to_string();
	let case_no = format!("CASE-QUERY-LIST-{suffix}");
	let product_id = format!("QUERY-PROD-{suffix}");

	let (status, raw_body) = post_raw(
		&app,
		&cookie,
		"/api/cases",
		json!({
		"data": {
				"safetyReportIdentification": {"safetyReportId": case_no},
				"dgPrdKey": product_id,
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&raw_body)
	);
	let created: Value = serde_json::from_slice(&raw_body)?;
	let case_id = created["data"]["id"]
		.as_str()
		.ok_or("missing created case id")?;

	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/cases/query",
		json!({
				"conditions": [{
					"page": "CASE",
					"item": "dg_prd_key",
					"operator": "equal",
					"values": [product_id]
				}],
				"resultPages": ["CASE"],
				"reportTypeLast": false,
			"noAckAcceptHistory": false
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(
		body["data"]["caseIds"].as_array().map(Vec::len),
		Some(1),
		"{body:?}"
	);
	assert_eq!(
		body["data"]["caseIds"][0].as_str(),
		Some(case_id),
		"{body:?}"
	);
	let items = body["data"]["items"]
		.as_array()
		.ok_or("missing query list-view items")?;
	assert_eq!(items.len(), 1, "{body:?}");
	assert_eq!(items[0]["caseId"].as_str(), Some(case_id), "{body:?}");
	assert_eq!(
		items[0]["caseNo"].as_str(),
		Some(case_no.as_str()),
		"{body:?}"
	);
	assert_eq!(
		items[0]["dgPrdKey"].as_str(),
		Some(product_id.as_str()),
		"{body:?}"
	);
	assert_eq!(
		body["data"]["elements"][0]["page"].as_str(),
		Some("CASE"),
		"{body:?}"
	);
	assert_eq!(
		body["data"]["elements"][0]["item"].as_str(),
		Some("dg_prd_key"),
		"{body:?}"
	);
	assert_eq!(
		body["data"]["elements"][0]["label"].as_str(),
		Some("Product ID"),
		"{body:?}"
	);
	assert_eq!(
		body["data"]["elementValues"][0]["caseId"].as_str(),
		Some(case_id),
		"{body:?}"
	);
	assert_eq!(
		body["data"]["elementValues"][0]["values"]["CASE.dg_prd_key"][0].as_str(),
		Some(product_id.as_str()),
		"{body:?}"
	);
	Ok(())
}
