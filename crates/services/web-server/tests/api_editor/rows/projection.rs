use crate::case_editor_contract_web::support::{create_case_for_editor, get_json};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;

pub(crate) async fn page_projection_orders_past_drugs_by_sequence_then_id(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	use crate::case_editor_contract_web::support::{
		create_case_for_editor, get_json, post_json,
	};
	use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
	use axum::http::StatusCode;
	use lib_auth::token::generate_web_token;
	use serde_json::json;

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "EDITOR-DH-ORDER", &["ich"]).await?;
	let rows_uri = format!("/api/cases/{case_id}/editor/pages/DH/rows");
	for (sequence, drug_name) in [(2, "Second prior drug"), (1, "First prior drug")]
	{
		let (status, body) = post_json(
			&app,
			&cookie,
			&rows_uri,
			json!({"authorities": ["ich"], "rows": {"pastDrugHistory": {
				"sequenceNumber": sequence, "drugName": drug_name
			}}}),
		)
		.await?;
		assert_eq!(status, StatusCode::CREATED, "{body}");
	}

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/DH"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let rows = body["rows"]["rows"].as_array().ok_or("missing DH rows")?;
	let drug_names = rows
		.iter()
		.map(|row| row["drugName"].as_str())
		.collect::<Vec<_>>();
	assert_eq!(
		drug_names,
		vec![Some("First prior drug"), Some("Second prior drug")]
	);

	Ok(())
}

pub(crate) async fn list_projection_includes_displayed_result_columns(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	use crate::case_editor_contract_web::support::{
		create_case_for_editor, get_json, post_json,
	};
	use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
	use axum::http::StatusCode;
	use lib_auth::token::generate_web_token;
	use serde_json::json;

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "EDITOR-LB-LIST", &["ich"]).await?;
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/test-results"),
		json!({"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"test_name": "Fixture",
			"test_result_code": "1",
			"test_result_value": "12.3",
			"result_unstructured": "Free result",
			"comments": "Lab comment"
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/LB"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let row = &body["rows"]["rows"][0];
	assert_eq!(row["testResultCode"], "1");
	assert_eq!(row["testResult"], "12.3");
	assert_eq!(row["testResultUnstructured"], "Free result");
	assert_eq!(row["comments"], "Lab comment");
	Ok(())
}

pub(crate) async fn page_projection_orders_references_by_sequence_then_id(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	use crate::case_editor_contract_web::support::{
		create_case_for_editor, get_json, post_json,
	};
	use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
	use axum::http::StatusCode;
	use lib_auth::token::generate_web_token;
	use serde_json::json;

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "EDITOR-LR-ORDER", &["ich"]).await?;
	let rows_uri = format!("/api/cases/{case_id}/editor/pages/LR/rows");
	for (sequence, reference) in [(2, "Second citation"), (1, "First citation")] {
		let (status, body) = post_json(
			&app,
			&cookie,
			&rows_uri,
			json!({"authorities": ["ich"], "rows": {"literatureReference": {
				"sequenceNumber": sequence, "referenceText": reference
			}}}),
		)
		.await?;
		assert_eq!(status, StatusCode::CREATED, "{body}");
	}

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/LR"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let rows = body["rows"]["rows"].as_array().ok_or("missing LR rows")?;
	let references = rows
		.iter()
		.map(|row| row["referenceText"].as_str())
		.collect::<Vec<_>>();
	assert_eq!(
		references,
		vec![Some("First citation"), Some("Second citation")]
	);

	Ok(())
}

pub(crate) async fn reporters_remain_ordered_by_sequence_and_id_after_update(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	use crate::case_editor_contract_web::support::{
		create_case_for_editor, get_json, patch_json,
	};
	use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
	use axum::http::StatusCode;
	use lib_auth::token::generate_web_token;
	use serde_json::{json, Value};

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "EDITOR-RP-ORDER", &["ich"]).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/RP");

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"primarySources": [
			{"sequenceNumber": 3, "reporterGivenName": "Sequence three"},
			{"sequenceNumber": 1, "reporterGivenName": "Sequence one"},
			{"sequenceNumber": 2, "reporterGivenName": "Sequence two"}
		]}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let rows = body["rows"]["primarySources"]
		.as_array()
		.ok_or("missing primarySources")?;
	assert_eq!(rows.len(), 3, "{body}");
	let sequence_one_id = rows
		.iter()
		.find(|row| row["reporterGivenName"] == "Sequence one")
		.and_then(|row| row["id"].as_str())
		.ok_or("missing sequence-one reporter id")?
		.to_string();
	let sibling_rows: Vec<Value> = rows
		.iter()
		.filter(|row| row["id"] != sequence_one_id)
		.cloned()
		.collect();

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"primarySources": [{
			"id": sequence_one_id,
			"sequenceNumber": 1,
			"reporterPostcode": "04524"
		}]}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let rows = body["rows"]["primarySources"]
		.as_array()
		.ok_or("missing primarySources")?;
	assert_eq!(rows.len(), 3, "{body}");
	let order: Vec<_> = rows
		.iter()
		.map(|row| {
			(
				row["sequenceNumber"].as_i64(),
				row["id"].as_str(),
				row["reporterGivenName"].as_str(),
			)
		})
		.collect();
	assert_eq!(order[0].0, Some(1), "{body}");
	assert_eq!(order[0].1, Some(sequence_one_id.as_str()), "{body}");
	assert_eq!(order[0].2, Some("Sequence one"), "{body}");
	assert_eq!(rows[0]["reporterPostcode"], "04524", "{body}");
	assert_eq!(order[1].0, Some(2), "{body}");
	assert_eq!(order[1].2, Some("Sequence two"), "{body}");
	assert_eq!(order[2].0, Some(3), "{body}");
	assert_eq!(order[2].2, Some("Sequence three"), "{body}");
	for sibling in sibling_rows {
		let sibling_id = sibling["id"].as_str().ok_or("missing sibling id")?;
		let current = rows
			.iter()
			.find(|row| row["id"] == sibling_id)
			.ok_or("missing sibling after update")?;
		assert_eq!(current, &sibling, "sibling changed: {body}");
	}
	Ok(())
}

pub(crate) async fn batched_rows_match_detail_and_require_auth(
	page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "BATCHED-ROWS", &["ich"]).await?;
	crate::case_editor_contract_web::support::assert_batched_rows(
		&app, &cookie, &case_id, page_id,
	)
	.await?;
	let (status, _) = get_json(
		&app,
		"",
		&format!("/api/cases/{case_id}/editor/pages/{page_id}?include_rows=true"),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN);
	let row_id = crate::case_editor_contract_web::support::repeatable_row_id(
		page_id, &app, &cookie, &case_id,
	)
	.await?;
	assert!(row_id.is_some(), "batched projection requires a row page");
	crate::case_editor_contract_web::support::assert_batched_rows(
		&app, &cookie, &case_id, page_id,
	)
	.await?;
	Ok(())
}
