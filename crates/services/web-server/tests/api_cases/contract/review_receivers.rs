use super::super::case_support::toggle_case_action;
use super::support::{delete_json, get_json, post_json, put_json};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::{Ctx, ROLE_SPONSOR_ADMIN_CRO};
use lib_core::model::audit::AuditLogBmc;
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::ModelManager;
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
struct ReviewReceiverAudit {
	id: i64,
	table_name: String,
	organization_id: Uuid,
	user_id: Uuid,
	action: String,
	reason: Option<String>,
	category: Option<String>,
	changed_fields: Option<Value>,
	old_values: Option<Value>,
	new_values: Option<Value>,
}

async fn case_audit_rows(
	mm: &ModelManager,
	ctx: &Ctx,
	case_id: Uuid,
) -> Result<Vec<ReviewReceiverAudit>> {
	let mut rows = AuditLogBmc::list_by_record(ctx, mm, "cases", case_id).await?;
	rows.sort_by_key(|row| row.id);
	Ok(rows
		.into_iter()
		.map(|row| ReviewReceiverAudit {
			id: row.id,
			table_name: row.table_name,
			organization_id: row.organization_id,
			user_id: row.user_id,
			action: row.action,
			reason: row.reason_for_change,
			category: row.change_category,
			changed_fields: row.changed_fields,
			old_values: row.old_values,
			new_values: row.new_values,
		})
		.collect())
}

async fn review_receiver_state(
	app: &axum::Router,
	admin_cookie: &str,
	mm: &ModelManager,
	ctx: &Ctx,
	case_id: Uuid,
) -> Result<(Value, Value, Vec<ReviewReceiverAudit>)> {
	let (status, case) =
		get_json(app, admin_cookie, &format!("/api/cases/{case_id}")).await?;
	assert_eq!(status, StatusCode::OK, "{case}");
	let (status, shell) = get_json(
		app,
		admin_cookie,
		&format!("/api/cases/{case_id}/editor/shell"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{shell}");
	Ok((case, shell, case_audit_rows(mm, ctx, case_id).await?))
}

#[allow(clippy::too_many_arguments)]
async fn assert_rejected_review_receiver_update(
	app: &axum::Router,
	admin_cookie: &str,
	actor_cookie: &str,
	mm: &ModelManager,
	ctx: &Ctx,
	case_id: Uuid,
	expected: StatusCode,
	label: &str,
) -> Result<()> {
	let before = review_receiver_state(app, admin_cookie, mm, ctx, case_id).await?;
	let (status, body) = put_json(
		app,
		actor_cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {"review_receivers_json": serde_json::to_string(&json!([{
				"receiver": format!("Rejected {label}"),
				"reportDueDate": "2026-10-01"
			}]))?},
			"reason_for_change": format!("reject {label} review receiver update")
		}),
	)
	.await?;
	assert_eq!(status, expected, "{label}: {body}");
	assert_eq!(
		review_receiver_state(app, admin_cookie, mm, ctx, case_id).await?,
		before,
		"{label}: rejected update changed Case GET, editor shell, or audit"
	);
	Ok(())
}

fn assert_review_receiver_audit(
	before: &[ReviewReceiverAudit],
	after: &[ReviewReceiverAudit],
	ctx: &Ctx,
	reason: &str,
	old: Value,
	new: Value,
) {
	let appended = after
		.iter()
		.filter(|row| !before.iter().any(|old_row| old_row.id == row.id))
		.collect::<Vec<_>>();
	assert_eq!(
		appended.len(),
		1,
		"unexpected Case audit rows: {appended:?}"
	);
	let row = appended[0];
	assert_eq!(row.action, "UPDATE");
	assert_eq!(row.user_id, ctx.user_id());
	assert_eq!(row.organization_id, ctx.organization_id());
	assert_eq!(row.reason.as_deref(), Some(reason));
	assert_eq!(row.category, None);
	assert_eq!(
		row.old_values
			.as_ref()
			.and_then(|value| value.get("review_receivers_json")),
		Some(&old)
	);
	assert_eq!(
		row.new_values
			.as_ref()
			.and_then(|value| value.get("review_receivers_json")),
		Some(&new)
	);
	assert_eq!(
		row.changed_fields
			.as_ref()
			.and_then(|value| value.get("review_receivers_json"))
			.and_then(|value| value.get("old")),
		Some(&old)
	);
	assert_eq!(
		row.changed_fields
			.as_ref()
			.and_then(|value| value.get("review_receivers_json"))
			.and_then(|value| value.get("new")),
		Some(&new)
	);
}

#[serial]
#[tokio::test]
async fn update_case_rejects_review_receiver_without_receiver_name() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safetyReportIdentification": {
					"safetyReportId": format!("SR-RE-MISSING-{}", Uuid::new_v4())
				},
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let case_id = body["data"]["id"].as_str().ok_or("missing case id")?;

	let (status, body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
				"review_receivers_json": serde_json::to_string(&json!([
					{
						"reportDue": 7,
						"reportDueDate": "2026-07-08"
					}
				]))?
			},
			"reason_for_change": "update review receiver"
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body["error"]["data"]["detail"]
			.as_str()
			.unwrap_or_default()
			.contains("receiver"),
		"{body:?}"
	);

	Ok(())
}

async fn create_re_validation_case(
	app: &axum::Router,
	cookie: &str,
	prefix: &str,
) -> Result<Uuid> {
	let (status, body) = post_json(
		app,
		cookie,
		"/api/cases",
		json!({
			"data": {
				"safetyReportIdentification": {
					"safetyReportId": format!("{prefix}-{}", Uuid::new_v4())
				},
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let case_id =
		Uuid::parse_str(body["data"]["id"].as_str().ok_or("missing case id")?)?;
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/message-header"),
		json!({
			"data": {
				"case_id": case_id,
				"message_number": format!("MSG-{case_id}"),
				"message_sender_identifier": "SENDER01",
				"message_receiver_identifier": "MFDS",
				"message_date": "20260917000000"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	Ok(case_id)
}

#[serial]
#[tokio::test]
async fn editor_shell_reads_saved_review_receivers_as_an_array() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let other = seed_org_with_users(&mm, "otheradminpwd", "otherviewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let viewer_cookie = cookie_header(
		&generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?.to_string(),
	);
	let other_cookie = cookie_header(
		&generate_web_token(&other.admin.email, other.admin.token_salt)?.to_string(),
	);
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let app = web_server::app(mm.clone());
	let case_id = create_re_validation_case(&app, &cookie, "SR-RE-SHELL").await?;
	let receivers = json!({
		"reviewReceivers": [{
			"receiver": "MFDS",
			"reportDueDate": "2026-09-10",
			"reportedDate": "2026-09-11"
		}]
	});

	let initial_audit = case_audit_rows(&mm, &ctx, case_id).await?;
	let initial_raw = serde_json::to_string(&receivers)?;
	let initial_reason = "save review receiver";
	let (status, body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": { "review_receivers_json": initial_raw.clone() },
			"reason_for_change": initial_reason
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let mut audit = case_audit_rows(&mm, &ctx, case_id).await?;
	assert_review_receiver_audit(
		&initial_audit,
		&audit,
		&ctx,
		initial_reason,
		Value::Null,
		Value::String(initial_raw.clone()),
	);

	let (status, shell) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/editor/shell"))
			.await?;
	assert_eq!(status, StatusCode::OK, "{shell:?}");
	assert_eq!(shell["reviewReceivers"], receivers["reviewReceivers"]);
	assert_eq!(
		shell["messageHeader"]["messageReceiverIdentifier"], "MFDS",
		"{shell:?}"
	);
	let before_omission =
		review_receiver_state(&app, &cookie, &mm, &ctx, case_id).await?;
	let (status, body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({"data":{},"reason_for_change":"leave review receivers omitted"}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let after_omission =
		review_receiver_state(&app, &cookie, &mm, &ctx, case_id).await?;
	assert_eq!(
		after_omission.0["data"]["review_receivers_json"],
		before_omission.0["data"]["review_receivers_json"]
	);
	assert_eq!(
		after_omission.1["reviewReceivers"],
		receivers["reviewReceivers"]
	);
	assert_eq!(
		after_omission.1["messageHeader"]["messageReceiverIdentifier"],
		"MFDS"
	);
	assert_eq!(
		after_omission.2, before_omission.2,
		"omission wrote an audit"
	);
	assert_rejected_review_receiver_update(
		&app,
		&cookie,
		&viewer_cookie,
		&mm,
		&ctx,
		case_id,
		StatusCode::FORBIDDEN,
		"viewer",
	)
	.await?;
	assert_rejected_review_receiver_update(
		&app,
		&cookie,
		&other_cookie,
		&mm,
		&ctx,
		case_id,
		StatusCode::NOT_FOUND,
		"other organization",
	)
	.await?;
	let (status, body) = toggle_case_action(&app, &cookie, case_id, "lock").await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_rejected_review_receiver_update(
		&app,
		&cookie,
		&cookie,
		&mm,
		&ctx,
		case_id,
		StatusCode::FORBIDDEN,
		"locked case",
	)
	.await?;
	let (status, body) = toggle_case_action(&app, &cookie, case_id, "lock").await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let before_archive =
		review_receiver_state(&app, &cookie, &mm, &ctx, case_id).await?;
	let (status, body) = delete_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({"reason_for_change":"preserve review receivers across archive",
			"e_signature":{"meaning":"delete case","password":"adminpwd"}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let archived = review_receiver_state(&app, &cookie, &mm, &ctx, case_id).await?;
	assert_eq!(archived.0["data"]["status"], "deleted");
	assert_eq!(
		archived.0["data"]["review_receivers_json"],
		before_archive.0["data"]["review_receivers_json"]
	);
	assert_eq!(
		archived.1["reviewReceivers"],
		before_archive.1["reviewReceivers"]
	);
	assert_rejected_review_receiver_update(
		&app,
		&cookie,
		&cookie,
		&mm,
		&ctx,
		case_id,
		StatusCode::FORBIDDEN,
		"archived case",
	)
	.await?;
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/restore"),
		json!({"reason_for_change":"restore Case with review receivers"}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let restored = review_receiver_state(&app, &cookie, &mm, &ctx, case_id).await?;
	assert_eq!(restored.0["data"]["status"], "draft");
	assert_eq!(
		restored.0["data"]["review_receivers_json"],
		before_archive.0["data"]["review_receivers_json"]
	);
	assert_eq!(
		restored.1["reviewReceivers"],
		before_archive.1["reviewReceivers"]
	);
	for (before, after, old, new, reason, expected_tables) in [
		(
			&before_archive.2,
			&archived.2,
			"draft",
			"deleted",
			"preserve review receivers across archive",
			vec!["cases", "e_signatures"],
		),
		(
			&archived.2,
			&restored.2,
			"deleted",
			"draft",
			"restore Case with review receivers",
			vec!["cases"],
		),
	] {
		assert!(before.iter().all(|row| after.contains(row)));
		let appended = after
			.iter()
			.filter(|row| !before.iter().any(|old| old.id == row.id))
			.collect::<Vec<_>>();
		let mut tables = appended
			.iter()
			.map(|row| row.table_name.as_str())
			.collect::<Vec<_>>();
		tables.sort_unstable();
		assert_eq!(tables, expected_tables, "unexpected child audit");
		let status_audit = appended
			.iter()
			.find(|row| row.table_name == "cases")
			.ok_or("missing Case status audit")?;
		assert_eq!(status_audit.action, "UPDATE");
		assert_eq!(status_audit.user_id, ctx.user_id());
		assert_eq!(status_audit.organization_id, ctx.organization_id());
		assert_eq!(status_audit.reason.as_deref(), Some(reason));
		assert_eq!(
			status_audit
				.old_values
				.as_ref()
				.ok_or("missing old Case status")?["status"],
			old
		);
		assert_eq!(
			status_audit
				.new_values
				.as_ref()
				.ok_or("missing new Case status")?["status"],
			new
		);
	}
	audit = case_audit_rows(&mm, &ctx, case_id).await?;

	let retained = receivers["reviewReceivers"].clone();
	let expanded = json!([
		retained[0],
		{"receiver":"FDA","reportDueDate":"2026-09-12","reportedDate":"2026-09-13"}
	]);
	// RE is a replacement JSON array, with no row DELETE/restore endpoint.
	let mut stored = Value::String(initial_raw);
	for (index, rows) in [expanded, retained.clone(), retained, json!([]), json!([])]
		.into_iter()
		.enumerate()
	{
		let reason = format!("replace review receivers step {index}");
		let raw = serde_json::to_string(&rows)?;
		let before = audit.clone();
		let (status, body) = put_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}"),
			json!({
				"data":{"review_receivers_json":raw.clone()},
				"reason_for_change":reason.clone()
			}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		let (status, shell) =
			get_json(&app, &cookie, &format!("/api/cases/{case_id}/editor/shell"))
				.await?;
		assert_eq!(status, StatusCode::OK, "{shell}");
		assert_eq!(
			shell["reviewReceivers"], rows,
			"removed receiver reappeared on resave"
		);
		assert_eq!(shell["messageHeader"]["messageReceiverIdentifier"], "MFDS");
		audit = case_audit_rows(&mm, &ctx, case_id).await?;
		let next = Value::String(raw);
		if next == stored {
			assert_eq!(audit, before, "identical replacement wrote an audit row");
		} else {
			assert_review_receiver_audit(
				&before,
				&audit,
				&ctx,
				&reason,
				stored.clone(),
				next.clone(),
			);
		}
		stored = next;
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn review_receiver_json_can_be_cleared_back_to_sql_null() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_re_validation_case(&app, &cookie, "SR-RE-CLEAR").await?;

	let (status, body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
				"review_receivers_json": serde_json::to_string(&json!([{
					"receiver": "E2BR3 review receiver",
					"reportDueDate": "2026-09-10"
				}]))?
			},
			"reason_for_change": "seed review receiver"
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": { "review_receivers_json": null },
			"reason_for_change": "restore review receiver baseline"
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert!(body["data"]["review_receivers_json"].is_null(), "{body:?}");

	let (status, body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}")).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert!(body["data"]["review_receivers_json"].is_null(), "{body:?}");

	let (status, shell) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/editor/shell"))
			.await?;
	assert_eq!(status, StatusCode::OK, "{shell:?}");
	assert_eq!(shell["reviewReceivers"], json!([]));

	Ok(())
}

#[serial]
#[tokio::test]
async fn explicit_review_receiver_date_does_not_require_a_presave_in_mixed_rows(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id =
		create_re_validation_case(&app, &cookie, "SR-RE-EXPLICIT-DATE").await?;

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
				"UPDATE safety_report_identification
				 SET date_of_most_recent_information = DATE '2026-09-11'
				 WHERE case_id = $1",
			)
			.bind(case_id),
		)
		.await?;
	mm.dbx().commit_txn().await?;

	let receivers = json!({"reviewReceivers": [
		{
			"receiver": "UI regression receiver",
			"reportDueDate": "2026-09-12",
			"reportedDate": "2026-09-11"
		},
		{
			"receiver": "Calculated date receiver",
			"reportDue": 7
		}
	]});
	let (status, body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": { "review_receivers_json": serde_json::to_string(&receivers)? },
			"reason_for_change": "save mixed review receiver dates"
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let saved: Value = serde_json::from_str(
		body["data"]["review_receivers_json"]
			.as_str()
			.ok_or("missing review_receivers_json")?,
	)?;
	assert_eq!(saved["reviewReceivers"][0], receivers["reviewReceivers"][0]);
	assert_eq!(
		saved["reviewReceivers"][1]["reportDueDate"],
		json!("2026-09-18")
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn update_case_rejects_malformed_review_receiver_json() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_re_validation_case(&app, &cookie, "SR-RE-BAD-JSON").await?;
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let (status, body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data":{"review_receivers_json":serde_json::to_string(&json!([
				{"receiver":"MFDS","reportDueDate":"2026-09-10"}
			]))?},
			"reason_for_change":"prepare preserved review receiver"
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let before = review_receiver_state(&app, &cookie, &mm, &ctx, case_id).await?;
	assert_ne!(before.0["data"]["report_year"], "2025");

	let (status, body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
				"report_year": "2025",
				"review_receivers_json": "{\"reviewReceivers\":["
			},
			"reason_for_change": "reject malformed review receiver json"
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body["error"]["data"]["detail"]
			.as_str()
			.ok_or("missing malformed JSON error detail")?
			.contains("valid JSON"),
		"{body:?}"
	);
	assert_eq!(
		review_receiver_state(&app, &cookie, &mm, &ctx, case_id).await?,
		before,
		"malformed receiver JSON persisted the valid companion change or audit"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn update_case_rejects_review_receiver_name_over_128_chars() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_re_validation_case(&app, &cookie, "SR-RE-LONG").await?;

	let (status, body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
				"review_receivers_json": serde_json::to_string(&json!({
					"reviewReceivers": [
						{
							"receiver": "R".repeat(129),
							"reportDue": 7,
							"reportDueDate": "2026-07-08"
						}
					]
				}))?
			},
			"reason_for_change": "reject long review receiver"
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	let detail = body["error"]["data"]["detail"].as_str().unwrap_or_default();
	assert!(
		detail.contains("receiver") && detail.contains("128"),
		"{body:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn update_case_rejects_invalid_review_receiver_dates() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let report_due_case_id =
		create_re_validation_case(&app, &cookie, "SR-RE-BAD-DUE-DATE").await?;
	let reported_case_id =
		create_re_validation_case(&app, &cookie, "SR-RE-BAD-REPORTED-DATE").await?;

	let (status, body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{report_due_case_id}"),
		json!({
			"data": {
				"review_receivers_json": serde_json::to_string(&json!({
					"reviewReceivers": [
						{
							"receiver": "MFDS(KR)",
							"reportDue": 7,
							"reportDueDate": "not-a-date"
						}
					]
				}))?
			},
			"reason_for_change": "reject invalid review receiver due date"
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body["error"]["data"]["detail"]
			.as_str()
			.unwrap_or_default()
			.contains("reportDueDate"),
		"{body:?}"
	);

	let (status, body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{reported_case_id}"),
		json!({
			"data": {
				"review_receivers_json": serde_json::to_string(&json!({
					"reviewReceivers": [
						{
							"receiver": "MFDS(KR)",
							"reportDue": 7,
							"reportDueDate": "2026-07-08",
							"reportedDate": "2026-99-99"
						}
					]
				}))?
			},
			"reason_for_change": "reject invalid review receiver reported date"
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body["error"]["data"]["detail"]
			.as_str()
			.unwrap_or_default()
			.contains("reportedDate"),
		"{body:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn update_case_drops_blank_review_receiver_rows() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_re_validation_case(&app, &cookie, "SR-RE-BLANK").await?;

	let (status, body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
				"review_receivers_json": serde_json::to_string(&json!({
					"reviewReceivers": [
						{
							"receiver": "",
							"reportDueDate": "",
							"reportedDate": null
						}
					]
				}))?
			},
			"reason_for_change": "drop blank review receiver row"
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let raw = body["data"]["review_receivers_json"]
		.as_str()
		.ok_or("missing review_receivers_json")?;
	let saved: Value = serde_json::from_str(raw)?;
	assert_eq!(
		saved["reviewReceivers"].as_array().map(Vec::len),
		Some(0),
		"{saved:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn update_case_review_receiver_defaults_report_due_date_from_c1_5(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safetyReportIdentification": {
					"safetyReportId": format!("SR-RE-DUE-{}", Uuid::new_v4())
				},
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let case_id =
		Uuid::parse_str(body["data"]["id"].as_str().ok_or("missing case id")?)?;

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
				"UPDATE safety_report_identification
				 SET date_of_most_recent_information = DATE '2026-07-01'
				 WHERE case_id = $1",
			)
			.bind(case_id),
		)
		.await?;
	mm.dbx().commit_txn().await?;

	let (status, body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
				"review_receivers_json": serde_json::to_string(&json!({
					"reviewReceivers": [
						{
							"receiver": "MFDS(KR)",
							"reportDue": 7
						}
					]
				}))?
			},
			"reason_for_change": "update review receiver"
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let raw = body["data"]["review_receivers_json"]
		.as_str()
		.ok_or("missing review_receivers_json")?;
	let saved: Value = serde_json::from_str(raw)?;
	assert_eq!(
		saved["reviewReceivers"][0]["reportDueDate"].as_str(),
		Some("2026-07-08"),
		"{saved:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn update_case_review_receiver_defaults_report_due_from_receiver_timeline(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let receiver_name = format!("RE Timeline Receiver {}", Uuid::new_v4());

	let (status, receiver_body) = post_json(
		&app,
		&cookie,
		"/api/presaves/receivers",
		json!({
			"data": { "rows": {
				"receiver": {
					"receiverType": "Regulatory Authority",
					"organizationName": receiver_name,
					"dayCountRule": "calendar",
					"nsaeNonSolicitedDayCount": 30,
					"saeNonSolicitedDayCount": 3,
					"nsaeSolicitedDayCount": 10,
					"saeSolicitedDayCount": 5
				},
				"consignees": [], "routes": []
			} }
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{receiver_body:?}");

	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safetyReportIdentification": {
					"safetyReportId": format!("SR-RE-TIMELINE-{}", Uuid::new_v4())
				},
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let case_id =
		Uuid::parse_str(body["data"]["id"].as_str().ok_or("missing case id")?)?;

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
				"UPDATE safety_report_identification
				 SET report_type = '1',
				     date_of_most_recent_information = DATE '2026-07-01'
				 WHERE case_id = $1",
			)
			.bind(case_id),
		)
		.await?;
	mm.dbx().commit_txn().await?;

	let (status, reaction_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/reactions"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"primary_source_reaction": "Headache",
				"reaction_meddra_code": "10019211",
				"reaction_meddra_version": "27.1",
				"serious": true
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{reaction_body:?}");

	let (status, body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
				"review_receivers_json": serde_json::to_string(&json!({
					"reviewReceivers": [
						{
							"receiver": receiver_name
						}
					]
				}))?
			},
			"reason_for_change": "default review receiver due from timeline"
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let raw = body["data"]["review_receivers_json"]
		.as_str()
		.ok_or("missing review_receivers_json")?;
	let saved: Value = serde_json::from_str(raw)?;
	assert_eq!(saved["reviewReceivers"][0]["reportDue"].as_i64(), Some(3));
	assert_eq!(
		saved["reviewReceivers"][0]["reportDueDate"].as_str(),
		Some("2026-07-04"),
		"{saved:?}"
	);

	Ok(())
}
