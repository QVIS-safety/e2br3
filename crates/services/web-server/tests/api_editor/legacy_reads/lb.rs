use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, post_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;
async fn fixture() -> Result<(axum::Router, String, String, String)> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "LEGACY-LB-READ", &["ich"]).await?;
	let(status,body)=post_json(&app,&cookie,&format!("/api/cases/{case_id}/test-results"),json!({"data":{"case_id":case_id,"sequence_number":1,"test_name":"Legacy test","test_result_value":"42","test_result_unit":"mg"}})).await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let row_id = body["data"]["id"]
		.as_str()
		.ok_or("missing test id")?
		.to_string();
	Ok((app, cookie, case_id, row_id))
}
#[serial]
#[tokio::test]
async fn test_lb_legacy_list_projects_rows() -> Result<()> {
	let (app, cookie, case_id, row_id) = fixture().await?;
	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/LB/list"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	let row = &body["rows"][0];
	assert_eq!(row["id"], row_id);
	assert_eq!(row["testName"], "Legacy test");
	assert_eq!(row["testResult"], "42");
	assert_eq!(row["deleted"], false);
	Ok(())
}
#[serial]
#[tokio::test]
async fn test_lb_legacy_detail_projects_row() -> Result<()> {
	let (app, cookie, case_id, row_id) = fixture().await?;
	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/LB/{row_id}"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	assert_eq!(body["rowId"], row_id);
	let row = &body["data"]["testResults"][0];
	assert_eq!(row["id"], row_id);
	assert_eq!(row["test_name"], "Legacy test");
	assert_eq!(row["test_result_value"], "42");
	assert_eq!(row["deleted"], false);
	Ok(())
}
