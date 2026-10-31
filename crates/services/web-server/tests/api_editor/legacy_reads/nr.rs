use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, patch_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;
#[serial]
#[tokio::test]
async fn test_nr_legacy_read_projects_persisted_rows() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "LEGACY-NR-READ", &["ich"]).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/NR");
	let (status,saved)=patch_json(&app,&cookie,&uri,json!({"authorities":["ich"],"rows":{"narrative":{"caseNarrative":"Legacy narrative"},"caseSummaryInformation":[{"sequenceNumber":1,"languageCode":"en","summaryText":"Legacy summary"}]}})).await?;
	assert_eq!(status, StatusCode::OK, "{saved}");
	let row_id = saved["rows"]["caseSummaryInformation"][0]["id"].clone();
	let (status, body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/editor/NR")).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	assert_eq!(
		body["data"]["narrative"]["case_narrative"],
		"Legacy narrative"
	);
	assert_eq!(body["data"]["caseSummaryInformation"][0]["id"], row_id);
	assert_eq!(
		body["data"]["caseSummaryInformation"][0]["summary_text"],
		"Legacy summary"
	);
	assert_eq!(body["data"]["caseSummaryInformation"][0]["deleted"], false);
	Ok(())
}
