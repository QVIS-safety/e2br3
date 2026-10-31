use super::support::*;

#[serial]
#[tokio::test]
#[ignore = "Requires real Java AS2 submitter running; set REAL_AS2_SUBMITTER_URL"]
async fn test_real_java_submitter_integration_mfds() -> Result<()> {
	clear_esg_env();
	let submitter_url = std::env::var("REAL_AS2_SUBMITTER_URL")?;
	let inbound_token = std::env::var("REAL_AS2_SUBMITTER_TOKEN")?;
	let callback_token = std::env::var("REAL_AS2_CALLBACK_TOKEN")?;
	if submitter_url.trim().is_empty()
		|| inbound_token.trim().is_empty()
		|| callback_token.trim().is_empty()
	{
		return Err("real AS2 endpoint and credentials must not be empty".into());
	}
	std::env::set_var("AS2_SUBMITTER_URL", submitter_url);
	std::env::set_var("AS2_SUBMITTER_TOKEN", inbound_token);
	std::env::set_var("AS2_CALLBACK_TOKEN", callback_token);
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let listener = TcpListener::bind("127.0.0.1:0").await?;
	let callback_addr = listener.local_addr()?;
	std::env::set_var(
		"AS2_ACK_CALLBACK_URL",
		format!("http://{callback_addr}/internal/submissions/callbacks/ack"),
	);
	let callback_app = app.clone();
	let callback_server =
		tokio::spawn(async move { axum::serve(listener, callback_app).await });

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
	assert_eq!(submit_body["data"]["gateway"], "as2-submitter-http-mfds");
	let remote_submission_id = submit_body["data"]["remote_submission_id"]
		.as_str()
		.ok_or("missing remote_submission_id")?;
	assert!(!remote_submission_id.trim().is_empty(), "{submit_body:?}");

	assert_eq!(
		submit_body["data"]["status"], "ack1_received",
		"{submit_body}"
	);
	assert_eq!(submit_body["data"]["ack1"]["success"], true);
	assert!(submit_body["data"]["ack2"].is_null());
	let id = submit_body["data"]["id"].as_str().ok_or("missing id")?;
	tokio::time::timeout(Duration::from_secs(30), async {
		loop {
			let (_, events) =
				get_json(&app, &cookie, &format!("/api/submissions/{id}/events"))
					.await?;
			let rows = events["data"]["items"].as_array().ok_or("missing events")?;
			if rows.iter().any(|event| {
				event["event_type"] == "ack_duplicate_ignored"
					|| event["event_data"]["source"] == "gateway_callback"
			}) {
				return Ok::<_, Box<dyn std::error::Error + Send + Sync>>(());
			}
			sleep(Duration::from_millis(200)).await;
		}
	})
	.await??;
	callback_server.abort();

	clear_esg_env();
	Ok(())
}
