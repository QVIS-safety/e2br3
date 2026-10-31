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
