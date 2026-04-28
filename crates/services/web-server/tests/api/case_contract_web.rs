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
async fn test_manual_case_save_updates_public_fields_without_import_noise() -> Result<()> {
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
				"validation_profile": "fda"
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
async fn test_imported_case_save_updates_public_fields_without_import_noise() -> Result<()> {
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
			validation_profile,
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
	.bind("fda")
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
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/lifecycle"))
			.await?;
	assert_eq!(lifecycle_status, StatusCode::OK, "{lifecycle_body:?}");
	assert!(
		lifecycle_body["data"]["items"].as_array().is_some_and(|items| items
			.iter()
			.any(|item| item["status"].as_str() == Some("deleted"))),
		"{lifecycle_body:?}"
	);

	Ok(())
}
