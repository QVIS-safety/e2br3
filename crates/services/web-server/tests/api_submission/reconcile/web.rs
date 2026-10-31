use super::super::support::*;

#[serial]
#[tokio::test]
async fn test_internal_reconcile_endpoint_auth_and_empty_result() -> Result<()> {
	clear_esg_env();
	std::env::set_var("SUBMISSION_RECONCILE_TOKEN", "reconcile-secret");
	let mm = init_test_mm().await?;
	let app = web_server::app(mm);

	let req = Request::builder()
		.method("POST")
		.uri("/internal/submissions/reconcile")
		.header("content-type", "application/json")
		.header("x-reconcile-token", "reconcile-secret")
		.body(Body::from(json!({ "limit": 5 }).to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let _attempted = value["data"]["result"]["attempted"]
		.as_u64()
		.ok_or("missing attempted")?;
	let req = Request::builder()
		.method("GET")
		.uri("/internal/submissions/reconcile/status")
		.header("x-reconcile-token", "reconcile-secret")
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert!(
		value["data"]["status"]["total_runs"]
			.as_u64()
			.ok_or("missing total_runs")?
			>= 1
	);

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_internal_reconcile_status_accumulates_runtime_counters() -> Result<()>
{
	clear_esg_env();
	std::env::set_var("SUBMISSION_RECONCILE_TOKEN", "reconcile-secret");
	let mm = init_test_mm().await?;
	let app = web_server::app(mm);

	for _ in 0..2 {
		let req = Request::builder()
			.method("POST")
			.uri("/internal/submissions/reconcile")
			.header("content-type", "application/json")
			.header("x-reconcile-token", "reconcile-secret")
			.body(Body::from(json!({ "limit": 25 }).to_string()))?;
		let res = app.clone().oneshot(req).await?;
		let status = res.status();
		let body = to_bytes(res.into_body(), usize::MAX).await?;
		let value: Value = serde_json::from_slice(&body)?;
		assert_eq!(status, StatusCode::OK, "{value:?}");
		let _attempted = value["data"]["result"]["attempted"]
			.as_u64()
			.ok_or("missing attempted")?;
	}

	let req = Request::builder()
		.method("GET")
		.uri("/internal/submissions/reconcile/status")
		.header("x-reconcile-token", "reconcile-secret")
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert!(
		value["data"]["status"]["total_runs"]
			.as_u64()
			.ok_or("missing total_runs")?
			>= 2
	);
	let _total_attempted = value["data"]["status"]["total_attempted"]
		.as_u64()
		.ok_or("missing total_attempted")?;

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_internal_reconcile_does_not_resend_esg_attempt() -> Result<()> {
	clear_esg_env();
	configure_mock_esg_transport().await?;
	std::env::set_var("AS2_CALLBACK_TOKEN", "callback-secret");
	std::env::set_var("SUBMISSION_RECONCILE_TOKEN", "reconcile-secret");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (submit_status, submit_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(submit_status, StatusCode::CREATED, "{submit_body:?}");
	let submission_id = submit_body["data"]["id"]
		.as_str()
		.ok_or("missing submission id")?
		.to_string();
	let submission_uuid = Uuid::parse_str(&submission_id)?;
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
				"UPDATE case_submissions
				 SET status = 'rejected', updated_at = now()
				 WHERE id = $1",
			)
			.bind(submission_uuid),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO submission_dispatch_state (
					submission_id, attempt_count, last_attempt_at, last_error, next_retry_at, terminal_at, created_at, updated_at
				)
				VALUES ($1, 1, now() - interval '2 seconds', 'seed-failure', now() - interval '1 second', NULL, now(), now())
				ON CONFLICT (submission_id)
				DO UPDATE SET
					attempt_count = EXCLUDED.attempt_count,
					last_attempt_at = EXCLUDED.last_attempt_at,
					last_error = EXCLUDED.last_error,
					next_retry_at = EXCLUDED.next_retry_at,
					terminal_at = NULL,
					updated_at = now()",
			)
			.bind(submission_uuid),
		)
		.await?;
	mm.dbx().commit_txn().await?;

	sleep(Duration::from_millis(100)).await;

	let req = Request::builder()
		.method("POST")
		.uri("/internal/submissions/reconcile")
		.header("content-type", "application/json")
		.header("x-reconcile-token", "reconcile-secret")
		.body(Body::from(json!({ "limit": 25 }).to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let reconcile_value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::OK, "{reconcile_value:?}");
	let processed = reconcile_value["data"]["result"]["processed_submission_ids"]
		.as_array()
		.ok_or("missing processed ids")?;
	assert!(
		processed.iter().any(|v| v.as_str() == Some(&submission_id)),
		"{reconcile_value:?}"
	);

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/submissions/{submission_id}"))
		.header("cookie", &cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let status_text = value["data"]["status"]
		.as_str()
		.ok_or("missing submission status")?;
	assert_eq!(status_text, "rejected", "{value:?}");

	let (dispatch_status, dispatch_body) = get_json(
		&app,
		&cookie,
		&format!("/api/submissions/{submission_id}/dispatch-state"),
	)
	.await?;
	assert_eq!(dispatch_status, StatusCode::OK, "{dispatch_body:?}");
	assert!(
		dispatch_body["data"]["state"]["attempt_count"]
			.as_i64()
			.ok_or("missing attempt_count")?
			>= 1
	);

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_internal_reconcile_transport_change_does_not_resend_esg_attempt(
) -> Result<()> {
	clear_esg_env();
	configure_mock_esg_transport().await?;
	std::env::set_var("AS2_CALLBACK_TOKEN", "callback-secret");
	std::env::set_var("SUBMISSION_RECONCILE_TOKEN", "reconcile-secret");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (submit_status, submit_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(submit_status, StatusCode::CREATED, "{submit_body:?}");
	let submission_id = submit_body["data"]["id"]
		.as_str()
		.ok_or("missing submission id")?
		.to_string();
	let submission_uuid = Uuid::parse_str(&submission_id)?;
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
				"UPDATE case_submissions
				 SET status = 'rejected', updated_at = now()
				 WHERE id = $1",
			)
			.bind(submission_uuid),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO submission_dispatch_state (
					submission_id, attempt_count, last_attempt_at, last_error, next_retry_at, terminal_at, created_at, updated_at
				)
				VALUES ($1, 1, now() - interval '2 seconds', 'seed-failure', now() - interval '1 second', NULL, now(), now())
				ON CONFLICT (submission_id)
				DO UPDATE SET
					attempt_count = EXCLUDED.attempt_count,
					last_attempt_at = EXCLUDED.last_attempt_at,
					last_error = EXCLUDED.last_error,
					next_retry_at = EXCLUDED.next_retry_at,
					terminal_at = NULL,
					updated_at = now()",
			)
			.bind(submission_uuid),
		)
		.await?;
	mm.dbx().commit_txn().await?;

	std::env::set_var("AS2_SUBMITTER_URL", "http://127.0.0.1:9");
	std::env::set_var("AS2_SUBMITTER_TOKEN", "test-token");
	std::env::set_var("AS2_SUBMITTER_TIMEOUT_SECS", "1");
	sleep(Duration::from_millis(100)).await;
	let req = Request::builder()
		.method("POST")
		.uri("/internal/submissions/reconcile")
		.header("content-type", "application/json")
		.header("x-reconcile-token", "reconcile-secret")
		.body(Body::from(json!({ "limit": 25 }).to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let reconcile_value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::OK, "{reconcile_value:?}");
	let processed = reconcile_value["data"]["result"]["processed_submission_ids"]
		.as_array()
		.ok_or("missing processed ids")?;
	assert!(
		processed.iter().any(|v| v.as_str() == Some(&submission_id)),
		"{reconcile_value:?}"
	);

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/submissions/{submission_id}"))
		.header("cookie", &cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["data"]["status"], "rejected", "{value:?}");

	let (dispatch_status, dispatch_body) = get_json(
		&app,
		&cookie,
		&format!("/api/submissions/{submission_id}/dispatch-state"),
	)
	.await?;
	assert_eq!(dispatch_status, StatusCode::OK, "{dispatch_body:?}");
	assert!(
		dispatch_body["data"]["state"]["attempt_count"]
			.as_i64()
			.ok_or("missing attempt_count")?
			>= 1
	);

	clear_esg_env();
	Ok(())
}
