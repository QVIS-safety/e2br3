use super::support::*;

#[serial]
#[tokio::test]
async fn test_follow_up_creation_preserves_scope_and_allocates_versions(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let safety_report_id = format!("FOLLOW-UP-{}", Uuid::new_v4());
	let product_id = format!("PRODUCT-{}", Uuid::new_v4());

	let (status, source_body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"status": "draft",
				"dgPrdKey": product_id,
				"safetyReportIdentification": {
					"safetyReportId": safety_report_id
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{source_body:?}");
	let source_case_id = Uuid::parse_str(
		source_body["data"]["id"]
			.as_str()
			.ok_or("missing source case id")?,
	)?;

	let (status, header) = post_json(&app, &cookie, &format!("/api/cases/{source_case_id}/message-header"), json!({"data": {
		"case_id": source_case_id, "message_number": format!("MSG-{source_case_id}"),
		"message_sender_identifier": "FOLLOW-UP-SENDER", "message_receiver_identifier": "KR",
		"batch_sender_identifier": "FOLLOW-UP-SENDER", "batch_receiver_identifier": "KR",
		"message_date": "20261019000000"
	}})).await?;
	assert_eq!(status, StatusCode::CREATED, "{header:?}");

	let mut invalid_pages = follow_up_pages("mfds");
	invalid_pages["ae"] = json!([
		{
			"clientRowId": "index-0",
			"request": {
				"authorities": ["mfds"],
				"rows": { "reaction": {
					"sequenceNumber": 1,
					"primarySourceReaction": "Headache",
					"meddraVersion": "27.0",
					"meddraCode": "10019211"
				} }
			}
		},
		{
			"clientRowId": "index-0",
			"request": {
				"authorities": ["mfds"],
				"rows": { "reaction": {
					"sequenceNumber": 2,
					"primarySourceReaction": "Nausea",
					"meddraVersion": "27.0",
					"meddraCode": "10028813"
				} }
			}
		}
	]);
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{source_case_id}/follow-up"),
		invalid_pages,
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");

	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	let (case_count,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (i64,)>(
				"SELECT count(*) FROM safety_report_identification WHERE safety_report_id = $1",
			)
			.bind(&safety_report_id),
		)
		.await?;
	mm.dbx().commit_txn().await?;
	assert_eq!(
		case_count, 1,
		"failed follow-up must roll back the case shell"
	);

	let mut follow_up_ids = Vec::new();
	for expected_version in [2, 3] {
		let (status, body) = post_json(
			&app,
			&cookie,
			&format!("/api/cases/{source_case_id}/follow-up"),
			follow_up_pages("mfds"),
		)
		.await?;
		assert_eq!(status, StatusCode::CREATED, "{body:?}");
		assert_eq!(body["data"]["safetyReportVersion"], expected_version);
		let created_id = body["data"]["caseId"]
			.as_str()
			.ok_or("missing follow-up id")?;
		let (shell_status, shell) = get_json(
			&app,
			&cookie,
			&format!("/api/cases/{created_id}/editor/shell"),
		)
		.await?;
		assert_eq!(shell_status, StatusCode::OK, "{shell:?}");
		let (header_status, header) = get_json(
			&app,
			&cookie,
			&format!("/api/cases/{created_id}/message-header"),
		)
		.await?;
		assert_eq!(header_status, StatusCode::OK, "{header:?}");
		assert_eq!(
			header["data"]["message_sender_identifier"],
			"FOLLOW-UP-SENDER"
		);
		assert_eq!(header["data"]["message_receiver_identifier"], "KR");
		assert_eq!(
			header["data"]["message_number"],
			format!("MSG-{created_id}")
		);
		follow_up_ids.push(Uuid::parse_str(
			body["data"]["caseId"]
				.as_str()
				.ok_or("missing follow-up case id")?,
		)?);
	}

	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	let rows = mm
		.dbx()
		.fetch_all(
			sqlx::query_as::<_, (Uuid, Option<String>, i32)>(
				"SELECT c.id, c.dg_prd_key, s.version
				   FROM cases c
				   JOIN safety_report_identification s ON s.case_id = c.id
				  WHERE s.safety_report_id = $1
				  ORDER BY s.version",
			)
			.bind(&safety_report_id),
		)
		.await?;
	mm.dbx().commit_txn().await?;

	assert_eq!(rows.len(), 3);
	assert_eq!(rows[1].0, follow_up_ids[0]);
	assert_eq!(rows[2].0, follow_up_ids[1]);
	for (_, product, _) in &rows[1..] {
		assert_eq!(product.as_deref(), Some(product_id.as_str()));
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_from_intake_requires_product_id() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", Uuid::new_v4());
	let intake_body = json!({
		"data": intake_data(&safety_report_id, 141, "1", json!({
			"patient_initials": null,
			"reaction_meddra_version": null,
			"dg_prd_key": null
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", intake_body.clone())
			.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");

	let override_body = json!({
		"data": intake_data(&safety_report_id, 142, "1", json!({
			"patient_initials": null,
			"reaction_meddra_version": null,
			"dg_prd_key": null,
			"allow_duplicate_override": true
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", override_body).await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_from_intake_rolls_back_shell_when_patient_create_fails(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let safety_report_id = format!("INTAKE-{}", Uuid::new_v4());
	let dg_prd_key = format!("ROLLBACK-{safety_report_id}");

	let body = json!({
		"data": intake_data(&safety_report_id, 143, "1", json!({
			"allow_duplicate_override": true,
			"dg_prd_key": dg_prd_key,
			"sex_d5": "9"
		}))
	});
	let (status, _) =
		post_json(&app, &cookie, "/api/cases/from-intake", body).await?;
	assert_ne!(status, StatusCode::CREATED);

	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	let (count,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (i64,)>(
				"SELECT COUNT(*) FROM cases WHERE dg_prd_key = $1",
			)
			.bind(&dg_prd_key),
		)
		.await?;
	mm.dbx().rollback_txn().await?;
	assert_eq!(count, 0, "failed intake must not leave a case shell");

	Ok(())
}
