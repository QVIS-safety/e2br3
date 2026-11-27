use super::support::*;

#[serial]
#[tokio::test]
async fn test_manual_export_allows_unlinked_sender_presave() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm).expect("web server configuration");
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
	let parsed = xml::validation::validate_e2b_xml_basic(&body, None)?;
	assert!(parsed.ok, "{:?}", parsed.errors);
	assert!(
		String::from_utf8_lossy(&body).contains("Test Sender"),
		"manually entered C.3 Sender information must survive export"
	);
	assert!(!xml::validation::validate_export_message_header(
		&body,
		lib_core::regulatory::RegulatoryAuthority::Ich,
	)?
	.is_empty());

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_from_intake_creates_batch_number_for_export() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone()).expect("web server configuration");
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

	create_narrative(&app, &cookie, &case_id).await?;
	let (status, xml) = get_bytes(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/export/xml?authority=fda"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&xml));
	let (status, header_body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/message-header"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{header_body:?}");
	assert_eq!(
		header_body["data"]["message_sender_identifier"],
		"TEST-SENDER"
	);
	assert_eq!(
		header_body["data"]["message_receiver_identifier"],
		"CDER_IND"
	);
	let expected_batch_number = format!("BATCH-{case_id}");
	assert_eq!(
		header_body["data"]["batch_number"].as_str(),
		Some(expected_batch_number.as_str())
	);

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
	let app = web_server::app(mm).expect("web server configuration");

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

#[serial]
#[tokio::test]
async fn test_intake_without_transport_identifiers_can_create_draft() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm).expect("web server configuration");
	for authority in ["ich", "fda", "mfds"] {
		let response = post_json(
			&app,
			&cookie,
			"/api/cases/from-intake",
			json!({
				"data": intake_data(&format!("INTAKE-{}", Uuid::new_v4()), 126, "1", json!({
					"authority": authority, "fda_report_type": "1", "mfds_report_type": "1"
				}))
			}),
		)
		.await;
		let (status, body) = response?;
		assert_eq!(status, StatusCode::CREATED, "{authority}: {body}");
		let case_id = extract_case_id(&body)?;
		let (status, body) =
			get_json(&app, &cookie, &format!("/api/cases/{case_id}")).await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		assert_eq!(body["data"]["status"], "draft", "{body}");
		let (status, header) = get_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/message-header"),
		)
		.await?;
		assert_eq!(status, StatusCode::NOT_FOUND, "{header}");
		let (status, follow_up) = post_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/follow-up"),
			follow_up_pages(authority),
		)
		.await?;
		assert_eq!(status, StatusCode::CREATED, "{follow_up}");
		let follow_up_id = follow_up["data"]["caseId"]
			.as_str()
			.ok_or("follow-up case id")?;
		let (status, header) = get_json(
			&app,
			&cookie,
			&format!("/api/cases/{follow_up_id}/message-header"),
		)
		.await?;
		assert_eq!(status, StatusCode::NOT_FOUND, "{header}");
	}
	Ok(())
}
