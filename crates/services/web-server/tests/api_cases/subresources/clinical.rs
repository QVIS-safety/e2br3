use super::support::*;

#[serial]
#[tokio::test]
async fn test_reaction_supports_ae_common_and_mfds_device_fields() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;

	let body = json!({"data": {
		"case_id": case_id,
		"sequence_number": 1,
		"primary_source_reaction": "Device site pain",
		"expectedness": "2",
		"severity": "severe",
		"mfds_device_ae_classification": "0",
		"mfds_device_ae_outcome": "10",
		"mfds_device_cause_medical_device": true,
		"mfds_device_cause_procedure_issue": false,
		"mfds_device_cause_patient_condition": true,
		"mfds_device_cause_unable_to_assess": false,
		"mfds_device_cause_other": "Other device cause",
		"mfds_device_action_reason": "Action reason text",
		"mfds_device_action_recall": true,
		"mfds_device_action_repair": true,
		"mfds_device_action_inspection": false,
		"mfds_device_action_replacement": true,
		"mfds_device_action_improvement": false,
		"mfds_device_action_monitoring": true,
		"mfds_device_action_notification": true,
		"mfds_device_action_label_change": false,
		"mfds_device_action_other": "Other action text"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/reactions"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let created: Value = serde_json::from_slice(&body)?;
	let reaction_id = created["data"]["id"]
		.as_str()
		.ok_or("missing reaction id")?;
	assert_eq!(created["data"]["expectedness"], "2");
	assert_eq!(created["data"]["mfds_device_ae_outcome"], "10");
	assert_eq!(
		created["data"]["mfds_device_action_reason"],
		"Action reason text"
	);

	let body = json!({"data": {
		"expectedness": "1",
		"severity": "moderate",
		"mfds_device_ae_classification": "1",
		"mfds_device_action_other": "Updated other action"
	}});
	let (status, body) = put_json_with_audit_reason(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/reactions/{reaction_id}"),
		body,
		"update AE common and MFDS device fields",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let updated: Value = serde_json::from_slice(&body)?;
	assert_eq!(updated["data"]["expectedness"], "1");
	assert_eq!(updated["data"]["severity"], "moderate");
	assert_eq!(updated["data"]["mfds_device_ae_classification"], "1");
	assert_eq!(
		updated["data"]["mfds_device_action_other"],
		"Updated other action"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_test_results_support_50_character_normal_ranges() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let low = "L".repeat(50);
	let high = "H".repeat(50);

	let body = json!({"data": {
		"case_id": case_id,
		"sequence_number": 1,
		"test_name": "ALT",
		"normal_low_value": low,
		"normal_high_value": high
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/test-results"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["normal_low_value"], low);
	assert_eq!(value["data"]["normal_high_value"], high);
	let test_result_id = value["data"]["id"]
		.as_str()
		.ok_or("missing test result id")?;

	let updated_low = "A".repeat(50);
	let updated_high = "B".repeat(50);
	let body = json!({"data": {
		"normal_low_value": updated_low,
		"normal_high_value": updated_high
	}});
	let (status, body) = put_json_with_audit_reason(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/test-results/{test_result_id}"),
		body,
		"test normal range widening",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["normal_low_value"], updated_low);
	assert_eq!(value["data"]["normal_high_value"], updated_high);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_relatedness_assessment_endpoints_ok() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let drug_id = create_drug(&app, &cookie, case_id).await?;
	let reaction_id = create_reaction(&app, &cookie, case_id).await?;
	let assessment_id =
		create_reaction_assessment(&app, &cookie, case_id, drug_id, reaction_id)
			.await?;

	let body = json!({"data": {
		"drug_reaction_assessment_id": assessment_id,
		"sequence_number": 1,
		"source_of_assessment": "Sponsor",
		"method_of_assessment": "Algorithm",
		"result_of_assessment": "1",
		"result_of_assessment_kr2": "KR result"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!(
			"/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness"
		),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(
		value["data"]["drug_reaction_assessment_id"],
		assessment_id.to_string()
	);
	assert_eq!(value["data"]["source_of_assessment"], "Sponsor");
	assert_eq!(value["data"]["method_of_assessment"], "Algorithm");
	assert_eq!(value["data"]["result_of_assessment"], "1");
	assert_eq!(value["data"]["result_of_assessment_kr2"], "KR result");

	Ok(())
}
