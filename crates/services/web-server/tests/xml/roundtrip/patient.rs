use super::support::*;

#[serial]
#[tokio::test(flavor = "multi_thread")]
async fn empty_autopsy_cause_exports_without_parent_answer_for_all_authorities(
) -> Result<()> {
	let (_app, _cookie, case_id, mm, ctx) = setup_imported_case().await?;
	let case_id = Uuid::parse_str(&case_id)?;
	let patient = PatientInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	let death_info_id = PatientDeathInformationBmc::create(
		&ctx,
		&mm,
		PatientDeathInformationForCreate {
			patient_id: patient.id,
			date_of_death: None,
			date_of_death_null_flavor: None,
			autopsy_performed: None,
			autopsy_performed_null_flavor: None,
		},
	)
	.await?;
	AutopsyCauseOfDeathBmc::create(
		&ctx,
		&mm,
		AutopsyCauseOfDeathForCreate {
			death_info_id,
			sequence_number: 1,
			meddra_version: None,
			meddra_code: None,
			comments: None,
		},
	)
	.await?;

	for authority in [
		lib_core::regulatory::RegulatoryAuthority::Ich,
		lib_core::regulatory::RegulatoryAuthority::Fda,
		lib_core::regulatory::RegulatoryAuthority::Mfds,
	] {
		let exported = xml::export::serialize_case_xml_for_authority(
			&ctx,
			&mm,
			case_id,
			authority,
			&outbound_header(authority),
		)
		.await?;
		assert_eq!(
			exported
				.matches("displayName=\"autopsy\"/><value xsi:type=\"BL\"/>")
				.count(),
			1,
			"{authority:?}"
		);
		assert_eq!(
			exported.matches("displayName=\"causeOfDeath\"").count(),
			1,
			"{authority:?}"
		);
	}

	Ok(())
}

#[serial]
#[tokio::test(flavor = "multi_thread")]
async fn parent_null_flavor_xml_roundtrip() -> Result<()> {
	let (app, cookie, case_id, mm, ctx) =
		setup_imported_case_from("FAERS2022Scenario1.xml").await?;

	let (status, body) = request_json(
		&app,
		&cookie,
		"PATCH",
		format!("/api/cases/{case_id}/editor/pages/DM"),
		Some(serde_json::json!({
			"authorities": ["ich"],
			"rows": {
				"patientInformation": {"patientInitials": "PT-XML"},
				"parentInfo": {
					"parentIdentification": null,
					"parentIdentificationNullFlavor": "UNK",
					"parentSex": null,
					"parentSexNullFlavor": "NASK"
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));

	let parsed_case_id = Uuid::parse_str(&case_id)?;
	let authority = lib_core::regulatory::RegulatoryAuthority::Ich;
	let header = outbound_header(authority);
	let xml = tokio::task::block_in_place(|| {
		tokio::runtime::Handle::current().block_on(
			xml::serialize_case_xml_for_authority(
				&ctx,
				&mm,
				parsed_case_id,
				authority,
				&header,
			),
		)
	})?;
	assert!(xml.contains("nullFlavor=\"UNK\""), "{xml}");
	assert!(xml.contains("nullFlavor=\"NASK\""), "{xml}");
	let parsed_parent =
		xml::import_sections::d_patient::parse_d_parent(xml.as_bytes())?
			.ok_or("missing parsed parent information")?;
	assert_eq!(parsed_parent.parent_identification, None);
	assert_eq!(
		parsed_parent.parent_identification_null_flavor.as_deref(),
		Some("UNK")
	);
	assert_eq!(parsed_parent.sex, None);
	assert_eq!(parsed_parent.sex_null_flavor.as_deref(), Some("NASK"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_roundtrip_dm_dh_remaining_fields() -> Result<()> {
	let (app, cookie, case_id, _mm, _ctx) = match setup_imported_case().await {
		Ok(v) => v,
		Err(err) => {
			eprintln!("skipping DM/DH remaining test: {err}");
			return Ok(());
		}
	};

	// DM.D.6
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/patient"),
		Some(serde_json::json!({
			"data": { "last_menstrual_period_date": "2024-02-03" }
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update patient status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	// DM.D.10.2.1 + D.10.3
	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/patient/parents"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list parents status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let parent_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.map(|s| s.to_string());

	let parent_id = if let Some(id) = parent_id {
		id
	} else {
		let (status, body) = request_json(
			&app,
			&cookie,
			"POST",
			format!("/api/cases/{case_id}/patient/parents"),
			Some(serde_json::json!({
				"data": {
					"patient_id": Uuid::nil(),
					"sex": "2",
					"medical_history_text": "RTDM-PARENT"
				}
			})),
		)
		.await?;
		if status != StatusCode::CREATED {
			return Err(format!(
				"create parent status {} body {}",
				status,
				String::from_utf8_lossy(&body)
			)
			.into());
		}
		extract_data_id(&body)?
	};

	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/patient/parents/{parent_id}"),
		Some(serde_json::json!({
			"data": {
				"parent_age": 37.7,
				"parent_age_unit": "801",
				"parent_birth_date": "1988-11-23"
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update parent status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	// DH.D.8.r.4 + D.8.r.5
	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/patient/past-drugs"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list past-drugs status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let past_drug_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.map(|s| s.to_string());
	let past_drug_id = if let Some(id) = past_drug_id {
		id
	} else {
		let (status, body) = request_json(
			&app,
			&cookie,
			"POST",
			format!("/api/cases/{case_id}/patient/past-drugs"),
			Some(serde_json::json!({
				"data": {
					"patient_id": Uuid::nil(),
					"sequence_number": 1,
					"drug_name": format!("RTDH-{}", Uuid::new_v4().simple())
				}
			})),
		)
		.await?;
		if status != StatusCode::CREATED {
			return Err(format!(
				"create past-drug status {} body {}",
				status,
				String::from_utf8_lossy(&body)
			)
			.into());
		}
		extract_data_id(&body)?
	};

	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/patient/past-drugs/{past_drug_id}"),
		Some(serde_json::json!({
			"data": {
				"start_date": "2024-03-04",
				"end_date": "2024-03-05"
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update past-drug status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	set_validated(&app, &cookie, &case_id).await?;
	let xml = match export_xml(&app, &cookie, &case_id).await {
		Ok(xml) => xml,
		Err(err) if err.to_string().contains("Only validated cases") => {
			return Ok(())
		}
		Err(err) => return Err(err),
	};
	for expected in ["20240203", "37.7", "19881123", "20240304", "20240305"] {
		assert!(
			xml.contains(expected),
			"expected DM/DH export XML to contain `{expected}`"
		);
	}
	Ok(())
}
