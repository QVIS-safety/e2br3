use crate::case_support::*;

#[serial]
#[tokio::test]
async fn test_reporter_validation_switches_from_collection_to_existing_fields(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let validation_uri = format!("/api/cases/{case_id}/validation?authority=ich");
	let (status, report) =
		crate::validation_common::get_json(&app, &cookie, validation_uri.clone())
			.await?;
	assert_eq!(status, StatusCode::OK, "{report}");
	let paths: Vec<_> = report["data"]["issues"]
		.as_array()
		.ok_or("missing issues")?
		.iter()
		.filter_map(|issue| issue["path"].as_str())
		.filter(|path| path.starts_with("primarySources"))
		.collect();
	assert_eq!(paths, vec!["primarySources"]);
	assert_eq!(report["data"]["ok"], false);

	let (status, saved) = editor_patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/RP"),
		json!({"authorities": ["ich"], "rows": {"primarySources": [{
			"reporterGivenName": "Reporter with missing qualification",
			"primarySourceForRegulatoryPurposes": "1"
		}]}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{saved}");
	assert_eq!(saved["rows"]["primarySources"].as_array().unwrap().len(), 1);
	let (status, report) =
		crate::validation_common::get_json(&app, &cookie, validation_uri.clone())
			.await?;
	assert_eq!(status, StatusCode::OK, "{report}");
	let paths: Vec<_> = report["data"]["issues"]
		.as_array()
		.ok_or("missing issues")?
		.iter()
		.filter_map(|issue| issue["path"].as_str())
		.filter(|path| path.starts_with("primarySources"))
		.collect();
	assert!(!paths.contains(&"primarySources"), "{paths:?}");
	assert!(
		paths.contains(&"primarySources.0.qualification"),
		"{paths:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_study_sequence_gaps_keep_validation_bound_to_editor_rows() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/SI");
	let (status, body) = editor_patch_json(
		&app,
		&cookie,
		&uri,
		json!({
			"authorities": ["ich"], "rows": {
				"studyInformation": {"studyName": "Sequence gaps"},
				"studyRegistrationNumbers": [
					{"registrationNumber": "LATER", "countryCode": "US", "sequenceNumber": 9},
					{"registrationNumber": "FIRST", "countryCode": "ZX", "sequenceNumber": 4}
				]
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (status, body) =
		crate::validation_common::get_json(&app, &cookie, uri.clone()).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["rows"]["studyRegistrationNumbers"][0]["registration_number"],
		"FIRST"
	);
	assert_eq!(
		body["rows"]["studyRegistrationNumbers"][1]["registration_number"],
		"LATER"
	);
	let (status, report) = crate::validation_common::get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/validation?authority=ich"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{report}");
	let paths: Vec<_> = report["data"]["issues"]
		.as_array()
		.ok_or("missing issues")?
		.iter()
		.filter(|issue| issue["code"] == "ICH.C.5.1.r.2.VOCABULARY")
		.map(|issue| issue["path"].as_str().unwrap())
		.collect();
	assert_eq!(
		paths,
		vec!["studyInformation.0.registrations.0.registrationCountry"],
		"{report}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_validation_defaults_to_ich_profile() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) =
		get_validation(&app, &cookie, &format!("/api/cases/{case_id}/validation"))
			.await?;

	assert_eq!(status, StatusCode::OK);
	assert_eq!(body["data"]["authority"], "ich");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_validation_supports_mfds_authority() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;
	create_sender(&app, &cookie, case_id, "3").await?;

	let (status, body) = get_validation(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/validation?authority=mfds"),
	)
	.await?;

	assert_eq!(status, StatusCode::OK);
	assert_eq!(body["data"]["authority"], "mfds");
	assert_eq!(body["data"]["authority"], "mfds");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_validation_rejects_conflicting_authority_profile() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) = get_validation(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/validation?authority=fda&authority=mfds"),
	)
	.await?;

	assert_eq!(status, StatusCode::BAD_REQUEST);
	assert!(
		body.to_string().contains("duplicate field `authority`"),
		"unexpected body={body}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_validation_rejects_unknown_profile() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) = get_validation(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/validation?authority=unknown"),
	)
	.await?;

	assert_eq!(status, StatusCode::BAD_REQUEST);
	assert!(
		body.to_string().contains("invalid validation authority"),
		"unexpected body={body}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_validator_endpoint_requires_token() -> Result<()> {
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) =
		validator_mark_validated(&app, &cookie, case_id, None).await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(body["error"]["data"]["detail"]
		.as_str()
		.unwrap_or_default()
		.contains("invalid validator token"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_validator_endpoint_rejects_invalid_cases() -> Result<()> {
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) =
		validator_mark_validated(&app, &cookie, case_id, Some("validator-secret"))
			.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(body["error"]["data"]["detail"]
		.as_str()
		.unwrap_or_default()
		.contains("validation issue(s) remain"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_validator_endpoint_marks_validated_when_clean() -> Result<()> {
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;

	let (status, validation) = crate::validation_common::get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/validation?authority=ich"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{validation:?}");
	assert_eq!(validation["data"]["authority"], "ich", "{validation:?}");
	assert_eq!(validation["data"]["ok"], true, "{validation:?}");
	assert_eq!(validation["data"]["issue_count"], 0, "{validation:?}");
	assert_eq!(
		validation["data"]["issues"].as_array().map(Vec::len),
		Some(0),
		"{validation:?}"
	);

	let (status, list) = crate::validation_common::get_json(
		&app,
		&cookie,
		"/api/cases/list-view?authorities=ich&list_options%5Blimit%5D=100"
			.to_string(),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{list:?}");
	let case_id_string = case_id.to_string();
	let row = list["data"]["items"]
		.as_array()
		.ok_or("missing list-view items")?
		.iter()
		.find(|item| item["caseId"].as_str() == Some(case_id_string.as_str()))
		.ok_or("missing clean validated case in list view")?;
	assert_eq!(row["warn"].as_str(), Some("0"), "{row:?}");
	assert_ne!(row["warn"].as_str(), Some("Not checked"), "{row:?}");

	let (status, body) =
		validator_mark_validated(&app, &cookie, case_id, Some("validator-secret"))
			.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["status"].as_str(), Some("validated"));

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	dbx.execute(sqlx::query("SET ROLE e2br3_auditor_role"))
		.await?;
	let reason = dbx
		.fetch_optional(
			sqlx::query_as::<_, (Option<String>,)>(
				r#"
				SELECT reason_for_change
				FROM audit_logs
				WHERE table_name = 'cases'
				  AND record_id = $1
				  AND action = 'UPDATE'
				  AND changed_fields ? 'status'
				  AND changed_fields->'status'->>'new' = 'validated'
				ORDER BY id DESC
				LIMIT 1
				"#,
			)
			.bind(case_id),
		)
		.await?;
	dbx.rollback_txn().await?;
	let reason = reason
		.and_then(|(v,)| v)
		.unwrap_or_default()
		.to_ascii_lowercase();
	assert!(
		reason.contains("system validation"),
		"expected system validation audit reason, got: {reason}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_validation_infers_mfds_profile_from_batch_receiver() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({"data": {"appendices": ["ICH", "MFDS"]}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	create_safety_report(&app, &cookie, case_id).await?;
	create_message_header(&app, &cookie, case_id).await?;
	update_message_header_receiver(&app, &cookie, case_id, "MFDS-O-KR").await?;
	update_message_header_message_receiver(&app, &cookie, case_id, "MFDS-O-KR")
		.await?;
	create_sender(&app, &cookie, case_id, "3").await?;

	let (status, body) =
		get_validation(&app, &cookie, &format!("/api/cases/{case_id}/validation"))
			.await?;

	assert_eq!(status, StatusCode::OK);
	assert_eq!(body["data"]["authority"], "mfds");
	assert!(
		body["data"]["issues"]
			.as_array()
			.map(|items| {
				items
					.iter()
					.any(|issue| issue["code"] == "MFDS.C.3.1.KR.1.REQUIRED")
			})
			.unwrap_or(false),
		"expected MFDS issue from inferred authority, body={body}"
	);
	Ok(())
}
