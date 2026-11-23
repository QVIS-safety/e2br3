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
	let app = web_server::app(mm).expect("web server configuration");
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

#[serial]
#[tokio::test]
async fn test_nr_editor_retains_deleted_children_for_restore_only() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm).expect("web server configuration");
	let case_id =
		create_case_for_editor(&app, &cookie, "NR-RESTORE", &["ich"]).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/NR");
	let (status, saved) = patch_json(&app, &cookie, &uri, json!({
		"authorities": ["ich"], "rows": {
			"narrative": {"caseNarrative": "Isolated contract fixture"},
			"senderDiagnoses": [{"sequenceNumber": 4}],
			"caseSummaryInformation": [{"sequenceNumber": 7, "languageCode": "eng", "summaryText": "Isolated summary fixture"}]
		}
	})).await?;
	assert_eq!(status, StatusCode::OK, "{saved}");
	let diagnosis_id = saved["rows"]["senderDiagnoses"][0]["id"].clone();
	let summary_id = saved["rows"]["caseSummaryInformation"][0]["id"].clone();
	assert!(diagnosis_id.is_string() && summary_id.is_string());
	for deleted in [true, false] {
		let (status, patched) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({
				"authorities": ["ich"], "rows": {
					"senderDiagnoses": [{"id": diagnosis_id, "deleted": deleted}],
					"caseSummaryInformation": [{"id": summary_id, "deleted": deleted}]
				}
			}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{patched}");
		for (path, owner) in [
			(&uri, "rows"),
			(&format!("/api/cases/{case_id}/editor/NR"), "data"),
		] {
			let (status, loaded) = get_json(&app, &cookie, path).await?;
			assert_eq!(status, StatusCode::OK, "{loaded}");
			let rows = &loaded[owner];
			assert_eq!(rows["senderDiagnoses"][0]["id"], diagnosis_id);
			assert_eq!(rows["senderDiagnoses"][0]["deleted"], deleted);
			assert_eq!(rows["senderDiagnoses"][0]["sequence_number"], 4);
			assert_eq!(rows["caseSummaryInformation"][0]["id"], summary_id);
			assert_eq!(rows["caseSummaryInformation"][0]["deleted"], deleted);
			assert_eq!(rows["caseSummaryInformation"][0]["sequence_number"], 7);
			assert_eq!(
				rows["caseSummaryInformation"][0]["summary_text"],
				"Isolated summary fixture"
			);
			assert_eq!(rows["caseSummaryInformation"][0]["language_code"], "eng");
		}
		for resource in ["sender-diagnoses", "summaries"] {
			let (status, listed) = get_json(
				&app,
				&cookie,
				&format!("/api/cases/{case_id}/narrative/{resource}"),
			)
			.await?;
			assert_eq!(status, StatusCode::OK, "{listed}");
			assert_eq!(
				listed["data"].as_array().expect("actual list").len(),
				if deleted { 0 } else { 1 }
			);
		}
	}
	Ok(())
}
