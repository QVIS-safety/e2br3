use super::support::*;

#[serial]
#[tokio::test(flavor = "multi_thread")]
async fn fda_export_reads_persisted_sections_through_rls() -> Result<()> {
	init_test_env().await?;
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "admin_pwd", "viewer_pwd").await?;
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let (status, body) = request_json(
		&app,
		&cookie,
		"POST",
		"/api/cases".to_string(),
		Some(serde_json::json!({
			"data": {
				"status": "draft",
				"safetyReportIdentification": {
					"safetyReportId": format!("N-RLS-{}", Uuid::new_v4())
				}
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let case_id = extract_data_id(&body)?;
	let (status, body) = request_json(
		&app,
		&cookie,
		"POST",
		format!("/api/cases/{case_id}/message-header"),
		Some(serde_json::json!({
			"data": {
				"case_id": case_id,
				"message_number": format!("MSG-{case_id}"),
				"message_sender_identifier": "SENDER01",
				"message_receiver_identifier": "RECEIVER01",
				"message_date": "20260805065000",
				"batch_receiver_identifier": "ZZFDA"
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/message-header"),
		Some(serde_json::json!({
			"data": { "batch_receiver_identifier": "ZZFDA" }
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let (status, body) = request_json(
		&app,
		&cookie,
		"POST",
		format!("/api/cases/{case_id}/patient"),
		Some(serde_json::json!({
			"data": {
				"case_id": case_id,
				"patient_initials": "NR",
				"sex": "1"
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let patient_id = extract_data_id(&body)?;
	let (status, body) = request_json(
		&app,
		&cookie,
		"POST",
		format!("/api/cases/{case_id}/patient/death-info"),
		Some(serde_json::json!({
			"data": {
				"patient_id": patient_id,
				"date_of_death": "2026-08-04",
				"autopsy_performed": true
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let death_info_id = extract_data_id(&body)?;
	let persisted_death = lib_core::model::patient::PatientDeathInformationBmc::get(
		&ctx,
		&mm,
		Uuid::parse_str(&death_info_id)?,
	)
	.await?;
	assert!(
		persisted_death.date_of_death.is_some(),
		"death setup did not persist date: {persisted_death:?}"
	);
	for (path, code, comments) in [
		("reported-causes", "10036807", "RLS-visible reported cause"),
		("autopsy-causes", "10067063", "RLS-visible autopsy cause"),
	] {
		let (status, body) = request_json(
			&app,
			&cookie,
			"POST",
			format!(
				"/api/cases/{case_id}/patient/death-info/{death_info_id}/{path}"
			),
			Some(serde_json::json!({
				"data": {
					"death_info_id": death_info_id,
					"sequence_number": 1,
					"meddra_version": "28.1",
					"meddra_code": code,
					"comments": comments
				}
			})),
		)
		.await?;
		assert_eq!(
			status,
			StatusCode::CREATED,
			"{}",
			String::from_utf8_lossy(&body)
		);
	}
	let (status, body) = request_json(
		&app,
		&cookie,
		"POST",
		format!("/api/cases/{case_id}/narrative"),
		Some(serde_json::json!({
			"data": { "case_id": case_id, "case_narrative": "N.2.r.4 test" }
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);

	let transmission_date = "20260805065000-0400";
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/safety-report"),
		Some(serde_json::json!({
			"data": {
				"transmission_date": transmission_date,
				"report_type": "2",
				"fulfil_expedited_criteria": true,
				"local_criteria_report_type": "1",
				"other_case_identifiers_exist": true
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));

	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/safety-report"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let report: Value = serde_json::from_slice(&body)?;
	assert_eq!(report["data"]["transmission_date"], transmission_date);
	assert_eq!(report["data"]["local_criteria_report_type"], "1");

	let mut study_id = None;
	for (uri, data) in [
		(
			format!("/api/cases/{case_id}/safety-report/senders"),
			serde_json::json!({
				"case_id": case_id,
				"sender_type": "1",
				"organization_name": "QVIS Safety CRO",
				"department": "Drug Safety",
				"street_address": "123 Teheran-ro",
				"city": "Seoul",
				"state": "Seoul",
				"postcode": "06236",
				"country_code": "KR",
				"person_title": "Ms",
				"person_given_name": "Sara J",
				"person_family_name": "Lee",
				"telephone": "+82-2-555-0100",
				"fax": "+82-2-555-0101",
				"email": "sara.lee@example.com"
			}),
		),
		(
			format!("/api/cases/{case_id}/safety-report/studies"),
			serde_json::json!({
				"case_id": case_id,
				"study_name": "QVIS Clinical Trial",
				"sponsor_study_number": "QVIS-CT-2026-004",
				"study_type_reaction": "1",
				"fda_ind_number_occurred": "123456",
				"fda_pre_anda_number_occurred": "654321"
			}),
		),
		(
			format!("/api/cases/{case_id}/other-identifiers"),
			serde_json::json!({
				"case_id": case_id,
				"sequence_number": 1,
				"source_of_identifier": "QVIS Safety CRO",
				"case_identifier": "KR-QVIS-PRIOR-2026-004"
			}),
		),
	] {
		let is_study = uri.ends_with("/studies");
		let (status, body) = request_json(
			&app,
			&cookie,
			"POST",
			uri,
			Some(serde_json::json!({ "data": data })),
		)
		.await?;
		assert_eq!(
			status,
			StatusCode::CREATED,
			"{}",
			String::from_utf8_lossy(&body)
		);
		if is_study {
			study_id = Some(extract_data_id(&body)?);
		}
	}
	let study_id = study_id.ok_or("missing persisted study id")?;
	let (status, body) = request_json(
		&app,
		&cookie,
		"POST",
		format!(
			"/api/cases/{case_id}/safety-report/studies/{study_id}/fda-cross-reported-inds"
		),
		Some(serde_json::json!({
			"data": {
				"study_information_id": study_id,
				"ind_number": "789012",
				"sequence_number": 1
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	for (uri, data) in [
		(
			format!("/api/cases/{case_id}/reactions"),
			serde_json::json!({
				"case_id": case_id,
				"sequence_number": 1,
				"primary_source_reaction": "RLS-visible Headache",
				"reaction_language": "eng",
				"reaction_meddra_version": "28.1",
				"reaction_meddra_code": "10019211",
				"serious": false,
				"criteria_death": false,
				"criteria_life_threatening": false,
				"criteria_hospitalization": false,
				"criteria_disabling": false,
				"criteria_congenital_anomaly": false,
				"criteria_other_medically_important": false,
				"required_intervention": false,
				"outcome": "1"
			}),
		),
		(
			format!("/api/cases/{case_id}/test-results"),
			serde_json::json!({
				"case_id": case_id,
				"sequence_number": 1,
				"test_date": "2026-08-05",
				"test_name": "RLS-visible CBC",
				"result_unstructured": "Normal",
				"more_info_available": false
			}),
		),
	] {
		let (status, body) = request_json(
			&app,
			&cookie,
			"POST",
			uri,
			Some(serde_json::json!({ "data": data })),
		)
		.await?;
		assert_eq!(
			status,
			StatusCode::CREATED,
			"{}",
			String::from_utf8_lossy(&body)
		);
	}

	let case_id = Uuid::parse_str(&case_id)?;
	let authority = lib_core::regulatory::RegulatoryAuthority::Fda;
	let header = outbound_header(authority);
	let exported = tokio::task::block_in_place(|| {
		tokio::runtime::Handle::current().block_on(
			xml::export::serialize_case_xml_for_authority(
				&ctx, &mm, case_id, authority, &header,
			),
		)
	})?;
	assert!(
		exported.contains(&format!("<creationTime value=\"{transmission_date}\"")),
		"N.2.r.4 did not use persisted C.1.2"
	);
	let deceased = exported
		.split_once("<deceasedTime")
		.and_then(|(_, rest)| rest.split_once('>'))
		.map(|(tag, _)| tag)
		.ok_or("missing FDA D deceasedTime")?;
	assert!(
		deceased.contains("value=\"20260804\""),
		"wrong FDA D deceasedTime: {deceased}"
	);
	for expected in [
		"<originalText>RLS-visible reported cause</originalText>",
		"<originalText>RLS-visible autopsy cause</originalText>",
	] {
		assert!(
			exported.contains(expected),
			"missing FDA D node: {expected}"
		);
	}
	assert!(
		exported.contains("code=\"10019211\" codeSystem=\"2.16.840.1.113883.6.163\" codeSystemVersion=\"28.1\""),
		"persisted E.i.2.1 MedDRA coding missing"
	);
	assert!(
		exported.contains("displayName=\"RLS-visible CBC\""),
		"persisted F.r test result missing"
	);
	for expected in [
		"<code code=\"C54588\" codeSystem=\"2.16.840.1.113883.3.26.1.1\"/>",
		"<value xsi:type=\"CE\" code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.5.1.2.2.1.1.1\"/>",
		"<code codeSystem=\"2.16.840.1.113883.3.989.2.1.1.7\" code=\"1\"/>",
		"<streetAddressLine>123 Teheran-ro</streetAddressLine>",
		"<city>Seoul</city>",
		"<state>Seoul</state>",
		"<postalCode>06236</postalCode>",
		"<prefix>Ms</prefix>",
		"<given>Sara J</given>",
		"<family>Lee</family>",
		"<telecom value=\"tel:+82-2-555-0100\"/>",
		"<telecom value=\"fax:+82-2-555-0101\"/>",
		"<telecom value=\"mailto:sara.lee@example.com\"/>",
		"<name>Drug Safety</name>",
		"<name>QVIS Safety CRO</name>",
		"<code codeSystem=\"1.0.3166.1.2.2\" code=\"KR\"/>",
		"<researchStudy classCode=\"CLNTRL\" moodCode=\"EVN\"><id extension=\"QVIS-CT-2026-004\" root=\"2.16.840.1.113883.3.989.2.1.3.5\"/><code code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.8\"/>",
		"<authorization typeCode=\"AUTH\"><studyRegistration classCode=\"ACT\" moodCode=\"EVN\"><id extension=\"123456\" root=\"2.16.840.1.113883.3.989.5.1.2.2.1.2.1\"/></studyRegistration></authorization>",
		"<authorization typeCode=\"AUTH\"><studyRegistration classCode=\"ACT\" moodCode=\"EVN\"><id extension=\"654321\" root=\"2.16.840.1.113883.3.989.5.1.2.2.1.2.2\"/></studyRegistration></authorization>",
		"<authorization typeCode=\"AUTH\"><studyRegistration classCode=\"ACT\" moodCode=\"EVN\"><id extension=\"789012\" root=\"2.16.840.1.113883.3.989.5.1.2.2.1.2.3\"/></studyRegistration></authorization>",
		"<id root=\"2.16.840.1.113883.3.989.2.1.3.3\" assigningAuthorityName=\"QVIS Safety CRO\" extension=\"KR-QVIS-PRIOR-2026-004\"/>",
	] {
		assert!(exported.contains(expected), "missing FDA C node: {expected}");
	}
	let expedited = exported
		.find("code=\"23\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"")
		.unwrap();
	let local_criteria = exported.find("code=\"C54588\"").unwrap();
	let sender = exported.find("<name>QVIS Safety CRO</name>").unwrap();
	assert!(expedited < local_criteria && local_criteria < sender);
	let primary_role = exported.find("<primaryRole").unwrap();
	let study = exported[primary_role..].find("<subjectOf1").unwrap();
	let patient_data = exported[primary_role..].find("<subjectOf2").unwrap();
	assert!(
		study < patient_data,
		"C.5 subjectOf1 must precede D/G subjectOf2"
	);
	let report = xml::validation::validate_e2b_xml(
		exported.as_bytes(),
		Some(xml::validation::XmlValidatorConfig {
			xsd_path: Some(resolved_xsd_path()),
			..Default::default()
		}),
	)?;
	assert!(report.ok, "FDA schema errors: {:?}", report.errors);
	Ok(())
}

#[serial]
#[tokio::test(flavor = "multi_thread")]
async fn fresh_full_build_validates_for_all_authorities() -> Result<()> {
	let (_app, _cookie, case_id, mm, ctx) =
		setup_imported_case_from("FAERS2022Scenario7.xml").await?;
	let case_id = Uuid::parse_str(&case_id)?;
	let devices = FdaDeviceInformationBmc::list(&ctx, &mm, None, None).await?;
	assert!(!devices.is_empty(), "canonical FDA device was not saved");
	let config = xml::validation::XmlValidatorConfig {
		xsd_path: Some(resolved_xsd_path()),
		..Default::default()
	};

	for authority in [
		lib_core::regulatory::RegulatoryAuthority::Ich,
		lib_core::regulatory::RegulatoryAuthority::Fda,
		lib_core::regulatory::RegulatoryAuthority::Mfds,
	] {
		let header = outbound_header(authority);
		let exported = tokio::task::block_in_place(|| {
			tokio::runtime::Handle::current().block_on(
				xml::export::serialize_case_xml_for_authority(
					&ctx, &mm, case_id, authority, &header,
				),
			)
		})?;
		let report = xml::validation::validate_e2b_xml(
			exported.as_bytes(),
			Some(config.clone()),
		)?;
		assert!(report.ok, "{authority:?}: {:?}", report.errors);
		let export_errors =
			xml::validation::validate_export_rules(exported.as_bytes(), authority)?;
		assert!(export_errors.is_empty(), "{authority:?}: {export_errors:?}");

		if matches!(authority, lib_core::regulatory::RegulatoryAuthority::Fda) {
			assert!(exported.contains("C54451"), "FDA device data missing");
		} else {
			assert!(!exported.contains("C54451"), "FDA device data leaked");
		}
		if !matches!(authority, lib_core::regulatory::RegulatoryAuthority::Mfds) {
			assert!(!exported.contains(".KR."), "MFDS regional data leaked");
			assert!(
				!exported.contains("2.16.840.1.113883.3.989.5.1.10."),
				"MFDS regional OID leaked"
			);
		}
	}

	Ok(())
}
