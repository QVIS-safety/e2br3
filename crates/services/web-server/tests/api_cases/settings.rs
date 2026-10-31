use crate::case_support::*;

#[serial]
#[tokio::test]
async fn test_workflow_config_runtime_endpoint_returns_default_statuses(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = get_workflow_config(&app, &cookie).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert!(body["data"]["workflowEnabled"].is_boolean(), "{body:?}");
	assert_eq!(body["data"]["statuses"][0]["name"].as_str(), Some("Saved"));
	assert_eq!(
		body["data"]["statuses"][0]["editable"].as_bool(),
		Some(true)
	);
	if let Some(role) = body["data"]["statuses"][0]["allowedRoles"][0].as_str() {
		assert_eq!(role, role.to_ascii_lowercase(), "{body:?}");
	}
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_settings_round_trips_alignment_fields() -> Result<()> {
	let mm = init_test_mm().await?;
	lib_core::model::terminology_import::stage_meddra_rows(
		&mm,
		system_user_id(),
		&[lib_core::model::terminology_import::MeddraRow {
			code: "10000001".to_string(),
			term: "Settings regression term".to_string(),
			level: "LLT".to_string(),
		}],
		"28.0",
		"en",
		"settings-regression",
	)
	.await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({
			"data": {
				"timezone": "Asia/Seoul",
				"meddra_language": "English",
				"meddra_version": "28.0",
				"idf_version": "3.0",
				"company_logo": "qvis.png",
				"orientation": "Landscape",
				"data_ordering": "Primary data will appear first",
				"upload_excel_template_without_element_label": true,
				"notation": false,
				"apply_comments_on_exported_xml": true,
				"apply_sender_info_to_imported_cases": true,
				"import_date_update": {
					"date_of_creation": true,
					"most_recent_info_date": true,
					"report_first_received_date": false
				},
				"appendices": ["ICH", "FDA", "MFDS"],
				"case_number_setting": "AE Row No.",
				"case_number_identifier": "SAFETY",
				"case_number_padding": 7,
				"case_number_sequence_condition": "Per sender",
				"case_number_format_fields": ["AE Row No."],
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"allowed_roles": ["pvs"],
							"due_days": 0,
							"description": "Default state"
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let data = &body;
	assert_eq!(data["meddra_version"].as_str(), Some("28.0"));
	assert_eq!(data["idf_version"].as_str(), Some("3.0"));
	assert_eq!(data["orientation"].as_str(), Some("Landscape"));
	assert_eq!(data["apply_comments_on_exported_xml"].as_bool(), Some(true));
	assert_eq!(
		data["import_date_update"]["most_recent_info_date"].as_bool(),
		Some(true)
	);
	assert_eq!(
		data["case_number_format_fields"][0].as_str(),
		Some("AE Row No.")
	);
	assert_eq!(
		data["workflow"]["statuses"][0]["due_days"].as_i64(),
		Some(0)
	);

	let (status, body) = get_admin_settings(&app, &cookie).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let data = &body;
	assert_eq!(data["company_logo"].as_str(), Some("qvis.png"));
	assert_eq!(
		data["case_number_sequence_condition"].as_str(),
		Some("Per sender")
	);

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({"data": {"import_date_update": {
			"date_of_creation": true,
			"most_recent_info_date": false,
			"report_first_received_date": false
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({"data": {"import_date_update": {
			"date_of_creation": true,
			"most_recent_info_date": false,
			"report_first_received_date": true
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_number_settings_generate_c11_and_c181() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let identifier = format!("SAFETY{}", Uuid::new_v4().simple());
	let first_case_number = format!("{identifier}001");
	let second_case_number = format!("{identifier}002");

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({
			"data": {
				"case_number_identifier": identifier,
				"case_number_padding": 3,
				"case_number_format_fields": ["AE Row No."]
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, first) = create_case_with_payload(
		&app,
		&cookie,
		json!({
			"data": {
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{first:?}");
	let first_id =
		Uuid::parse_str(first["data"]["id"].as_str().ok_or("missing first id")?)?;

	let (status, second) = create_case_with_payload(
		&app,
		&cookie,
		json!({
			"data": {
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{second:?}");
	let second_id =
		Uuid::parse_str(second["data"]["id"].as_str().ok_or("missing second id")?)?;

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{first_id}/safety-report"))
		.header("cookie", &cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let safety_report: Value = serde_json::from_slice(&body)?;
	assert_eq!(
		status,
		StatusCode::OK,
		"safety report status {} body {}",
		status,
		String::from_utf8_lossy(&body)
	);
	assert_eq!(
		safety_report["data"]["safety_report_id"].as_str(),
		Some(first_case_number.as_str())
	);
	assert_eq!(safety_report["data"]["version"].as_i64(), Some(1));
	assert_eq!(
		safety_report["data"]["worldwide_unique_id"].as_str(),
		Some(first_case_number.as_str())
	);

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{second_id}/safety-report"))
		.header("cookie", &cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let safety_report: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::OK, "{safety_report:?}");
	assert_eq!(
		safety_report["data"]["safety_report_id"].as_str(),
		Some(second_case_number.as_str())
	);
	assert_eq!(safety_report["data"]["version"].as_i64(), Some(1));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_workflow_settings_reject_negative_due_days() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"allowed_roles": ["pvs"],
							"due_days": -1
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(body.to_string().contains("due_days"), "{body:?}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_workflow_settings_reject_unknown_role() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"allowed_roles": ["not_a_real_role"],
							"due_days": 0
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string().contains("unknown role 'not_a_real_role'"),
		"{body:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_workflow_settings_reject_system_admin_role() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"allowed_roles": ["system_admin"],
							"due_days": 0
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string().contains("unknown role 'system_admin'"),
		"{body:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_workflow_settings_allow_empty_roles_as_unrestricted() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"allowed_roles": [],
							"due_days": 0
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["workflow"]["statuses"][0]["allowed_roles"], json!([]));
	Ok(())
}
