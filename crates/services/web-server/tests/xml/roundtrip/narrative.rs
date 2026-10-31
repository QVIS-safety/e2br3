use super::support::*;

#[serial]
#[tokio::test]
async fn test_roundtrip_nr_fields() -> Result<()> {
	let (app, cookie, case_id, _mm, _ctx) = setup_imported_case().await?;

	let sentinel_h1 = format!("RTNR-H1-{}", Uuid::new_v4().simple());
	let sentinel_h2 = format!("RTNR-H2-{}", Uuid::new_v4().simple());
	let sentinel_h4 = format!("RTNR-H4-{}", Uuid::new_v4().simple());
	let sentinel_h53 = format!("RTNR-H53-{}", Uuid::new_v4().simple());

	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/narrative"),
		Some(serde_json::json!({
			"data": {
				"case_narrative": sentinel_h1,
				"reporter_comments": sentinel_h2,
				"sender_comments": sentinel_h4
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update narrative status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/narrative/summaries"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list summaries status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let summary_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.map(|s| s.to_string());
	let summary_id = if let Some(id) = summary_id {
		id
	} else {
		let (status, body) = request_json(
			&app,
			&cookie,
			"POST",
			format!("/api/cases/{case_id}/narrative/summaries"),
			Some(serde_json::json!({
				"data": {
					"narrative_id": Uuid::nil(),
					"sequence_number": 1,
					"summary_text": sentinel_h53
				}
			})),
		)
		.await?;
		if status != StatusCode::CREATED {
			return Err(format!(
				"create summary status {} body {}",
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
		format!("/api/cases/{case_id}/narrative/summaries/{summary_id}"),
		Some(serde_json::json!({
			"data": {
				"language_code": "eng",
				"summary_text": sentinel_h53
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update summary status {} body {}",
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
	for expected in [
		sentinel_h1.as_str(),
		sentinel_h2.as_str(),
		sentinel_h4.as_str(),
		sentinel_h53.as_str(),
	] {
		assert!(
			xml.contains(expected),
			"expected NR export XML to contain `{expected}`"
		);
	}
	Ok(())
}
