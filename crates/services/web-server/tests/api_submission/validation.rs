use super::support::*;

#[serial]
#[tokio::test]
async fn test_submission_does_not_require_case_validated_status() -> Result<()> {
	clear_esg_env();
	configure_mock_esg_transport().await?;
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_rejects_case_validation_before_routing_or_dispatch(
) -> Result<()> {
	use lib_core::ctx::Ctx;
	use lib_core::model::case_validation_summary::CaseValidationSummaryBmc;
	use lib_core::regulatory::RegulatoryAuthority;
	clear_esg_env();
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;
	let (status, report) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/validation?authority=fda"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{report:?}");
	let cached = CaseValidationSummaryBmc::cached_totals_by_case(
		&Ctx::root_ctx(),
		&mm,
		&[case_id],
		&[RegulatoryAuthority::Fda],
		validator::VALIDATOR_REVISION,
	)
	.await?;
	assert!(cached.contains_key(&case_id));
	let (status, body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/patient"),
		json!({"data": {"age_at_time_of_onset": 40, "age_unit": "a", "age_group": "5"}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let cached = CaseValidationSummaryBmc::cached_totals_by_case(
		&Ctx::root_ctx(),
		&mm,
		&[case_id],
		&[RegulatoryAuthority::Fda],
		validator::VALIDATOR_REVISION,
	)
	.await?;
	assert!(
		!cached.contains_key(&case_id),
		"patient update must invalidate cached counts"
	);
	for authority in ["fda", "mfds"] {
		let (status, body) = post_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/submissions/{authority}"),
			valid_compliance_payload(),
		)
		.await?;
		assert_eq!(status, StatusCode::BAD_REQUEST, "{authority}: {body:?}");
		assert!(body.to_string().contains("validation issue(s)"), "{body:?}");
	}
	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{case_id}/export/xml?authority=ich"))
		.header("cookie", &cookie)
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_unavailable_meddra_allows_save_but_blocks_export_and_submission(
) -> Result<()> {
	clear_esg_env();
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;
	let (status, reactions) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/reactions")).await?;
	assert_eq!(status, StatusCode::OK, "{reactions:?}");
	let reaction_id = reactions["data"][0]["id"]
		.as_str()
		.ok_or("missing reaction id")?;
	let (status, saved) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/reactions/{reaction_id}"),
		json!({ "data": { "reaction_meddra_version": "12.0" } }),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{saved:?}");
	for authority in ["ich", "fda", "mfds"] {
		if authority == "mfds" {
			// The fixture is a US report, so use the foreign-report receiver.
			let (status, body) = put_json(&app, &cookie, &format!("/api/cases/{case_id}/message-header"), json!({"data": {
				"batch_receiver_identifier":"MFDS-O-FR", "message_receiver_identifier":"MFDS-O-FR"
			}})).await?;
			assert_eq!(status, StatusCode::OK, "{body:?}");
		}
		let (status, report) = get_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/validation?authority={authority}"),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{report:?}");
		let issue = report["data"]["issues"]
			.as_array()
			.ok_or("missing issues")?
			.iter()
			.find(|issue| issue["code"] == "ICH.MEDDRA.VERSION.UNAVAILABLE")
			.ok_or("missing dictionary issue")?;
		assert!(issue.get("blocking").is_none());
		assert_eq!(issue["message"], "MedDRA 12.0 is not loaded");
		let mut codes = report["data"]["issues"]
			.as_array()
			.ok_or("missing issues")?
			.iter()
			.map(|issue| issue["code"].as_str().ok_or("missing issue code"))
			.collect::<std::result::Result<Vec<_>, _>>()?;
		codes.sort_unstable();
		let expected = if authority == "mfds" {
			// A valid FR receiver also requires the fixture's absent product/substance code.
			vec![
				"ICH.MEDDRA.VERSION.UNAVAILABLE",
				"MFDS.G.k.2.3.r.1.KR.1b.REQUIRED",
			]
		} else {
			vec!["ICH.MEDDRA.VERSION.UNAVAILABLE"]
		};
		assert_eq!(codes, expected, "{report:?}");
		assert_eq!(report["data"]["issue_count"], expected.len(), "{report:?}");
		let response = app
			.clone()
			.oneshot(
				Request::builder()
					.uri(format!(
						"/api/cases/{case_id}/export/xml?authority={authority}"
					))
					.header("cookie", &cookie)
					.body(Body::empty())?,
			)
			.await?;
		let status = response.status();
		let body = to_bytes(response.into_body(), usize::MAX).await?;
		assert_eq!(
			status,
			StatusCode::BAD_REQUEST,
			"{authority}: {}",
			String::from_utf8_lossy(&body)
		);
		assert!(String::from_utf8_lossy(&body)
			.contains("Correct the highlighted case fields"));
		if authority != "ich" {
			let (status, error) = post_json(
				&app,
				&cookie,
				&format!("/api/cases/{case_id}/submissions/{authority}"),
				valid_compliance_payload(),
			)
			.await?;
			assert_eq!(status, StatusCode::BAD_REQUEST, "{error:?}");
			assert!(
				error.to_string().contains("MedDRA 12.0 is not loaded"),
				"{error:?}"
			);
		}
	}
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_editor_null_flavor_persists_and_exports() -> Result<()> {
	clear_esg_env();
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;

	let (status, created) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/LB/rows"),
		json!({"authorities":["fda"],"rows":{"testResult":{
			"sequenceNumber":1,"testName":"NullFlavor persistence",
			"testDate":null,"testDateNullFlavor":"UNK"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{created}");
	let row_id = created["rowId"]
		.as_str()
		.ok_or("missing LB rowId")?
		.to_string();
	let request = Request::builder()
		.method("PATCH")
		.uri(format!(
			"/api/cases/{case_id}/editor/pages/LB/rows/{row_id}"
		))
		.header("cookie", &cookie)
		.header("content-type", "application/json")
		.body(Body::from(
			json!({"authorities":["fda"],"rows":{"testResult":{
				"testDate":null,"testDateNullFlavor":"NASK"
			}}})
			.to_string(),
		))?;
	let response = app.clone().oneshot(request).await?;
	let status = response.status();
	let body = to_bytes(response.into_body(), usize::MAX).await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));

	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	let stored: (Option<String>, Option<String>) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as(
				"SELECT test_date, test_date_null_flavor FROM test_results WHERE id = $1",
			)
			.bind(Uuid::parse_str(&row_id)?),
		)
		.await?;
	mm.dbx().commit_txn().await?;
	assert_eq!(stored, (None, Some("NASK".to_string())));

	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.uri(format!("/api/cases/{case_id}/export/xml?authority=fda"))
				.header("cookie", &cookie)
				.body(Body::empty())?,
		)
		.await?;
	let status = response.status();
	let xml = to_bytes(response.into_body(), usize::MAX).await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&xml));
	assert!(String::from_utf8_lossy(&xml)
		.contains("<effectiveTime nullFlavor=\"NASK\"/>"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_manual_export_is_not_blocked_by_workflow_status() -> Result<()> {
	clear_esg_env();
	std::env::set_var("E2BR3_EXPORT_VALIDATE_FDA", "0");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	for status in ["draft", "reviewed", "locked"] {
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
				sqlx::query("UPDATE cases SET status = $1 WHERE id = $2")
					.bind(status)
					.bind(case_id),
			)
			.await?;
		mm.dbx().commit_txn().await?;

		let req = Request::builder()
			.method("GET")
			.uri(format!("/api/cases/{case_id}/export/xml?authority=fda"))
			.header("cookie", &cookie)
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		let response_status = res.status();
		let body = to_bytes(res.into_body(), usize::MAX).await?;

		assert_eq!(
			response_status,
			StatusCode::OK,
			"status={status} body={}",
			String::from_utf8_lossy(&body)
		);
	}

	Ok(())
}
