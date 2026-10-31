use crate::case_editor_contract_web::support::{
	create_case_for_editor, create_reaction_contract_fixture, get_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serial_test::serial;
async fn fixture() -> Result<(axum::Router, String, String, String)> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "LEGACY-AE-READ", &["ich"]).await?;
	let row_id = create_reaction_contract_fixture(&app, &cookie, &case_id).await?;
	Ok((app, cookie, case_id, row_id))
}
#[serial]
#[tokio::test]
async fn test_ae_legacy_list_projects_rows() -> Result<()> {
	let (app, cookie, case_id, row_id) = fixture().await?;
	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/AE/list"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	let row = &body["rows"][0];
	assert_eq!(row["id"], row_id);
	assert_eq!(row["reactionPrimarySourceNative"], "Fixture reaction");
	assert_eq!(row["deleted"], false);
	Ok(())
}
#[serial]
#[tokio::test]
async fn test_ae_legacy_detail_projects_row() -> Result<()> {
	let (app, cookie, case_id, row_id) = fixture().await?;
	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/AE/{row_id}"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	assert_eq!(body["rowId"], row_id);
	let row = &body["data"]["reactions"][0];
	assert_eq!(row["id"], row_id);
	assert_eq!(row["primary_source_reaction"], "Fixture reaction");
	assert_eq!(row["deleted"], false);
	Ok(())
}
