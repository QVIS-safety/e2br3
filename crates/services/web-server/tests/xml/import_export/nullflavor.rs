use super::support::*;

#[serial]
#[tokio::test]
async fn test_import_f_nullflavor_then_readback_test_results() -> Result<()> {
	init_test_env().await?;
	std::env::set_var("E2BR3_XSD_PATH", resolved_xsd_path());

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "admin_pwd", "viewer_pwd").await?;
	let product_presave_id =
		create_test_import_product(&mm, seed.org_id, seed.admin.id).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let xml = scenario6_with_first_test_date_null_flavor()?;

	let import_value = import_xml_string(
		&app,
		&cookie,
		"FAERS2022Scenario6-nullflavor.xml",
		&xml,
		product_presave_id,
	)
	.await?;
	let case_id = import_case_id(&import_value)?;

	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/test-results"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	let tests = value["data"]
		.as_array()
		.ok_or("missing test results array")?;
	assert!(
		tests.iter().any(|row| {
			row.get("test_name").and_then(Value::as_str) == Some("Calcium Level")
				&& row.get("test_date_null_flavor").and_then(Value::as_str)
					== Some("UNK")
		}),
		"expected imported test result with test_date_null_flavor=UNK, got {value}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_import_e_nullflavor_then_readback_reactions() -> Result<()> {
	init_test_env().await?;
	std::env::set_var("E2BR3_XSD_PATH", resolved_xsd_path());

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "admin_pwd", "viewer_pwd").await?;
	let product_presave_id =
		create_test_import_product(&mm, seed.org_id, seed.admin.id).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let xml = scenario6_xml()?;

	let import_value = import_xml_string(
		&app,
		&cookie,
		"FAERS2022Scenario6.xml",
		&xml,
		product_presave_id,
	)
	.await?;
	let case_id = import_case_id(&import_value)?;

	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/reactions"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	let reactions = value["data"].as_array().ok_or("missing reactions array")?;
	assert!(
		reactions.iter().any(|row| {
			row.get("criteria_death_null_flavor")
				.and_then(Value::as_str)
				== Some("NI")
		}),
		"expected imported death nullFlavor, got {value}"
	);
	assert!(
		reactions.iter().any(|row| {
			row.get("start_date_null_flavor").and_then(Value::as_str) == Some("NASK")
		}),
		"expected imported start-date nullFlavor, got {value}"
	);
	Ok(())
}
