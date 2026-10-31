use super::support::*;

#[serial]
#[tokio::test]
async fn test_primary_source_supports_regional_rp_fields() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let body = json!({"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"reporter_title_null_flavor": "UNK",
			"reporter_given_name": null,
			"reporter_given_name_null_flavor": "ASKU",
			"organization": null,
			"organization_null_flavor": "NASK",
			"telephone_null_flavor": "MSK",
			"country_code": "KR",
			"email": "mina.initial@example.test",
			"qualification": "3",
			"qualification_kr1": "1",
			"primary_source_regulatory": "1"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/primary-sources"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["reporter_title_null_flavor"], "UNK");
	assert_eq!(value["data"]["reporter_given_name"], Value::Null);
	assert_eq!(value["data"]["reporter_given_name_null_flavor"], "ASKU");
	assert_eq!(value["data"]["organization"], Value::Null);
	assert_eq!(value["data"]["organization_null_flavor"], "NASK");
	assert_eq!(value["data"]["telephone_null_flavor"], "MSK");
	assert!(value["data"].get("reporter_name_null_flavor").is_none());
	assert!(value["data"].get("reporter_address_null_flavor").is_none());
	assert_eq!(value["data"]["country_code"], "KR");
	assert_eq!(value["data"]["email"], "mina.initial@example.test");
	assert_eq!(value["data"]["qualification"], "3");
	assert_eq!(value["data"]["qualification_kr1"], "1");
	assert_eq!(value["data"]["primary_source_regulatory"], "1");
	let primary_source_id = extract_id(&body)?;

	let body = json!({"data": {
		"case_id": case_id,
		"sequence_number": 2,
		"reporter_given_name": "Backup",
		"organization": "Backup Reporter Org",
		"country_code": "KR",
		"qualification": "3",
		"primary_source_regulatory": "1"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/primary-sources"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);

	let body = json!({"data": {
		"email": "mina.updated@example.test",
		"reporter_given_name_null_flavor": "NASK",
		"organization_null_flavor": "MSK",
		"qualification_kr1": "2",
		"qualification_null_flavor": "UNK",
		"primary_source_regulatory": "2"
	}});
	let (status, body) = put_json_with_audit_reason(
		&app,
		&cookie,
		format!(
			"/api/cases/{case_id}/safety-report/primary-sources/{primary_source_id}"
		),
		body,
		"update primary source regional fields",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["email"], "mina.updated@example.test");
	assert_eq!(value["data"]["reporter_given_name_null_flavor"], "NASK");
	assert_eq!(value["data"]["organization_null_flavor"], "MSK");
	assert_eq!(value["data"]["qualification_kr1"], "2");
	assert_eq!(value["data"]["qualification_null_flavor"], "UNK");
	assert_eq!(value["data"]["primary_source_regulatory"], "2");

	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/primary-sources"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	let sources = value["data"]
		.as_array()
		.ok_or("missing primary source list")?;
	let primary_source_id_str = primary_source_id.to_string();
	let saved = sources
		.iter()
		.find(|source| source["id"].as_str() == Some(primary_source_id_str.as_str()))
		.ok_or("missing saved primary source")?;
	assert_eq!(saved["reporter_title_null_flavor"], "UNK");
	assert_eq!(saved["reporter_given_name_null_flavor"], "NASK");
	assert_eq!(saved["organization_null_flavor"], "MSK");
	assert_eq!(saved["telephone_null_flavor"], "MSK");
	assert!(saved.get("reporter_name_null_flavor").is_none());
	assert!(saved.get("reporter_address_null_flavor").is_none());
	assert_eq!(saved["qualification_null_flavor"], "UNK");

	Ok(())
}
