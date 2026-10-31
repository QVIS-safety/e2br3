use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, patch_json, post_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use uuid::Uuid;

pub(crate) async fn non_nullable_ci_child_text_distinguishes_null_from_omission(
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

pub(crate) async fn clearing_phpid_survives_dg_row_reload(
	_page_id: &str,
	_check_id: &str,
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case_for_editor(
		&app,
		&cookie,
		&format!("EDITOR-DG-PHPID-CLEAR-{}", Uuid::new_v4()),
		&["ich"],
	)
	.await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"drug_characterization": "1",
				"medicinal_product": "Regression drug",
				"phpid": "PHPID-OLD",
				"phpid_version": "202608"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let drug_id = body["data"]["id"].as_str().ok_or("missing drug id")?;
	let uri = format!("/api/cases/{case_id}/editor/pages/DG/rows/{drug_id}");

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({
			"authorities": ["ich"],
			"rows": {"drug": {"phpid": null, "phpidVersion": null}}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["drug"]["phpid"], Value::Null);
	assert_eq!(body["data"]["drug"]["phpid_version"], Value::Null);
	Ok(())
}

pub(crate) async fn blank_optional_text_clears_without_changing_omitted_sibling(
	_page_id: &str,
	_check_id: &str,
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case_for_editor(
		&app,
		&cookie,
		&format!("EDITOR-DG-BLANK-CLEAR-{}", Uuid::new_v4()),
		&["ich"],
	)
	.await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs"),
		json!({"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"drug_characterization": "1",
			"medicinal_product": "Regression drug",
			"phpid": "PHPID-OLD",
			"phpid_version": "202608"
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let drug_id = body["data"]["id"].as_str().ok_or("missing drug id")?;
	let uri = format!("/api/cases/{case_id}/editor/pages/DG/rows/{drug_id}");

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({
			"authorities": ["ich"],
			"rows": {"drug": {"phpid": "  "}}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["drug"]["phpid"], Value::Null);
	assert_eq!(body["data"]["drug"]["phpid_version"], json!("202608"));
	Ok(())
}

pub(crate) async fn clearing_numeric_result_clears_qualifier_and_keeps_free_text(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	use crate::case_editor_contract_web::support::{
		create_case_for_editor, get_json, patch_json, post_json,
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
		create_case_for_editor(&app, &cookie, "EDITOR-LB-NUMERIC-CLEAR", &["ich"])
			.await?;
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/LB/rows"),
		json!({"authorities": ["ich"], "rows": {"testResult": {
			"sequenceNumber": 1,
			"testName": "Fixture",
			"testResult": "5.5",
			"testResultQualifier": "LE",
			"testResultUnstructured": "Free result"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let test_id = body["rowId"].as_str().ok_or("missing test result id")?;
	let uri = format!("/api/cases/{case_id}/editor/pages/LB/rows/{test_id}");
	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["testResult"]["test_result_value"], "5.5",
		"{body}"
	);
	assert_eq!(
		body["data"]["testResult"]["test_result_qualifier"], "LE",
		"{body}"
	);
	assert_eq!(
		body["data"]["testResult"]["result_unstructured"], "Free result",
		"{body}"
	);
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"testResult": {"testResult": "5.6"}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["testResult"]["test_result_value"], "5.6",
		"{body}"
	);
	assert_eq!(
		body["data"]["testResult"]["test_result_qualifier"], "LE",
		"{body}"
	);

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"testResult": {"testResultQualifier": "GE"}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["testResult"]["test_result_value"], "5.6",
		"{body}"
	);
	assert_eq!(
		body["data"]["testResult"]["test_result_qualifier"], "GE",
		"{body}"
	);

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"testResult": {"testResultQualifier": null}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["testResult"]["test_result_value"], "5.6",
		"{body}"
	);
	assert_eq!(
		body["data"]["testResult"]["test_result_qualifier"],
		Value::Null,
		"{body}"
	);

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"testResult": {"testResultQualifier": "LE"}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"testResult": {"testResult": null}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["testResult"]["test_result_value"],
		Value::Null,
		"{body}"
	);
	assert_eq!(
		body["data"]["testResult"]["test_result_qualifier"],
		Value::Null,
		"{body}"
	);
	assert_eq!(
		body["data"]["testResult"]["result_unstructured"], "Free result",
		"{body}"
	);

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"testResult": {"testResultQualifier": "LE"}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["testResult"]["test_result_value"],
		Value::Null,
		"{body}"
	);
	assert_eq!(
		body["data"]["testResult"]["test_result_qualifier"],
		Value::Null,
		"{body}"
	);
	assert_eq!(
		body["data"]["testResult"]["result_unstructured"], "Free result",
		"{body}"
	);
	Ok(())
}

pub(crate) async fn reporter_title_can_be_cleared_without_changing_siblings(
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
		create_case_for_editor(&app, &cookie, "EDITOR-RP-CLEAR", &["ich"]).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/RP");

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"primarySources": [{
			"reporterTitle": "Dr",
			"reporterGivenName": "Sibling"
		}]}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let source_id = body["rows"]["primarySources"][0]["id"]
		.as_str()
		.ok_or("missing primary source id")?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"primarySources": [{
			"id": source_id,
			"reporterTitle": null
		}]}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let source = &body["rows"]["primarySources"][0];
	assert_eq!(source["reporterTitle"], Value::Null);
	assert_eq!(source["reporterGivenName"], "Sibling");
	Ok(())
}

pub(crate) async fn summary_language_and_text_clear_together(
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
	let uri = format!("{base}/NR");
	let (status, body) = patch_json(&app, &cookie, &uri, json!({"authorities":["ich"],"rows":{"caseSummaryInformation":[{"languageCode":"eng","summaryText":"TEXT"}]}})).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	let summary_id = &saved["rows"]["caseSummaryInformation"][0]["id"];
	let (status, body) = patch_json(&app, &cookie, &uri, json!({"authorities":["ich"],"rows":{"caseSummaryInformation":[{"id":summary_id,"languageCode":null,"summaryText":null}]}})).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	assert!(
		saved["rows"]["caseSummaryInformation"][0]["language_code"].is_null(),
		"{saved}"
	);

	Ok(())
}

pub(crate) async fn blank_test_name_keeps_meddra_selection(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let id = create_case_for_editor(&app, &cookie, "CLEAR-LB-TEST-NAME", &["ich"])
		.await?;
	let uri = format!("/api/cases/{id}/editor/pages/LB/rows");
	let (status, body) = post_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{"testResult":{"testName":"Before"}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let row_id = body["rowId"].as_str().ok_or("missing LB row id")?;
	let detail = format!("{uri}/{row_id}");
	let (status, body) = patch_json(
		&app,
		&cookie,
		&detail,
		json!({"authorities":["ich"],"rows":{"testResult":{
			"testDate":"20210303",
			"testName":"",
			"testMeddraVersion":"28.1",
			"testMeddraCode":"10088978"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, saved) = get_json(&app, &cookie, &detail).await?;
	assert_eq!(saved["data"]["testResult"]["test_name"], "", "{saved}");
	assert_eq!(
		saved["data"]["testResult"]["test_meddra_version"], "28.1",
		"{saved}"
	);
	assert_eq!(
		saved["data"]["testResult"]["test_meddra_code"], "10088978",
		"{saved}"
	);

	Ok(())
}
