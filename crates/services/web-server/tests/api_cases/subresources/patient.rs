use super::support::*;

#[serial]
#[tokio::test]
async fn patient_physical_fields_preserve_values_and_ucum_units() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, seed.org_id).await?;

	let body = json!({
		"data": {
			"case_id": case_id,
			"age_at_time_of_onset": 7,
			"age_unit": "10.a",
			"gestation_period": 2,
			"gestation_period_unit": "{Trimester}",
			"weight_kg": 25.5,
			"height_cm": 120
		}
	});
	let (status, body) =
		post_json(&app, &cookie, format!("/api/cases/{case_id}/patient"), body)
			.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"create patient body {}",
		String::from_utf8_lossy(&body)
	);
	let payload: Value = serde_json::from_slice(&body)?;
	assert_eq!(payload["data"]["age_unit"], "10.a");
	assert_eq!(payload["data"]["gestation_period_unit"], "{Trimester}");
	assert_eq!(payload["data"]["weight_kg"], "25.50");
	assert_eq!(payload["data"]["height_cm"], "120.00");
	assert!(payload["data"].get("weight_kg_null_flavor").is_none());
	assert!(payload["data"].get("height_cm_null_flavor").is_none());

	Ok(())
}

#[serial]
#[tokio::test]
async fn parent_information_preserves_dob_null_flavor_and_decade_unit() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let patient_id = create_patient(&app, &cookie, case_id).await?;

	let body = json!({
		"data": {
			"patient_id": patient_id,
			"parent_age": 4,
			"parent_age_unit": "10.a"
		}
	});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/parents"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"create parent body {}",
		String::from_utf8_lossy(&body)
	);
	let payload: Value = serde_json::from_slice(&body)?;
	assert_eq!(payload["data"]["parent_age"], "4.00");
	assert_eq!(payload["data"]["parent_age_unit"], "10.a");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_audit_reason_header_records_normal_patient_update_reason() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let patient_id = create_patient(&app, &cookie, case_id).await?;

	let reason = "Edited Data: Corrected patient initials";
	let body = json!({"data": {
		"patient_initials": "CD",
		"sex": "2"
	}});
	let (status, body) = put_json_with_audit_reason(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient"),
		body,
		reason,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::OK,
		"update patient status {} body {}",
		status,
		String::from_utf8_lossy(&body)
	);

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	dbx.execute(sqlx::query("SET ROLE e2br3_auditor_role"))
		.await?;
	let recorded_reason = dbx
		.fetch_optional(
			sqlx::query_as::<_, (Option<String>,)>(
				r#"
				SELECT reason_for_change
				FROM audit_logs
				WHERE table_name = 'patient_information'
				  AND record_id = $1
				  AND action = 'UPDATE'
				  AND changed_fields ? 'patient_initials'
				ORDER BY id DESC
				LIMIT 1
				"#,
			)
			.bind(patient_id),
		)
		.await?;
	dbx.rollback_txn().await?;

	assert_eq!(
		recorded_reason.and_then(|(value,)| value),
		Some(reason.to_string())
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_patient_subresources_endpoints_ok() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let patient_id = create_patient(&app, &cookie, case_id).await?;

	// Medical history episode
	let body = json!({"data": {"patient_id": patient_id, "sequence_number": 1, "meddra_code": "100"}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/medical-history"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/medical-history"),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"medical-history list status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let data = value
		.get("data")
		.and_then(|v| v.as_array())
		.ok_or("missing data array")?;
	assert!(!data.is_empty());

	// Past drug history
	let body = json!({"data": {"patient_id": patient_id, "sequence_number": 1, "drug_name": "Test"}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/past-drugs"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/past-drugs"),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"past-drugs list status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let data = value
		.get("data")
		.and_then(|v| v.as_array())
		.ok_or("missing data array")?;
	assert!(!data.is_empty());

	// Death info
	let body =
		json!({"data": {"patient_id": patient_id, "date_of_death": [2024, 1]}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/death-info"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let death_info_id = extract_id(&body)?;

	// Reported cause of death
	let body = json!({"data": {"death_info_id": death_info_id, "sequence_number": 1, "meddra_code": "100"}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);

	// Autopsy cause of death
	let body = json!({"data": {"death_info_id": death_info_id, "sequence_number": 1, "meddra_code": "100"}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!(
			"/api/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes"
		),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);

	// Parent info
	let body = json!({"data": {"patient_id": patient_id, "sex": "2", "medical_history_text": "none"}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/parents"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let parent_id = extract_id(&body)?;

	// Parent past drug history
	let body = json!({"data": {
		"parent_id": parent_id,
		"sequence_number": 1,
		"mfds_medicinal_product_version": "MFDS-V1",
		"mfds_medicinal_product_id": "MFDS-ID",
		"mpid_version": "MPID-V1",
		"mpid": "MPID-123",
		"phpid_version": "PHPID-V1",
		"phpid": "PHPID-123",
		"start_date_null_flavor": "ASKU",
			"end_date_null_flavor": "NASK",
		"indication_meddra_version": "27.1",
		"indication_meddra_code": "10012345",
		"reaction_meddra_version": "27.1",
		"reaction_meddra_code": "10054321"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/parent/{parent_id}/past-drugs"),
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
	assert_eq!(value["data"]["drug_name"], Value::Null);
	assert_eq!(value["data"]["mfds_medicinal_product_version"], "MFDS-V1");
	assert_eq!(value["data"]["mfds_medicinal_product_id"], "MFDS-ID");
	assert_eq!(value["data"]["mpid_version"], "MPID-V1");
	assert_eq!(value["data"]["mpid"], "MPID-123");
	assert_eq!(value["data"]["phpid_version"], "PHPID-V1");
	assert_eq!(value["data"]["phpid"], "PHPID-123");
	assert_eq!(value["data"]["start_date_null_flavor"], "ASKU");
	assert_eq!(value["data"]["end_date_null_flavor"], "NASK");
	assert_eq!(value["data"]["indication_meddra_version"], "27.1");
	assert_eq!(value["data"]["indication_meddra_code"], "10012345");
	assert_eq!(value["data"]["reaction_meddra_version"], "27.1");
	assert_eq!(value["data"]["reaction_meddra_code"], "10054321");

	Ok(())
}

#[serial]
#[tokio::test]
async fn nested_patient_children_preserve_crud_scope_and_restore_contracts(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let other_case_id = create_case(&app, &cookie, seed.org_id).await?;
	let patients = [
		(case_id, create_patient(&app, &cookie, case_id).await?),
		(
			other_case_id,
			create_patient(&app, &cookie, other_case_id).await?,
		),
	];
	for (parent_collection, parent_path, parent_field, resources) in [
		(
			"parents",
			"parent",
			"parent_id",
			[
				("medical-history", "meddra_code", "10012345", "10054321"),
				("past-drugs", "drug_name", "original drug", "updated drug"),
			],
		),
		(
			"death-info",
			"death-info",
			"death_info_id",
			[
				("reported-causes", "meddra_code", "10012345", "10054321"),
				("autopsy-causes", "meddra_code", "10012345", "10054321"),
			],
		),
	] {
		let mut parents = Vec::new();
		for (parent_case_id, patient_id) in patients {
			let (status, body) = post_json(
				&app,
				&cookie,
				format!("/api/cases/{parent_case_id}/patient/{parent_collection}"),
				json!({"data": {"patient_id": patient_id}}),
			)
			.await?;
			assert_eq!(
				status,
				StatusCode::CREATED,
				"{}",
				String::from_utf8_lossy(&body)
			);
			parents.push(extract_id(&body)?);
		}
		let (parent_id, other_parent_id) = (parents[0], parents[1]);
		assert_ne!(parent_id, other_parent_id);

		for (resource, field, initial, updated) in resources {
			let collection = format!(
				"/api/cases/{case_id}/patient/{parent_path}/{parent_id}/{resource}"
			);
			let create = json!({"data": {(parent_field): other_parent_id, "sequence_number": 1, (field): initial}});
			let update = json!({"data": {(field): updated}});
			let (status, body) =
				post_json(&app, &cookie, collection.clone(), create.clone()).await?;
			assert_eq!(
				status,
				StatusCode::CREATED,
				"{}",
				String::from_utf8_lossy(&body)
			);
			let id = extract_id(&body)?;
			let created: Value = serde_json::from_slice(&body)?;
			assert_eq!(
				created["data"][parent_field],
				parent_id.to_string(),
				"path parent must override input"
			);
			assert_eq!(created["data"][field], initial);
			assert_eq!(created["data"]["deleted"], false);
			let item = format!("{collection}/{id}");
			let other_parent_collection = format!(
			"/api/cases/{other_case_id}/patient/{parent_path}/{other_parent_id}/{resource}"
		);
			let (status, body) =
				get_json(&app, &cookie, other_parent_collection.clone()).await?;
			assert_eq!(status, StatusCode::OK);
			assert_eq!(serde_json::from_slice::<Value>(&body)?["data"], json!([]));

			// Check every route, not just GET: rejected writes must leave the row unchanged.
			for (method, suffix, input) in [
				("GET", String::new(), json!({})),
				("POST", String::new(), create.clone()),
				("GET", format!("/{id}"), json!({})),
				("PUT", format!("/{id}"), update.clone()),
				("DELETE", format!("/{id}"), json!({})),
				("POST", format!("/{id}/restore"), json!({})),
			] {
				let wrong_case = format!(
				"/api/cases/{other_case_id}/patient/{parent_path}/{parent_id}/{resource}"
			);
				let missing_parent = format!(
					"/api/cases/{case_id}/patient/{parent_path}/{}/{resource}",
					Uuid::new_v4()
				);
				for (request_cookie, base, expected) in [
					(&cookie, &wrong_case, StatusCode::NOT_FOUND),
					(&cookie, &missing_parent, StatusCode::NOT_FOUND),
					(&cookie, &other_parent_collection, StatusCode::NOT_FOUND),
					(&viewer_cookie, &collection, StatusCode::FORBIDDEN),
				] {
					if (base == &other_parent_collection && suffix.is_empty())
						|| (request_cookie == &viewer_cookie && method == "GET")
					{
						continue;
					}
					let req = Request::builder()
						.method(method)
						.uri(format!("{base}{suffix}"))
						.header("cookie", request_cookie)
						.header("content-type", "application/json")
						.header("x-e2br3-reason-for-change", "scope regression")
						.body(Body::from(input.to_string()))?;
					let res = app.clone().oneshot(req).await?;
					let status = res.status();
					let body = to_bytes(res.into_body(), usize::MAX).await?;
					assert_eq!(
						status,
						expected,
						"{method} {base}{suffix}: {}",
						String::from_utf8_lossy(&body)
					);
				}
			}
			let (status, body) = get_json(&app, &cookie, item.clone()).await?;
			assert_eq!(status, StatusCode::OK);
			assert_eq!(serde_json::from_slice::<Value>(&body)?, created);
			let (status, body) = put_json_with_audit_reason(
				&app,
				&cookie,
				item.clone(),
				update,
				"nested patient child regression",
			)
			.await?;
			assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
			assert_eq!(
				serde_json::from_slice::<Value>(&body)?["data"][field],
				updated
			);

			for deleted in [true, false] {
				if deleted {
					let (status, body) =
						delete_json(&app, &cookie, item.clone()).await?;
					assert_eq!(
						status,
						StatusCode::NO_CONTENT,
						"{}",
						String::from_utf8_lossy(&body)
					);
					assert!(body.is_empty());
				} else {
					let (status, body) = post_json(
						&app,
						&cookie,
						format!("{item}/restore"),
						json!({}),
					)
					.await?;
					assert_eq!(
						status,
						StatusCode::OK,
						"{}",
						String::from_utf8_lossy(&body)
					);
					assert_eq!(
						serde_json::from_slice::<Value>(&body)?["data"]["id"],
						id.to_string()
					);
					assert_eq!(
						serde_json::from_slice::<Value>(&body)?["data"]["deleted"],
						false
					);
				}
				// Existing item GET exposes the soft-deleted row; list hides it.
				let (status, body) = get_json(&app, &cookie, item.clone()).await?;
				assert_eq!(status, StatusCode::OK);
				let row: Value = serde_json::from_slice(&body)?;
				assert_eq!(row["data"]["id"], id.to_string());
				assert_eq!(row["data"]["deleted"], deleted);
				assert_eq!(row["data"][field], updated);
				let (status, body) =
					get_json(&app, &cookie, collection.clone()).await?;
				assert_eq!(status, StatusCode::OK);
				let list: Value = serde_json::from_slice(&body)?;
				if deleted {
					assert_eq!(list["data"], json!([]));
				} else {
					assert_eq!(list["data"], json!([row["data"]]));
				}
			}
		}
	}
	Ok(())
}

#[serial]
#[tokio::test]
async fn patient_identifier_preserves_scope_and_delete_response() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let first_case_id = create_case(&app, &cookie, seed.org_id).await?;
	let second_case_id = create_case(&app, &cookie, seed.org_id).await?;
	let first_patient_id = create_patient(&app, &cookie, first_case_id).await?;
	create_patient(&app, &cookie, second_case_id).await?;
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{first_case_id}/patient/identifiers"),
		json!({"data": {
			"patient_id": first_patient_id,
			"sequence_number": 1,
			"identifier_type_code": "1",
			"identifier_value": "contract-id"
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let id = extract_id(&body)?;
	let (status, _) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{second_case_id}/patient/identifiers/{id}"),
	)
	.await?;
	assert_eq!(status, StatusCode::NOT_FOUND);
	let (status, body) = delete_json(
		&app,
		&cookie,
		format!("/api/cases/{first_case_id}/patient/identifiers/{id}"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["id"], id.to_string());
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_past_drugs_support_mfds_product_fields_and_200_char_ids() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let patient_id = create_patient(&app, &cookie, case_id).await?;
	let long_mpid = "M".repeat(200);
	let long_phpid = "P".repeat(200);

	let body = json!({"data": {
		"patient_id": patient_id,
		"sequence_number": 1,
		"drug_name": "Past drug",
		"mfds_medicinal_product_version": "KR-VERSION-123456",
		"mfds_medicinal_product_id": "KRPROD1234",
		"mpid_version": "MPID-V1",
		"mpid": long_mpid,
		"phpid_version": "PHPID-V1",
		"phpid": long_phpid
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/past-drugs"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"past-drug create status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let data = &value["data"];
	assert_eq!(data["mfds_medicinal_product_version"], "KR-VERSION-123456");
	assert_eq!(data["mfds_medicinal_product_id"], "KRPROD1234");
	assert_eq!(data["mpid"], long_mpid);
	assert_eq!(data["phpid"], long_phpid);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_narrative_preview_resolves_patient_tokens() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_patient_with_narrative_preview_values(&app, &cookie, case_id).await?;

	let body = json!({
		"template": "{D.2.2a}세의 {D.5} 환자 {UNKNOWN.CODE}"
	});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/preview"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["rendered"], "30세의 여성 환자 {UNKNOWN.CODE}");
	assert_eq!(value["data"]["tokens"][0]["code"], "D.2.2a");
	assert_eq!(value["data"]["tokens"][0]["resolved"], true);
	assert_eq!(value["data"]["tokens"][1]["code"], "D.5");
	assert_eq!(value["data"]["tokens"][1]["resolved"], true);
	assert_eq!(value["data"]["tokens"][2]["code"], "UNKNOWN.CODE");
	assert_eq!(value["data"]["tokens"][2]["resolved"], false);

	Ok(())
}
