use super::support::{get_json, post_json, put_json};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::store::set_full_context_dbx;
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;

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
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_re_validation_case(&app, &cookie, "SR-RE-SHELL").await?;
	let receivers = json!({
		"reviewReceivers": [{
			"receiver": "MFDS",
			"reportDueDate": "2026-09-10",
			"reportedDate": "2026-09-11"
		}]
	});

	let (status, body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": { "review_receivers_json": serde_json::to_string(&receivers)? },
			"reason_for_change": "save review receiver"
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, shell) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/editor/shell"))
			.await?;
	assert_eq!(status, StatusCode::OK, "{shell:?}");
	assert_eq!(shell["reviewReceivers"], receivers["reviewReceivers"]);
	assert_eq!(
		shell["messageHeader"]["messageReceiverIdentifier"], "MFDS",
		"{shell:?}"
	);

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
	let app = web_server::app(mm);
	let case_id = create_re_validation_case(&app, &cookie, "SR-RE-BAD-JSON").await?;

	let (status, body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
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
			.unwrap_or_default()
			.contains("valid JSON"),
		"{body:?}"
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
