use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, patch_json, post_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::json;

pub(crate) async fn child_creation_and_update_preserve_parent_identity(
	page_id: &str,
	check_id: &str,
) -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let contract = crate::case_editor_contract_web::support::behavior_contract(
		page_id,
		"relations",
		check_id,
	)?;
	let parameters = &contract["parameters"];
	let parent_key = parameters["parent"]["owner"]
		.as_str()
		.ok_or("missing parent owner")?;
	let parent_field = parameters["parent"]["inputField"]
		.as_str()
		.ok_or("missing parent input field")?;
	let parent_column = parameters["parent"]["column"]
		.as_str()
		.ok_or("missing parent column")?;
	let cases = parameters["cases"]
		.as_array()
		.ok_or("missing child-first cases")?;
	assert!(
		!cases.is_empty(),
		"child-first contract must exercise an input"
	);
	for case in cases {
		let section = page_id;
		let rows = case.get("rows").ok_or("missing child-first rows")?;
		let collection = case["collection"].as_str().ok_or("missing collection")?;
		let field = case["field"].as_str().ok_or("missing child field")?;
		let expected = case
			.get("expected")
			.ok_or("missing expected child value")?
			.clone();

		let case_id =
			create_case_for_editor(&app, &cookie, "CHILD-FIRST", &["ich"]).await?;
		let uri = format!("/api/cases/{case_id}/editor/pages/{section}");
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities": ["ich"], "rows": rows}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{section}: {body}");
		let (_, saved) = get_json(&app, &cookie, &uri).await?;
		assert_eq!(
			saved["rows"][collection].as_array().map(Vec::len),
			Some(1),
			"{saved}"
		);
		assert_eq!(saved["rows"][collection][0][field], expected, "{saved}");
		let parent_id = saved["rows"][parent_key]["id"].clone();
		assert!(parent_id.is_string(), "{saved}");
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities": ["ich"], "rows": {parent_key: {parent_field: "KEEP"}}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		// A child-only update must reuse the parent and preserve its scalar values.
		let row_id = saved["rows"][collection][0]["id"].clone();
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities": ["ich"], "rows": {collection: [{"id": row_id}]}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		let (_, saved) = get_json(&app, &cookie, &uri).await?;
		assert_eq!(saved["rows"][parent_key]["id"], parent_id, "{saved}");
		assert_eq!(saved["rows"][parent_key][parent_column], "KEEP", "{saved}");
		assert_eq!(saved["rows"][collection][0][field], expected, "{saved}");
	}

	Ok(())
}

pub(crate) async fn past_drug_creation_reuses_patient_parent(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "DH-FIRST", &["ich"]).await?;
	let dh_uri = format!("/api/cases/{case_id}/editor/pages/DH/rows");
	let dm_uri = format!("/api/cases/{case_id}/editor/pages/DM");
	for (sequence, name) in [(1, "First drug"), (2, "Second drug")] {
		let (status, body) = post_json(
			&app,
			&cookie,
			&dh_uri,
			json!({"authorities": ["ich"], "rows": {"pastDrugHistory": {"drugName": name, "sequenceNumber": sequence}}}),
		)
		.await?;
		assert_eq!(status, StatusCode::CREATED, "{body}");
		let row_id = body["rowId"].as_str().ok_or("missing DH row id")?;
		let (_, saved) =
			get_json(&app, &cookie, &format!("{dh_uri}/{row_id}")).await?;
		assert_eq!(
			saved["data"]["pastDrugHistory"]["drug_name"], name,
			"{saved}"
		);
		let (_, patient) = get_json(&app, &cookie, &dm_uri).await?;
		assert!(
			patient["rows"]["patientInformation"]["id"].is_string(),
			"{patient}"
		);
		if name == "Second drug" {
			assert_eq!(
				patient["rows"]["patientInformation"]["patient_initials"], "KEEP",
				"{patient}"
			);
		}
		let (status, body) = patch_json(&app, &cookie, &dm_uri, json!({"authorities": ["ich"], "rows": {"patientInformation": {"patientInitials": "KEEP"}}})).await?;
		assert_eq!(status, StatusCode::OK, "{body}");
	}

	Ok(())
}

pub(crate) async fn partial_narrative_and_children_keep_parent(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	for rows in [
		json!({"narrative": {"senderComments": "Persist this comment"}}),
		json!({"caseSummaryInformation": [{"languageCode": "en", "summaryText": "Persist summary"}]}),
		json!({"narrative": {"reporterComments": "Reporter comment"}, "senderDiagnoses": [{"diagnosisMeddraVersion": "28.1", "diagnosisMeddraCode": "10019211"}]}),
	] {
		let case_id =
			create_case_for_editor(&app, &cookie, "DRAFT-NR", &["ich"]).await?;
		let uri = format!("/api/cases/{case_id}/editor/pages/NR");
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities": ["ich"], "rows": rows}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		let (_, saved) = get_json(&app, &cookie, &uri).await?;
		let narrative_id = saved["rows"]["narrative"]["id"].clone();
		assert!(narrative_id.is_string(), "{saved}");
		assert_eq!(saved["rows"]["narrative"]["case_narrative"], "", "{saved}");
		if rows["narrative"]["senderComments"].is_string() {
			assert_eq!(
				saved["rows"]["narrative"]["sender_comments"],
				"Persist this comment"
			);
		}
		if rows["caseSummaryInformation"].is_array() {
			assert_eq!(
				saved["rows"]["caseSummaryInformation"][0]["summary_text"],
				"Persist summary",
				"{saved}"
			);
		}
		if rows["senderDiagnoses"].is_array() {
			assert_eq!(
				saved["rows"]["senderDiagnoses"][0]["diagnosis_meddra_code"],
				"10019211",
				"{saved}"
			);
			assert_eq!(
				saved["rows"]["narrative"]["reporter_comments"],
				"Reporter comment"
			);
		}
		// Updating a child must keep the same parent, not replace it with a fresh blank row.
		let (status, body) = patch_json(&app, &cookie, &uri, json!({"authorities": ["ich"], "rows": {"narrative": {"senderComments": "Updated"}}})).await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		assert_eq!(body["rows"]["narrative"]["id"], narrative_id);
	}

	Ok(())
}

pub(crate) async fn registration_creates_study_with_empty_name(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "DRAFT-SI", &["ich"]).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/SI");
	let (status, body) = patch_json(&app, &cookie, &uri, json!({"authorities": ["ich"], "rows": {"studyRegistrationNumbers": [{"registrationNumber": "REG-TEST", "countryCode": "KR"}]}})).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	assert!(
		saved["rows"]["studyInformation"]["id"].is_string(),
		"{saved}"
	);
	assert!(
		saved["rows"]["studyInformation"]["study_name"].is_null(),
		"{saved}"
	);
	assert_eq!(
		saved["rows"]["studyRegistrationNumbers"][0]["registration_number"],
		"REG-TEST",
		"{saved}"
	);

	Ok(())
}
