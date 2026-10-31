use super::support::{delete_json, get_json, post_json, put_json};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_delete_case_requires_reason_for_change() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (create_status, create_body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safetyReportIdentification": {"safetyReportId": format!("SR-{}", Uuid::new_v4())},
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(create_status, StatusCode::CREATED, "{create_body:?}");
	let case_id = create_body["data"]["id"]
		.as_str()
		.ok_or("missing created case id")?
		.to_string();

	let (delete_status, delete_body) =
		delete_json(&app, &cookie, &format!("/api/cases/{case_id}"), json!({}))
			.await?;
	assert_eq!(delete_status, StatusCode::BAD_REQUEST, "{delete_body:?}");
	assert!(
		delete_body
			.to_string()
			.contains("reason_for_change is required"),
		"{delete_body:?}"
	);

	let (delete_status, delete_body) = delete_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({ "reason_for_change": "duplicate created in error" }),
	)
	.await?;
	assert_eq!(delete_status, StatusCode::BAD_REQUEST, "{delete_body:?}");
	assert!(
		delete_body
			.to_string()
			.contains("e_signature is required for delete"),
		"{delete_body:?}"
	);

	let (delete_status, delete_body) = delete_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"reason_for_change": "duplicate created in error",
			"e_signature": {
				"meaning": "delete duplicate case",
				"password": "wrong-password"
			}
		}),
	)
	.await?;
	assert_eq!(delete_status, StatusCode::BAD_REQUEST, "{delete_body:?}");
	assert!(
		delete_body
			.to_string()
			.contains("invalid e-signature credentials"),
		"{delete_body:?}"
	);

	let (get_status, get_body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}")).await?;
	assert_eq!(get_status, StatusCode::OK, "{get_body:?}");
	assert_eq!(
		get_body["data"]["status"].as_str(),
		Some("draft"),
		"{get_body:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_delete_case_soft_deletes_and_keeps_case_visible() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let report_id = format!("SR-{}", Uuid::new_v4());
	let (create_status, create_body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safetyReportIdentification": {"safetyReportId": report_id},
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(create_status, StatusCode::CREATED, "{create_body:?}");
	let case_id = create_body["data"]["id"]
		.as_str()
		.ok_or("missing created case id")?
		.to_string();

	let (delete_status, delete_body) = delete_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"reason_for_change": "client requested soft delete",
			"e_signature": {
				"meaning": "delete case",
				"password": "adminpwd"
			}
		}),
	)
	.await?;
	assert_eq!(delete_status, StatusCode::OK, "{delete_body:?}");
	assert_eq!(
		delete_body["data"]["status"].as_str(),
		Some("deleted"),
		"{delete_body:?}"
	);

	let (get_status, get_body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}")).await?;
	assert_eq!(get_status, StatusCode::OK, "{get_body:?}");
	assert_eq!(
		get_body["data"]["status"].as_str(),
		Some("deleted"),
		"{get_body:?}"
	);

	let (list_status, list_body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases?filters%5Bsafety_report_id%5D%5B%24eq%5D={report_id}"),
	)
	.await?;
	assert_eq!(list_status, StatusCode::OK, "{list_body:?}");
	assert!(
		list_body["data"].as_array().is_some_and(|items| items
			.iter()
			.any(|item| item["id"].as_str() == Some(case_id.as_str()))),
		"{list_body:?}"
	);

	let (lifecycle_status, lifecycle_body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/lifecycle")).await?;
	assert_eq!(lifecycle_status, StatusCode::OK, "{lifecycle_body:?}");
	assert!(
		lifecycle_body["data"]["items"]
			.as_array()
			.is_some_and(|items| items
				.iter()
				.any(|item| item["status"].as_str() == Some("deleted"))),
		"{lifecycle_body:?}"
	);

	let mut tx = mm.dbx().db().begin().await?;
	sqlx::query("SET ROLE e2br3_auditor_role")
		.execute(&mut *tx)
		.await?;
	let audit = sqlx::query_as::<_, (Option<String>, Option<Uuid>)>(
		r#"
				SELECT reason_for_change, e_signature_id
				FROM audit_logs
				WHERE table_name = 'cases'
				  AND record_id = $1
				  AND action = 'UPDATE'
				  AND changed_fields ? 'status'
				  AND changed_fields->'status'->>'new' = 'deleted'
				ORDER BY id DESC
				LIMIT 1
				"#,
	)
	.bind(Uuid::parse_str(&case_id)?)
	.fetch_optional(&mut *tx)
	.await?;
	tx.rollback().await?;
	assert_eq!(
		audit.as_ref().and_then(|(reason, _)| reason.as_deref()),
		Some("client requested soft delete")
	);
	assert!(audit.and_then(|(_, signature_id)| signature_id).is_some());

	let (missing_reason_status, missing_reason_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/restore"),
		json!({}),
	)
	.await?;
	assert_eq!(
		missing_reason_status,
		StatusCode::BAD_REQUEST,
		"{missing_reason_body:?}"
	);
	assert!(
		missing_reason_body
			.to_string()
			.contains("reason_for_change is required for restore"),
		"{missing_reason_body:?}"
	);

	let (restore_status, restore_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/restore"),
		json!({ "reason_for_change": "deletion was entered in error" }),
	)
	.await?;
	assert_eq!(restore_status, StatusCode::OK, "{restore_body:?}");
	assert_eq!(restore_body["data"]["status"].as_str(), Some("draft"));

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	dbx.execute(sqlx::query("SET ROLE e2br3_auditor_role"))
		.await?;
	let restore_reason = dbx
		.fetch_optional(
			sqlx::query_as::<_, (Option<String>,)>(
				r#"
				SELECT reason_for_change
				FROM audit_logs
				WHERE table_name = 'cases'
				  AND record_id = $1
				  AND action = 'UPDATE'
				  AND changed_fields->'status'->>'old' = 'deleted'
				  AND changed_fields->'status'->>'new' = 'draft'
				ORDER BY id DESC
				LIMIT 1
				"#,
			)
			.bind(Uuid::parse_str(&case_id)?),
		)
		.await?;
	dbx.rollback_txn().await?;
	assert_eq!(
		restore_reason.and_then(|(v,)| v).as_deref(),
		Some("deletion was entered in error")
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_deleted_case_rejects_content_updates() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (create_status, create_body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safetyReportIdentification": {"safetyReportId": format!("SR-{}", Uuid::new_v4())},
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(create_status, StatusCode::CREATED, "{create_body:?}");
	let case_id = create_body["data"]["id"]
		.as_str()
		.ok_or("missing created case id")?
		.to_string();

	let (delete_status, delete_body) = delete_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"reason_for_change": "client requested soft delete",
			"e_signature": {
				"meaning": "delete case",
				"password": "adminpwd"
			}
		}),
	)
	.await?;
	assert_eq!(delete_status, StatusCode::OK, "{delete_body:?}");

	let (update_status, update_body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
				"report_year": "2027"
			}
		}),
	)
	.await?;
	assert_eq!(update_status, StatusCode::FORBIDDEN, "{update_body:?}");
	assert!(
		update_body.to_string().contains("PERMISSION_DENIED"),
		"{update_body:?}"
	);

	Ok(())
}
