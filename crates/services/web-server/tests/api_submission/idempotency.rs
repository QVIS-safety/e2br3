use super::support::*;

#[serial]
#[tokio::test]
async fn test_submission_idempotency_key_reuses_submission_when_enabled(
) -> Result<()> {
	clear_esg_env();
	configure_mock_esg_transport().await?;
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let idem_key = "idem-fda-001";
	let (status1, body1) = post_json_with_headers(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
		&[("x-idempotency-key", idem_key)],
	)
	.await?;
	assert_eq!(status1, StatusCode::CREATED, "{body1:?}");
	let id1 = body1["data"]["id"]
		.as_str()
		.ok_or("missing submission id 1")?;

	let (status2, body2) = post_json_with_headers(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
		&[("x-idempotency-key", idem_key)],
	)
	.await?;

	assert_eq!(status2, StatusCode::CREATED, "{body2:?}");
	let id2 = body2["data"]["id"]
		.as_str()
		.ok_or("missing submission id 2")?;

	let (list_status, list_body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/submissions"))
			.await?;
	assert_eq!(list_status, StatusCode::OK, "{list_body:?}");
	let items = list_body["data"]["items"]
		.as_array()
		.ok_or("missing submissions list")?;
	assert_eq!(id1, id2);
	assert_eq!(items.len(), 1, "{list_body:?}");

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_idempotency_key_parallel_requests_single_submission(
) -> Result<()> {
	clear_esg_env();
	configure_mock_esg_transport().await?;
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let idem_key = "idem-fda-parallel-001";
	let uri = format!("/api/cases/{case_id}/submissions/fda");
	let headers = [("x-idempotency-key", idem_key)];
	let req_a = async {
		post_json_with_headers(
			&app,
			&cookie,
			&uri,
			valid_compliance_payload(),
			&headers,
		)
		.await
		.expect("parallel submission A failed")
	};
	let req_b = async {
		post_json_with_headers(
			&app,
			&cookie,
			&uri,
			valid_compliance_payload(),
			&headers,
		)
		.await
		.expect("parallel submission B failed")
	};
	let ((status_a, body_a), (status_b, body_b)) = tokio::join!(req_a, req_b);

	assert_eq!(status_a, StatusCode::CREATED, "{body_a:?}");
	assert_eq!(status_b, StatusCode::CREATED, "{body_b:?}");
	let id_a = body_a["data"]["id"]
		.as_str()
		.ok_or("missing submission id a")?;
	let id_b = body_b["data"]["id"]
		.as_str()
		.ok_or("missing submission id b")?;
	assert_eq!(id_a, id_b, "{body_a:?} vs {body_b:?}");

	let (list_status, list_body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/submissions"))
			.await?;
	assert_eq!(list_status, StatusCode::OK, "{list_body:?}");
	let items = list_body["data"]["items"]
		.as_array()
		.ok_or("missing submissions list")?;
	assert_eq!(items.len(), 1, "{list_body:?}");

	clear_esg_env();
	Ok(())
}
