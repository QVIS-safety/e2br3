use super::support::*;

fn batch_timestamp(xml: &[u8]) -> Result<String> {
	xpath_value(xml, "string(/*[local-name()='MCCI_IN200100UV01']/*[local-name()='creationTime']/@value)")
}

#[serial]
#[tokio::test]
async fn test_import_export_preserves_batch_timestamp_and_explicit_edit_replaces_it(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "admin_pwd", "viewer_pwd").await?;
	let product_id =
		create_test_import_product(&mm, seed.org_id, seed.admin.id).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm).expect("web server configuration");
	let source = fixture_xml("FAERS2022Scenario2.xml")?;
	let old = batch_timestamp(source.as_bytes())?;
	let raw = "20240229123059.1234+0900";
	let xml = source.replacen(
		&format!("<creationTime value=\"{old}\"/>"),
		&format!("<creationTime value=\"{raw}\"/>"),
		1,
	);
	assert_ne!(xml, source, "exact fixture root timestamp must be replaced");
	let imported =
		import_xml_string(&app, &cookie, "batch-date.xml", &xml, product_id).await?;
	let case_id = import_case_id(&imported)?;
	let header_path = format!("/api/cases/{case_id}/message-header");
	let (status, bytes) =
		request_json(&app, &cookie, "GET", header_path.clone(), None).await?;
	assert_eq!(status, StatusCode::OK);
	let header: Value = serde_json::from_slice(&bytes)?;
	assert_eq!(header["data"]["batch_transmission_date_raw"], raw);
	assert_eq!(header["data"]["batch_transmission_date"], Value::Null);
	let export_path = format!("/api/cases/{case_id}/export/xml?authority=fda");
	let (status, first) =
		request_json(&app, &cookie, "GET", export_path.clone(), None).await?;
	assert_eq!(
		status,
		StatusCode::OK,
		"{}",
		String::from_utf8_lossy(&first)
	);
	assert_eq!(batch_timestamp(&first)?, raw);
	let (status, second) =
		request_json(&app, &cookie, "GET", export_path.clone(), None).await?;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(
		first, second,
		"repeated downloads must not rewrite stored values"
	);
	let (status, bytes) = request_json(
		&app,
		&cookie,
		"PUT",
		header_path.clone(),
		Some(
			serde_json::json!({"data":{"batch_transmission_date":[2024,32,1,1,1,0,0,0,0]}}),
		),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::OK,
		"{}",
		String::from_utf8_lossy(&bytes)
	);
	let (status, bytes) =
		request_json(&app, &cookie, "GET", header_path, None).await?;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(
		serde_json::from_slice::<Value>(&bytes)?["data"]
			["batch_transmission_date_raw"],
		Value::Null
	);
	let (status, bytes) =
		request_json(&app, &cookie, "GET", export_path, None).await?;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(batch_timestamp(&bytes)?, "20240201010101+0000");
	Ok(())
}
