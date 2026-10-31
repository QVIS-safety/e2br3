use super::support::*;

#[serial]
#[tokio::test]
async fn test_submission_rejects_enabled_esg_without_base_url() -> Result<()> {
	clear_esg_env();
	std::env::set_var("FDA_ESG_ENABLED", "1");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(body.to_string().contains("FDA_ESG_BASE_URL"), "{body:?}");
	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_esg_transport_sends_expected_headers_and_payload(
) -> Result<()> {
	clear_esg_env();
	let (esg_url, received) = start_mock_esg(
		StatusCode::OK,
		json!({
			"submission_id": format!("ESG-{}", Uuid::new_v4().simple()),
			"ack": { "level": 1, "success": true, "code": "ACK1" }
		}),
	)
	.await?;
	std::env::set_var("FDA_ESG_ENABLED", "1");
	std::env::set_var("FDA_ESG_BASE_URL", esg_url);
	std::env::set_var("FDA_ESG_BEARER_TOKEN", "test-esg-token");
	std::env::set_var("FDA_ESG_API_KEY", "test-esg-api-key");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id =
		create_case_with_profile(&app, &cookie, seed.org_id, "fda").await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	assert_eq!(body["data"]["gateway"], "fda-esg-nextgen-api");

	let msgs = received.lock().await;
	assert_eq!(msgs.len(), 1, "{msgs:?}");
	assert_eq!(msgs[0]["authorization"], "Bearer test-esg-token");
	assert_eq!(msgs[0]["x_api_key"], "test-esg-api-key");
	let xml = msgs[0]["body"]["xml"]
		.as_str()
		.ok_or("missing xml payload")?;
	assert!(!xml.trim().is_empty());

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_esg_non_success_preserves_unknown_without_resend(
) -> Result<()> {
	clear_esg_env();
	let (esg_url, received) = start_mock_esg(
		StatusCode::BAD_GATEWAY,
		json!({ "error": "gateway unavailable" }),
	)
	.await?;
	std::env::set_var("FDA_ESG_ENABLED", "1");
	std::env::set_var("FDA_ESG_BASE_URL", esg_url);
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id =
		create_case_with_profile(&app, &cookie, seed.org_id, "fda").await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	assert_eq!(body["data"]["status"], "dispatch_unknown");
	assert!(body["data"]["remote_submission_id"].is_null());
	assert!(body["data"]["ack1"].is_null());
	assert_eq!(received.lock().await.len(), 1);
	let id = body["data"]["id"].as_str().ok_or("missing id")?;
	let (_, dispatch) = get_json(
		&app,
		&cookie,
		&format!("/api/submissions/{id}/dispatch-state"),
	)
	.await?;
	assert!(dispatch["data"]["state"]["next_retry_at"].is_null());
	assert!(dispatch["data"]["state"]["last_error"]
		.as_str()
		.ok_or("missing dispatch error")?
		.contains("FDA ESG submit failed"));

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_accepts_mfds_route_for_mfds_profile() -> Result<()> {
	clear_esg_env();
	configure_mock_as2_transport().await?;
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
	let remote_submission_id = submit_body["data"]["remote_submission_id"]
		.as_str()
		.ok_or("missing remote_submission_id")?;
	assert!(remote_submission_id.starts_with("AS2-MOCK-"));

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_uses_request_authority_not_case_appendices() -> Result<()> {
	clear_esg_env();
	configure_mock_as2_transport().await?;
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	mm.dbx()
		.execute(
			sqlx::query("UPDATE cases SET status = 'validated' WHERE id = $1")
				.bind(case_id),
		)
		.await?;
	mm.dbx().commit_txn().await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/mfds"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	assert!(body["data"]["remote_submission_id"]
		.as_str()
		.unwrap_or_default()
		.starts_with("AS2-MOCK-"));

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_retains_unknown_when_as2_submitter_unreachable(
) -> Result<()> {
	clear_esg_env();
	std::env::set_var("AS2_SUBMITTER_URL", "http://127.0.0.1:9");
	std::env::set_var("AS2_SUBMITTER_TOKEN", "test-token");
	std::env::set_var("AS2_SUBMITTER_TIMEOUT_SECS", "1");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, body) = post_json_with_headers(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
		&[("x-idempotency-key", "unknown-key")],
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	assert_eq!(body["data"]["status"], "dispatch_unknown");
	assert!(body["data"]["remote_submission_id"].is_null());
	for level in 1..=4 {
		assert!(body["data"][format!("ack{level}")].is_null());
	}

	let (list_status, list_body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/submissions"))
			.await?;
	assert_eq!(list_status, StatusCode::OK, "{list_body:?}");
	let items = list_body["data"]["items"]
		.as_array()
		.ok_or("missing submissions items")?;
	assert_eq!(items.len(), 1);
	assert_eq!(items[0]["status"], "dispatch_unknown");
	assert!(items[0]["remote_submission_id"].is_null());

	let id = Uuid::parse_str(body["data"]["id"].as_str().ok_or("missing id")?)?;
	let ctx = lib_core::ctx::Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let (_, original) =
		web_server::submission::get_submission_artifact(&ctx, &mm, id).await?;
	assert!(!original.is_empty());
	let (repeated_status, repeated) = post_json_with_headers(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
		&[("x-idempotency-key", "unknown-key")],
	)
	.await?;
	assert_eq!(repeated_status, StatusCode::CREATED, "{repeated}");
	assert_eq!(repeated["data"]["id"], body["data"]["id"]);
	let req = Request::builder()
		.uri(format!("/api/submissions/{id}/xml"))
		.header("cookie", &cookie)
		.body(Body::empty())?;
	let downloaded = app.clone().oneshot(req).await?;
	assert_eq!(downloaded.status(), StatusCode::OK);
	assert_eq!(
		to_bytes(downloaded.into_body(), usize::MAX).await?.as_ref(),
		original.as_slice()
	);
	let req = Request::builder()
		.uri(format!("/api/submissions/{id}/xml"))
		.body(Body::empty())?;
	assert!(!app.clone().oneshot(req).await?.status().is_success());
	let mut tx = mm.dbx().db().begin().await?;
	sqlx::query(
		"WITH user_context AS MATERIALIZED (
			SELECT set_current_user_context($1)
		)
		SELECT set_org_context($2, $3)
		FROM user_context",
	)
	.bind(seed.admin.id)
	.bind(seed.org_id)
	.bind(ROLE_SPONSOR_ADMIN_CRO)
	.execute(&mut *tx)
	.await?;
	let changed =
		sqlx::query("UPDATE case_submissions SET submitted_xml=$2 WHERE id=$1")
			.bind(id)
			.bind({
				let mut bytes = original.clone();
				bytes[0] ^= 1;
				bytes
			})
			.execute(&mut *tx)
			.await;
	let error = changed.expect_err("submitted bytes must be immutable");
	assert!(
		error
			.to_string()
			.contains("Submission identity and original XML are immutable"),
		"{error}"
	);
	tx.rollback().await?;
	let (_, retained) =
		web_server::submission::get_submission_artifact(&ctx, &mm, id).await?;
	assert_eq!(retained, original);
	let outsider = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let outsider_token =
		generate_web_token(&outsider.admin.email, outsider.admin.token_salt)?;
	let req = Request::builder()
		.uri(format!("/api/submissions/{id}/xml"))
		.header("cookie", cookie_header(&outsider_token.to_string()))
		.body(Body::empty())?;
	assert_eq!(
		app.clone().oneshot(req).await?.status(),
		StatusCode::NOT_FOUND
	);

	clear_esg_env();
	Ok(())
}
