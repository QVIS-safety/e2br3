use crate::case_support::*;

#[serial]
#[tokio::test]
async fn test_reporter_validation_switches_from_collection_to_existing_fields(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm).expect("web server configuration");
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
			"sequenceNumber": 1,
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
	let app = web_server::app(mm).expect("web server configuration");
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
async fn test_validation_requires_explicit_authority() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone()).expect("web server configuration");

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) =
		get_validation(&app, &cookie, &format!("/api/cases/{case_id}/validation"))
			.await?;

	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	assert!(body.to_string().contains("authority is required"), "{body}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_validation_supports_mfds_authority() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone()).expect("web server configuration");

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
	let app = web_server::app(mm.clone()).expect("web server configuration");

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
	let app = web_server::app(mm.clone()).expect("web server configuration");

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
async fn test_validation_clean_report_updates_list_without_changing_status(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone()).expect("web server configuration");

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

	let (status, case) = crate::validation_common::get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{case}");
	assert_eq!(case["data"]["status"], "draft", "{case}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_validation_uses_explicit_authority_independent_of_settings_and_header(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm).expect("web server configuration");

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;
	create_message_header(&app, &cookie, case_id).await?;
	update_message_header_receiver(&app, &cookie, case_id, "MFDS-O-KR").await?;
	update_message_header_message_receiver(&app, &cookie, case_id, "MFDS-O-KR")
		.await?;
	create_sender(&app, &cookie, case_id, "3").await?;

	// This organization only enables ICH; the old implicit branch rejected MFDS.
	for authority in ["ich", "fda", "mfds"] {
		let (status, body) = get_validation(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/validation?authority={authority}"),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		assert_eq!(body["data"]["authority"], authority, "{body}");
	}
	let (status, body) =
		get_validation(&app, &cookie, &format!("/api/cases/{case_id}/validation"))
			.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	assert!(body.to_string().contains("authority is required"), "{body}");
	Ok(())
}
