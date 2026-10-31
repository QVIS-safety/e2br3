use super::support::*;

#[serial]
#[tokio::test]
async fn test_submission_history_includes_latest_ack_time_and_event() -> Result<()> {
	clear_esg_env();
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let submission_id = Uuid::new_v4();
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO case_submissions (
					id, case_id, gateway, remote_submission_id, status, xml_bytes,
					submitted_by, submitted_at, created_at, updated_at
				)
				VALUES ($1, $2, $3, $4, $5, $6, $7, now(), now(), now())",
			)
			.bind(submission_id)
			.bind(case_id)
			.bind("fda-esg-nextgen-api")
			.bind("BATCH-HISTORY-1")
			.bind("ack2_received")
			.bind(2048_i32)
			.bind(seed.admin.id),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO submission_acks (
					submission_id, ack_level, success, ack_code, ack_message, received_at, raw_payload
				)
				VALUES ($1, $2, $3, $4, $5, now() - interval '5 minutes', $6),
				       ($1, $7, $8, $9, $10, now() - interval '1 minutes', $11)",
			)
			.bind(submission_id)
			.bind(1_i16)
			.bind(true)
			.bind("ACK1")
			.bind(Option::<String>::None)
			.bind(json!({ "level": 1, "success": true, "code": "ACK1" }))
			.bind(2_i16)
			.bind(true)
			.bind("ACK2")
			.bind(Option::<String>::None)
			.bind(json!({ "level": 2, "success": true, "code": "ACK2" })),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO submission_events (
					submission_id, event_type, event_data, created_at
				)
				VALUES ($1, $2, $3, now() - interval '10 minutes'),
				       ($1, $4, $5, now() - interval '30 seconds')",
			)
			.bind(submission_id)
			.bind("submission_created")
			.bind(json!({ "status": "ack1_received" }))
			.bind("ack_recorded")
			.bind(json!({ "ack_level": 2, "ack_code": "ACK2" })),
		)
		.await?;
	mm.dbx().commit_txn().await?;

	let (status, history) =
		get_json(&app, &cookie, "/api/submissions/history").await?;
	assert_eq!(status, StatusCode::OK, "{history:?}");
	let items = history["data"]["items"]
		.as_array()
		.ok_or("missing submission history items")?;
	let submission_id_str = submission_id.to_string();
	let item = items
		.iter()
		.find(|item| {
			item["submissionId"].as_str() == Some(submission_id_str.as_str())
		})
		.ok_or("missing submission history row for created submission")?;
	assert_eq!(item["status"].as_str(), Some("ack2_received"));
	assert_eq!(item["batchResult"].as_str(), Some("ack2_received"));
	assert_eq!(item["messageResult"].as_str(), Some("ACK2"));
	assert_eq!(item["latestEventType"].as_str(), Some("ack_recorded"));
	assert!(
		item["latestAckReceivedAt"]
			.as_str()
			.is_some_and(|value| !value.trim().is_empty()),
		"{item:?}"
	);
	assert_eq!(item["acknowledgedDate"], item["latestAckReceivedAt"]);
	assert_eq!(item["icsrCount"].as_i64(), Some(1));
	assert_eq!(
		item["dataFileName"],
		format!("submission-{submission_id}.xml")
	);
	assert_eq!(
		item["dataFileDownloadUrl"],
		format!("/api/submissions/{submission_id}/xml")
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_ack_can_be_downloaded_as_text() -> Result<()> {
	clear_esg_env();
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let submission_id = Uuid::new_v4();
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO case_submissions (
					id, case_id, gateway, remote_submission_id, status, xml_bytes,
					submitted_by, submitted_at, created_at, updated_at
				)
				VALUES ($1, $2, $3, $4, $5, $6, $7, now(), now(), now())",
			)
			.bind(submission_id)
			.bind(case_id)
			.bind("fda")
			.bind("REMOTE-ACK-DOWNLOAD")
			.bind("ack2_received")
			.bind(2048_i32)
			.bind(seed.admin.id),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO submission_acks (
					submission_id, ack_level, success, ack_code, ack_message, received_at, raw_payload
				)
				VALUES ($1, $2, $3, $4, $5, now(), $6)",
			)
			.bind(submission_id)
			.bind(2_i16)
			.bind(true)
			.bind("ACK2")
			.bind("Accepted by gateway")
			.bind(json!({
				"level": 2,
				"success": true,
				"code": "ACK2",
				"message": "Accepted by gateway"
			})),
		)
		.await?;
	mm.dbx().commit_txn().await?;

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/submissions/{submission_id}/acks/2/download"))
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let content_type = res
		.headers()
		.get("content-type")
		.and_then(|v| v.to_str().ok())
		.unwrap_or("")
		.to_string();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let text = String::from_utf8(body.to_vec())?;
	assert_eq!(status, StatusCode::OK, "{text}");
	assert!(
		content_type.starts_with("text/plain"),
		"unexpected content type: {content_type}"
	);
	assert!(text.contains("Submission ID:"));
	assert!(text.contains("ACK Level: 2"));
	assert!(text.contains("ACK Code: ACK2"));
	assert!(text.contains("Accepted by gateway"));

	Ok(())
}
