use super::support::{create_case_for_editor, get_json, patch_json, post_json};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::json;

#[serial_test::serial]
#[tokio::test]
async fn draft_save_ui_regressions() -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let id =
		create_case_for_editor(&app, &cookie, "UI-REGRESSION", &["ich"]).await?;
	let base = format!("/api/cases/{id}/editor/pages");
	let (status, body) = post_json(&app, &cookie, &format!("{base}/DG/rows"), json!({"authorities":["ich"],"rows":{"drug":{
        "drugCharacterization":"", "medicinalProduct":"",
        "mfdsDeviceInfo":{"serialNumber":"SN-1","udiManufacturingDate":"20260901","models":[{"value":"MODEL-1"}]},
        "fdaDevices":[{"deviceBrandName":"DEVICE-1","followUpTypes":[{"valueCode":"1"}]}]
    }}})).await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let drug_id = body["rowId"].as_str().ok_or("missing drug id")?;
	let uri = format!("{base}/DG/rows/{drug_id}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	let drug = &saved["data"]["drug"];
	assert_eq!(drug["drug_characterization"], "", "{saved}");
	assert_eq!(
		drug["fdaDevices"].as_array().map(Vec::len),
		Some(1),
		"{saved}"
	);
	assert!(
		!drug["deviceCharacteristics"]
			.as_array()
			.unwrap()
			.iter()
			.any(|r| r["value_value"] == "SN-1"),
		"{saved}"
	);
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{"drug":{"fdaDevices":[]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(
		saved["data"]["drug"]["fdaDevices"].as_array().map(Vec::len),
		Some(0),
		"{saved}"
	);

	let uri = format!("{base}/CI");
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{"case":{"reportYear":"abcd"}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
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

#[serial_test::serial]
#[tokio::test]
async fn draft_save_empty_lists_rows_and_cleared_values() -> crate::common::Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	for (section, empty, parent) in [
		(
			"SI",
			json!({"studyRegistrationNumbers": []}),
			"studyInformation",
		),
		(
			"SI",
			json!({"studyInformation": {"fdaCrossReportedIndNumbers": []}}),
			"studyInformation",
		),
		(
			"NR",
			json!({"senderDiagnoses": [], "caseSummaryInformation": []}),
			"narrative",
		),
		(
			"DM",
			json!({"medicalHistoryEpisodes": [], "parentInfo": {"pastDrugHistory": []}, "parentPastDrugs": []}),
			"patientInformation",
		),
	] {
		let id =
			create_case_for_editor(&app, &cookie, "EMPTY-LIST", &["ich"]).await?;
		let uri = format!("/api/cases/{id}/editor/pages/{section}");
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":empty}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		let (_, saved) = get_json(&app, &cookie, &uri).await?;
		assert!(saved["rows"][parent].is_null(), "{saved}");
	}
	for owner in ["parentMedicalHistory", "parentPastDrugs"] {
		let id =
			create_case_for_editor(&app, &cookie, "EMPTY-ROW", &["ich"]).await?;
		let uri = format!("/api/cases/{id}/editor/pages/DM");
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":{owner:[{}]}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		assert_eq!(
			body["rows"][owner].as_array().map(Vec::len),
			Some(1),
			"{body}"
		);
	}
	for (section, owner, field, column) in [
		("LB", "testResult", "testName", "test_name"),
		("DG", "drug", "medicinalProduct", "medicinal_product"),
	] {
		let id =
			create_case_for_editor(&app, &cookie, "CLEAR-NAME", &["ich"]).await?;
		let uri = format!("/api/cases/{id}/editor/pages/{section}/rows");
		let mut row = json!({field:"Before"});
		if section == "DG" {
			row["activeSubstances"] = json!([{}]);
			row["dosageInformation"] = json!([{}]);
			row["indications"] = json!([{}]);
		}
		let (status, body) = post_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":{owner:row}}),
		)
		.await?;
		assert_eq!(status, StatusCode::CREATED, "{body}");
		let row_id = body["rowId"].as_str().ok_or("missing row id")?;
		let detail = format!("{uri}/{row_id}");
		let (status, body) = patch_json(
			&app,
			&cookie,
			&detail,
			json!({"authorities":["ich"],"rows":{owner:{field:""}}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		let (_, saved) = get_json(&app, &cookie, &detail).await?;
		assert_eq!(saved["data"][owner][column], "", "{saved}");
		if section == "DG" {
			for child in ["activeSubstances", "dosageInformation", "indications"] {
				assert_eq!(
					saved["data"][owner][child].as_array().map(Vec::len),
					Some(1),
					"{saved}"
				);
			}
		}
	}
	let id = create_case_for_editor(&app, &cookie, "CLEAR-NR", &["ich"]).await?;
	let uri = format!("/api/cases/{id}/editor/pages/NR");
	for value in ["Before", ""] {
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":{"narrative":{"caseNarrative":value}}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		let (_, saved) = get_json(&app, &cookie, &uri).await?;
		assert_eq!(
			saved["rows"]["narrative"]["case_narrative"], value,
			"{saved}"
		);
	}
	Ok(())
}

#[serial_test::serial]
#[tokio::test]
async fn draft_save_child_first_preserves_data() -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	for (section, rows, collection, field, expected) in [
		(
			"SI",
			json!({"studyRegistrationNumbers": [{"registrationNumber": "", "countryCode": "KR"}]}),
			"studyRegistrationNumbers",
			"country_code",
			json!("KR"),
		),
		(
			"SI",
			json!({"studyRegistrationNumbers": [{}]}),
			"studyRegistrationNumbers",
			"registration_number",
			serde_json::Value::Null,
		),
		(
			"DM",
			json!({"medicalHistoryEpisodes": [{"comments": "History first"}]}),
			"medicalHistoryEpisodes",
			"comments",
			json!("History first"),
		),
		(
			"DM",
			json!({"reportedCauses": [{"causeText": "Death cause first"}]}),
			"reportedCauses",
			"comments",
			json!("Death cause first"),
		),
		(
			"DM",
			json!({"parentMedicalHistory": [{"comments": "Parent history first"}]}),
			"parentMedicalHistory",
			"comments",
			json!("Parent history first"),
		),
		(
			"DM",
			json!({"parentPastDrugs": [{"drugName": "Parent drug first"}]}),
			"parentPastDrugs",
			"drug_name",
			json!("Parent drug first"),
		),
	] {
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
		let parent_key = if section == "SI" {
			"studyInformation"
		} else {
			"patientInformation"
		};
		let parent_field = if section == "SI" {
			"studyName"
		} else {
			"patientInitials"
		};
		let parent_column = if section == "SI" {
			"study_name"
		} else {
			"patient_initials"
		};
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

#[serial_test::serial]
#[tokio::test]
async fn draft_save_creates_incomplete_rows_and_missing_parents(
) -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	for (section, owner, payload, column) in [
		(
			"LB",
			"testResult",
			json!({"testMeddraVersion": "28.1"}),
			"test_name",
		),
		(
			"DG",
			"drug",
			json!({"drugCharacterization": "1", "medicinalProduct": ""}),
			"medicinal_product",
		),
	] {
		let case_id =
			create_case_for_editor(&app, &cookie, "DRAFT-SAVE", &["ich"]).await?;
		let uri = format!("/api/cases/{case_id}/editor/pages/{section}/rows");
		let (status, body) = post_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities": ["ich"], "rows": {owner: payload}}),
		)
		.await?;
		assert_eq!(status, StatusCode::CREATED, "{section}: {body}");
		let row_id = body["rowId"].as_str().ok_or("missing row id")?;
		let (status, saved) =
			get_json(&app, &cookie, &format!("{uri}/{row_id}")).await?;
		assert_eq!(status, StatusCode::OK, "{saved}");
		assert_eq!(saved["data"][owner][column], "", "{saved}");
	}

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
