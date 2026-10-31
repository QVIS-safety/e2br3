use super::support::*;

#[serial]
#[tokio::test]
async fn test_ich_export_without_required_narrative_still_downloads() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let safety_report_id = format!("INTAKE-NO-NARRATIVE-{}", Uuid::new_v4());
	let intake_body = json!({
		"data": intake_data(&safety_report_id, 121, "1", json!({ "authority": "ich" }))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", intake_body).await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let case_id = extract_case_id(&body)?;
	create_unlinked_sender(&app, &cookie, &case_id).await?;

	let (status, body) = get_bytes(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/export/xml?authority=ich"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	assert!(String::from_utf8(body)?.contains("<MCCI_IN200100UV01"));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_from_intake_creates_batch_number_for_export() -> Result<()> {
	std::env::set_var("E2BR3_EXPORT_VALIDATE", "0");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let safety_report_id = format!("INTAKE-BATCH-{}", Uuid::new_v4());
	let intake_body = json!({
		"data": intake_data(&safety_report_id, 124, "1", json!({
			"authority": "fda",
			"fda_report_type": "1"
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", intake_body).await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let case_id = extract_case_id(&body)?;
	create_sender(&app, &cookie, &case_id, "fda").await?;
	set_reaction_outcome(&app, &cookie, &case_id).await?;
	create_suspect_drug(&app, &cookie, &case_id).await?;

	let (status, header_body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/message-header"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{header_body:?}");
	let expected_batch_number = format!("BATCH-{case_id}");
	assert_eq!(
		header_body["data"]["batch_number"].as_str(),
		Some(expected_batch_number.as_str())
	);
	create_narrative(&app, &cookie, &case_id).await?;

	let update_body = json!({
		"data": {
			"batch_receiver_identifier": "FDA"
		}
	});
	let (status, update_response) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/message-header"),
		update_body,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{update_response:?}");
	let (status, workflow_response) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/review/toggle"),
		json!({"data":{"expected_status":"draft"}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{workflow_response:?}");

	let (status, xml) = get_bytes(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/export/xml?authority=fda"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&xml));
	let xml = String::from_utf8(xml)?;
	assert!(
		xml.contains(&format!("extension=\"{expected_batch_number}\"")),
		"N.1.2 batch number missing from export: {xml}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_from_intake_persists_distinct_c_1_dates() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", Uuid::new_v4());
	let intake_body = json!({
		"data": intake_data(&safety_report_id, 123, "1", json!({
			"transmission_date": "2024-04-30",
			"date_first_received_from_source": "20240501",
			"date_of_most_recent_information": "20240502"
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", intake_body).await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let case_id = extract_case_id(&body)?;

	let (status, value) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/safety-report"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["data"]["transmission_date"], "20240430000000");
	assert_eq!(value["data"]["date_first_received_from_source"], "20240501");
	assert_eq!(value["data"]["date_of_most_recent_information"], "20240502");

	Ok(())
}
