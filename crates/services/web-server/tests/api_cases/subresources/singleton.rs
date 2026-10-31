use super::support::*;

#[serial]
#[tokio::test]
async fn test_singleton_post_endpoints_are_idempotent() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	for (resource, expected) in [
		("patient", StatusCode::NOT_FOUND),
		("message-header", StatusCode::NOT_FOUND),
		("receiver", StatusCode::OK),
		("narrative", StatusCode::OK),
	] {
		let (status, body) =
			get_json(&app, &cookie, format!("/api/cases/{case_id}/{resource}"))
				.await?;
		assert_eq!(
			status,
			expected,
			"{resource}: {}",
			String::from_utf8_lossy(&body)
		);
		if expected == StatusCode::OK {
			assert!(serde_json::from_slice::<Value>(&body)?["data"].is_null());
		}
	}
	let msg_a = format!("MSG-A-{case_id}");
	let msg_b = format!("MSG-B-{case_id}");

	// message header
	let body = json!({"data": {
		"case_id": case_id,
		"batch_transmission_date": [2024, 32, 1, 1, 1, 0, 0, 0, 0],
		"message_number": msg_a,
		"message_sender_identifier": "SEND-A",
		"message_receiver_identifier": "RECV-A",
		"message_date": "20240201010101"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/message-header"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let header_id = extract_id(&body)?;
	let body = json!({"data": {
		"case_id": case_id,
		"message_number": msg_b,
		"message_sender_identifier": "SEND-B",
		"message_receiver_identifier": "RECV-B",
		"message_date": "20240202020202"
	}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/message-header"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/message-header"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	let value: Value = serde_json::from_slice(&body)?;
	assert!(value["data"]["id"].as_str().is_some(), "{value:?}");
	assert_eq!(value["data"]["batch_transmission_date"][0], 2024);
	assert_eq!(value["data"]["id"], header_id.to_string());
	assert_eq!(value["data"]["message_number"], msg_a);

	// patient
	let patient_id = create_patient(&app, &cookie, case_id).await?;
	let body = json!({"data": {
		"case_id": case_id,
		"patient_initials": "CD",
		"sex": "2"
	}});
	let (status, _) =
		post_json(&app, &cookie, format!("/api/cases/{case_id}/patient"), body)
			.await?;
	assert_eq!(status, StatusCode::OK);
	let (status, body) =
		get_json(&app, &cookie, format!("/api/cases/{case_id}/patient")).await?;
	assert_eq!(status, StatusCode::OK);
	let value: Value = serde_json::from_slice(&body)?;
	assert!(value["data"]["id"].as_str().is_some(), "{value:?}");

	assert_eq!(value["data"]["id"], patient_id.to_string());
	assert_eq!(value["data"]["patient_initials"], "AB");

	// receiver
	let receiver_id = create_receiver(&app, &cookie, case_id).await?;
	let body = json!({"data": {
		"case_id": case_id,
		"receiver_type": "2",
		"organization_name": "Receiver Org B"
	}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/receiver"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	let (status, body) =
		get_json(&app, &cookie, format!("/api/cases/{case_id}/receiver")).await?;
	assert_eq!(status, StatusCode::OK);
	let value: Value = serde_json::from_slice(&body)?;
	assert!(value["data"]["id"].as_str().is_some(), "{value:?}");

	assert_eq!(value["data"]["id"], receiver_id.to_string());
	assert_eq!(value["data"]["organization_name"], "Receiver Org A");

	// narrative
	let narrative_id = create_narrative(&app, &cookie, case_id).await?;
	let body = json!({"data": {
		"case_id": case_id,
		"case_narrative": "updated narrative",
		"additional_information": "updated sponsor information"
	}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	let (_, original) =
		get_json(&app, &cookie, format!("/api/cases/{case_id}/narrative")).await?;
	let original: Value = serde_json::from_slice(&original)?;
	assert_eq!(original["data"]["id"], narrative_id.to_string());
	assert_eq!(original["data"]["case_narrative"], "test narrative");
	let body = json!({"data": {
		"case_narrative": "updated narrative",
		"additional_information": "updated sponsor information"
	}});
	let (status, _) = put_json_with_audit_reason(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative"),
		body,
		"update narrative sponsor information",
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	let (status, body) =
		get_json(&app, &cookie, format!("/api/cases/{case_id}/narrative")).await?;
	assert_eq!(status, StatusCode::OK);
	let value: Value = serde_json::from_slice(&body)?;
	assert!(value["data"]["id"].as_str().is_some(), "{value:?}");
	assert_eq!(
		value["data"]["additional_information"],
		"updated sponsor information"
	);

	// safety report
	let safety_report_id = create_safety_report(&app, &cookie, case_id).await?;
	let body = json!({"data": {
		"case_id": case_id,
		"transmission_date": "20250101000000",
		"report_type": "2",
		"date_first_received_from_source": "20250101",
		"date_of_most_recent_information": "20250101",
		"fulfil_expedited_criteria": true
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["id"], safety_report_id.to_string());
	let (status, body) =
		get_json(&app, &cookie, format!("/api/cases/{case_id}/safety-report"))
			.await?;
	assert_eq!(status, StatusCode::OK);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["id"], safety_report_id.to_string());

	Ok(())
}
