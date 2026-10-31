use super::support::*;

#[serial]
#[tokio::test]
async fn test_internal_ack_callback_updates_submission_by_remote_id() -> Result<()> {
	clear_esg_env();
	configure_mock_esg_transport().await?;
	std::env::set_var("AS2_CALLBACK_TOKEN", "callback-secret");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, submit_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{submit_body:?}");
	let remote_submission_id = submit_body["data"]["remote_submission_id"]
		.as_str()
		.ok_or("missing remote_submission_id")?
		.to_string();
	let submission_id = submit_body["data"]["id"]
		.as_str()
		.ok_or("missing submission_id")?
		.to_string();

	let req = Request::builder()
		.method("POST")
		.uri("/internal/submissions/callbacks/ack")
		.header("content-type", "application/json")
		.header("x-callback-token", "callback-secret")
		.body(Body::from(
			json!({
				"remote_submission_id": remote_submission_id,
				"ack_level": 3,
				"success": true,
				"ack_code": "ACK3",
				"ack_message": "Processed",
			})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let callback_status = res.status();
	let callback_body = to_bytes(res.into_body(), usize::MAX).await?;
	let callback_value: Value = serde_json::from_slice(&callback_body)?;
	assert_eq!(callback_status, StatusCode::OK, "{callback_value:?}");
	assert_eq!(callback_value["data"]["status"], "ack3_received");

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/submissions/{submission_id}"))
		.header("cookie", &cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let get_status = res.status();
	let get_body = to_bytes(res.into_body(), usize::MAX).await?;
	let get_value: Value = serde_json::from_slice(&get_body)?;
	assert_eq!(get_status, StatusCode::OK, "{get_value:?}");
	assert_eq!(get_value["data"]["status"], "ack3_received");
	let (dispatch_status, dispatch_body) = get_json(
		&app,
		&cookie,
		&format!("/api/submissions/{submission_id}/dispatch-state"),
	)
	.await?;
	if dispatch_status == StatusCode::OK {
		assert_eq!(dispatch_body["data"]["state"]["attempt_count"], 1);
		assert!(
			!dispatch_body["data"]["state"]["last_attempt_at"].is_null(),
			"{dispatch_body:?}"
		);
		assert!(!dispatch_body["data"]["state"]["terminal_at"].is_null());
	}
	let (events_status, events_body) = get_json(
		&app,
		&cookie,
		&format!("/api/submissions/{submission_id}/events"),
	)
	.await?;
	assert_eq!(events_status, StatusCode::OK, "{events_body:?}");
	let items = events_body["data"]["items"]
		.as_array()
		.ok_or("missing events list")?;
	assert!(items.len() >= 3, "{events_body:?}");
	let event_types: Vec<&str> = items
		.iter()
		.filter_map(|v| v["event_type"].as_str())
		.collect();
	assert!(
		event_types.contains(&"submission_created"),
		"{event_types:?}"
	);
	assert!(event_types.contains(&"ack_recorded"), "{event_types:?}");
	assert!(event_types.contains(&"status_changed"), "{event_types:?}");

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_internal_submission_endpoints_require_valid_callback_token(
) -> Result<()> {
	clear_esg_env();
	std::env::set_var("AS2_CALLBACK_TOKEN", "callback-secret");
	std::env::set_var("SUBMISSION_RECONCILE_TOKEN", "reconcile-secret");
	let mm = init_test_mm().await?;
	let app = web_server::app(mm);

	let req = Request::builder()
		.method("POST")
		.uri("/internal/submissions/callbacks/ack")
		.header("content-type", "application/json")
		.body(Body::from(
			json!({
				"remote_submission_id": "AS2-X",
				"ack_level": 1,
				"success": true
			})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert!(
		value.to_string().contains("missing x-callback-token"),
		"{value:?}"
	);

	let req = Request::builder()
		.method("POST")
		.uri("/internal/submissions/callbacks/ack")
		.header("content-type", "application/json")
		.header("x-callback-token", "wrong-token")
		.body(Body::from(
			json!({
				"remote_submission_id": "AS2-X",
				"ack_level": 1,
				"success": true
			})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert!(
		value.to_string().contains("invalid x-callback-token"),
		"{value:?}"
	);

	let req = Request::builder()
		.method("POST")
		.uri("/internal/submissions/reconcile")
		.header("content-type", "application/json")
		.body(Body::from(json!({ "limit": 5 }).to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert!(
		value.to_string().contains("missing x-reconcile-token"),
		"{value:?}"
	);

	let req = Request::builder()
		.method("GET")
		.uri("/internal/submissions/reconcile/status")
		.header("x-reconcile-token", "wrong-token")
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert!(
		value.to_string().contains("invalid x-reconcile-token"),
		"{value:?}"
	);

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_rust_to_submitter_bridge_payload_and_ack_flow() -> Result<()> {
	clear_esg_env();
	let (submitter_url, received_payloads) = start_mock_submitter().await?;
	std::env::set_var("AS2_SUBMITTER_URL", submitter_url);
	std::env::set_var("AS2_SUBMITTER_TOKEN", "test-token");
	std::env::set_var(
		"AS2_ACK_CALLBACK_URL",
		"http://127.0.0.1:8080/internal/submissions/callbacks/ack",
	);
	std::env::set_var("AS2_CALLBACK_TOKEN", "callback-secret");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id =
		create_case_with_profile(&app, &cookie, seed.org_id, "mfds").await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, submit_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/mfds"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{submit_body:?}");
	let submission_id = submit_body["data"]["id"]
		.as_str()
		.ok_or("missing submission id")?
		.to_string();
	let remote_submission_id = submit_body["data"]["remote_submission_id"]
		.as_str()
		.ok_or("missing remote_submission_id")?
		.to_string();
	assert!(remote_submission_id.starts_with("AS2-MOCK-"));

	let payloads = received_payloads.lock().await;
	assert_eq!(payloads.len(), 1, "{payloads:?}");
	let p = &payloads[0];
	assert_eq!(p["authority"], "mfds");
	assert_eq!(p["caseId"], case_id.to_string());
	assert_eq!(p["idempotencyKey"], case_id.to_string());
	assert_eq!(
		p["callbackUrl"],
		"http://127.0.0.1:8080/internal/submissions/callbacks/ack"
	);
	let xml_payload = p["xmlPayload"]
		.as_str()
		.ok_or("missing xmlPayload string")?;
	assert!(!xml_payload.trim().is_empty());
	drop(payloads);

	let req = Request::builder()
		.method("POST")
		.uri("/internal/submissions/callbacks/ack")
		.header("content-type", "application/json")
		.header("x-callback-token", "callback-secret")
		.body(Body::from(
			json!({
				"remote_submission_id": remote_submission_id,
				"ack_level": 3,
				"success": true,
				"ack_code": "ACK3",
				"ack_message": "Processed",
			})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/submissions/{submission_id}"))
		.header("cookie", &cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["status"], "ack3_received", "{value:?}");

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_internal_ack_callback_duplicate_payload_is_idempotent() -> Result<()> {
	clear_esg_env();
	configure_mock_esg_transport().await?;
	std::env::set_var("AS2_CALLBACK_TOKEN", "callback-secret");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, submit_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{submit_body:?}");
	let submission_id = submit_body["data"]["id"]
		.as_str()
		.ok_or("missing submission id")?
		.to_string();
	let remote_submission_id = submit_body["data"]["remote_submission_id"]
		.as_str()
		.ok_or("missing remote_submission_id")?
		.to_string();

	let ack_payload = json!({
		"remote_submission_id": remote_submission_id,
		"ack_level": 3,
		"success": true,
		"ack_code": "ACK3",
		"ack_message": "Processed",
	})
	.to_string();
	for _ in 0..2 {
		let req = Request::builder()
			.method("POST")
			.uri("/internal/submissions/callbacks/ack")
			.header("content-type", "application/json")
			.header("x-callback-token", "callback-secret")
			.body(Body::from(ack_payload.clone()))?;
		let res = app.clone().oneshot(req).await?;
		let status = res.status();
		let body = to_bytes(res.into_body(), usize::MAX).await?;
		let value: Value = serde_json::from_slice(&body)?;
		assert_eq!(status, StatusCode::OK, "{value:?}");
		assert_eq!(value["data"]["status"], "ack3_received", "{value:?}");
	}

	let (events_status, events_body) = get_json(
		&app,
		&cookie,
		&format!("/api/submissions/{submission_id}/events"),
	)
	.await?;
	assert_eq!(events_status, StatusCode::OK, "{events_body:?}");
	let items = events_body["data"]["items"]
		.as_array()
		.ok_or("missing events list")?;
	let event_types: Vec<&str> = items
		.iter()
		.filter_map(|v| v["event_type"].as_str())
		.collect();
	assert!(event_types.contains(&"ack_recorded"), "{event_types:?}");
	assert!(
		event_types.contains(&"ack_duplicate_ignored"),
		"{event_types:?}"
	);

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_internal_ack_callback_rejects_malformed_payload() -> Result<()> {
	clear_esg_env();
	std::env::set_var("AS2_CALLBACK_TOKEN", "callback-secret");
	let mm = init_test_mm().await?;
	let app = web_server::app(mm);

	let req = Request::builder()
		.method("POST")
		.uri("/internal/submissions/callbacks/ack")
		.header("content-type", "application/json")
		.header("x-callback-token", "callback-secret")
		.body(Body::from(
			json!({
				"remote_submission_id": "AS2-TEST-MISSING-LEVEL",
				"success": true
			})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	assert!(
		status == StatusCode::BAD_REQUEST
			|| status == StatusCode::UNPROCESSABLE_ENTITY,
		"status={status} body={}",
		String::from_utf8_lossy(&body)
	);
	let text = match serde_json::from_slice::<Value>(&body) {
		Ok(v) => v.to_string().to_ascii_lowercase(),
		Err(_) => String::from_utf8_lossy(&body).to_ascii_lowercase(),
	};
	assert!(
		text.contains("ack_level")
			|| text.contains("invalid")
			|| text.contains("failed to deserialize"),
		"text={text}"
	);

	clear_esg_env();
	Ok(())
}
