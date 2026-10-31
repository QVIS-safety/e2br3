use super::support::*;

#[serial]
#[tokio::test]
async fn test_import_update_dg_fields_then_export_contains_updates() -> Result<()> {
	init_test_env().await?;
	let examples_dir = resolve_from_workspace(std::path::PathBuf::from(
		std::env::var("E2BR3_EXAMPLES_DIR")?,
	));
	std::env::set_var("E2BR3_XSD_PATH", resolved_xsd_path());

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "admin_pwd", "viewer_pwd").await?;
	let product_presave_id =
		create_test_import_product(&mm, seed.org_id, seed.admin.id).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let xml_path = examples_dir.join("FAERS2022Scenario2.xml");
	let original_xml = std::fs::read_to_string(&xml_path)?;
	let xml = unique_safety_report_id_xml(original_xml.clone());
	if xml == original_xml {
		return Err(format!(
			"fixture has no safety report ID to make unique: {}",
			xml_path.display()
		)
		.into());
	}
	let boundary = "X-BOUNDARY-XML-IMPORT-DG";
	let body = format!(
		"--{boundary}\r\nContent-Disposition: form-data; name=\"format\"\r\n\r\nfda\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"productPresaveId\"\r\n\r\n{product_presave_id}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"case.xml\"\r\nContent-Type: application/xml\r\n\r\n{xml}\r\n--{boundary}--\r\n"
	);
	let req = Request::builder()
		.method("POST")
		.uri("/api/import/xml")
		.header(
			"content-type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header("cookie", cookie.clone())
		.body(Body::from(body))?;
	let res = app.clone().oneshot(req).await?;
	let import_status = res.status();
	let import_body = to_bytes(res.into_body(), usize::MAX).await?;
	if import_status != StatusCode::OK {
		return Err(format!(
			"import status {} body {}",
			import_status,
			String::from_utf8_lossy(&import_body)
		)
		.into());
	}
	let import_value: Value = serde_json::from_slice(&import_body)?;
	let case_id = import_value
		.get("data")
		.and_then(|v| v.get("caseId"))
		.and_then(|v| v.as_str())
		.ok_or("missing case_id in import response")?
		.to_string();
	prepare_test_fda_export_case(
		&mm,
		seed.org_id,
		seed.admin.id,
		Uuid::parse_str(&case_id)?,
	)
	.await?;

	// Resolve primary drug/reaction IDs from imported case.
	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/drugs"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list drugs status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let drugs_value: Value = serde_json::from_slice(&body)?;
	let drug_id = drugs_value
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing first drug id")?
		.to_string();

	// DG sentinels for assertions.
	let sentinel_indication = format!("RTDG15-{}", Uuid::new_v4().simple());
	let sentinel_substance = format!("RTDG21-{}", Uuid::new_v4().simple());
	let sentinel_batch = format!("RTDG32-{}", Uuid::new_v4().simple());

	// Upsert active substance row #1.
	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list active-substances status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let substance_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing active substance id")?
		.to_string();
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!(
			"/api/cases/{case_id}/drugs/{drug_id}/active-substances/{substance_id}"
		),
		Some(serde_json::json!({
			"data": {
				"substance_name": sentinel_substance,
				"strength_value": 5,
				"strength_unit": "mg"
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update active-substance status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	// Upsert dosage row #1.
	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/drugs/{drug_id}/dosages"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list dosages status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let dosage_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing dosage id")?
		.to_string();
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/drugs/{drug_id}/dosages/{dosage_id}"),
		Some(serde_json::json!({
			"data": {
				"dose_value": 10.5,
				"dose_unit": "mg",
				"first_administration_date": "2024-01-02",
				"last_administration_date": "2024-01-02",
				"duration_value": 4,
				"duration_unit": "801",
				"batch_lot_number": sentinel_batch
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update dosage status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	// Upsert indication row #1.
	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/drugs/{drug_id}/indications"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list indications status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let indication_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing indication id")?
		.to_string();
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/drugs/{drug_id}/indications/{indication_id}"),
		Some(serde_json::json!({
			"data": {
				"indication_text": sentinel_indication,
				"indication_meddra_version": "25.0",
				"indication_meddra_code": "10019211"
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update indication status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	ensure_reaction_language(&app, &cookie, &case_id).await?;
	ensure_batch_transmission_date(&app, &cookie, &case_id).await?;

	// Export and assert updated DG values are present in XML.
	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{case_id}/export/xml?authority=fda"))
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"export status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let xml = String::from_utf8_lossy(&body);
	for expected in [
		"Roger",
		sentinel_indication.as_str(),
		sentinel_substance.as_str(),
		"mg",
		"20240102",
		"unit=\"a\"",
		sentinel_batch.as_str(),
	] {
		assert!(
			xml.contains(expected),
			"expected DG export XML to contain `{expected}`"
		);
	}
	assert!(
		xml.contains(drug_id.as_str()),
		"expected DG export XML to contain current drug id root"
	);
	assert!(
		!xml.contains("68d6f5ce-3b3b-45c7-92dd-69e06730c3a9"),
		"expected DG export XML to exclude stale template product root ids"
	);
	for stale in [
		"68d6f5ce-3b3b-45c7-92dd-69e06730c3a9",
		"59d6f5ce-3b3b-45c7-92dd-69e06730c2b7",
		"40d6f5ce-3b3b-45c7-92dd-69e06730c2b6",
		"154eb889-958b-45f2-a02f-42d4d6f4657f",
		"154eb889-958b-45f2-a02f-42d4d6f4555f",
	] {
		assert!(
			!xml.contains(stale),
			"expected DG export XML to exclude stale template root id `{stale}`"
		);
	}

	Ok(())
}
