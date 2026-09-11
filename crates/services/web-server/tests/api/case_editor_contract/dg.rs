use super::support::{
	create_case_for_editor, field_contract_test, get_json, patch_json, post_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;

const PAGE_ID: &str = "DG";
include!("generated/dg_fields.rs");

#[serial]
#[tokio::test]
async fn g_k_3_2_patch_survives_row_reload() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case_for_editor(
		&app,
		&cookie,
		&format!("EDITOR-DG-GK32-{}", Uuid::new_v4()),
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
				"medicinal_product": "Regression drug"
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
			"rows": {"drug": {"drugAuthorizationCountry": "KR"}}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["drug"]["manufacturer_country"], json!("KR"));
	assert_eq!(
		body["data"]["drug"]["drugAuthorizationCountry"],
		json!("KR")
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn clearing_phpid_survives_dg_row_reload() -> Result<()> {
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

#[serial]
#[tokio::test]
async fn device_subresources_survive_dg_row_reload() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case_for_editor(
		&app,
		&cookie,
		&format!("EDITOR-DG-DEVICE-{}", Uuid::new_v4()),
		&["fda", "mfds"],
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
				"medicinal_product": "Device drug"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let drug_id = body["data"]["id"].as_str().ok_or("missing drug id")?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs/{drug_id}/devices"),
		json!({
			"data": {
				"drug_id": drug_id,
				"sequence_number": 1,
				"malfunction": true,
				"device_brand_name": "Imported Brand",
				"common_device_name_null_flavor": "NI"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let device_id = body["data"]["id"].as_str().ok_or("missing device id")?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs/{drug_id}/devices/{device_id}/codes"),
		json!({
			"data": {
				"device_id": device_id,
				"element": "device_problem",
				"sequence_number": 1,
				"value_code": "1234567"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs/{drug_id}/device-characteristics"),
		json!({
			"data": {
				"drug_id": drug_id,
				"sequence_number": 1,
				"code": "KR_DVC_MFR",
				"value_type": "ST",
				"value_value": "Imported Maker"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/DG/rows/{drug_id}"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["drug"]["fdaDevices"][0]["device_brand_name"],
		json!("Imported Brand")
	);
	assert_eq!(
		body["data"]["drug"]["fdaDevices"][0]["common_device_name_null_flavor"],
		json!("NI")
	);
	assert_eq!(
		body["data"]["drug"]["fdaDevices"][0]["deviceProblemCodes"][0]["value_code"],
		json!("1234567")
	);
	assert_eq!(
		body["data"]["drug"]["deviceCharacteristics"][0]["code"],
		json!("KR_DVC_MFR")
	);
	assert_eq!(
		body["data"]["drug"]["deviceCharacteristics"][0]["value_value"],
		json!("Imported Maker")
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn relatedness_delete_preserves_sibling_and_parent_delete_removes_assessment(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case_for_editor(
		&app,
		&cookie,
		"EDITOR-DG-ASSESSMENT-DELETE",
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
			"medicinal_product": "Assessment delete drug"
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let drug_id = body["data"]["id"].as_str().ok_or("missing drug id")?;
	let uri = format!("/api/cases/{case_id}/editor/pages/DG/rows/{drug_id}");

	let mut reaction_ids = Vec::new();
	for sequence_number in 1..=3 {
		let (status, body) = post_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/reactions"),
			json!({"data": {
				"case_id": case_id,
				"sequence_number": sequence_number,
				"primary_source_reaction": format!("Assessment reaction {sequence_number}")
			}}),
		)
		.await?;
		assert_eq!(status, StatusCode::CREATED, "{body}");
		reaction_ids.push(
			body["data"]["id"]
				.as_str()
				.ok_or("missing reaction id")?
				.to_string(),
		);
	}

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"drug": {
			"drugReactionAssessments": [
				{"reactionId": reaction_ids[0], "sequenceNumber": 1, "sourceOfAssessment": "Reporter"},
				{"reactionId": reaction_ids[0], "sequenceNumber": 2, "sourceOfAssessment": "Sponsor"},
				{"reactionId": reaction_ids[1], "expectedness": "1"}
			]
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let assessments = body["data"]["drug"]["drugReactionAssessments"]
		.as_array()
		.ok_or("missing assessments")?;
	let sibling_rows = assessments
		.iter()
		.filter(|row| row["reactionId"] == json!(reaction_ids[0]))
		.collect::<Vec<_>>();
	assert_eq!(sibling_rows.len(), 2, "{body}");
	let relatedness_id = sibling_rows[0]["id"]
		.as_str()
		.ok_or("missing relatedness id")?
		.to_string();
	let assessment_id = sibling_rows[0]["drugReactionAssessmentId"]
		.as_str()
		.ok_or("missing assessment id")?
		.to_string();
	assert_eq!(
		sibling_rows[1]["drugReactionAssessmentId"],
		json!(assessment_id)
	);
	let sibling_id = sibling_rows[1]["id"]
		.as_str()
		.ok_or("missing sibling relatedness id")?
		.to_string();
	assert_ne!(relatedness_id, sibling_id);
	let parent_only = assessments
		.iter()
		.find(|row| row["reactionId"] == json!(reaction_ids[1]))
		.ok_or("missing parent-only assessment")?;
	let parent_only_id = parent_only["drugReactionAssessmentId"]
		.as_str()
		.ok_or("missing parent-only assessment id")?
		.to_string();
	assert!(parent_only.get("id").is_none());

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"drug": {
			"drugReactionAssessments": [{
				"id": relatedness_id,
				"drugReactionAssessmentId": assessment_id,
				"reactionId": reaction_ids[0],
				"deleted": true
			}]
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let siblings_after_delete = body["data"]["drug"]["drugReactionAssessments"]
		.as_array()
		.ok_or("missing assessments after relatedness delete")?
		.iter()
		.filter(|row| row["reactionId"] == json!(reaction_ids[0]))
		.collect::<Vec<_>>();
	assert_eq!(siblings_after_delete.len(), 1, "{body}");
	assert_eq!(siblings_after_delete[0]["id"], json!(sibling_id), "{body}");

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"drug": {
			"drugReactionAssessments": [{
				"reactionId": reaction_ids[2],
				"deleted": true
			}]
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	assert!(body["error"]["data"]["detail"]
		.as_str()
		.unwrap_or_default()
		.contains("id for deletion"));
	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert!(body["data"]["drug"]["drugReactionAssessments"]
		.as_array()
		.ok_or("missing assessments after rejected deletion")?
		.iter()
		.all(|row| row["reactionId"] != json!(reaction_ids[2])));

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"drug": {
			"drugReactionAssessments": [{
				"drugReactionAssessmentId": parent_only_id,
				"reactionId": reaction_ids[1],
				"deleted": true,
				"_deleteAssessment": true
			}]
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert!(body["data"]["drug"]["drugReactionAssessments"]
		.as_array()
		.ok_or("missing assessments after parent delete")?
		.iter()
		.all(|row| row["reactionId"] != json!(reaction_ids[1])));
	Ok(())
}
