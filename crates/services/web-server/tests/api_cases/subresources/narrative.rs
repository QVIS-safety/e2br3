use super::support::*;

#[serial]
#[tokio::test]
async fn test_narrative_rejects_legacy_additional_fields() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let body = json!({"data": {
		"case_id": case_id,
		"case_narrative": "test narrative",
		"case_summary": "legacy additional narrative field"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::UNPROCESSABLE_ENTITY,
		"status={status} body={}",
		String::from_utf8_lossy(&body)
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_narrative_subresources_endpoints_ok() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let narrative_id = create_narrative(&app, &cookie, case_id).await?;

	let body = json!({"data": {"narrative_id": narrative_id, "sequence_number": 1, "diagnosis_meddra_version": "27.1", "diagnosis_meddra_code": "100"}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/sender-diagnoses"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["diagnosis_meddra_version"], "27.1");
	assert_eq!(value["data"]["diagnosis_meddra_code"], "100");

	let body = json!({"data": {"narrative_id": narrative_id, "sequence_number": 1, "language_code": "eng", "summary_text": "summary"}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/summaries"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["language_code"], "eng");
	assert_eq!(value["data"]["summary_text"], "summary");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_narrative_child_lists_return_empty_when_parent_narrative_missing(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;

	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/sender-diagnoses"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"].as_array().map(|rows| rows.len()), Some(0));

	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/summaries"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"].as_array().map(|rows| rows.len()), Some(0));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sender_diagnosis_soft_delete_restore() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let narrative_id = create_narrative(&app, &cookie, case_id).await?;

	let body = json!({
		"data": {
			"narrative_id": narrative_id,
			"sequence_number": 1,
			"diagnosis_meddra_version": "27.1",
			"diagnosis_meddra_code": "100"
		}
	});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/sender-diagnoses"),
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
	let row_id = value["data"]["id"].as_str().expect("sender diagnosis id");
	assert_eq!(value["data"]["deleted"], false);

	let (status, body) = delete_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/sender-diagnoses/{row_id}"),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::NO_CONTENT,
		"{}",
		String::from_utf8_lossy(&body)
	);

	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/sender-diagnoses"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"].as_array().map(|rows| rows.len()), Some(0));

	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/sender-diagnoses/{row_id}/restore"),
		json!({}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["deleted"], false);

	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/sender-diagnoses"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"].as_array().map(|rows| rows.len()), Some(1));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_narrative_preview_resolves_patient_tokens() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_patient_with_narrative_preview_values(&app, &cookie, case_id).await?;

	let body = json!({
		"template": "{D.2.2a}세의 {D.5} 환자 {UNKNOWN.CODE}"
	});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/preview"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["rendered"], "30세의 여성 환자 {UNKNOWN.CODE}");
	assert_eq!(value["data"]["tokens"][0]["code"], "D.2.2a");
	assert_eq!(value["data"]["tokens"][0]["resolved"], true);
	assert_eq!(value["data"]["tokens"][1]["code"], "D.5");
	assert_eq!(value["data"]["tokens"][1]["resolved"], true);
	assert_eq!(value["data"]["tokens"][2]["code"], "UNKNOWN.CODE");
	assert_eq!(value["data"]["tokens"][2]["resolved"], false);

	Ok(())
}
