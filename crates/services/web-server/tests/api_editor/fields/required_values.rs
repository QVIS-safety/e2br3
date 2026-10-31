use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, patch_json, post_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::json;

pub(crate) async fn report_year_rejects_non_numeric_input(
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
		json!({"authorities":["ich"],"rows":{"case":{"reportYear":"abcd"}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");

	Ok(())
}

pub(crate) async fn blank_required_drug_values_preserve_children(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
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

	Ok(())
}

pub(crate) async fn narrative_blank_rejection_preserves_saved_comments(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
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

pub(crate) async fn blank_safety_report_id_is_rejected(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
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
