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
		"drugCharacterization":"1", "medicinalProduct":"DEVICE TEST",
        "mfdsDeviceInfo":{"serialNumber":"SN-1","udiManufacturingDate":"20260901","models":[{"value":"MODEL-1"}]},
        "fdaDevices":[{"deviceBrandName":"DEVICE-1","followUpTypes":[{"valueCode":"1"}]}]
    }}})).await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let drug_id = body["rowId"].as_str().ok_or("missing drug id")?;
	let uri = format!("{base}/DG/rows/{drug_id}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	let drug = &saved["data"]["drug"];
	assert_eq!(drug["drug_characterization"], "1", "{saved}");
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
async fn draft_save_fda_device_identity_regressions() -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let id = create_case_for_editor(&app, &cookie, "FDA-DEVICE-IDENTITY", &["fda"])
		.await?;
	let base = format!("/api/cases/{id}/editor/pages");
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("{base}/DG/rows"),
		json!({"authorities":["fda"],"rows":{"drug":{
			"drugCharacterization":"1", "medicinalProduct":"DEVICE TEST",
			"fdaDevices":[{"deviceBrandName":"DEVICE-1","followUpTypes":[{"valueCode":"1"}]}]
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let drug_id = body["rowId"].as_str().ok_or("missing drug id")?;
	let uri = format!("{base}/DG/rows/{drug_id}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	let drug = &saved["data"]["drug"];
	assert_eq!(drug["drug_characterization"], "1", "{saved}");
	let original_device = &drug["fdaDevices"][0];
	let original_device_id = original_device["id"].clone();
	let original_created_at = original_device["created_at"].clone();
	let original_code_id = original_device["followUpTypes"][0]["id"].clone();
	let original_code_created_at =
		original_device["followUpTypes"][0]["created_at"].clone();
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[{
			"id":original_device_id,
			"deviceBrandName":null,
			"deviceBrandNameNullFlavor":null,
			"followUpTypes":[{"id":original_code_id,"valueCode":"1"}]
		}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	let edited = &saved["data"]["drug"]["fdaDevices"][0];
	assert_eq!(edited["id"], original_device_id, "{saved}");
	assert_eq!(edited["created_at"], original_created_at, "{saved}");
	assert_eq!(edited["device_brand_name"], json!(null), "{saved}");
	assert_eq!(
		edited["device_brand_name_null_flavor"],
		json!(null),
		"{saved}"
	);
	assert_eq!(
		edited["followUpTypes"][0]["id"], original_code_id,
		"{saved}"
	);
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[{
			"id":original_device_id,
			"deviceBrandName":null,
			"deviceBrandNameNullFlavor":null,
			"followUpTypes":[{"id":original_code_id,"valueCode":"2"}]
		}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, code_edited) = get_json(&app, &cookie, &uri).await?;
	let edited_code =
		&code_edited["data"]["drug"]["fdaDevices"][0]["followUpTypes"][0];
	assert_eq!(edited_code["id"], original_code_id, "{code_edited}");
	assert_eq!(
		edited_code["created_at"], original_code_created_at,
		"{code_edited}"
	);
	assert_eq!(edited_code["value_code"], "2", "{code_edited}");
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[{
			"id":original_device_id,
			"followUpTypes":[
				{"id":original_code_id,"valueCode":"2"},
				{"id":original_code_id,"valueCode":"2"}
			]
		}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	let (_, duplicate_code_unchanged) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(duplicate_code_unchanged, code_edited);

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[{
			"id":original_device_id,
			"deviceBrandName":null,
			"deviceBrandNameNullFlavor":null,
			"followUpTypes":[{"id":original_code_id,"valueCode":"1"}]
		},{"deviceBrandName":"DEVICE-2"}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	let devices = saved["data"]["drug"]["fdaDevices"]
		.as_array()
		.ok_or("missing FDA devices")?;
	assert_eq!(devices.len(), 2, "{saved}");
	assert_eq!(devices[0]["id"], original_device_id, "{saved}");
	let second_id = devices[1]["id"].clone();
	let second_created_at = devices[1]["created_at"].clone();
	let second_before_sibling_edit = devices[1].clone();
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[{
			"id":original_device_id,"deviceBrandName":"DEVICE-1-EDITED",
			"followUpTypes":[{"id":original_code_id,"valueCode":"1"}]
		},{"id":second_id,"deviceBrandName":"DEVICE-2"}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, after_sibling_edit) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(
		after_sibling_edit["data"]["drug"]["fdaDevices"][1],
		second_before_sibling_edit,
		"{after_sibling_edit}"
	);
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[{
			"id":second_id,
			"deviceBrandName":"DEVICE-2"
		}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	let devices = saved["data"]["drug"]["fdaDevices"]
		.as_array()
		.ok_or("missing FDA devices")?;
	assert_eq!(devices.len(), 1, "{saved}");
	assert_eq!(devices[0]["id"], second_id, "{saved}");
	assert_eq!(devices[0]["created_at"], second_created_at, "{saved}");
	let retained_sequence = devices[0]["sequence_number"].clone();
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[
			{"id":second_id,"deviceBrandName":"DEVICE-2"},
			{"deviceBrandName":"DEVICE-3"}
		]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, appended) = get_json(&app, &cookie, &uri).await?;
	let appended_devices = appended["data"]["drug"]["fdaDevices"]
		.as_array()
		.ok_or("missing appended FDA devices")?;
	assert_eq!(appended_devices.len(), 2, "{appended}");
	assert_eq!(appended_devices[0]["id"], second_id, "{appended}");
	assert_eq!(
		appended_devices[0]["sequence_number"], retained_sequence,
		"{appended}"
	);
	assert_ne!(
		appended_devices[1]["sequence_number"], retained_sequence,
		"{appended}"
	);
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[{
			"id":second_id,"deviceBrandName":"DEVICE-2"
		}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[
			{"id":second_id,"deviceBrandName":"DEVICE-2"},
			{"id":second_id,"deviceBrandName":"DEVICE-2"}
		]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	let (_, unchanged) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(
		unchanged["data"]["drug"]["fdaDevices"],
		saved["data"]["drug"]["fdaDevices"]
	);
	let (status, other) = post_json(
		&app,
		&cookie,
		&format!("{base}/DG/rows"),
		json!({"authorities":["fda"],"rows":{"drug":{
			"drugCharacterization":"1", "medicinalProduct":"OTHER", "sequenceNumber":2,
			"fdaDevices":[{"deviceBrandName":"FOREIGN","followUpTypes":[{"valueCode":"2"}]}]
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{other}");
	let other_id = other["rowId"].as_str().ok_or("missing other drug id")?;
	let (_, other_saved) =
		get_json(&app, &cookie, &format!("{base}/DG/rows/{other_id}")).await?;
	let foreign_device_id =
		other_saved["data"]["drug"]["fdaDevices"][0]["id"].clone();
	let foreign_code_id = other_saved["data"]["drug"]["fdaDevices"][0]
		["followUpTypes"][0]["id"]
		.clone();
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[{
			"id":foreign_device_id,"deviceBrandName":"FOREIGN"
		}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
	let (_, unchanged) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(
		unchanged["data"]["drug"]["fdaDevices"],
		saved["data"]["drug"]["fdaDevices"]
	);
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[{
			"id":second_id,"deviceBrandName":"DEVICE-2",
			"followUpTypes":[{"id":foreign_code_id,"valueCode":"2"}]
		}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	let (_, unchanged) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(
		unchanged["data"]["drug"]["fdaDevices"],
		saved["data"]["drug"]["fdaDevices"]
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

	for (section, owner, field, column, initial, row, rule_code, path) in [
		(
			"DG",
			"drug",
			"medicinalProduct",
			"medicinal_product",
			"Before",
			json!({"drugCharacterization":"1", "medicinalProduct":"Before"}),
			"INPUT.NULL.REJECTED",
			"drugs.0.medicinalProduct",
		),
		(
			"DG",
			"drug",
			"drugCharacterization",
			"drug_characterization",
			"1",
			json!({"drugCharacterization":"1", "medicinalProduct":"Before"}),
			"INPUT.NULL.REJECTED",
			"drugs.0.drugCharacterization",
		),
	] {
		let id =
			create_case_for_editor(&app, &cookie, "CLEAR-NAME", &["ich"]).await?;
		let uri = format!("/api/cases/{id}/editor/pages/{section}/rows");
		let mut row = row;
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
		let (_, before) = get_json(&app, &cookie, &detail).await?;
		let (status, body) = patch_json(
			&app,
			&cookie,
			&detail,
			json!({"authorities":["ich"],"rows":{owner:{field:""}}}),
		)
		.await?;
		assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
		assert_eq!(body["error"]["data"]["detail"]["ruleCode"], rule_code);
		assert_eq!(body["error"]["data"]["detail"]["path"], path);
		assert_eq!(body["error"]["data"]["detail"]["message"], "cannot be null");
		let (_, saved) = get_json(&app, &cookie, &detail).await?;
		assert_eq!(saved, before, "{body}");
		assert_eq!(saved["data"][owner][column], initial, "{saved}");
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
	for (value, expected) in [
		(json!("Before"), json!("Before")),
		(json!(""), serde_json::Value::Null),
	] {
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":{"narrative":{"senderComments":value}}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		let (_, saved) = get_json(&app, &cookie, &uri).await?;
		assert_eq!(
			saved["rows"]["narrative"]["sender_comments"], expected,
			"{saved}"
		);
	}
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{"narrative":{"caseNarrative":"Before"}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, before) = get_json(&app, &cookie, &uri).await?;
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{"narrative":{"caseNarrative":""}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
	assert_eq!(
		body["error"]["data"]["detail"],
		json!({
			"message": "cannot be null",
			"path": "narrative.caseNarrative",
			"ruleCode": "INPUT.NULL.REJECTED"
		})
	);
	let (_, after) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(after, before);
	Ok(())
}

#[serial_test::serial]
#[tokio::test]
async fn blank_safety_report_id_is_rejected() -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "BLANK-SAFETY-REPORT-ID", &["ich"])
			.await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/CI");
	let (_, before) = get_json(&app, &cookie, &uri).await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{"safetyReportIdentification":{"safetyReportId":"  "}}}),
	)
	.await?;

	assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
	assert_eq!(
		body["error"]["data"]["detail"],
		json!({
			"message": "cannot be null",
			"path": "safetyReportIdentification.safetyReportId",
			"ruleCode": "INPUT.NULL.REJECTED"
		})
	);
	let (_, after) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(after, before);
	Ok(())
}

#[serial_test::serial]
#[tokio::test]
async fn canonical_ci_business_required_fields_allow_draft_clear(
) -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "CI-REQUIRED-CLEAR", &["ich"]).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/CI");

	for field in [
		"additionalDocumentsAvailable",
		"worldwideUniqueId",
		"firstSenderType",
	] {
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":{"safetyReportIdentification":{(field):null}}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{field}: {body}");
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

	for (section, owner, payload, column) in [(
		"LB",
		"testResult",
		json!({"testMeddraVersion": "28.1"}),
		"test_name",
	)] {
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

	let case_id =
		create_case_for_editor(&app, &cookie, "DRAFT-SAVE", &["ich"]).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/DG");
	let (_, before) = get_json(&app, &cookie, &uri).await?;
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("{uri}/rows"),
		json!({"authorities": ["ich"], "rows": {"drug": {
			"drugCharacterization": "1", "medicinalProduct": ""
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
	assert_eq!(
		body["error"]["data"]["detail"],
		json!({
			"message": "cannot be null",
			"path": "drugs.0.medicinalProduct",
			"ruleCode": "INPUT.NULL.REJECTED"
		})
	);
	let (_, after) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(after, before);

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
