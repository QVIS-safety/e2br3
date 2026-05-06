use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::store::{set_org_context, set_user_context};
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

async fn post_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("POST")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice::<Value>(&body)?))
}

async fn post_raw(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Vec<u8>)> {
	let req = Request::builder()
		.method("POST")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, body.to_vec()))
}

async fn put_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("PUT")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice::<Value>(&body)?))
}

async fn delete_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method(Method::DELETE)
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice::<Value>(&body)?))
}

async fn get_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("GET")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice::<Value>(&body)?))
}

async fn get_raw(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
) -> Result<(StatusCode, Vec<u8>)> {
	let req = Request::builder()
		.method("GET")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, body.to_vec()))
}

#[serial]
#[tokio::test]
async fn test_case_list_view_projects_reference_grid_fields() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let suffix = Uuid::new_v4().simple().to_string();
	let case_no = format!("CASE-LIST-{suffix}");
	let worldwide_unique_no = format!("WWUID-{suffix}");
	let sender = format!("Sender Org {suffix}");
	let receiver = format!("Receiver Org {suffix}");
	let ae_term = format!("Headache {suffix}");
	let study_no = format!("STUDY-{suffix}");
	let manufacturer = format!("Manufacturer {suffix}");

	let (status, raw_body) = post_raw(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safety_report_id": case_no,
				"dg_prd_key": "DG-12345",
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
		.ok_or("missing created case id")?
		.to_string();

	let (status, raw_body) = post_raw(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/safety-report"),
		json!({
			"data": {
				"case_id": case_id,
				"transmission_date": "2026-05-01",
				"report_type": "2",
				"date_first_received_from_source": "2026-04-30",
				"date_of_most_recent_information": "2026-05-01",
				"worldwide_unique_id": worldwide_unique_no
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

	let (status, raw_body) = post_raw(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/safety-report/senders"),
		json!({
			"data": {
				"case_id": case_id,
				"organization_name": sender,
				"sender_type": "2"
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

	let (status, raw_body) = post_raw(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/receiver"),
		json!({
			"data": {
				"case_id": case_id,
				"organization_name": receiver,
				"receiver_type": "2"
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

	let (status, raw_body) = post_raw(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/reactions"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"primary_source_reaction": ae_term,
				"reaction_meddra_code": "10019211",
				"reaction_meddra_version": "27.1",
				"serious": true
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

	let (status, raw_body) = post_raw(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/safety-report/studies"),
		json!({
			"data": {
				"case_id": case_id,
				"study_name": "Pivotal Migraine Study",
				"sponsor_study_number": study_no,
				"study_type_reaction": "1"
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

	let (status, raw_body) = post_raw(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"drug_characterization": "1",
				"medicinal_product": "Example Product",
				"manufacturer_name": manufacturer
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

	let (status, raw_body) = get_raw(&app, &cookie, "/api/cases/list-view").await?;
	assert_eq!(
		status,
		StatusCode::OK,
		"{}",
		String::from_utf8_lossy(&raw_body)
	);
	let body: Value = serde_json::from_slice(&raw_body)?;
	let items = body["data"]["items"]
		.as_array()
		.ok_or("missing list-view items")?;
	let row = items
		.iter()
		.find(|item| item["caseNo"].as_str() == Some(case_no.as_str()))
		.ok_or("missing projected case row")?;

	assert_eq!(
		row["worldwideUniqueNo"].as_str(),
		Some(worldwide_unique_no.as_str()),
		"{row:?}"
	);
	assert_eq!(row["sender"].as_str(), Some(sender.as_str()), "{row:?}");
	assert_eq!(row["aeTerm"].as_str(), Some(ae_term.as_str()), "{row:?}");
	assert_eq!(row["studyNo"].as_str(), Some(study_no.as_str()), "{row:?}");
	assert_eq!(
		row["dateOfCreation"].as_str(),
		Some("2026-05-01"),
		"{row:?}"
	);
	assert_eq!(
		row["manufacturer"].as_str(),
		Some(manufacturer.as_str()),
		"{row:?}"
	);
	assert_eq!(row["receiver"].as_str(), Some(receiver.as_str()), "{row:?}");
	assert_eq!(
		row["typeOfReport"].as_str(),
		Some("Report from study"),
		"{row:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_public_case_create_derives_org_and_version() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let attacker_org_id = Uuid::new_v4();

	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"organization_id": attacker_org_id,
				"version": 99,
				"safety_report_id": format!("SR-{}", Uuid::new_v4()),
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
	assert_eq!(body["data"]["version"].as_i64(), Some(1), "{body:?}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_public_case_create_derives_profile_from_appendices() -> Result<()> {
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
				"safety_report_id": format!("SR-{}", Uuid::new_v4()),
				"status": "draft",
				"appendices_json": "[\"mfds\",\"fda\"]"
			}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	assert!(body["data"].get("validation_profile").is_none(), "{body:?}");
	assert_eq!(
		body["data"]["appendices_json"], "[\"mfds\",\"fda\"]",
		"{body:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_public_case_update_ignores_system_managed_fields() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let report_id = format!("SR-{}", Uuid::new_v4());
	let (create_status, create_body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safety_report_id": report_id,
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

	let bogus_submitter = Uuid::new_v4();
	let (update_status, update_body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
				"submitted_by": bogus_submitter,
				"submitted_at": "2026-04-13T00:00:00Z",
				"raw_xml": "ZmFrZQ==",
				"dirty_c": true,
				"dirty_d": true,
				"dirty_e": true,
				"dirty_f": true,
				"dirty_g": true,
				"dirty_h": true
			}
		}),
	)
	.await?;
	assert_eq!(update_status, StatusCode::OK, "{update_body:?}");

	let (get_status, get_body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}")).await?;
	assert_eq!(get_status, StatusCode::OK, "{get_body:?}");
	assert_eq!(
		get_body["data"]["submitted_by"],
		Value::Null,
		"{get_body:?}"
	);
	assert_eq!(
		get_body["data"]["submitted_at"],
		Value::Null,
		"{get_body:?}"
	);
	assert_eq!(get_body["data"]["raw_xml"], Value::Null, "{get_body:?}");
	assert_eq!(
		get_body["data"]["dirty_c"].as_bool(),
		Some(false),
		"{get_body:?}"
	);
	assert_eq!(
		get_body["data"]["dirty_d"].as_bool(),
		Some(false),
		"{get_body:?}"
	);
	assert_eq!(
		get_body["data"]["dirty_e"].as_bool(),
		Some(false),
		"{get_body:?}"
	);
	assert_eq!(
		get_body["data"]["dirty_f"].as_bool(),
		Some(false),
		"{get_body:?}"
	);
	assert_eq!(
		get_body["data"]["dirty_g"].as_bool(),
		Some(false),
		"{get_body:?}"
	);
	assert_eq!(
		get_body["data"]["dirty_h"].as_bool(),
		Some(false),
		"{get_body:?}"
	);
	assert_eq!(
		get_body["data"]["status"].as_str(),
		Some("draft"),
		"{get_body:?}"
	);
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
				"safety_report_id": format!("SR-{}", Uuid::new_v4()),
				"status": "draft",
				"appendices_json": "[\"fda\"]"
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
				"mfds_report_type": "spontaneous"
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
		update_body["data"]["mfds_report_type"].as_str(),
		Some("spontaneous"),
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
			safety_report_id,
			version,
			status,
			appendices_json,
			raw_xml,
			dirty_c,
			dirty_d,
			dirty_e,
			dirty_f,
			dirty_g,
			dirty_h,
			created_by,
			updated_by
		) VALUES ($1, $2, $3, $4, $5, $6, $7, false, false, false, false, false, false, $8, $8)",
	)
	.bind(case_id)
	.bind(seed.org_id)
	.bind(format!("SR-SHAPED-SAVE-{case_id}"))
	.bind(1_i32)
	.bind("draft")
	.bind("[\"fda\"]")
	.bind(b"<ichicsr/>".to_vec())
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
				"report_year": "2026",
				"source_document_name": "source-followup.pdf"
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
		update_body["data"]["source_document_name"].as_str(),
		Some("source-followup.pdf"),
		"{update_body:?}"
	);
	let rendered = update_body.to_string().to_ascii_lowercase();
	assert!(!rendered.contains("batch"), "{update_body:?}");
	assert!(!rendered.contains("header"), "{update_body:?}");
	assert!(!rendered.contains("import"), "{update_body:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_identity_update_requires_reason_for_change() -> Result<()> {
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
				"safety_report_id": format!("SR-{}", Uuid::new_v4()),
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
				"safety_report_id": format!("SR-RENAMED-{}", Uuid::new_v4())
			}
		}),
	)
	.await?;
	assert_eq!(update_status, StatusCode::BAD_REQUEST, "{update_body:?}");
	assert!(
		update_body.to_string().contains(
			"reason_for_change is required for case identity/scope updates"
		),
		"{update_body:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_identity_update_records_reason_for_change() -> Result<()> {
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
				"safety_report_id": format!("SR-{}", Uuid::new_v4()),
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
	let next_safety_report_id = format!("SR-RENAMED-{}", Uuid::new_v4());

	let (update_status, update_body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
				"safety_report_id": next_safety_report_id
			},
			"reason_for_change": "correct case identifier after source reconciliation"
		}),
	)
	.await?;
	assert_eq!(update_status, StatusCode::OK, "{update_body:?}");

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	dbx.execute(sqlx::query("SET ROLE e2br3_auditor_role"))
		.await?;
	let reason = dbx
		.fetch_optional(
			sqlx::query_as::<_, (Option<String>,)>(
				r#"
				SELECT reason_for_change
				FROM audit_logs
				WHERE table_name = 'cases'
				  AND record_id = $1
				  AND action = 'UPDATE'
				  AND changed_fields ? 'safety_report_id'
				ORDER BY id DESC
				LIMIT 1
				"#,
			)
			.bind(case_id),
		)
		.await?;
	dbx.rollback_txn().await?;
	assert_eq!(
		reason.and_then(|(v,)| v).as_deref(),
		Some("correct case identifier after source reconciliation")
	);

	Ok(())
}

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
				"safety_report_id": format!("SR-{}", Uuid::new_v4()),
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
				"safety_report_id": report_id,
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
			"reason_for_change": "client requested soft delete"
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

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	dbx.execute(sqlx::query("SET ROLE e2br3_auditor_role"))
		.await?;
	let reason = dbx
		.fetch_optional(
			sqlx::query_as::<_, (Option<String>,)>(
				r#"
				SELECT reason_for_change
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
			.bind(Uuid::parse_str(&case_id)?),
		)
		.await?;
	dbx.rollback_txn().await?;
	assert_eq!(
		reason.and_then(|(v,)| v).as_deref(),
		Some("client requested soft delete")
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
				"safety_report_id": format!("SR-{}", Uuid::new_v4()),
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
		json!({ "reason_for_change": "client requested soft delete" }),
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
	assert_eq!(update_status, StatusCode::BAD_REQUEST, "{update_body:?}");
	assert!(
		update_body
			.to_string()
			.contains("deleted cases are read-only"),
		"{update_body:?}"
	);

	Ok(())
}
