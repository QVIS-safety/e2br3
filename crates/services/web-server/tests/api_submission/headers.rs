use super::support::*;

#[serial]
#[tokio::test]
async fn test_submission_receiver_options_list_defaults_by_authority() -> Result<()>
{
	clear_esg_env();
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, value) = get_json(
		&app,
		&cookie,
		"/api/submissions/receiver-options?authority=fda",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let items = value["data"]["items"]
		.as_array()
		.ok_or("missing receiver option items")?;
	assert!(items.iter().any(|item| {
		item["receiver_label"].as_str() == Some("FDA(Postmarket)")
			&& item["batch_receiver_identifier"].as_str() == Some("ZZFDA")
			&& item["message_receiver_identifier"].as_str() == Some("CDER")
			&& item["condition_field_code"].as_str() == Some("FDA_REPORT_TYPE")
			&& item["condition_value_code"].as_str() == Some("4")
	}));

	let (mfds_status, mfds_value) = get_json(
		&app,
		&cookie,
		"/api/submissions/receiver-options?authority=mfds",
	)
	.await?;
	assert_eq!(mfds_status, StatusCode::OK, "{mfds_value:?}");

	let mfds_items = mfds_value["data"]["items"]
		.as_array()
		.ok_or("missing MFDS receiver option items")?;
	assert_eq!(mfds_items.len(), 5, "{mfds_items:?}");
	for (
		receiver_label,
		condition_value_code,
		condition_value_label,
		batch_receiver_identifier,
		message_receiver_identifier,
	) in [
		(
			"MFDS(CT)",
			"1",
			"임상시험계획의 승인을 받은 자",
			"MFDS-O-CT",
			"MFDS-O-CT",
		),
		(
			"MFDS(CU)",
			"2",
			"임상시험용의약품의 치료목적 사용승인을 받은 자",
			"MFDS-O-CU",
			"MFDS-O-CU",
		),
		(
			"MFDS(KR)",
			"3",
			"시판 후 이상사례 국내보고",
			"MFDS-O-KR",
			"MFDS-O-KR",
		),
		(
			"MFDS(FR)",
			"4",
			"시판 후 이상사례 국외보고",
			"MFDS-O-FR",
			"MFDS-O-FR",
		),
		(
			"MFDS(CF)",
			"5",
			"임상시험계획의 승인을 받은 자 (국외)",
			"MFDS-O-CF",
			"MFDS-O-CF",
		),
	] {
		assert!(
			mfds_items.iter().any(|item| {
				item["receiver_label"].as_str() == Some(receiver_label)
					&& item["condition_field_code"].as_str()
						== Some("MFDS_REPORT_TYPE")
					&& item["condition_value_code"].as_str()
						== Some(condition_value_code)
					&& item["condition_value_label"].as_str()
						== Some(condition_value_label)
					&& item["batch_receiver_identifier"].as_str()
						== Some(batch_receiver_identifier)
					&& item["message_receiver_identifier"].as_str()
						== Some(message_receiver_identifier)
			}),
			"missing MFDS receiver option {receiver_label}: {mfds_items:?}"
		);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_receiver_selection_updates_message_header_n_identifiers(
) -> Result<()> {
	clear_esg_env();
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	for lifecycle in ["draft", "reviewed", "validated", "locked"] {
		let case_id = create_case(&app, &cookie, seed.org_id).await?;
		create_message_header(&app, &cookie, case_id).await?;
		let safety_report_id = format!("SYSTEM-HEADER-{case_id}");

		mm.dbx().begin_txn().await?;
		set_full_context_dbx(
			mm.dbx(),
			seed.admin.id,
			seed.org_id,
			ROLE_SPONSOR_ADMIN_CRO,
		)
		.await?;
		let sender_presave_id = Uuid::new_v4();
		mm.dbx()
			.execute(
				sqlx::query(
					"INSERT INTO sender_presaves (id, organization_id, organization_name, created_by)
					 VALUES ($1, $2, 'Outbound Sender', $3)",
				)
				.bind(sender_presave_id)
				.bind(seed.org_id)
				.bind(seed.admin.id),
			)
			.await?;
		mm.dbx()
			.execute(
				sqlx::query(
					"INSERT INTO sender_presave_gateways (
						sender_presave_id, sequence_number, gateway_authority,
						sender_identifier, is_default_for_authority, created_by
					 ) VALUES ($1, 1, 'mfds', 'OUTBOUND-SENDER', true, $2)",
				)
				.bind(sender_presave_id)
				.bind(seed.admin.id),
			)
			.await?;
		mm.dbx()
			.execute(
				sqlx::query(
					"INSERT INTO sender_information (case_id, source_sender_presave_id, created_by)
					 VALUES ($2, $1, $3)
					 ON CONFLICT (case_id) DO UPDATE SET source_sender_presave_id = EXCLUDED.source_sender_presave_id",
				)
				.bind(sender_presave_id)
				.bind(case_id)
				.bind(seed.admin.id),
			)
			.await?;
		mm.dbx()
			.execute(
				sqlx::query(
					"UPDATE cases SET status = $1, mfds_report_type = '3' WHERE id = $2",
				)
				.bind(lifecycle)
				.bind(case_id),
			)
			.await?;
		mm.dbx()
			.execute(
				sqlx::query(
					"UPDATE safety_report_identification
					 SET safety_report_id = $1, transmission_date = '20260806010203'
					 WHERE case_id = $2",
				)
				.bind(&safety_report_id)
				.bind(case_id),
			)
			.await?;
		mm.dbx().commit_txn().await?;

		let (response_status, body) = put_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/submission-receiver"),
			json!({
				"data": {
					"authority": "mfds",
					"receiver_label": "MFDS(KR)"
				}
			}),
		)
		.await?;

		assert_eq!(response_status, StatusCode::OK, "{body:?}");
		assert_eq!(
			body["data"]["batch_receiver_identifier"].as_str(),
			Some("MFDS-O-KR"),
			"{body:?}"
		);
		assert_eq!(
			body["data"]["message_receiver_identifier"].as_str(),
			Some("MFDS-O-KR"),
			"{body:?}"
		);

		let (header_status, header) = get_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/message-header"),
		)
		.await?;
		assert_eq!(header_status, StatusCode::OK, "{header:?}");
		assert_eq!(
			header["data"]["message_number"].as_str(),
			Some(format!("MSG-{case_id}").as_str())
		);
		assert_eq!(
			header["data"]["message_sender_identifier"].as_str(),
			Some("OUTBOUND-SENDER")
		);
		assert_eq!(
			header["data"]["batch_number"].as_str(),
			Some(format!("BATCH-{case_id}").as_str())
		);
		assert_eq!(
			header["data"]["batch_sender_identifier"].as_str(),
			Some("OUTBOUND-SENDER")
		);
		assert_eq!(
			header["data"]["message_date"].as_str(),
			Some("20260806010203")
		);
		assert_eq!(header["data"]["batch_transmission_date"][0], 2026);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn concurrent_authority_header_preparation_returns_request_scoped_values(
) -> Result<()> {
	clear_esg_env();
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_case(&app, &cookie, seed.org_id).await?;

	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	let sender_presave_id = Uuid::new_v4();
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO sender_presaves (id, organization_id, organization_name, created_by)
				 VALUES ($1, $2, 'Concurrent Sender', $3)",
			)
			.bind(sender_presave_id)
			.bind(seed.org_id)
			.bind(seed.admin.id),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO sender_presave_gateways (
					sender_presave_id, sequence_number, gateway_authority,
					sender_identifier, is_default_for_authority, created_by
				 ) VALUES
					($1, 1, 'fda', 'FDA-SENDER', true, $2),
					($1, 2, 'mfds', 'MFDS-SENDER', true, $2)",
			)
			.bind(sender_presave_id)
			.bind(seed.admin.id),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO sender_information (case_id, source_sender_presave_id, created_by)
				 VALUES ($1, $2, $3)",
			)
			.bind(case_id)
			.bind(sender_presave_id)
			.bind(seed.admin.id),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"UPDATE cases SET fda_report_type = '4', mfds_report_type = '3'
				 WHERE id = $1",
			)
			.bind(case_id),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"UPDATE safety_report_identification
				 SET safety_report_id = $1, transmission_date = '20260814000000'
				 WHERE case_id = $2",
			)
			.bind(format!("CONCURRENT-{case_id}"))
			.bind(case_id),
		)
		.await?;
	mm.dbx().commit_txn().await?;

	let uri = format!("/api/cases/{case_id}/submission-receiver");
	let (fda, mfds) = tokio::join!(
		put_json(
			&app,
			&cookie,
			&uri,
			json!({ "data": { "authority": "fda" } }),
		),
		put_json(
			&app,
			&cookie,
			&uri,
			json!({ "data": { "authority": "mfds" } }),
		)
	);
	let (fda_status, fda) = fda?;
	let (mfds_status, mfds) = mfds?;
	assert_eq!(fda_status, StatusCode::OK, "{fda:?}");
	assert_eq!(mfds_status, StatusCode::OK, "{mfds:?}");
	assert_eq!(
		fda["data"]["message_receiver_identifier"].as_str(),
		Some("CDER"),
		"{fda:?}"
	);
	assert_eq!(
		mfds["data"]["message_receiver_identifier"].as_str(),
		Some("MFDS-O-KR"),
		"{mfds:?}"
	);
	assert_eq!(
		fda["data"]["message_sender_identifier"].as_str(),
		Some("FDA-SENDER"),
		"{fda:?}"
	);
	assert_eq!(
		mfds["data"]["message_sender_identifier"].as_str(),
		Some("MFDS-SENDER"),
		"{mfds:?}"
	);

	Ok(())
}
