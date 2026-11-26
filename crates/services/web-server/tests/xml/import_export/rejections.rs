use super::support::*;

#[serial]
#[tokio::test]
async fn test_manual_export_allows_invalid_drafts_and_unvalidated_routes(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "admin_pwd", "viewer_pwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm).expect("web server configuration");
	let (status, bytes) = request_json(&app, &cookie, "POST", "/api/cases".into(), Some(serde_json::json!({"data": {
        "safetyReportIdentification": {"safetyReportId": format!("N-EXPORT-{}", Uuid::new_v4())}, "status":"draft"
    }}))).await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&bytes)
	);
	let created: Value = serde_json::from_slice(&bytes)?;
	let id = created["data"]["id"].as_str().ok_or("missing case id")?;
	let attachment =
		"H4sIAAAAAAAAA8sszs9JLElNUcjLz0vOyczLTE7MUUgsKUlMzshNzSvhAgCw1cKiIAAAAA==";
	for (method, path, data, expected) in [
		(
			"PATCH",
			format!("/api/cases/{id}/editor/pages/CI"),
			serde_json::json!({"authorities":["ich"], "rows":{
				"safetyReportIdentification":{"reportType":"1","transmissionDate":"20240101000000"},
				"documentsHeldBySender":[{"sequenceNumber":1,"documentDescription":"Isolated compressed attachment","includedDocument":attachment,"fileName":"attachment.txt","mediaType":"text/plain","representation":"B64","compression":"GZ"}]
			}}),
			StatusCode::OK,
		),
		(
			"PATCH",
			format!("/api/cases/{id}/editor/pages/NR"),
			serde_json::json!({"authorities":["ich"],"rows":{"caseSummaryInformation":[{"sequenceNumber":1,"languageCode":"JA","summaryText":"Isolated draft summary"}]}}),
			StatusCode::OK,
		),
		(
			"POST",
			format!("/api/cases/{id}/message-header"),
			serde_json::json!({"data":{
				"case_id":id,"message_number":format!("MSG-{id}"),"message_date":"20240101000000",
				"batch_sender_identifier":"SENDER","message_sender_identifier":"SENDER",
				"batch_receiver_identifier":"ZZFDA","message_receiver_identifier":"CDER"
			}}),
			StatusCode::CREATED,
		),
	] {
		let (status, bytes) =
			request_json(&app, &cookie, method, path, Some(data)).await?;
		assert_eq!(status, expected, "{}", String::from_utf8_lossy(&bytes));
	}
	let mut saved_pages = Vec::new();
	for page in ["CI", "NR"] {
		let path = format!("/api/cases/{id}/editor/pages/{page}?authorities=ich");
		let (status, bytes) =
			request_json(&app, &cookie, "GET", path.clone(), None).await?;
		assert_eq!(status, StatusCode::OK);
		saved_pages.push((path, serde_json::from_slice::<Value>(&bytes)?));
	}
	let (status, bytes) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{id}/validation?authority=ich"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(
		serde_json::from_slice::<Value>(&bytes)?["data"]["ok"],
		false,
		"must exercise a draft with Case business errors"
	);

	for (authority, batch, message) in [
		("ich", "ICHTEST", "ICHTEST"),
		("fda", "ZZFDA", "CDER"),
		("mfds", "MFDS-O-KR", "MFDS-O-KR"),
	] {
		for valid in [false, true] {
			if authority == "ich" && !valid {
				continue;
			}
			let batch = if valid { batch } else { "INVALID-ROUTE" };
			let message = if valid { message } else { "INVALID-ROUTE" };
			let (status, bytes) = request_json(
				&app,
				&cookie,
				"PUT",
				format!("/api/cases/{id}/message-header"),
				Some(serde_json::json!({"data":{
					"batch_receiver_identifier":batch,"message_receiver_identifier":message
				}})),
			)
			.await?;
			assert_eq!(
				status,
				StatusCode::OK,
				"{}",
				String::from_utf8_lossy(&bytes)
			);
			for (method, path, body) in [
				(
					"GET",
					format!("/api/cases/{id}/export/xml?authority={authority}"),
					None,
				),
				(
					"POST",
					"/api/cases/export/xml".to_string(),
					Some(serde_json::json!({"case_ids":[id],"authority":authority})),
				),
			] {
				let (status, bytes) =
					request_json(&app, &cookie, method, path, body).await?;
				assert_eq!(
					status,
					StatusCode::OK,
					"{authority} {method}: {}",
					String::from_utf8_lossy(&bytes)
				);
				let xml = if method == "GET" {
					String::from_utf8(bytes.to_vec())?
				} else {
					let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
					assert_eq!(archive.len(), 1);
					let mut xml = String::new();
					archive.by_index(0)?.read_to_string(&mut xml)?;
					xml
				};
				let parsed =
					xml::validation::validate_e2b_xml_basic(xml.as_bytes(), None)?;
				assert!(parsed.ok, "{:?}", parsed.errors);
				assert_eq!(
					xml.matches("compression=\"GZ\"").count(),
					1,
					"compressed attachment metadata lost"
				);
				assert_eq!(
					xml.matches(attachment).count(),
					1,
					"compressed payload changed"
				);
				let summaries =
					xml::import_sections::h_narrative::parse_h_case_summaries(
						xml.as_bytes(),
					)?;
				assert_eq!(summaries.len(), 1);
				assert_eq!(summaries[0].language_code.as_deref(), Some("JA"));
				assert_eq!(
					summaries[0].summary_text.as_deref(),
					Some("Isolated draft summary")
				);
				assert!(
					xml::import_sections::d_patient::parse_d_patient(
						xml.as_bytes()
					)?
					.is_none(),
					"an absent patient must not be fabricated for draft export"
				);
				assert_eq!(
					parsed.root_element.as_deref(),
					Some("MCCI_IN200100UV01")
				);
				assert!(
					xml.contains(batch),
					"stored batch receiver must survive export"
				);
				assert!(
					xml.contains(message),
					"stored message receiver must survive export"
				);
			}
		}
	}
	let (status, bytes) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{id}/exports/history"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	let history: Value = serde_json::from_slice(&bytes)?;
	let items = history["data"]["items"]
		.as_array()
		.ok_or("missing export history")?;
	assert_eq!(
		items
			.iter()
			.filter(|item| item["status"] == "success")
			.count(),
		10
	);
	assert_eq!(
		items
			.iter()
			.filter(|item| item["status"] == "error")
			.count(),
		0
	);
	for (path, before) in saved_pages {
		let (status, bytes) = request_json(&app, &cookie, "GET", path, None).await?;
		assert_eq!(status, StatusCode::OK);
		assert_eq!(
			serde_json::from_slice::<Value>(&bytes)?,
			before,
			"export changed stored metadata"
		);
	}
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_manual_export_preserves_missing_headers_and_records_serialization_errors(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "admin_pwd", "viewer_pwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone()).expect("web server configuration");

	// Incomplete data exports; unrepresentable XML data leaves downloadable error history.
	let create_body = serde_json::json!({
		"data": {
			"safetyReportIdentification": {
				"safetyReportId": format!("SR-{}", Uuid::new_v4())
			},
			"status": "draft"
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri("/api/cases")
		.header("content-type", "application/json")
		.header("cookie", cookie.clone())
		.body(Body::from(create_body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let case_id = value
		.get("data")
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing data.id in case create response")?;

	let (status, body) = request_json(
		&app,
		&cookie,
		"POST",
		format!("/api/cases/{case_id}/message-header"),
		Some(serde_json::json!({ "data": {
			"case_id": case_id,
			"message_number": format!("MSG-{case_id}"),
			"batch_sender_identifier": "SENDER01",
			"batch_receiver_identifier": null,
			"message_sender_identifier": "SENDER01",
			"message_receiver_identifier": "RECEIVER01",
			"message_date": "20240201010101"
		} })),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);

	// The missing batch receiver is emitted as absent, never filled from message receiver.
	for (method, uri, input) in [
		(
			"GET",
			format!("/api/cases/{case_id}/export/xml?authority=mfds"),
			None,
		),
		(
			"POST",
			"/api/cases/export/xml".to_owned(),
			Some(serde_json::json!({"case_ids":[case_id],"authority":"mfds"})),
		),
	] {
		let (status, bytes) =
			request_json(&app, &cookie, method, uri, input).await?;
		assert_eq!(
			status,
			StatusCode::OK,
			"{}",
			String::from_utf8_lossy(&bytes)
		);
		let xml = if method == "GET" {
			String::from_utf8(bytes.to_vec())?
		} else {
			let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
			let mut xml = String::new();
			archive.by_index(0)?.read_to_string(&mut xml)?;
			xml
		};
		assert_eq!(xpath_value(xml.as_bytes(), "count(/*[local-name()='MCCI_IN200100UV01']/*[local-name()='receiver']/*[local-name()='device']/*[local-name()='id']/@extension)")?, "0");
	}
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/message-header"),
		Some(serde_json::json!({"data":{"batch_sender_identifier":"bad\u{1}"}})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::UNPROCESSABLE_ENTITY,
		"{}",
		String::from_utf8_lossy(&body)
	);
	assert!(String::from_utf8_lossy(&body).contains("INPUT.CONTROL_CHAR.REJECTED"));
	// Seed legacy corrupt data directly in this isolated DB to exercise export error history.
	mm.dbx().begin_txn().await?;
	lib_core::model::store::set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	mm.dbx().execute(sqlx::query("UPDATE message_headers SET batch_sender_identifier=$2 WHERE case_id=$1")
        .bind(Uuid::parse_str(case_id)?).bind("bad\u{1}")).await?;
	mm.dbx().commit_txn().await?;
	// XML 1.0 cannot encode U+0001; silently deleting it would lose stored data.
	for (method, uri, input) in [
		(
			"GET",
			format!("/api/cases/{case_id}/export/xml?authority=mfds"),
			None,
		),
		(
			"POST",
			"/api/cases/export/xml".to_string(),
			Some(serde_json::json!({ "case_ids": [case_id], "authority": "mfds" })),
		),
	] {
		let (status, body) = request_json(&app, &cookie, method, uri, input).await?;
		assert_eq!(
			status,
			StatusCode::INTERNAL_SERVER_ERROR,
			"{}",
			String::from_utf8_lossy(&body)
		);
	}
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/message-header"),
		Some(
			serde_json::json!({ "data": { "batch_sender_identifier": "SENDER01" } }),
		),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));

	for (method, uri, input) in [
		(
			"GET",
			format!("/api/cases/{case_id}/export/xml?authority=fda"),
			None,
		),
		(
			"POST",
			"/api/cases/export/xml".to_string(),
			Some(serde_json::json!({ "case_ids": [case_id], "authority": "fda" })),
		),
	] {
		let (status, body) = request_json(&app, &cookie, method, uri, input).await?;
		assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
		let xml = if method == "GET" {
			String::from_utf8(body.to_vec())?
		} else {
			let mut archive = zip::ZipArchive::new(Cursor::new(body))?;
			assert_eq!(archive.len(), 1);
			let mut xml = String::new();
			archive.by_index(0)?.read_to_string(&mut xml)?;
			xml
		};
		assert!(
			xml::import_sections::d_patient::parse_d_patient(xml.as_bytes())?
				.is_none(),
			"fixing the header must allow export without inventing a patient"
		);
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
	assert_eq!(items.len(), 6);
	assert_eq!(
		items
			.iter()
			.filter(|item| item["status"] == "error")
			.count(),
		2
	);
	assert_eq!(
		items
			.iter()
			.filter(|item| item["status"] == "success")
			.count(),
		4
	);
	assert!(items.iter().all(|item| item["caseId"] == case_id && item["exportedBy"] == seed.admin.id.to_string()), "history must survive the failed export's outer transaction with the original user and case");
	assert_eq!(
		items
			.iter()
			.filter(|item| { item["errorMessage"] == "Export.InvalidXml" })
			.count(),
		2
	);
	let header_errors: Vec<_> = items
		.iter()
		.filter(|item| item["errorMessage"] == "Export.InvalidXml")
		.collect();
	assert_eq!(header_errors.len(), 2);
	let error = header_errors[0];
	let history_id = error["id"].as_str().ok_or("missing history id")?;
	let file_name = format!("{case_id}-mfds.xml");
	assert_eq!(error["fileName"], file_name);
	assert!(items
		.iter()
		.all(|item| item.get("validationAuthority").is_none()));
	let req = Request::builder()
		.uri(format!("/api/exports/history/{history_id}/error.txt"))
		.header("cookie", &cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	assert_eq!(res.headers()["content-type"], "text/plain; charset=utf-8");
	assert_eq!(
		res.headers()["content-disposition"],
		format!(
			"attachment; filename=\"export-error-{history_id}-{file_name}.txt\""
		)
	);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	assert_eq!(
		String::from_utf8(body.to_vec())?,
		error["errorMessage"]
			.as_str()
			.ok_or("missing error message")?
	);

	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		"/api/exports/history".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	let all_history: Value = serde_json::from_slice(&body)?;
	let all_items = all_history["data"]["items"]
		.as_array()
		.ok_or("missing global export history")?;
	assert!(all_items.iter().any(|item| item["id"] == history_id));
	assert!(all_items
		.iter()
		.all(|item| item.get("validationAuthority").is_none()));

	Ok(())
}
