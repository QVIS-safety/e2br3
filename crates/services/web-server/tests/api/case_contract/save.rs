use super::support::{get_json, post_json, post_raw, put_json, put_raw};
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
async fn test_kr_device_characteristics_round_trip_via_api() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let suffix = Uuid::new_v4().simple().to_string();
	let case_no = format!("KR-DVC-{suffix}");

	let (status, raw_body) = post_raw(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safetyReportIdentification": {"safetyReportId": case_no},
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&raw_body)
	);
	let body: Value = serde_json::from_slice(&raw_body)?;
	let case_id = body["data"]["id"]
		.as_str()
		.ok_or("missing created case id")?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"drug_characterization": "1",
				"medicinal_product": "KR Device Product"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let drug_id = body["data"]["id"].as_str().ok_or("missing drug id")?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs/{drug_id}/device-characteristics"),
		json!({
			"data": {
				"drug_id": drug_id,
				"sequence_number": 1,
				"code": "KR_DVC_PROBC",
				"value_code": "PROB-1"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs/{drug_id}/device-characteristics"),
		json!({
			"data": {
				"drug_id": drug_id,
				"sequence_number": 2,
				"code": "KR_DVC_MFR",
				"value_value": "KR Maker"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs/{drug_id}/device-characteristics"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let rows = body["data"].as_array().ok_or("missing rows")?;
	assert_eq!(rows.len(), 2, "{rows:?}");
	assert!(
		rows.iter().any(|row| {
			row["code"].as_str() == Some("KR_DVC_PROBC")
				&& row["value_code"].as_str() == Some("PROB-1")
		}),
		"{rows:?}"
	);
	assert!(
		rows.iter().any(|row| {
			row["code"].as_str() == Some("KR_DVC_MFR")
				&& row["value_value"].as_str() == Some("KR Maker")
		}),
		"{rows:?}"
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

#[serial]
#[tokio::test]
async fn safety_report_rejects_c1_date_null_flavor_fields() -> Result<()> {
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
				"safetyReportIdentification": {
					"safetyReportId": format!("SR-C1-NF-{}", Uuid::new_v4())
				},
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
		&format!("/api/cases/{case_id}/safety-report"),
		json!({
			"data": {
				"transmission_date_null_flavor": "NI",
				"date_first_received_from_source_null_flavor": "NI",
				"date_of_most_recent_information_null_flavor": "NI"
			}
		}),
	)
	.await?;

	assert_eq!(
		update_status,
		StatusCode::UNPROCESSABLE_ENTITY,
		"{}",
		String::from_utf8_lossy(&update_body)
	);
	let body_text = String::from_utf8_lossy(&update_body);
	assert!(
		body_text.contains("transmission_date_null_flavor")
			|| body_text.contains("unknown field"),
		"{}",
		body_text
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn long_hangul_narrative_update_preserves_jsonb_and_audit_contracts(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let (semantics_match,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (bool,)>(
				r#"
				SELECT bool_and(actual IS NOT DISTINCT FROM expected)
				FROM (VALUES
					(audit_jsonb_is_distinct(NULL::jsonb, NULL::jsonb), false),
					(audit_jsonb_is_distinct(NULL::jsonb, 'null'::jsonb), true),
					(audit_jsonb_is_distinct('null'::jsonb, 'null'::jsonb), false),
					(audit_jsonb_is_distinct('1'::jsonb, '1.0'::jsonb), false),
					(audit_jsonb_is_distinct('{"a":1,"b":2}'::jsonb, '{"b":2,"a":1}'::jsonb), false),
					(audit_jsonb_is_distinct('[1,2]'::jsonb, '[2,1]'::jsonb), true),
					(audit_jsonb_is_distinct('[1,1]'::jsonb, '[1]'::jsonb), true),
					(audit_jsonb_is_distinct('{}'::jsonb -> 'missing', 'null'::jsonb), true),
					(audit_jsonb_is_distinct('{"a":{"b":"가"}}'::jsonb, '{"a":{"b":"나"}}'::jsonb), true),
					(audit_jsonb_is_distinct('{}'::jsonb, '[]'::jsonb), true),
					(pg_catalog.jsonb_hash_extended('{}'::jsonb, 0) =
					 pg_catalog.jsonb_hash_extended('[]'::jsonb, 0), true)
				) AS checks(actual, expected)
				"#,
			),
		)
		.await?;
	assert!(semantics_match, "guard changed native JSONB semantics");

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
