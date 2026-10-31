use super::support::*;

#[serial]
#[tokio::test]
async fn test_import_then_export_xml() -> Result<()> {
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
	let mut xml = std::fs::read_to_string(xml_path)?;
	let unique_safety_report_id = format!("DSJP-TEST-{}", uuid::Uuid::new_v4());
	let marker = "extension=\"US-APHARMA-8744554B-UPDATE-TESTING222\"";
	if xml.contains(marker) {
		xml =
			xml.replace(marker, &format!("extension=\"{unique_safety_report_id}\""));
	} else {
		return Err("failed to locate safety_report_id marker in example XML".into());
	}

	let boundary = "X-BOUNDARY-XML-IMPORT";
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
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"import status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let case_id = value
		.get("data")
		.and_then(|v| v.get("caseId"))
		.and_then(|v| v.as_str())
		.ok_or("missing case_id in import response")?;
	prepare_test_fda_export_case(
		&mm,
		seed.org_id,
		seed.admin.id,
		Uuid::parse_str(case_id)?,
	)
	.await?;

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{case_id}/message-header"))
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"message header status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let message_number = value
		.get("data")
		.and_then(|v| v.get("message_number"))
		.and_then(|v| v.as_str())
		.unwrap_or_default();
	assert!(
		!message_number.is_empty(),
		"imported message header should include message_number"
	);

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{case_id}/patient"))
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"patient status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let patient_initials = value
		.get("data")
		.and_then(|v| v.get("patient_initials"))
		.and_then(|v| v.as_str())
		.unwrap_or_default();
	assert!(
		!patient_initials.is_empty(),
		"imported patient should include patient_initials"
	);

	ensure_reaction_language(&app, &cookie, case_id).await?;
	ensure_batch_transmission_date(&app, &cookie, case_id).await?;

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{case_id}/export/xml?authority=fda"))
		.header("cookie", cookie.clone())
		.body(Body::empty())?;

	let res = app.clone().oneshot(req).await?;
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
	assert!(xml.contains("<MCCI_IN200100UV01"));
	assert!(xml.contains(&unique_safety_report_id));

	let second_import = import_xml_string(
		&app,
		&cookie,
		"second.xml",
		&fixture_xml("FAERS2022Scenario2.xml")?,
		product_presave_id,
	)
	.await?;
	let second_id = import_case_id(&second_import)?;
	prepare_test_fda_export_case(
		&mm,
		seed.org_id,
		seed.admin.id,
		Uuid::parse_str(second_id)?,
	)
	.await?;
	ensure_reaction_language(&app, &cookie, second_id).await?;
	ensure_batch_transmission_date(&app, &cookie, second_id).await?;
	for authority in ["fda", "mfds"] {
		for notation in [true, false] {
			let (status, body) = request_json(
				&app,
				&cookie,
				"PUT",
				"/api/admin/settings".to_string(),
				Some(serde_json::json!({"data": {"notation": notation}})),
			)
			.await?;
			assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
			assert_eq!(
				serde_json::from_slice::<Value>(&body)?["notation"],
				notation
			);
			let (status, body) = request_json(
				&app,
				&cookie,
				"GET",
				format!(
				"/api/cases/{case_id}/export/xml?authority={authority}&include_notation={}",
				!notation
			),
				None,
			)
			.await?;
			assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
			let single_xml = String::from_utf8(body)?;
			assert!(single_xml.contains(&unique_safety_report_id));
			if notation {
				// Explicit override above disables comments.
				assert!(!single_xml.contains("<!--"));
			}
			let (status, body) = request_json(
			&app, &cookie, "POST", "/api/cases/export/xml".to_string(),
			Some(serde_json::json!({ "case_ids": [case_id, second_id, case_id], "authority": authority })),
		).await?;
			assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
			let mut zip = zip::ZipArchive::new(Cursor::new(body))?;
			assert_eq!(
				zip.len(),
				2,
				"duplicate case IDs must yield one entry per case"
			);
			for (index, expected_id) in [case_id, second_id].into_iter().enumerate()
			{
				let mut entry = zip.by_index(index)?;
				assert!(entry
					.name()
					.ends_with(&format!("-{expected_id}-{authority}.xml")));
				let mut zipped_xml = String::new();
				entry.read_to_string(&mut zipped_xml)?;
				assert!(zipped_xml.contains("<MCCI_IN200100UV01"));
				if !notation {
					assert!(!zipped_xml.contains("<!--"));
				}
				if index == 0 {
					assert_eq!(
						entry.name(),
						format!(
							"{unique_safety_report_id}-{case_id}-{authority}.xml"
						)
					);
					assert!(zipped_xml.contains(&unique_safety_report_id));
				}
			}
		}
	}

	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/export/xml?authority=fda&authority=mfds"),
		None,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::BAD_REQUEST,
		"{}",
		String::from_utf8_lossy(&body)
	);
	assert!(String::from_utf8_lossy(&body).contains("duplicate field `authority`"));

	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let missing_id = Uuid::new_v4().to_string();
	for (request_cookie, target_id, expected) in [
		(&cookie, missing_id.as_str(), StatusCode::NOT_FOUND),
		(&viewer_cookie, case_id, StatusCode::FORBIDDEN),
	] {
		for (method, uri, input) in [
			(
				"GET",
				format!("/api/cases/{target_id}/export/xml?authority=fda"),
				None,
			),
			(
				"POST",
				"/api/cases/export/xml".to_string(),
				Some(
					serde_json::json!({ "case_ids": [target_id], "authority": "fda" }),
				),
			),
		] {
			let (status, body) =
				request_json(&app, request_cookie, method, uri, input).await?;
			assert_eq!(status, expected, "{}", String::from_utf8_lossy(&body));
		}
	}

	// A stored settings error must still fail inside the per-case history boundary,
	// including when the single export supplies an explicit notation override.
	let ctx = lib_core::ctx::Ctx::new(
		seed.admin.id,
		seed.org_id,
		lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let mut invalid_settings =
		lib_core::model::admin_settings::AdminSettingsBmc::get(&ctx, &mm, "system")
			.await?
			.ok_or("missing test settings")?;
	invalid_settings["timezone"] = Value::Null;
	lib_core::model::admin_settings::AdminSettingsBmc::upsert(
		&ctx,
		&mm,
		"system",
		&invalid_settings,
		Some(seed.admin.id),
	)
	.await?;
	for (method, uri, input) in [
		("GET", format!("/api/cases/{case_id}/export/xml?authority=fda&include_notation=false"), None),
		("POST", "/api/cases/export/xml".to_string(), Some(serde_json::json!({ "case_ids": [case_id, second_id], "authority": "fda" }))),
	] {
		let (status, body) = request_json(&app, &cookie, method, uri, input).await?;
		assert_eq!(status, StatusCode::BAD_REQUEST, "{}", String::from_utf8_lossy(&body));
		assert!(String::from_utf8_lossy(&body).contains("admin settings field 'timezone' is required"));
	}
	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/exports/history"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	let history: Value = serde_json::from_slice(&body)?;
	let items = history["data"]["items"]
		.as_array()
		.ok_or("missing export history")?;
	assert_eq!(
		items
			.iter()
			.filter(|item| item["status"] == "success")
			.count(),
		9,
		"single and ZIP successes must be recorded once per exported case"
	);
	assert!(items.iter().all(|item| item["caseId"] == case_id
		&& item["exportedBy"] == seed.admin.id.to_string()));
	assert!(items
		.iter()
		.all(|item| item.get("validationAuthority").is_none()));
	let errors: Vec<_> = items
		.iter()
		.filter(|item| item["status"] == "error")
		.collect();
	assert_eq!(errors.len(), 2);
	assert!(errors.iter().all(|item| item["errorMessage"]
		.as_str()
		.is_some_and(|message| message.contains("timezone"))));

	Ok(())
}
