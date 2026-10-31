use super::support::*;
use crate::validation_common::post_json as post_value;

#[serial]
#[tokio::test]
async fn safety_report_preserves_c1_boolean_null_flavors() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, seed.org_id).await?;

	let body = json!({
		"data": {
			"case_id": case_id,
			"safety_report_id": format!("SR-NF-{}", Uuid::new_v4()),
			"transmission_date": "20260630010101",
			"report_type": "1",
			"date_first_received_from_source": "20260630",
			"date_of_most_recent_information": "20260630",
			"fulfil_expedited_criteria": null,
			"fulfil_expedited_criteria_null_flavor": "NI",
			"other_case_identifiers_exist": null,
			"other_case_identifiers_exist_null_flavor": "NI"
		}
	});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);

	let value: Value = serde_json::from_slice(&body)?;
	assert!(value["data"]["fulfil_expedited_criteria"].is_null());
	assert_eq!(value["data"]["fulfil_expedited_criteria_null_flavor"], "NI");
	assert!(value["data"]["other_case_identifiers_exist"].is_null());
	assert_eq!(
		value["data"]["other_case_identifiers_exist_null_flavor"],
		"NI"
	);

	let update = json!({
		"data": {
			"fulfil_expedited_criteria": true,
			"other_case_identifiers_exist": true
		}
	});
	let (status, body) = put_json_with_audit_reason(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report"),
		update,
		"set boolean values",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["fulfil_expedited_criteria"], true);
	assert!(value["data"]["fulfil_expedited_criteria_null_flavor"].is_null());
	assert_eq!(value["data"]["other_case_identifiers_exist"], true);
	assert!(value["data"]["other_case_identifiers_exist_null_flavor"].is_null());

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_safety_report_subresources_endpoints_ok() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let body = json!({"data": {"case_id": case_id, "sender_type": "1", "organization_name": "Org"}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/senders"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);

	let body = json!({"data": {"case_id": case_id, "sequence_number": 1, "qualification": "1"}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/primary-sources"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);

	let body = json!({"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"reference_text": null,
			"reference_text_null_flavor": "ASKU",
			"document_base64": "cGRm",
			"media_type": "application/pdf",
			"representation": "B64",
			"compression": "DF"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/literature"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["reference_text"], Value::Null);
	assert_eq!(value["data"]["reference_text_null_flavor"], "ASKU");
	assert_eq!(value["data"]["document_base64"], "cGRm");
	assert_eq!(value["data"]["media_type"], "application/pdf");
	assert_eq!(value["data"]["representation"], "B64");
	assert_eq!(value["data"]["compression"], "DF");

	let body = json!({"data": {
		"case_id": case_id,
		"study_name": null,
		"study_name_null_flavor": "NASK",
		"sponsor_study_number": null,
		"sponsor_study_number_null_flavor": "ASKU",
		"fda_ind_number_occurred": "1234567890",
		"fda_pre_anda_number_occurred": "9876543210"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/studies"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["study_name_null_flavor"], "NASK");
	assert_eq!(value["data"]["sponsor_study_number_null_flavor"], "ASKU");
	assert_eq!(value["data"]["fda_ind_number_occurred"], "1234567890");
	assert_eq!(value["data"]["fda_pre_anda_number_occurred"], "9876543210");
	let study_id = extract_id(&body)?;

	let body = json!({"data": {
		"study_information_id": study_id,
		"registration_number": "REG-1",
		"country_code": "KR",
		"sequence_number": 1
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!(
			"/api/cases/{case_id}/safety-report/studies/{study_id}/registrations"
		),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);

	let body = json!({"data": {"study_information_id": study_id, "ind_number": "IND-123", "sequence_number": 1}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!(
			"/api/cases/{case_id}/safety-report/studies/{study_id}/fda-cross-reported-inds"
		),
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
	assert_eq!(value["data"]["ind_number"], "IND-123");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_other_case_identifier_soft_delete_restore() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;

	let body = json!({"data": {
		"case_id": case_id,
		"sequence_number": 1,
		"source_of_identifier": "Regulator",
		"case_identifier": "REG-CASE-1"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/other-identifiers"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let row_id = extract_id(&body)?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["deleted"], false);

	let (status, body) = delete_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/other-identifiers/{row_id}"),
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
		format!("/api/cases/{case_id}/other-identifiers"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	let rows = value["data"].as_array().ok_or("data is not an array")?;
	assert!(
		rows.iter()
			.all(|row| row["id"].as_str() != Some(&row_id.to_string())),
		"soft-deleted row should be excluded from default list"
	);

	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/other-identifiers/{row_id}/restore"),
		json!({}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["id"], row_id.to_string());
	assert_eq!(value["data"]["deleted"], false);

	Ok(())
}

#[serial]
#[tokio::test]
async fn safety_report_rejects_c1_date_null_flavor_fields() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (create_status, create_body) = post_value(
		&app,
		&cookie,
		"/api/cases".to_string(),
		json!({
			"data": {
				"safetyReportIdentification": {
					"safetyReportId": format!("SR-C1-NF-{}", Uuid::new_v4())
				},
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(create_status, StatusCode::CREATED, "{create_body:?}");
	let case_id = create_body["data"]["id"]
		.as_str()
		.ok_or("missing created case id")?
		.to_string();

	let request = Request::builder()
		.method("PUT")
		.uri(format!("/api/cases/{case_id}/safety-report"))
		.header("cookie", &cookie)
		.header("content-type", "application/json")
		.body(Body::from(
			json!({
				"data": {
					"transmission_date_null_flavor": "NI",
					"date_first_received_from_source_null_flavor": "NI",
					"date_of_most_recent_information_null_flavor": "NI"
				}
			})
			.to_string(),
		))?;
	let response = app.clone().oneshot(request).await?;
	let update_status = response.status();
	let update_body = to_bytes(response.into_body(), usize::MAX).await?;

	assert_eq!(
		update_status,
		StatusCode::UNPROCESSABLE_ENTITY,
		"{}",
		String::from_utf8_lossy(&update_body)
	);
	let body_text = String::from_utf8_lossy(&update_body);
	assert!(
		body_text.contains("transmission_date_null_flavor")
			|| body_text.contains("unknown field"),
		"{}",
		body_text
	);

	Ok(())
}
