use super::support::*;

#[serial]
#[tokio::test]
async fn test_reconcile_blocks_legacy_attempts_without_reexport_or_resend(
) -> Result<()> {
	clear_esg_env();
	let (base_url, received) = start_mock_esg(
		StatusCode::CREATED,
		json!({
			"remote_submission_id": "TEST-RETRY",
			"ack": { "level": 1, "success": true, "code": "ACK1" }
		}),
	)
	.await?;
	std::env::set_var("FDA_ESG_ENABLED", "true");
	std::env::set_var("FDA_ESG_BASE_URL", base_url);
	std::env::set_var("FDA_ESG_SUBMIT_PATH", "/submissions");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let mut blocked_ids = Vec::new();
	for (index, status) in ["invalid", "locked", "deleted", "archived", "draft"]
		.iter()
		.enumerate()
	{
		let case_id = create_case(&app, &cookie, seed.org_id).await?;
		seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;
		let submission_id = Uuid::new_v4();
		mm.dbx().begin_txn().await?;
		set_full_context_dbx(
			mm.dbx(),
			seed.admin.id,
			seed.org_id,
			ROLE_SPONSOR_ADMIN_CRO,
		)
		.await?;
		lib_core::model::store::set_compliance_context_dbx(
			mm.dbx(),
			Some("seed retry regression"),
			None,
			None,
		)
		.await?;
		mm.dbx().execute(sqlx::query(
			"INSERT INTO case_submissions (id, case_id, gateway, remote_submission_id, status, xml_bytes, submitted_by, submitted_at, created_at, updated_at)
			 VALUES ($1, $2, 'fda', $3, 'rejected', 0, $4, now(), now(), now())"
		).bind(submission_id).bind(case_id).bind(format!("RETRY-{submission_id}")).bind(seed.admin.id)).await?;
		mm.dbx().execute(sqlx::query(
			"INSERT INTO submission_dispatch_state (submission_id, attempt_count, next_retry_at, created_at, updated_at)
			 VALUES ($1, 1, now() - interval '10 minutes' + $2 * interval '1 second', now(), now())"
		).bind(submission_id).bind(index as f64)).await?;
		if *status == "invalid" {
			mm.dbx().execute(sqlx::query("UPDATE patient_information SET age_at_time_of_onset = 40, age_unit = 'a', age_group = '5' WHERE case_id = $1").bind(case_id)).await?;
		} else {
			mm.dbx()
				.execute(
					sqlx::query("UPDATE cases SET status = $2 WHERE id = $1")
						.bind(case_id)
						.bind(status),
				)
				.await?;
		}
		mm.dbx().commit_txn().await?;
		blocked_ids.push(submission_id);
	}
	let result =
		web_server::submission::reconcile_due_submissions_with_runtime_status(
			&mm, 25,
		)
		.await?;
	assert_eq!(result.attempted, 5);
	assert_eq!(result.failed, 5);
	assert_eq!(result.succeeded, 0);
	assert_eq!(
		received.lock().await.len(),
		0,
		"legacy attempts must never re-export or resend even if the Case is valid"
	);
	for submission_id in blocked_ids {
		let state = web_server::submission::get_submission_dispatch_state(
			&lib_core::ctx::Ctx::root_ctx(),
			&mm,
			submission_id,
		)
		.await?;
		let state = state.ok_or("missing dispatch state")?;
		assert!(state.next_retry_at.is_none());
		assert!(state.last_error.is_some());
		assert_eq!(
			state.attempt_count, 1,
			"preparation failure is not a gateway attempt"
		);
	}
	let result =
		web_server::submission::reconcile_due_submissions_with_runtime_status(
			&mm, 25,
		)
		.await?;
	assert_eq!(
		result.attempted, 0,
		"invalid cases must not keep poisoning the retry queue"
	);
	clear_esg_env();
	Ok(())
}

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

#[serial]
#[tokio::test]
async fn test_as2_pending_reconciles_without_resending_and_ignores_late_ack(
) -> Result<()> {
	use std::sync::atomic::{AtomicUsize, Ordering};
	clear_esg_env();
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let observer = mm.clone();
	let observer_user = seed.admin.id;
	let observer_org = seed.org_id;
	let sends = Arc::new(AtomicUsize::new(0));
	let polls = Arc::new(AtomicUsize::new(0));
	let submit_count = sends.clone();
	let poll_count = polls.clone();
	// Deterministic protocol fixture, not regulator acceptance evidence.
	let transport = Router::new()
        .route("/submit", post(move |Json(payload): Json<Value>| { let count = submit_count.clone(); let db = observer.clone(); async move {
            // A separate DB session must see the reservation before the first network send.
            db.dbx().begin_txn().await.unwrap();
            set_full_context_dbx(db.dbx(), observer_user, observer_org, ROLE_SPONSOR_ADMIN_CRO).await.unwrap();
            let case_id = Uuid::parse_str(payload["caseId"].as_str().unwrap()).unwrap();
            let durable = db.dbx().fetch_one(sqlx::query_as::<_, (i64,)>(
                "SELECT count(*) FROM case_submissions s JOIN submission_idempotency i ON i.submission_id=s.id
                 WHERE s.case_id=$1 AND s.authority='fda' AND s.submitted_xml=convert_to($2,'UTF8') AND i.idempotency_key='durable-key'")
                .bind(case_id).bind(payload["xmlPayload"].as_str().unwrap())).await.unwrap().0;
            assert_eq!(durable, 1, "reservation and key must be committed before I/O");
            let unlocked = db.dbx().fetch_one(sqlx::query_as::<_, (bool,)>(
                "SELECT pg_try_advisory_xact_lock(hashtextextended('e2br3.audit_logs.hash_chain',0))")).await.unwrap().0;
            assert!(unlocked, "network I/O must not hold the audit chain lock");
            db.dbx().commit_txn().await.unwrap();
            count.fetch_add(1, Ordering::SeqCst);
            Json(json!({"remote_submission_id":"pending-fixture", "status":"submitted_ack1_pending"}))
        }}))
        .route("/submissions/status", post(move || { let count = poll_count.clone(); async move {
            count.fetch_add(1, Ordering::SeqCst);
            Json(json!({"remote_submission_id":"pending-fixture", "status":"ack3_received",
                "latest_ack":{"level":3,"success":true,"code":"ACK3"}}))
        }}));
	let listener = TcpListener::bind("127.0.0.1:0").await?;
	let addr = listener.local_addr()?;
	let server = tokio::spawn(async move { axum::serve(listener, transport).await });
	std::env::set_var("AS2_SUBMITTER_URL", format!("http://{addr}"));
	std::env::set_var("AS2_SUBMITTER_TOKEN", "fixture-token");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");

	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&mm, &app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;
	let (status, body) = post_json_with_headers(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
		&[("x-idempotency-key", "durable-key")],
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	assert_eq!(body["data"]["status"], "submitted_ack1_pending");
	for level in 1..=4 {
		assert!(body["data"][format!("ack{level}")].is_null());
	}

	let id = Uuid::parse_str(body["data"]["id"].as_str().ok_or("id missing")?)?;
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	mm.dbx().execute(sqlx::query("UPDATE submission_dispatch_state SET next_retry_at=now()-interval '1 second' WHERE submission_id=$1").bind(id)).await?;
	mm.dbx().commit_txn().await?;
	web_server::submission::reconcile_due_submissions_with_runtime_status(&mm, 25)
		.await?;
	assert_eq!(sends.load(Ordering::SeqCst), 1);
	assert_eq!(polls.load(Ordering::SeqCst), 1);
	let (_, observed) =
		get_json(&app, &cookie, &format!("/api/submissions/{id}")).await?;
	assert_eq!(observed["data"]["status"], "ack3_received", "{observed}");
	let ack = web_server::submission::GatewayAckCallbackInput {
		remote_submission_id: "pending-fixture".into(),
		ack_level: 1,
		success: false,
		ack_code: Some("late-negative".into()),
		ack_message: Some("out of order fixture".into()),
	};
	let after =
		web_server::submission::apply_gateway_ack_by_remote(&mm, ack).await?;
	assert_eq!(
		after.status,
		web_server::submission::SubmissionStatus::Ack3Received
	);
	let (_, dispatch) = get_json(
		&app,
		&cookie,
		&format!("/api/submissions/{id}/dispatch-state"),
	)
	.await?;
	assert!(
		!dispatch["data"]["state"]["terminal_at"].is_null(),
		"{dispatch}"
	);
	assert!(dispatch["data"]["state"]["next_retry_at"].is_null());
	server.abort();
	clear_esg_env();
	Ok(())
}
