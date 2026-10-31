use super::support::field_contract_test;
const PAGE_ID: &str = "CI";
include!("generated/ci_fields.rs");

#[serial_test::serial]
#[tokio::test]
async fn report_type_can_be_cleared_and_reloaded() -> crate::common::Result<()> {
	use super::support::{create_case_for_editor, get_json, patch_json};
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
		create_case_for_editor(&app, &cookie, "EDITOR-CI-CLEAR", &["ich"]).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/CI");

	for report_type in [json!("1"), Value::Null] {
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities": ["ich"], "rows": {
				"safetyReportIdentification": {"reportType": report_type}
			}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
	}

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["rows"]["safetyReportIdentification"]["reportType"],
		Value::Null
	);
	Ok(())
}

#[serial_test::serial]
#[tokio::test]
async fn fulfil_expedited_criteria_null_flavor_round_trips(
) -> crate::common::Result<()> {
	use super::support::{create_case_for_editor, get_json, patch_json};
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
		create_case_for_editor(&app, &cookie, "EDITOR-CI-NF", &["ich"]).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/CI");
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({
			"authorities": ["ich"],
			"rows": {"safetyReportIdentification": {
				"fulfilExpeditedCriteriaNullFlavor": "NI"
			}}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["rows"]["safetyReportIdentification"]
			["fulfilExpeditedCriteriaNullFlavor"],
		"NI"
	);
	Ok(())
}

#[serial_test::serial]
#[tokio::test]
async fn non_nullable_ci_child_text_distinguishes_null_from_omission(
) -> crate::common::Result<()> {
	use super::support::{create_case_for_editor, get_json, patch_json};
	use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
	use axum::http::StatusCode;
	use lib_auth::token::generate_web_token;
	use serde_json::{json, Value};

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case_for_editor(
		&app,
		&cookie,
		"EDITOR-CI-NON-NULL-TEXT-PATCH",
		&["ich"],
	)
	.await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/CI");

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{
			"otherCaseIdentifiers":[{"source":"SOURCE-1","caseIdentifier":"US-SENDER-CASE1"}],
			"linkedReports":[{"linkedReportNumber":"LINK-1"}]
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let other_id = body["rows"]["otherCaseIdentifiers"][0]["id"].clone();
	let linked_id = body["rows"]["linkedReports"][0]["id"].clone();

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{
			"otherCaseIdentifiers":[{"id":other_id.clone(),"source":"SOURCE-2","caseIdentifier":"US-SENDER-CASE2"}],
			"linkedReports":[{"id":linked_id.clone(),"linkedReportNumber":"LINK-2"}]
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{
			"otherCaseIdentifiers":[{"id":other_id.clone()}],
			"linkedReports":[{"id":linked_id.clone()}]
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["rows"]["otherCaseIdentifiers"][0]["source"],
		"SOURCE-2"
	);
	assert_eq!(
		body["rows"]["otherCaseIdentifiers"][0]["caseIdentifier"],
		"US-SENDER-CASE2"
	);
	assert_eq!(
		body["rows"]["linkedReports"][0]["linkedReportNumber"],
		"LINK-2"
	);

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{
			"otherCaseIdentifiers":[{"id":other_id.clone(),"source":null}],
			"linkedReports":[{"id":linked_id.clone(),"linkedReportNumber":null}]
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["rows"]["otherCaseIdentifiers"][0]["source"], "");
	assert_eq!(
		body["rows"]["otherCaseIdentifiers"][0]["caseIdentifier"],
		"US-SENDER-CASE2"
	);
	assert_eq!(body["rows"]["linkedReports"][0]["linkedReportNumber"], "");

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["rows"]["otherCaseIdentifiers"][0]["source"], "");
	assert_eq!(
		body["rows"]["otherCaseIdentifiers"][0]["caseIdentifier"],
		"US-SENDER-CASE2"
	);
	assert_eq!(body["rows"]["linkedReports"][0]["linkedReportNumber"], "");

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{
			"otherCaseIdentifiers":[{"id":other_id.clone(),"caseIdentifier":null}],
			"linkedReports":[{"id":linked_id.clone(),"linkedReportNumber":"LINK-RESTORED"}]
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["rows"]["otherCaseIdentifiers"][0]["source"], "");
	assert_eq!(
		body["rows"]["otherCaseIdentifiers"][0]["caseIdentifier"],
		""
	);
	assert_eq!(
		body["rows"]["linkedReports"][0]["linkedReportNumber"],
		"LINK-RESTORED"
	);

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{
			"otherCaseIdentifiers":[{"id":other_id.clone(),"source":""}],
			"linkedReports":[{"id":linked_id.clone(),"linkedReportNumber":""}]
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["rows"]["otherCaseIdentifiers"][0]["source"], "");
	assert_eq!(
		body["rows"]["otherCaseIdentifiers"][0]["caseIdentifier"],
		""
	);
	assert_eq!(body["rows"]["linkedReports"][0]["linkedReportNumber"], "");

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{
			"otherCaseIdentifiers":[{"id":other_id.clone(),"source":"SOURCE-RESTORED","caseIdentifier":"US-SENDER-RESTORED"}],
			"linkedReports":[{"id":linked_id.clone(),"linkedReportNumber":"LINK-RESTORED"}]
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["rows"]["otherCaseIdentifiers"][0]["source"],
		"SOURCE-RESTORED"
	);
	assert_eq!(
		body["rows"]["otherCaseIdentifiers"][0]["caseIdentifier"],
		"US-SENDER-RESTORED"
	);
	assert_eq!(
		body["rows"]["linkedReports"][0]["linkedReportNumber"],
		"LINK-RESTORED"
	);

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["rows"]["otherCaseIdentifiers"][0]["source"],
		"SOURCE-RESTORED"
	);
	assert_eq!(
		body["rows"]["otherCaseIdentifiers"][0]["caseIdentifier"],
		"US-SENDER-RESTORED"
	);
	assert_eq!(
		body["rows"]["linkedReports"][0]["linkedReportNumber"],
		"LINK-RESTORED"
	);

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{"linkedReports":[{}]}}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	assert!(
		body.to_string().contains("linkedReportNumber is required"),
		"{body}"
	);

	assert_ne!(other_id, Value::Null);
	assert_ne!(linked_id, Value::Null);
	Ok(())
}
