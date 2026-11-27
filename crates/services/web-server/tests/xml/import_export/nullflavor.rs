use super::support::*;

#[serial]
#[tokio::test(flavor = "multi_thread")]
async fn study_numbers_and_null_flavors_survive_cross_organization_export_import(
) -> Result<()> {
	init_test_env().await?;
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "admin_pwd", "viewer_pwd").await?;
	let product =
		create_test_import_product(&mm, seed.org_id, seed.admin.id).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let target_seed =
		seed_org_with_users(&mm, "target_admin_pwd", "target_viewer_pwd").await?;
	assert_ne!(seed.org_id, target_seed.org_id);
	let target_product =
		create_test_import_product(&mm, target_seed.org_id, target_seed.admin.id)
			.await?;
	let target_token =
		generate_web_token(&target_seed.admin.email, target_seed.admin.token_salt)?;
	let target_cookie = cookie_header(&target_token.to_string());

	let ctx = lib_core::ctx::Ctx::new(
		seed.admin.id,
		seed.org_id,
		lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let app = web_server::app(mm.clone())?;
	let source = scenario6_xml()?;
	assert!(source.contains("extension=\"23489-34\""));
	let mut xml =
		source.replacen("extension=\"23489-34\"", "nullFlavor=\"ASKU\"", 1);
	let mut fda = String::new();
	for (suffix, number) in [
		("1", "012345"),
		("2", "054321"),
		("3", "078901"),
		("3", "023456"),
	] {
		fda.push_str(&format!(r#"<authorization typeCode="AUTH"><studyRegistration classCode="ACT" moodCode="EVN"><id root="2.16.840.1.113883.3.989.5.1.2.2.1.2.{suffix}" extension="{number}"/></studyRegistration></authorization>"#));
	}
	assert_eq!(xml.matches("</researchStudy>").count(), 1);
	xml = xml.replacen("</researchStudy>", &format!("{fda}</researchStudy>"), 1);
	let mut source_case_id = None;
	for (pass, (cookie, product)) in
		[(&cookie, product), (&target_cookie, target_product)]
			.into_iter()
			.enumerate()
	{
		let imported =
			import_xml_string(&app, &cookie, "study-values.xml", &xml, product)
				.await?;
		let case_id = import_case_id(&imported)?;
		if pass == 0 {
			source_case_id = Some(case_id.to_owned());
		} else {
			assert_ne!(
				source_case_id.as_deref(),
				Some(case_id),
				"must persist a distinct target-organization case"
			);
		}

		let (status, body) = request_json(
			&app,
			&cookie,
			"GET",
			format!("/api/cases/{case_id}/safety-report/studies"),
			None,
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
		let studies: Value = serde_json::from_slice(&body)?;
		let studies = studies["data"].as_array().ok_or("missing studies")?;
		assert_eq!(studies.len(), 1);
		let study = &studies[0];
		assert_eq!(study["fda_ind_number_occurred"], "012345", "pass {pass}");
		assert_eq!(
			study["fda_pre_anda_number_occurred"], "054321",
			"pass {pass}"
		);
		let study_id = study["id"].as_str().ok_or("missing study ID")?;
		for collection in ["registrations", "fda-cross-reported-inds"] {
			let (status, body) = request_json(&app, &cookie, "GET", format!("/api/cases/{case_id}/safety-report/studies/{study_id}/{collection}"), None).await?;
			assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
			let rows: Value = serde_json::from_slice(&body)?;
			let rows = rows["data"].as_array().ok_or("missing study children")?;
			assert_eq!(rows.len(), 2, "{collection}, pass {pass}");
			if collection == "registrations" {
				let masked = rows
					.iter()
					.find(|row| row["registration_number_null_flavor"] == "ASKU")
					.ok_or("missing ASKU registration")?;
				assert!(
					masked["registration_number"].is_null(),
					"must store SQL NULL, not an empty string"
				);
				assert_eq!(masked["country_code"], "US");
				assert!(rows
					.iter()
					.any(|row| row["registration_number"] == "876444"
						&& row["country_code"] == "FR"));
			} else {
				for expected in ["078901", "023456"] {
					assert!(rows.iter().any(|row| row["ind_number"] == expected));
				}
			}
		}
		if pass == 0 {
			let header = xml::OutboundMessageHeader {
				batch_number: Some("STUDY-ROUNDTRIP".into()),
				batch_sender_identifier: Some("TEST-SENDER".into()),
				batch_receiver_identifier: Some("ZZFDA".into()),
				batch_transmission_date: Some("20261122080000+0000".into()),
				message_number: Some("STUDY-ROUNDTRIP".into()),
				message_date: Some("20261122080000+0000".into()),
				message_sender_identifier: Some("TEST-SENDER".into()),
				message_receiver_identifier: Some("CDER".into()),
			};
			xml = xml::export::serialize_case_xml_for_authority(
				&ctx,
				&mm,
				Uuid::parse_str(case_id)?,
				lib_core::regulatory::RegulatoryAuthority::Fda,
				&header,
			)
			.await?;
		}
	}
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_import_f_nullflavor_then_readback_test_results() -> Result<()> {
	init_test_env().await?;

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "admin_pwd", "viewer_pwd").await?;
	let product_presave_id =
		create_test_import_product(&mm, seed.org_id, seed.admin.id).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm).expect("web server configuration");
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

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "admin_pwd", "viewer_pwd").await?;
	let product_presave_id =
		create_test_import_product(&mm, seed.org_id, seed.admin.id).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm).expect("web server configuration");
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
