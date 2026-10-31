use super::support::*;

#[serial]
#[tokio::test]
async fn test_roundtrip_ae_remaining_fields() -> Result<()> {
	let (app, cookie, case_id, _mm, _ctx) = setup_imported_case().await?;

	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/reactions"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list reactions status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let reaction_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.map(|s| s.to_string());
	let reaction_id = if let Some(id) = reaction_id {
		id
	} else {
		let sentinel = format!("RTAE1-{}", Uuid::new_v4().simple());
		let (status, body) = request_json(
			&app,
			&cookie,
			"POST",
			format!("/api/cases/{case_id}/reactions"),
			Some(serde_json::json!({
				"data": { "sequence_number": 1, "primary_source_reaction": sentinel }
			})),
		)
		.await?;
		if status != StatusCode::CREATED {
			return Err(format!(
				"create reaction status {} body {}",
				status,
				String::from_utf8_lossy(&body)
			)
			.into());
		}
		extract_data_id(&body)?
	};

	let sentinel_text = format!("RTAE1-{}", Uuid::new_v4().simple());
	let sentinel_required_intervention = false;
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/reactions/{reaction_id}"),
		Some(serde_json::json!({
			"data": {
				"primary_source_reaction": sentinel_text,
				"reaction_language": "eng",
				"reaction_meddra_code": "10012345",
				"reaction_meddra_version": "27.0",
				"required_intervention": sentinel_required_intervention,
				"start_date": "2024-04-06",
				"end_date": "2024-04-07",
				"outcome": "2"
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update reaction status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	let xml = export_xml(&app, &cookie, &case_id).await?;
	for expected in [
		sentinel_text.as_str(),
		"10012345",
		"27.0",
		"20240406",
		"20240407",
	] {
		assert!(
			xml.contains(expected),
			"expected AE export XML to contain `{expected}`"
		);
	}
	let ri_code = "code=\"7\" codeSystem=\"2.16.840.1.113883.3.989.5.1.2.2.1.3\"";
	let ri_pos = xml
		.find(ri_code)
		.ok_or("missing required intervention code node")?;
	let ri_end = core::cmp::min(ri_pos + 280, xml.len());
	let ri_window = &xml[ri_pos..ri_end];
	assert!(
		ri_window.contains("value=\"false\""),
		"expected required intervention node window to contain value=\"false\", got: {ri_window}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_api_persistence_ae_sd_all_fields() -> Result<()> {
	let (app, cookie, case_id, _mm, _ctx) = setup_imported_case().await?;

	// AE: ensure there is a reaction, then update all AE fields via API.
	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/reactions"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list reactions status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let reaction_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.map(|s| s.to_string());
	let reaction_id = if let Some(id) = reaction_id {
		id
	} else {
		let sentinel = format!("RTAE-CREATE-{}", Uuid::new_v4().simple());
		let (status, body) = request_json(
			&app,
			&cookie,
			"POST",
			format!("/api/cases/{case_id}/reactions"),
			Some(serde_json::json!({
				"data": { "sequence_number": 1, "primary_source_reaction": sentinel }
			})),
		)
		.await?;
		if status != StatusCode::CREATED {
			return Err(format!(
				"create reaction status {} body {}",
				status,
				String::from_utf8_lossy(&body)
			)
			.into());
		}
		extract_data_id(&body)?
	};

	let ae_primary = format!("RTAE-PSR-{}", Uuid::new_v4().simple());
	let ae_translation = format!("RTAE-TR-{}", Uuid::new_v4().simple());
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/reactions/{reaction_id}"),
		Some(serde_json::json!({
			"data": {
				"primary_source_reaction": ae_primary,
				"primary_source_reaction_translation": ae_translation,
				"reaction_language": "fr",
				"reaction_meddra_code": "10012345",
				"reaction_meddra_version": "27.0",
				"term_highlighted": "3",
				"serious": true,
				"criteria_death": true,
				"criteria_life_threatening": false,
				"criteria_hospitalization": true,
				"criteria_disabling": true,
				"criteria_congenital_anomaly": false,
				"criteria_other_medically_important": true,
				"required_intervention": false,
				"start_date": "2024-04-06",
				"end_date": "2024-04-07",
				"duration_value": "5.5",
				"duration_unit": "804",
				"outcome": "2",
				"medical_confirmation": true,
				"country_code": "US"
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update reaction status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/reactions/{reaction_id}"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"get reaction status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let reaction = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.cloned()
		.ok_or("missing reaction data")?;
	let mut ae_mismatches: Vec<String> = Vec::new();
	let reaction_str = |key: &str| -> String {
		reaction
			.get(key)
			.and_then(|v| v.as_str())
			.unwrap_or_default()
			.to_string()
	};
	let reaction_bool =
		|key: &str| -> Option<bool> { reaction.get(key).and_then(|v| v.as_bool()) };
	let check_date = |field: &str,
	                  primary: &str,
	                  fallback: &str,
	                  expected_str: &str,
	                  expected_year: i64,
	                  expected_ordinal: i64,
	                  out: &mut Vec<String>| {
		let value = reaction.get(primary).or_else(|| reaction.get(fallback));
		let ok = match value {
			Some(v) if v.as_str() == Some(expected_str) => true,
			Some(v) => v
				.as_array()
				.and_then(|arr| {
					let y = arr.first()?.as_i64()?;
					let ord = arr.get(1)?.as_i64()?;
					Some(y == expected_year && ord == expected_ordinal)
				})
				.unwrap_or(false),
			None => false,
		};
		if !ok {
			out.push(format!(
					"{field}: expected `{expected_str}` or [{expected_year},{expected_ordinal}], got `{:?}`",
					value
				));
		}
	};
	let check_str =
		|field: &str, actual: String, expected: &str, out: &mut Vec<String>| {
			if actual != expected {
				out.push(format!("{field}: expected `{expected}`, got `{actual}`"));
			}
		};
	let check_bool = |field: &str,
	                  actual: Option<bool>,
	                  expected: bool,
	                  out: &mut Vec<String>| {
		if actual != Some(expected) {
			out.push(format!(
				"{field}: expected `{expected}`, got `{:?}`",
				actual
			));
		}
	};

	check_str(
		"primary_source_reaction",
		reaction_str("primary_source_reaction"),
		&ae_primary,
		&mut ae_mismatches,
	);
	check_str(
		"reaction_language",
		reaction_str("reaction_language"),
		"fr",
		&mut ae_mismatches,
	);
	check_str(
		"primary_source_reaction_translation",
		reaction_str("primary_source_reaction_translation"),
		&ae_translation,
		&mut ae_mismatches,
	);
	check_str(
		"reaction_meddra_code",
		reaction_str("reaction_meddra_code"),
		"10012345",
		&mut ae_mismatches,
	);
	check_str(
		"reaction_meddra_version",
		reaction_str("reaction_meddra_version"),
		"27.0",
		&mut ae_mismatches,
	);
	check_str(
		"term_highlighted",
		reaction_str("term_highlighted"),
		"3",
		&mut ae_mismatches,
	);
	check_bool(
		"serious",
		reaction_bool("serious"),
		true,
		&mut ae_mismatches,
	);
	check_bool(
		"criteria_death",
		reaction_bool("criteria_death"),
		true,
		&mut ae_mismatches,
	);
	check_bool(
		"criteria_life_threatening",
		reaction_bool("criteria_life_threatening"),
		false,
		&mut ae_mismatches,
	);
	check_bool(
		"criteria_hospitalization",
		reaction_bool("criteria_hospitalization"),
		true,
		&mut ae_mismatches,
	);
	check_bool(
		"criteria_disabling",
		reaction_bool("criteria_disabling"),
		true,
		&mut ae_mismatches,
	);
	check_bool(
		"criteria_congenital_anomaly",
		reaction_bool("criteria_congenital_anomaly"),
		false,
		&mut ae_mismatches,
	);
	check_bool(
		"criteria_other_medically_important",
		reaction_bool("criteria_other_medically_important"),
		true,
		&mut ae_mismatches,
	);
	check_bool(
		"required_intervention",
		reaction_bool("required_intervention"),
		false,
		&mut ae_mismatches,
	);
	check_date(
		"start_date",
		"start_date",
		"reaction_start_date",
		"2024-04-06",
		2024,
		97,
		&mut ae_mismatches,
	);
	check_date(
		"end_date",
		"end_date",
		"reaction_end_date",
		"2024-04-07",
		2024,
		98,
		&mut ae_mismatches,
	);
	check_str(
		"duration_unit",
		reaction_str("duration_unit"),
		"804",
		&mut ae_mismatches,
	);
	check_str("outcome", reaction_str("outcome"), "2", &mut ae_mismatches);
	check_bool(
		"medical_confirmation",
		reaction_bool("medical_confirmation"),
		true,
		&mut ae_mismatches,
	);
	check_str(
		"country_code",
		reaction_str("country_code"),
		"US",
		&mut ae_mismatches,
	);

	// SD: update all sender fields via API, then verify persisted values from GET.
	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/safety-report/senders"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list senders status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let sender_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.map(|s| s.to_string());
	let sender_id = if let Some(id) = sender_id {
		id
	} else {
		let (status, body) = request_json(
			&app,
			&cookie,
			"POST",
			format!("/api/cases/{case_id}/safety-report/senders"),
			Some(serde_json::json!({
				"data": {
					"case_id": case_id,
					"sender_type": "2",
					"organization_name": "RTSD-CREATE"
				}
			})),
		)
		.await?;
		if status != StatusCode::CREATED {
			return Err(format!(
				"create sender status {} body {}",
				status,
				String::from_utf8_lossy(&body)
			)
			.into());
		}
		extract_data_id(&body)?
	};

	let sd_org = format!("RTSD-ORG-{}", Uuid::new_v4().simple());
	let sd_dept = format!("RTSD-DEPT-{}", Uuid::new_v4().simple());
	let sd_title = format!("RTSD-TITLE-{}", Uuid::new_v4().simple());
	let sd_given = format!("RTSD-GIVEN-{}", Uuid::new_v4().simple());
	let sd_middle = format!("RTSD-MID-{}", Uuid::new_v4().simple());
	let sd_family = format!("RTSD-FAM-{}", Uuid::new_v4().simple());
	let sd_street = format!("RTSD-STREET-{}", Uuid::new_v4().simple());
	let sd_city = format!("RTSD-CITY-{}", Uuid::new_v4().simple());
	let sd_email = format!("{}@example.com", Uuid::new_v4().simple());
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/safety-report/senders/{sender_id}"),
		Some(serde_json::json!({
			"data": {
				"sender_type": "2",
				"organization_name": sd_org,
				"department": sd_dept,
				"street_address": sd_street,
				"city": sd_city,
				"state": "CA",
				"postcode": "90210",
				"country_code": "US",
				"person_title": sd_title,
				"person_given_name": sd_given,
				"person_middle_name": sd_middle,
				"person_family_name": sd_family,
				"telephone": "15550101234",
				"fax": "15550109999",
				"email": sd_email
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update sender status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/safety-report/senders/{sender_id}"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"get sender status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let sender = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.cloned()
		.ok_or("missing sender data")?;
	let mut sd_mismatches: Vec<String> = Vec::new();
	let sender_str = |key: &str| -> String {
		sender
			.get(key)
			.and_then(|v| v.as_str())
			.unwrap_or_default()
			.to_string()
	};
	check_str(
		"sender_type",
		sender_str("sender_type"),
		"2",
		&mut sd_mismatches,
	);
	check_str(
		"organization_name",
		sender_str("organization_name"),
		&sd_org,
		&mut sd_mismatches,
	);
	check_str(
		"department",
		sender_str("department"),
		&sd_dept,
		&mut sd_mismatches,
	);
	check_str(
		"street_address",
		sender_str("street_address"),
		&sd_street,
		&mut sd_mismatches,
	);
	check_str("city", sender_str("city"), &sd_city, &mut sd_mismatches);
	check_str("state", sender_str("state"), "CA", &mut sd_mismatches);
	check_str(
		"postcode",
		sender_str("postcode"),
		"90210",
		&mut sd_mismatches,
	);
	check_str(
		"country_code",
		sender_str("country_code"),
		"US",
		&mut sd_mismatches,
	);
	check_str(
		"person_title",
		sender_str("person_title"),
		&sd_title,
		&mut sd_mismatches,
	);
	check_str(
		"person_given_name",
		sender_str("person_given_name"),
		&sd_given,
		&mut sd_mismatches,
	);
	check_str(
		"person_middle_name",
		sender_str("person_middle_name"),
		&sd_middle,
		&mut sd_mismatches,
	);
	check_str(
		"person_family_name",
		sender_str("person_family_name"),
		&sd_family,
		&mut sd_mismatches,
	);
	check_str(
		"telephone",
		sender_str("telephone"),
		"15550101234",
		&mut sd_mismatches,
	);
	check_str("fax", sender_str("fax"), "15550109999", &mut sd_mismatches);
	check_str("email", sender_str("email"), &sd_email, &mut sd_mismatches);

	if !ae_mismatches.is_empty() || !sd_mismatches.is_empty() {
		return Err(format!(
			"AE mismatches: {:?}; SD mismatches: {:?}; reaction={}; sender={}",
			ae_mismatches, sd_mismatches, reaction, sender
		)
		.into());
	}

	// Also verify API-persisted AE/SD values are reflected in exported XML.
	let xml = export_xml(&app, &cookie, &case_id).await?;
	for expected in [
		ae_primary.as_str(),
		ae_translation.as_str(),
		"10012345",
		"27.0",
		"20240406",
		"20240407",
		sd_org.as_str(),
		sd_dept.as_str(),
		sd_street.as_str(),
		sd_city.as_str(),
		sd_title.as_str(),
		sd_given.as_str(),
		sd_middle.as_str(),
		sd_family.as_str(),
		sd_email.as_str(),
	] {
		assert!(
			xml.contains(expected),
			"expected AE/SD export XML to contain `{expected}`"
		);
	}

	Ok(())
}
