use super::support::*;

#[serial]
#[tokio::test]
async fn test_manual_export_checks_n_routes_and_blocks_invalid_draft_cases(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "admin_pwd", "viewer_pwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
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
	for (method, path, data, expected) in [
		(
			"PATCH",
			format!("/api/cases/{id}/editor/pages/CI"),
			serde_json::json!({"authorities":["ich"], "rows":{"safetyReportIdentification":{"reportType":"1","transmissionDate":"20240101000000"}}}),
			StatusCode::OK,
		),
		(
			"POST",
			format!("/api/cases/{id}/patient"),
			serde_json::json!({"data":{"case_id":id,"patient_initials":"N"}}),
			StatusCode::CREATED,
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
					StatusCode::BAD_REQUEST,
					"{authority} {method}: {}",
					String::from_utf8_lossy(&bytes)
				);
				let error: Value = serde_json::from_slice(&bytes)?;
				let message = error["error"]["data"]["detail"]
					.as_str()
					.ok_or("missing export error")?;
				if valid {
					assert!(message.contains("Correct the highlighted case fields"));
				} else {
					assert!(
						message.contains("Review export history for details"),
						"{error}"
					);
				}
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
			.filter(|item| item["status"] == "error")
			.count(),
		10
	);
	assert_eq!(
		items
			.iter()
			.filter(|item| item["status"] == "error")
			.filter(|item| item["errorMessage"].as_str().is_some_and(|message| {
				message.contains("export header validation failed")
			}))
			.count(),
		4
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_manual_export_rejects_missing_headers_and_records_errors() -> Result<()>
{
	let original = std::env::var("E2BR3_EXPORT_VALIDATE").ok();

	let mm = init_test_mm().await?;
	std::env::remove_var("E2BR3_EXPORT_VALIDATE");
	let seed = seed_org_with_users(&mm, "admin_pwd", "viewer_pwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	// Missing required export data must fail and leave a downloadable error history.
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

	// Missing batch receiver must fail before serialization, without a fallback.
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
			StatusCode::BAD_REQUEST,
			"{}",
			String::from_utf8_lossy(&body)
		);
		assert!(String::from_utf8_lossy(&body).contains(
			"N.1.4 Batch Receiver Identifier is required before XML export"
		));
	}
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/message-header"),
		Some(
			serde_json::json!({ "data": { "batch_receiver_identifier": "RECEIVER01" } }),
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
		assert_eq!(
			status,
			StatusCode::BAD_REQUEST,
			"{}",
			String::from_utf8_lossy(&body)
		);
		let body: Value = serde_json::from_slice(&body)?;
		assert_eq!(body["error"]["message"], "INVALID_REQUEST", "{body}");
		assert_eq!(
			body["error"]["data"]["detail"],
			"D Patient Information is required for export.",
			"{body}"
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
	assert_eq!(items.len(), 4);
	assert!(items.iter().all(|item| item["status"] == "error"));
	assert!(items.iter().all(|item| item["caseId"] == case_id && item["exportedBy"] == seed.admin.id.to_string()), "history must survive the failed export's outer transaction with the original user and case");
	assert_eq!(
		items
			.iter()
			.filter(|item| {
				item["errorMessage"]
					== "N.1.4 Batch Receiver Identifier is required before XML export"
			})
			.count(),
		2
	);
	let fda_errors: Vec<_> = items
		.iter()
		.filter(|item| {
			item["errorMessage"] == "D Patient Information is required for export."
		})
		.collect();
	assert_eq!(fda_errors.len(), 2);
	let error = fda_errors[0];
	let history_id = error["id"].as_str().ok_or("missing history id")?;
	let file_name = format!(
		"{}-{case_id}-fda.xml",
		create_body["data"]["safetyReportIdentification"]["safetyReportId"]
			.as_str()
			.unwrap()
	);
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

	match original {
		Some(v) => std::env::set_var("E2BR3_EXPORT_VALIDATE", v),
		None => std::env::remove_var("E2BR3_EXPORT_VALIDATE"),
	}
	Ok(())
}
