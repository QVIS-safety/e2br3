use super::support::*;

#[serial]
#[tokio::test]
#[ignore = "Requires Java submitter, REAL_AS2_PROFILE_PATH and REAL_AS2_BACKEND_BIND; configure AS2_PROFILE_URL to that backend"]
async fn test_real_java_submitter_integration_mfds() -> Result<()> {
	clear_submission_env();
	let submitter_url = std::env::var("REAL_AS2_SUBMITTER_URL")?;
	let inbound_token = std::env::var("REAL_AS2_SUBMITTER_TOKEN")?;
	let callback_token = std::env::var("REAL_AS2_CALLBACK_TOKEN")?;
	let profile_path = std::env::var("REAL_AS2_PROFILE_PATH")?;
	let backend_bind = std::env::var("REAL_AS2_BACKEND_BIND")?;
	let mut configuration: Value =
		serde_json::from_slice(&std::fs::read(profile_path)?)?;
	let routing_id = configuration.as_object_mut().ok_or("profile configuration must be an object")?
		.remove("senderRoutingId").ok_or("REAL_AS2_PROFILE_PATH must include senderRoutingId for the loopback partner")?;
	let routing_id = routing_id
		.as_str()
		.ok_or("senderRoutingId must be a string")?;
	let profile: lib_core::model::organization_as2_profile::OrganizationAs2ProfileInput =
		serde_json::from_value(configuration)?;
	let endpoint = reqwest::Url::parse(&profile.endpoint_url)?;
	if !matches!(
		endpoint.host_str(),
		Some("127.0.0.1" | "localhost" | "[::1]")
	) {
		return Err("fixture integration requires a loopback authority endpoint; never send synthetic cases to a regulator".into());
	}
	if submitter_url.trim().is_empty()
		|| inbound_token.trim().is_empty()
		|| callback_token.trim().is_empty()
	{
		return Err("real AS2 endpoint and credentials must not be empty".into());
	}
	configure_as2_test_endpoint(submitter_url);
	std::env::set_var("AS2_SUBMITTER_TOKEN", inbound_token);
	std::env::set_var("AS2_SUBMITTER_TIMEOUT_SECS", "30");
	std::env::set_var("AS2_CALLBACK_TOKEN", callback_token);

	let mm = init_test_mm().await?;
	let seed =
		crate::common::seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	lib_core::model::organization_as2_profile::OrganizationAs2ProfileBmc::upsert(
		&lib_core::ctx::Ctx::root_ctx(),
		&mm,
		seed.org_id,
		"mfds",
		profile,
	)
	.await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone()).expect("web server configuration");
	let listener = TcpListener::bind(backend_bind).await?;
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
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	let changed = mm
		.dbx()
		.execute(
			sqlx::query(
				"UPDATE sender_presave_gateways SET routing_identifier=$1
		 WHERE gateway_authority='mfds' AND sender_presave_id IN
		 (SELECT source_sender_presave_id FROM sender_information WHERE case_id=$2)",
			)
			.bind(routing_id)
			.bind(case_id),
		)
		.await?;
	assert_eq!(changed, 1);
	mm.dbx().commit_txn().await?;

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

	clear_submission_env();
	Ok(())
}
