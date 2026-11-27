use super::support::*;

#[serial]
#[tokio::test]
async fn sender_presave_route_is_required_and_read_live_for_the_linked_sender(
) -> Result<()> {
	use lib_core::ctx::Ctx;
	use lib_core::model::organization_as2_profile::OrganizationAs2ProfileBmc;
	use lib_core::model::presave::{
		SenderPresaveGatewayBmc, SenderPresaveGatewayForUpdate,
	};
	use lib_core::model::safety_report::SenderInformationBmc;
	clear_submission_env();
	let (url, received) = start_mock_submitter().await?;
	configure_as2_test_endpoint(url);
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let cookie = cookie_header(
		&generate_web_token(&seed.admin.email, seed.admin.token_salt)?.to_string(),
	);
	let app = web_server::app(mm.clone()).expect("configuration");
	// Another valid Sender exists; missing linkage must not select it implicitly.
	crate::common::create_test_import_product(&mm, seed.org_id, seed.admin.id)
		.await?;
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;
	let (_, rows) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/safety-report/senders"),
	)
	.await?;
	let sender_id =
		Uuid::parse_str(rows["data"][0]["id"].as_str().ok_or("missing sender")?)?;
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let sender = SenderInformationBmc::get(&ctx, &mm, sender_id).await?;
	let parent = sender
		.source_sender_presave_id
		.ok_or("missing Sender link")?;
	let gateway = SenderPresaveGatewayBmc::list_by_parent(&ctx, &mm, parent)
		.await?
		.into_iter()
		.find(|g| g.gateway_authority == "fda")
		.ok_or("missing FDA gateway")?;
	let submit_uri = format!("/api/cases/{case_id}/submissions/fda");
	for (linked, route, expected_error) in [
		(
			false,
			"VALID-ROUTE",
			"case Sender must reference a Sender template",
		),
		(true, "", "Sender presave AS2 Routing ID"),
		(true, "invalid\r\nroute", "without quotes or line breaks"),
	] {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx(
			mm.dbx(),
			seed.admin.id,
			seed.org_id,
			ROLE_SPONSOR_ADMIN_CRO,
		)
		.await?;
		mm.dbx().execute(sqlx::query("UPDATE sender_information SET source_sender_presave_id=$2 WHERE id=$1")
			.bind(sender_id).bind(if linked {Some(parent)} else {None::<Uuid>})).await?;
		mm.dbx().commit_txn().await?;
		SenderPresaveGatewayBmc::update(
			&ctx,
			&mm,
			gateway.id,
			SenderPresaveGatewayForUpdate {
				routing_identifier: Some(route.into()),
				..Default::default()
			},
		)
		.await?;
		let (status, body) =
			post_json(&app, &cookie, &submit_uri, valid_compliance_payload())
				.await?;
		assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
		assert!(body.to_string().contains(expected_error), "{body}");
		assert!(received.lock().await.is_empty());
		let (_, case) =
			get_json(&app, &cookie, &format!("/api/cases/{case_id}")).await?;
		assert_eq!(case["data"]["status"], "draft");
		let (_, history) =
			get_json(&app, &cookie, &format!("/api/cases/{case_id}/submissions"))
				.await?;
		assert_eq!(history["data"]["items"], json!([]));
	}
	SenderPresaveGatewayBmc::update(
		&ctx,
		&mm,
		gateway.id,
		SenderPresaveGatewayForUpdate {
			routing_identifier: Some("UPDATED-LINKED-SENDER".into()),
			..Default::default()
		},
	)
	.await?;
	let runtime =
		OrganizationAs2ProfileBmc::runtime_profiles(&Ctx::root_ctx(), &mm).await?;
	let fda: Vec<_> = runtime
		.iter()
		.filter(|p| p.organization_id == seed.org_id && p.authority == "fda")
		.collect();
	assert_eq!(fda.len(), 2);
	assert_eq!(fda[0].id, fda[1].id);
	assert!(fda.iter().any(|p| p.routing_id == "UPDATED-LINKED-SENDER"));
	assert!(!fda
		.iter()
		.any(|p| p.routing_id == format!("ROUTE-{case_id}-fda")));
	let (status, body) =
		post_json(&app, &cookie, &submit_uri, valid_compliance_payload()).await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	assert_eq!(
		received.lock().await[0]["routingId"],
		"UPDATED-LINKED-SENDER"
	);
	clear_submission_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_rejects_unconfigured_transport() -> Result<()> {
	clear_submission_env();

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone()).expect("web server configuration");

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body:?}");
	assert_eq!(body["error"]["message"], "CONFIGURATION_ERROR", "{body:?}");
	clear_submission_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_sends_xml_to_as2_submitter() -> Result<()> {
	clear_submission_env();
	let (submitter_url, received) = start_mock_submitter().await?;
	configure_as2_test_endpoint(submitter_url);

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone()).expect("web server configuration");

	let case_id =
		create_case_with_profile(&app, &cookie, seed.org_id, "fda").await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	assert_eq!(body["data"]["gateway"], "as2-submitter-http-fda");

	let msgs = received.lock().await;
	assert_eq!(msgs.len(), 1, "{msgs:?}");
	let xml = msgs[0]["xmlPayload"]
		.as_str()
		.ok_or("missing xml payload")?;
	assert!(!xml.trim().is_empty());
	assert!(msgs[0].get("transportProfile").is_none());
	assert!(Uuid::parse_str(
		msgs[0]["profileId"].as_str().ok_or("missing profileId")?
	)
	.is_ok());
	assert_eq!(msgs[0]["profileVersion"], 1);
	assert_eq!(msgs[0]["routingId"], format!("ROUTE-{case_id}-fda"));
	assert!(msgs[0].get("signingPkcs12Base64").is_none());

	clear_submission_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_accepts_mfds_route_for_mfds_profile() -> Result<()> {
	clear_submission_env();
	configure_mock_as2_transport().await?;

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone()).expect("web server configuration");

	let case_id =
		create_case_with_profile(&app, &cookie, seed.org_id, "mfds").await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;

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

	clear_submission_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_uses_request_authority_not_case_appendices() -> Result<()> {
	clear_submission_env();
	configure_mock_as2_transport().await?;

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone()).expect("web server configuration");

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;

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

	clear_submission_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_records_failure_when_as2_submitter_unreachable(
) -> Result<()> {
	clear_submission_env();
	configure_as2_test_endpoint("http://127.0.0.1:9");
	std::env::set_var("AS2_SUBMITTER_TIMEOUT_SECS", "1");

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone()).expect("web server configuration");

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;

	let (status, reviewed) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/review/toggle"),
		json!({"data": {"expected_status": "draft"}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{reviewed}");
	assert_eq!(reviewed["data"]["status"], "reviewed", "{reviewed}");

	let (status, body) = post_json_with_headers(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
		&[("x-idempotency-key", "unreachable-key")],
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	assert_eq!(body["data"]["status"], "dispatch_failed");
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
	assert_eq!(items[0]["status"], "dispatch_failed");
	assert!(items[0]["remote_submission_id"].is_null());

	let id = Uuid::parse_str(body["data"]["id"].as_str().ok_or("missing id")?)?;
	let ctx = lib_core::ctx::Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let (case_status, restored) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}")).await?;
	assert_eq!(case_status, StatusCode::OK, "{restored}");
	assert_eq!(restored["data"]["status"], "reviewed", "{restored}");
	let (dispatch_status, dispatch) = get_json(
		&app,
		&cookie,
		&format!("/api/submissions/{id}/dispatch-state"),
	)
	.await?;
	assert_eq!(dispatch_status, StatusCode::OK, "{dispatch}");
	assert!(
		dispatch["data"]["state"]["next_retry_at"].is_null(),
		"{dispatch}"
	);
	assert!(!dispatch["data"]["state"]["last_error"]
		.as_str()
		.ok_or("missing dispatch error")?
		.is_empty());
	let (_, original) =
		web_server::submission::get_submission_artifact(&ctx, &mm, id).await?;
	assert!(!original.is_empty());
	let (repeated_status, repeated) = post_json_with_headers(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
		&[("x-idempotency-key", "unreachable-key")],
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

	clear_submission_env();
	Ok(())
}
