use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, patch_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::json;

pub(crate) async fn source_document_delete_hides_the_target(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let id =
		create_case_for_editor(&app, &cookie, "EDITOR-CONTRACT", &["ich"]).await?;
	let base = format!("/api/cases/{id}/editor/pages");

	let uri = format!("{base}/CI");
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{
			"otherCaseIdentifiers":[{"source":null,"caseIdentifier":null}],
			"sourceDocuments":[{"sourceDocumentName":"DOC","deleted":false}]
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	let doc = &saved["rows"]["sourceDocuments"][0];
	let (status, body) = patch_json(&app, &cookie, &uri, json!({"authorities":["ich"],"rows":{"sourceDocuments":[{"id":doc["id"],"deleted":true}]}})).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(
		saved["rows"]["sourceDocuments"].as_array().map(Vec::len),
		Some(0),
		"{saved}"
	);

	Ok(())
}
