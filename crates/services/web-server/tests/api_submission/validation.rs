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
async fn test_unavailable_meddra_allows_save_and_export_but_blocks_submission(
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
		assert_eq!(report["data"]["issue_count"], 1, "{report:?}");
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
		let xml = to_bytes(response.into_body(), usize::MAX).await?;
		assert_eq!(
			status,
			StatusCode::OK,
			"{authority}: {}",
			String::from_utf8_lossy(&xml)
		);
		assert!(String::from_utf8_lossy(&xml).contains("12.0"));
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
