use super::support::{get_json, post_raw};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_case_link_options_projects_created_case() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_no = format!("LINK-OPTION-{}", Uuid::new_v4().simple());
	let (status, raw) = post_raw(
		&app,
		&cookie,
		"/api/cases",
		json!({"data": {
			"safetyReportIdentification": {"safetyReportId": case_no}, "status": "draft"
		}}),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&raw)
	);
	let created: Value = serde_json::from_slice(&raw)?;
	let case_id = created["data"]["id"].as_str().ok_or("missing case id")?;

	let (status, body) = get_json(&app, &cookie, "/api/cases/link-options").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let item = body["data"]["items"]
		.as_array()
		.ok_or("missing link options")?
		.iter()
		.find(|item| item["caseId"] == case_id)
		.ok_or("created case absent from link options")?;
	assert_eq!(item["safetyReportId"], case_no);
	assert_eq!(item["version"], 1);
	assert!(item["createdAt"].as_str().is_some());
	Ok(())
}
