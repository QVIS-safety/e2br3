use super::support::field_contract_test;
const PAGE_ID: &str = "LB";
include!("generated/lb_fields.rs");

#[serial_test::serial]
#[tokio::test]
async fn comments_can_be_cleared_and_reloaded() -> crate::common::Result<()> {
	use super::support::{create_case_for_editor, get_json, patch_json, post_json};
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
		create_case_for_editor(&app, &cookie, "EDITOR-LB-CLEAR", &["ich"]).await?;
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/test-results"),
		json!({"data": {"case_id": case_id, "sequence_number": 1, "test_name": "Fixture"}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let test_id = body["data"]["id"]
		.as_str()
		.ok_or("missing test result id")?;
	let uri = format!("/api/cases/{case_id}/editor/pages/LB/rows/{test_id}");

	for comments in [json!("Lab comment"), Value::Null] {
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities": ["ich"], "rows": {"testResult": {"comments": comments}}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
	}

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["testResult"]["comments"], Value::Null);
	Ok(())
}

#[serial_test::serial]
#[tokio::test]
async fn list_projection_includes_displayed_result_columns() -> crate::common::Result<()> {
	use super::support::{create_case_for_editor, get_json, post_json};
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
