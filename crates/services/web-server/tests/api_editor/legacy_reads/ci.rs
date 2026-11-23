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
async fn test_ci_legacy_read_projects_persisted_rows() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm).expect("web server configuration");
	let case_id =
		create_case_for_editor(&app, &cookie, "LEGACY-CI-READ", &["ich"]).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/CI");
	let (status, saved) = patch_json(&app, &cookie, &uri, json!({"authorities":["ich"],"rows":{"case":{"reportYear":"2026"},"otherCaseIdentifiers":[{"sequenceNumber":1,"source":"Legacy source","caseIdentifier":"LEGACY-CI-ID"}]}})).await?;
	assert_eq!(status, StatusCode::OK, "{saved}");
	let row_id = saved["rows"]["otherCaseIdentifiers"][0]["id"].clone();
	let (status, body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/editor/CI")).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	assert_eq!(body["data"]["case"]["reportYear"], "2026");
	assert_eq!(body["data"]["otherCaseIdentifiers"][0]["id"], row_id);
	assert_eq!(
		body["data"]["otherCaseIdentifiers"][0]["caseIdentifier"],
		"LEGACY-CI-ID"
	);
	assert_eq!(body["data"]["otherCaseIdentifiers"][0]["deleted"], false);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_ci_editor_retains_soft_deleted_children_for_restore_only() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm).expect("web server configuration");
	let case_id =
		create_case_for_editor(&app, &cookie, "CI-RESTORE", &["ich"]).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/CI");
	let (status, saved) = patch_json(&app, &cookie, &uri, json!({
		"authorities":["ich"], "rows": {
			"documentsHeldBySender":[{"sequenceNumber":4,"documentDescription":"Isolated document","includedDocument":"Zml4dHVyZQ==","fileName":"fixture.txt","mediaType":"text/plain","representation":"B64"}],
			"otherCaseIdentifiers":[{"sequenceNumber":7,"source":"Isolated source","caseIdentifier":"Isolated identifier"}],
			"linkedReports":[{"sequenceNumber":2,"linkedReportNumber":"Isolated linked report"}]
		}
	})).await?;
	assert_eq!(status, StatusCode::OK, "{saved}");
	for deleted in [true, false] {
		let mut patch = json!({});
		for owner in [
			"documentsHeldBySender",
			"otherCaseIdentifiers",
			"linkedReports",
		] {
			let id = &saved["rows"][owner][0]["id"];
			assert!(id.is_string());
			patch[owner] = json!([{"id":id,"deleted":deleted}]);
		}
		let (status, changed) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":patch}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{changed}");
		for (path, root) in [
			(&uri, "rows"),
			(&format!("/api/cases/{case_id}/editor/CI"), "data"),
		] {
			let (status, loaded) = get_json(&app, &cookie, path).await?;
			assert_eq!(status, StatusCode::OK, "{loaded}");
			for owner in [
				"documentsHeldBySender",
				"otherCaseIdentifiers",
				"linkedReports",
			] {
				let mut expected = saved["rows"][owner].clone();
				expected[0]["deleted"] = json!(deleted);
				assert_eq!(
					loaded[root][owner], expected,
					"{owner}: identity, sequence, text and attachment preserved"
				);
			}
		}
		for resource in [
			"safety-report/documents",
			"other-identifiers",
			"linked-reports",
		] {
			let (status, listed) =
				get_json(&app, &cookie, &format!("/api/cases/{case_id}/{resource}"))
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
