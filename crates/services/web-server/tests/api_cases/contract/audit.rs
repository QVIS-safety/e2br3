use super::support::{get_json, post_json, put_json};
use crate::case_support::*;

#[serial]
#[tokio::test]
async fn test_audit_trail_case_crud() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_id = seed.admin.id;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;

	let app = web_server::app(mm.clone());
	let case_body = json!({
		"data": {
			"safetyReportIdentification": {
				"safetyReportId": format!("SR-{}", Uuid::new_v4())
			},
			"status": "draft"
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri("/api/cases")
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(case_body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::CREATED);

	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	let case_id = value
		.get("data")
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing case id")?;

	let update_body = json!({
		"data": {
			"status": "submitted"
		},
		"reason_for_change": "audit test submit transition",
		"e_signature": {
			"meaning": "submit case for audit test",
			"password": "adminpwd"
		}
	});
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/cases/{case_id}"))
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(update_body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("DELETE")
		.uri(format!("/api/cases/{case_id}"))
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(
			json!({
				"reason_for_change": "audit test delete transition",
				"e_signature": {
					"meaning": "delete case for audit test",
					"password": "adminpwd"
				}
			})
			.to_string(),
		))?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	let query = sqlx::query("SET ROLE e2br3_auditor_role");
	dbx.execute(query).await?;
	let query = sqlx::query_as::<_, (i64,)>(
		"SELECT COUNT(*) FROM audit_logs WHERE table_name = 'cases' AND record_id = $1 AND user_id = $2",
	)
	.bind(Uuid::parse_str(case_id)?)
	.bind(admin_id);
	let (count,) = dbx.fetch_one(query).await?;
	dbx.rollback_txn().await?;
	assert!(count >= 3);

	Ok(())
}

#[serial]
#[tokio::test]
async fn long_hangul_narrative_update_preserves_jsonb_and_audit_contracts(
) -> Result<()> {
	let mm = init_test_mm().await?;

	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let old_narrative = "가".repeat(100_000);
	let new_narrative = "나".repeat(100_000);

	let (create_status, create_body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safetyReportIdentification": {
					"safetyReportId": format!("SR-HANGUL-{}", Uuid::new_v4())
				},
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(create_status, StatusCode::CREATED, "{create_body:?}");
	let case_id = Uuid::parse_str(
		create_body["data"]["id"]
			.as_str()
			.ok_or("missing created case id")?,
	)?;

	let (narrative_status, narrative_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/narrative"),
		json!({"data": {
			"case_id": case_id,
			"case_narrative": old_narrative
		}}),
	)
	.await?;
	assert_eq!(narrative_status, StatusCode::CREATED, "{narrative_body:?}");
	let narrative_id = Uuid::parse_str(
		narrative_body["data"]["id"]
			.as_str()
			.ok_or("missing narrative id")?,
	)?;

	let (update_status, update_body) = tokio::time::timeout(
		std::time::Duration::from_secs(10),
		put_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/narrative"),
			json!({"data": {"case_narrative": new_narrative}}),
		),
	)
	.await
	.map_err(|_| "100,000-character Hangul narrative update timed out")??;
	assert_eq!(update_status, StatusCode::OK, "{update_body:?}");
	assert_eq!(
		update_body["data"]["case_narrative"].as_str(),
		Some(new_narrative.as_str())
	);

	let (get_status, get_body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/narrative")).await?;
	assert_eq!(get_status, StatusCode::OK, "{get_body:?}");
	assert_eq!(
		get_body["data"]["case_narrative"].as_str(),
		Some(new_narrative.as_str())
	);

	let mut tx = mm.dbx().db().begin().await?;
	sqlx::query(
		"WITH user_context AS MATERIALIZED (
			SELECT set_current_user_context($1)
		)
		SELECT set_org_context($2, $3)
		FROM user_context",
	)
	.bind(seed.admin.id)
	.bind(seed.org_id)
	.bind(ROLE_SPONSOR_ADMIN_CRO)
	.execute(&mut *tx)
	.await?;
	let (dirty_h,) =
		sqlx::query_as::<_, (bool,)>("SELECT dirty_h FROM cases WHERE id = $1")
			.bind(case_id)
			.fetch_one(&mut *tx)
			.await?;
	assert!(dirty_h);
	sqlx::query("SET ROLE e2br3_auditor_role")
		.execute(&mut *tx)
		.await?;
	let (audit_count, first_audit_id, audit_old, audit_new, stored_new) =
		sqlx::query_as::<
			_,
			(
				i64,
				Option<i64>,
				Option<String>,
				Option<String>,
				Option<String>,
			),
		>(
			r#"
				SELECT count(*)::bigint,
				       min(id),
				       max(changed_fields->'case_narrative'->>'old'),
				       max(changed_fields->'case_narrative'->>'new'),
				       max(new_values->>'case_narrative')
				FROM audit_logs
				WHERE table_name = 'narrative_information'
				  AND record_id = $1
				  AND action = 'UPDATE'
				"#,
		)
		.bind(narrative_id)
		.fetch_one(&mut *tx)
		.await?;
	tx.rollback().await?;
	assert_eq!(audit_count, 1);
	assert_eq!(audit_old.as_deref(), Some(old_narrative.as_str()));
	assert_eq!(audit_new.as_deref(), Some(new_narrative.as_str()));
	assert_eq!(stored_new.as_deref(), Some(new_narrative.as_str()));
	let (broken_hash_rows,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (i64,)>(
				"SELECT broken_rows FROM verify_audit_log_hash_chain($1)",
			)
			.bind(first_audit_id.ok_or("missing narrative update audit id")?),
		)
		.await?;
	assert_eq!(broken_hash_rows, 0, "audit hash chain verification failed");

	let (noop_status, noop_body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/narrative"),
		json!({"data": {"case_narrative": new_narrative}}),
	)
	.await?;
	assert_eq!(noop_status, StatusCode::OK, "{noop_body:?}");

	mm.dbx().begin_txn().await?;
	mm.dbx()
		.execute(sqlx::query("SET ROLE e2br3_auditor_role"))
		.await?;
	let (audit_count_after_noop,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (i64,)>(
				"SELECT count(*) FROM audit_logs WHERE table_name = 'narrative_information' AND record_id = $1 AND action = 'UPDATE'",
			)
			.bind(narrative_id),
		)
		.await?;
	mm.dbx().rollback_txn().await?;
	assert_eq!(audit_count_after_noop, audit_count);

	Ok(())
}
