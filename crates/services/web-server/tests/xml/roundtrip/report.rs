use super::support::*;

#[serial]
#[tokio::test]
async fn test_roundtrip_ci_si_fields() -> Result<()> {
	let (app, cookie, case_id, _mm, _ctx) = setup_imported_case().await?;

	let sentinel_wuid = format!("RTCIW-{}", Uuid::new_v4().simple());
	let sentinel_null_reason = format!("RTCINULL-{}", Uuid::new_v4().simple());
	let sentinel_batch = format!("RTSI-BATCH-{}", Uuid::new_v4().simple());
	let sentinel_batch_sender = format!("RTSI-BS-{}", Uuid::new_v4().simple());
	let sentinel_batch_receiver = format!("RTSI-BR-{}", Uuid::new_v4().simple());
	let sentinel_msg_number = format!("RTSI-MSG-{}", Uuid::new_v4().simple());
	let sentinel_msg_sender = format!("RTSI-MS-{}", Uuid::new_v4().simple());
	let sentinel_msg_receiver = format!("RTSI-MR-{}", Uuid::new_v4().simple());
	let sentinel_study_type_reaction = "1";

	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/safety-report"),
		Some(serde_json::json!({
			"data": {
				"transmission_date": "2024-06-01",
				"report_type": "2",
				"date_first_received_from_source": "2024-05-01",
				"date_of_most_recent_information": "2024-06-01",
				"fulfil_expedited_criteria": true,
				"local_criteria_report_type": "1",
				"combination_product_report_indicator": "2",
				"worldwide_unique_id": sentinel_wuid,
				"nullification_reason": sentinel_null_reason
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update safety-report status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/message-header"),
		Some(serde_json::json!({
			"data": {
				"batch_number": sentinel_batch,
				"batch_sender_identifier": sentinel_batch_sender,
				"batch_receiver_identifier": sentinel_batch_receiver,
				"batch_transmission_date": [2024, 32, 1, 1, 1, 0, 0, 0, 0],
				"message_number": sentinel_msg_number,
				"message_sender_identifier": sentinel_msg_sender,
				"message_receiver_identifier": sentinel_msg_receiver,
				"message_date": "20240601112233"
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update message-header status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/safety-report/studies"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list studies status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let study_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.map(|v| v.to_string());
	let study_id = if let Some(study_id) = study_id {
		study_id
	} else {
		let (status, body) = request_json(
			&app,
			&cookie,
			"POST",
			format!("/api/cases/{case_id}/safety-report/studies"),
			Some(serde_json::json!({
				"data": {
					"case_id": case_id,
					"study_name": "CI/SI Roundtrip Study"
				}
			})),
		)
		.await?;
		if status != StatusCode::CREATED {
			return Err(format!(
				"create study-information status {} body {}",
				status,
				String::from_utf8_lossy(&body)
			)
			.into());
		}
		extract_data_id(&body)?
	};
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/safety-report/studies/{study_id}"),
		Some(serde_json::json!({
			"data": {
				"study_type_reaction": sentinel_study_type_reaction
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update study-information status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	let xml = export_xml(&app, &cookie, &case_id).await?;
	for expected in [
		sentinel_wuid.as_str(),
		sentinel_null_reason.as_str(),
		sentinel_batch.as_str(),
		sentinel_batch_sender.as_str(),
		sentinel_batch_receiver.as_str(),
		sentinel_msg_sender.as_str(),
		sentinel_msg_receiver.as_str(),
	] {
		assert!(
			xml.contains(expected),
			"expected CI/SI export XML to contain `{expected}`"
		);
	}
	// N.2.r.1/N.2.r.4 derive from C.1.1/C.1.2, not legacy message-header values.
	assert!(!xml.contains(&sentinel_msg_number));
	assert!(!xml.contains("20240601112233"));
	let expected_study_type = format!(
		"<code code=\"{sentinel_study_type_reaction}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.8\"/>"
	);
	assert!(
		xml.contains(&expected_study_type),
		"expected CI/SI export XML to contain `{expected_study_type}`"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_roundtrip_rp_sd_fields() -> Result<()> {
	let (app, cookie, case_id, _mm, _ctx) = setup_imported_case().await?;

	let sentinel_rp_given = format!("RTRP-G-{}", Uuid::new_v4().simple());
	let sentinel_rp_family = format!("RTRP-F-{}", Uuid::new_v4().simple());
	let sentinel_rp_org = format!("RTRP-O-{}", Uuid::new_v4().simple());
	let sentinel_sd_org = format!("RTSD-O-{}", Uuid::new_v4().simple());
	let sentinel_sd_given = format!("RTSD-G-{}", Uuid::new_v4().simple());
	let sentinel_sd_email = format!("{}@example.com", Uuid::new_v4().simple());

	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/safety-report/primary-sources"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list primary-sources status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let primary_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing primary source id")?
		.to_string();

	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/safety-report/primary-sources/{primary_id}"),
		Some(serde_json::json!({
			"data": {
				"reporter_given_name": sentinel_rp_given,
				"reporter_family_name": sentinel_rp_family,
				"organization": sentinel_rp_org,
				"street": "RP Street 11",
				"city": "RP City",
				"state": "RP State",
				"postcode": "12345",
				"telephone": "1234567890",
				"email": "rp@example.com",
				"country_code": "KR",
				"qualification": "1",
				"primary_source_regulatory": "1"
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update primary-source status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/safety-report/senders"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list senders status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let sender_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing sender id")?
		.to_string();
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/safety-report/senders/{sender_id}"),
		Some(serde_json::json!({
			"data": {
				"sender_type": "2",
				"organization_name": sentinel_sd_org,
				"person_given_name": sentinel_sd_given,
				"person_family_name": "SenderFamily",
				"street_address": "Sender Street 22",
				"city": "Sender City",
				"state": "Sender State",
				"postcode": "54321",
				"country_code": "US",
				"telephone": "3334445555",
				"email": sentinel_sd_email
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update sender status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	let xml = export_xml(&app, &cookie, &case_id).await?;
	for expected in [
		sentinel_rp_given.as_str(),
		sentinel_rp_family.as_str(),
		sentinel_rp_org.as_str(),
		sentinel_sd_org.as_str(),
		sentinel_sd_given.as_str(),
		sentinel_sd_email.as_str(),
	] {
		assert!(
			xml.contains(expected),
			"expected RP/SD export XML to contain `{expected}`"
		);
	}
	Ok(())
}
