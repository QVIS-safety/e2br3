use super::support::{post_json, post_raw, put_json, put_raw};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::store::{
	set_full_context_dbx, set_org_context, set_user_context,
};
use serde_json::{json, Value};
use serial_test::serial;
use sqlx::types::time::OffsetDateTime;
use time::Duration;
use uuid::Uuid;

fn format_e2b_creation_timestamp(value: OffsetDateTime) -> String {
	format!(
		"{:04}{:02}{:02}{:02}{:02}{:02}",
		value.year(),
		value.month() as u8,
		value.day(),
		value.hour(),
		value.minute(),
		value.second()
	)
}

#[serial]
#[tokio::test]
async fn create_case_defaults_c1_2_to_backend_creation_timestamp() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let lower_bound = format_e2b_creation_timestamp(
		OffsetDateTime::now_utc() - Duration::seconds(2),
	);

	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safetyReportIdentification": {
					"safetyReportId": format!("SR-C1-DEFAULT-{}", Uuid::new_v4())
				},
				"status": "draft"
			}
		}),
	)
	.await?;
	let upper_bound = format_e2b_creation_timestamp(
		OffsetDateTime::now_utc() + Duration::seconds(2),
	);
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
	let (transmission_date,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (String,)>(
				"SELECT transmission_date
				   FROM safety_report_identification
				  WHERE case_id = $1",
			)
			.bind(case_id),
		)
		.await?;
	mm.dbx().commit_txn().await?;

	assert_eq!(transmission_date.len(), 14);
	assert!(
		transmission_date >= lower_bound && transmission_date <= upper_bound,
		"transmission_date {transmission_date} not within {lower_bound}..={upper_bound}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_public_case_create_uses_nested_safety_report_identification(
) -> Result<()> {
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
				"safetyReportIdentification": {"safetyReportId": format!("SR-{}", Uuid::new_v4())},
				"status": "draft"
			}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let expected_org_id = seed.org_id.to_string();
	assert_eq!(
		body["data"]["organization_id"].as_str(),
		Some(expected_org_id.as_str()),
		"{body:?}"
	);
	assert!(body["data"].get("version").is_none(), "{body:?}");
	assert!(body["data"].get("safety_report_id").is_none(), "{body:?}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_manual_case_save_updates_public_fields_without_import_noise(
) -> Result<()> {
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

	let (update_status, update_body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
				"report_year": "2026"
			}
		}),
	)
	.await?;

	assert_eq!(update_status, StatusCode::OK, "{update_body:?}");
	assert_eq!(
		update_body["data"]["report_year"].as_str(),
		Some("2026"),
		"{update_body:?}"
	);
	let response_text = update_body.to_string().to_ascii_lowercase();
	assert!(!response_text.contains("batch"), "{response_text}");
	assert!(!response_text.contains("header"), "{response_text}");
	assert!(!response_text.contains("import"), "{response_text}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_save_updates_fda_report_type_public_field() -> Result<()> {
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
				"safetyReportIdentification": {"safetyReportId": format!("SR-FDA-{}", Uuid::new_v4())},
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

	let (update_status, update_body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
				"report_year": "2026",
				"fda_report_type": "4"
			}
		}),
	)
	.await?;

	assert_eq!(update_status, StatusCode::OK, "{update_body:?}");
	assert_eq!(
		update_body["data"]["report_year"].as_str(),
		Some("2026"),
		"{update_body:?}"
	);
	assert_eq!(
		update_body["data"]["fda_report_type"].as_str(),
		Some("4"),
		"{update_body:?}"
	);

	let (invalid_update_status, invalid_update_body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
				"fda_report_type": "9"
			}
		}),
	)
	.await?;
	assert_eq!(
		invalid_update_status,
		StatusCode::BAD_REQUEST,
		"{invalid_update_body:?}"
	);

	let (invalid_create_status, invalid_create_body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safetyReportIdentification": {"safetyReportId": format!("SR-FDA-BAD-{}", Uuid::new_v4())},
				"fdaReportType": "9",
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(
		invalid_create_status,
		StatusCode::BAD_REQUEST,
		"{invalid_create_body:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn update_case_optional_metadata_clears_blanks_and_null_but_preserves_omitted(
) -> Result<()> {
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
					"safetyReportId": format!("SR-METADATA-{}", Uuid::new_v4())
				},
				"dgPrdKey": "",
				"mfdsReportType": "   ",
				"fdaReportType": null,
				"reportYear": "\t",
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	for field in [
		"dg_prd_key",
		"mfds_report_type",
		"fda_report_type",
		"report_year",
	] {
		assert!(body["data"][field].is_null(), "{field}: {body:?}");
	}
	let case_id = body["data"]["id"].as_str().ok_or("missing case id")?;
	let uri = format!("/api/cases/{case_id}");

	let populated = json!({
		"dg_prd_key": "PRODUCT-1",
		"mfds_report_type": "1",
		"fda_report_type": "4",
		"report_year": "2026"
	});
	let (status, body) = put_json(
		&app,
		&cookie,
		&uri,
		json!({"data": populated.clone(), "reason_for_change": "set case metadata"}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let (status, body) = put_json(&app, &cookie, &uri, json!({"data": {}})).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	for (field, expected) in [
		("dg_prd_key", "PRODUCT-1"),
		("mfds_report_type", "1"),
		("fda_report_type", "4"),
		("report_year", "2026"),
	] {
		assert_eq!(body["data"][field], expected, "{field}: {body:?}");
	}
	let (status, body) =
		put_json(&app, &cookie, &uri, json!({"data": {"dg_prd_key": null}})).await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body["error"]["data"]["detail"]
			.as_str()
			.ok_or("missing error detail")?
			.contains("reason_for_change"),
		"{body:?}"
	);

	let (status, body) = put_json(
		&app,
		&cookie,
		&uri,
		json!({
			"data": {
				"dg_prd_key": "",
				"mfds_report_type": "   ",
				"fda_report_type": null,
				"report_year": "\t"
			},
			"reason_for_change": "clear case metadata"
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	for field in [
		"dg_prd_key",
		"mfds_report_type",
		"fda_report_type",
		"report_year",
	] {
		assert!(body["data"][field].is_null(), "{field}: {body:?}");
	}

	let (status, _) = put_json(
		&app,
		&cookie,
		&uri,
		json!({"data": populated, "reason_for_change": "restore case metadata"}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	let (status, body) =
		put_json(&app, &cookie, &uri, json!({"data": {"report_year": null}}))
			.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["dg_prd_key"], "PRODUCT-1");
	assert_eq!(body["data"]["mfds_report_type"], "1");
	assert_eq!(body["data"]["fda_report_type"], "4");
	assert!(body["data"]["report_year"].is_null(), "{body:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_imported_case_save_updates_public_fields_without_import_noise(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = Uuid::new_v4();

	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, seed.admin.id).await?;
	set_org_context(&mut tx, seed.org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	sqlx::query(
		"INSERT INTO cases (
			id,
			organization_id,
			status,
			raw_xml,
			dirty_c,
			dirty_d,
			dirty_e,
			dirty_f,
			dirty_g,
			dirty_h,
			created_by,
			updated_by
		) VALUES ($1, $2, $3, $4, false, false, false, false, false, false, $5, $5)",
	)
	.bind(case_id)
	.bind(seed.org_id)
	.bind("draft")
	.bind(b"<ichicsr/>".to_vec())
	.bind(seed.admin.id)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
		"INSERT INTO safety_report_identification (case_id, safety_report_id, version, created_by, updated_by)
		 VALUES ($1, $2, 1, $3, $3)",
	)
	.bind(case_id)
	.bind(format!("SR-SHAPED-SAVE-{case_id}"))
	.bind(seed.admin.id)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;

	let (update_status, update_body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
				"report_year": "2026"
			}
		}),
	)
	.await?;

	assert_eq!(update_status, StatusCode::OK, "{update_body:?}");
	assert_eq!(
		update_body["data"]["report_year"].as_str(),
		Some("2026"),
		"{update_body:?}"
	);
	let rendered = update_body.to_string().to_ascii_lowercase();
	assert!(!rendered.contains("batch"), "{update_body:?}");
	assert!(!rendered.contains("header"), "{update_body:?}");
	assert!(!rendered.contains("import"), "{update_body:?}");
	assert!(!rendered.contains("raw_xml"), "{update_body:?}");
	assert!(!rendered.contains("rawxml"), "{update_body:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_public_case_update_rejects_legacy_safety_report_id() -> Result<()> {
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

	let (update_status, update_body) = put_raw(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
				"safety_report_id": format!("SR-RENAMED-{}", Uuid::new_v4())
			}
		}),
	)
	.await?;
	let update_body = String::from_utf8_lossy(&update_body);
	assert_eq!(
		update_status,
		StatusCode::UNPROCESSABLE_ENTITY,
		"{update_body}"
	);
	assert!(
		update_body.contains("unknown field `safety_report_id`"),
		"{update_body}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_public_case_update_rejects_legacy_safety_report_id_with_reason(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

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
	let case_id = Uuid::parse_str(
		create_body["data"]["id"]
			.as_str()
			.ok_or("missing created case id")?,
	)?;
	let (update_status, update_body) = put_raw(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
				"safety_report_id": format!("SR-RENAMED-{}", Uuid::new_v4())
			},
			"reason_for_change": "correct case identifier after source reconciliation"
		}),
	)
	.await?;
	let update_body = String::from_utf8_lossy(&update_body);
	assert_eq!(
		update_status,
		StatusCode::UNPROCESSABLE_ENTITY,
		"{update_body}"
	);
	assert!(
		update_body.contains("unknown field `safety_report_id`"),
		"{update_body}"
	);

	Ok(())
}
