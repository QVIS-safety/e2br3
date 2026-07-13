#![allow(unused_imports, dead_code)]

use crate::common::{
	cookie_header, init_test_mm, insert_user, seed_org_with_users, system_user_id,
	Result, TEST_CUSTOM_MANAGER_ROLE,
};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use lib_auth::token::generate_web_token;
use lib_core::ctx::{
	ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN,
};
use lib_core::model::acs::{
	has_permission, CASE_APPROVE, CASE_CREATE, CASE_UPDATE, PRESAVE_TEMPLATE_CREATE,
	PRESAVE_TEMPLATE_DELETE, PRESAVE_TEMPLATE_LIST, PRESAVE_TEMPLATE_READ,
	PRESAVE_TEMPLATE_UPDATE, SETTINGS_READ, SETTINGS_UPDATE, TERMINOLOGY_APPROVE,
	TERMINOLOGY_IMPORT, USER_CREATE, USER_DELETE, USER_LIST, USER_READ, USER_UPDATE,
	XML_EXPORT, XML_EXPORT_READ, XML_IMPORT, XML_IMPORT_READ,
};
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::ModelManager;
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

fn extract_id(value: &Value) -> Result<Uuid> {
	let id = value["data"]["id"].as_str().ok_or("missing data.id")?;
	Ok(Uuid::parse_str(id)?)
}

async fn request_json(
	app: &Router,
	method: &str,
	cookie: &str,
	uri: String,
	body: Option<Value>,
) -> Result<(StatusCode, Value)> {
	let mut req = Request::builder().method(method).uri(uri);
	if !cookie.is_empty() {
		req = req.header("cookie", cookie);
	}
	if body.is_some() {
		req = req.header("content-type", "application/json");
	}
	let req = req.body(match body {
		Some(body) => Body::from(body.to_string()),
		None => Body::empty(),
	})?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let bytes = to_bytes(res.into_body(), usize::MAX).await?;
	let value = serde_json::from_slice(&bytes)
		.unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&bytes) }));
	Ok((status, value))
}

async fn request_raw_status(
	app: &Router,
	method: &str,
	cookie: &str,
	uri: &str,
	content_type: Option<&str>,
	body: impl Into<Body>,
) -> Result<StatusCode> {
	let mut req = Request::builder().method(method).uri(uri);
	if !cookie.is_empty() {
		req = req.header("cookie", cookie);
	}
	if let Some(content_type) = content_type {
		req = req.header("content-type", content_type);
	}
	let res = app.clone().oneshot(req.body(body.into())?).await?;
	Ok(res.status())
}

async fn create_empty_custom_role(
	app: &Router,
	admin_cookie: &str,
	profile_id: &str,
) -> Result<String> {
	create_empty_custom_role_with_generated_id(app, admin_cookie, profile_id).await
}

async fn create_empty_custom_role_with_generated_id(
	app: &Router,
	admin_cookie: &str,
	profile_id: &str,
) -> Result<String> {
	let (status, value) = request_json(
		app,
		"POST",
		admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"name": profile_id,
				"description": format!("Effective permission test role {profile_id}"),
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	Ok(value["id"]
		.as_str()
		.ok_or("missing created role id")?
		.to_string())
}

async fn update_role_privileges(
	app: &Router,
	admin_cookie: &str,
	profile_id: &str,
	privileges: Value,
) -> Result<Value> {
	let (status, value) = request_json(
		app,
		"PUT",
		admin_cookie,
		format!("/api/admin/permission-profiles/{profile_id}"),
		Some(json!({ "data": { "privileges": privileges } })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	Ok(value)
}

async fn custom_role_user(
	mm: &ModelManager,
	org_id: Uuid,
	profile_id: &str,
) -> Result<(Uuid, String)> {
	let user =
		insert_user(mm, org_id, profile_id, system_user_id(), Some("custompwd"))
			.await?;
	let token = generate_web_token(&user.email, user.token_salt)?;
	Ok((user.id, cookie_header(&token.to_string())))
}

async fn assert_get_status(
	app: &Router,
	cookie: &str,
	uri: &str,
	expected: StatusCode,
) -> Result<Value> {
	let (status, value) =
		request_json(app, "GET", cookie, uri.to_string(), None).await?;
	assert_eq!(status, expected, "{uri} body={value:?}");
	Ok(value)
}

async fn assert_get_not_status(
	app: &Router,
	cookie: &str,
	uri: &str,
	disallowed: StatusCode,
) -> Result<Value> {
	let (status, value) =
		request_json(app, "GET", cookie, uri.to_string(), None).await?;
	assert_ne!(status, disallowed, "{uri} body={value:?}");
	Ok(value)
}

async fn assert_profile_capabilities(
	app: &Router,
	cookie: &str,
	expected: &[(&str, &str, bool)],
) -> Result<Value> {
	let (status, profile) = request_json(
		app,
		"GET",
		cookie,
		"/api/users/me/profile".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{profile:?}");
	for (module, action, expected) in expected {
		assert_eq!(
			profile["data"]["capabilities"][*module][*action].as_bool(),
			Some(*expected),
			"{module}.{action} capability mismatch: {profile:?}"
		);
	}
	Ok(profile)
}

async fn assert_workflow_assign_status(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	target_role: &str,
	expected: StatusCode,
) -> Result<Value> {
	let (status, value) = request_json(
		app,
		"POST",
		cookie,
		format!("/api/cases/{case_id}/workflow/assign"),
		Some(json!({
			"data": {
				"target_role": target_role
			}
		})),
	)
	.await?;
	assert_eq!(status, expected, "{value:?}");
	Ok(value)
}

async fn create_case(
	app: &Router,
	cookie: &str,
	safety_report_id: &str,
	dg_prd_key: Option<&str>,
) -> Result<Uuid> {
	let (status, value) = request_json(
		app,
		"POST",
		cookie,
		"/api/cases".to_string(),
		Some(json!({
			"data": {
				"safetyReportIdentification": {
					"safetyReportId": safety_report_id
				},
				"status": "draft",
				"dgPrdKey": dg_prd_key
			}
		})),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create case failed: status={status} body={value}").into()
		);
	}
	extract_id(&value)
}

async fn create_message_header(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	sender: &str,
) -> Result<()> {
	let (status, value) = request_json(
		app,
		"POST",
		cookie,
		format!("/api/cases/{case_id}/message-header"),
		Some(json!({
			"data": {
				"case_id": case_id,
				"message_number": format!("MSG-{case_id}"),
				"message_sender_identifier": sender,
				"message_receiver_identifier": "RECV-01",
				"message_date": "20240201010101"
			}
		})),
	)
	.await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(format!(
			"create message header failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

async fn create_sender_information(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	organization_name: &str,
) -> Result<()> {
	let (status, value) = request_json(
		app,
		"POST",
		cookie,
		format!("/api/cases/{case_id}/safety-report/senders"),
		Some(json!({
			"data": {
				"case_id": case_id,
				"sender_type": "1",
				"organization_name": organization_name,
				"person_given_name": "Safety"
			}
		})),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create sender information failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

async fn create_sender_presave(
	app: &Router,
	cookie: &str,
	name: &str,
	sender_identifier: &str,
) -> Result<Uuid> {
	let (status, value) = request_json(
		app,
		"POST",
		cookie,
		"/api/presaves/senders".to_string(),
		Some(json!({
			"data": {
				"authority": "fda",
				"name": name,
				"comments": "Routing source-of-truth test sender",
				"sender_type": "2",
				"organization_name": name,
				"person_given_name": "Safety",
				"email": format!("{sender_identifier}@example.test")
			}
		})),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create sender presave failed: status={status} body={value}"
		)
		.into());
	}
	let id = extract_id(&value)?;
	let (status, value) = request_json(
		app,
		"POST",
		cookie,
		format!("/api/presaves/senders/{id}/gateways"),
		Some(json!({
			"data": {
				"sequence_number": 1,
				"gateway_authority": "fda",
				"sender_identifier": sender_identifier,
				"is_default_for_authority": true
			}
		})),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create sender presave gateway failed: status={status} body={value}"
		)
		.into());
	}
	Ok(id)
}

async fn create_study(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	study_number: &str,
) -> Result<()> {
	let (status, value) = request_json(
		app,
		"POST",
		cookie,
		format!("/api/cases/{case_id}/safety-report/studies"),
		Some(json!({
			"data": {
				"case_id": case_id,
				"study_name": study_number,
				"sponsor_study_number": study_number
			}
		})),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create study failed: status={status} body={value}").into()
		);
	}
	Ok(())
}

async fn create_drug(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	blinded: bool,
) -> Result<()> {
	let (status, value) = request_json(
		app,
		"POST",
		cookie,
		format!("/api/cases/{case_id}/drugs"),
		Some(json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"drug_characterization": "1",
				"medicinal_product": "Demo Product",
				"brand_name": "Demo Product"
			}
		})),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create drug failed: status={status} body={value}").into()
		);
	}
	let drug_id = extract_id(&value)?;
	if blinded {
		let (status, value) = request_json(
			app,
			"PUT",
			cookie,
			format!("/api/cases/{case_id}/drugs/{drug_id}"),
			Some(json!({
				"data": {
					"investigational_product_blinded": true
				}
			})),
		)
		.await?;
		if status != StatusCode::OK {
			return Err(
				format!("update drug failed: status={status} body={value}").into()
			);
		}
	}
	Ok(())
}

async fn create_drug_with_brand(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	brand_name: &str,
) -> Result<()> {
	let (status, value) = request_json(
		app,
		"POST",
		cookie,
		format!("/api/cases/{case_id}/drugs"),
		Some(json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"drug_characterization": "1",
				"medicinal_product": "Demo Product",
				"brand_name": brand_name
			}
		})),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create drug with brand failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

async fn update_user_scope(
	app: &Router,
	admin_cookie: &str,
	user_id: Uuid,
	body: Value,
) -> Result<()> {
	let (status, value) = request_json(
		app,
		"PUT",
		admin_cookie,
		format!("/api/users/{user_id}"),
		Some(json!({ "data": body })),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update user scope failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

async fn insert_history_rows_for_case(
	mm: &ModelManager,
	case_id: Uuid,
	case_number: &str,
	user_id: Uuid,
	org_id: Uuid,
	suffix: &str,
) -> Result<()> {
	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	set_full_context_dbx(dbx, user_id, org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	dbx.execute(
		sqlx::query(
			"INSERT INTO xml_import_history (
					uploaded_file_name,
					source_file_name,
					case_id,
					case_number,
					status,
					uploaded_by
				) VALUES ($1, $2, $3, $4, 'success', $5)",
		)
		.bind(format!("import-{suffix}.zip"))
		.bind(format!("source-{suffix}.xml"))
		.bind(case_id)
		.bind(case_number)
		.bind(user_id),
	)
	.await?;
	dbx.execute(
		sqlx::query(
			"INSERT INTO xml_export_history (
						case_id,
						case_number,
						file_name,
						status,
						exported_by
					) VALUES ($1, $2, $3, 'success', $4)",
		)
		.bind(case_id)
		.bind(case_number)
		.bind(format!("export-{suffix}.xml"))
		.bind(user_id),
	)
	.await?;
	dbx.execute(
		sqlx::query(
			"INSERT INTO case_submissions (
					case_id,
					gateway,
					remote_submission_id,
					status,
					xml_bytes,
					submitted_by
				) VALUES ($1, 'fda', $2, 'ack1_received', 128, $3)",
		)
		.bind(case_id)
		.bind(format!("REMOTE-{suffix}"))
		.bind(user_id),
	)
	.await?;
	dbx.commit_txn().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_list_is_filtered_by_sender_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);

	let case_a = create_case(
		&app,
		&admin_cookie,
		&format!("SR-A-{}", Uuid::new_v4()),
		None,
	)
	.await?;
	let case_b = create_case(
		&app,
		&admin_cookie,
		&format!("SR-B-{}", Uuid::new_v4()),
		None,
	)
	.await?;
	create_message_header(&app, &admin_cookie, case_a, "SEND-A").await?;
	create_message_header(&app, &admin_cookie, case_b, "SEND-B").await?;
	create_sender_information(&app, &admin_cookie, case_a, "Sender Org A").await?;
	create_sender_information(&app, &admin_cookie, case_b, "Sender Org B").await?;

	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({
			"access_sender_ids": "[\"Sender Org A\"]"
		}),
	)
	.await?;

	let (status, value) =
		request_json(&app, "GET", &viewer_cookie, "/api/cases".to_string(), None)
			.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let cases = value["data"].as_array().ok_or("missing cases array")?;
	assert!(cases.iter().any(|row| row["id"] == case_a.to_string()));
	assert!(!cases.iter().any(|row| row["id"] == case_b.to_string()));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_get_does_not_match_sender_scope_by_message_header_only(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(
		&app,
		&admin_cookie,
		&format!("SR-SENDER-HEADER-ONLY-{}", Uuid::new_v4()),
		None,
	)
	.await?;
	create_message_header(&app, &admin_cookie, case_id, "MSG-ONLY").await?;
	create_sender_information(&app, &admin_cookie, case_id, "Sender Org B").await?;
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": ["MSG-ONLY"] }),
	)
	.await?;

	let (status, _value) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		format!("/api/cases/{case_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_get_allows_case_without_sender_org_when_user_has_sender_scope(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);

	// Case carries no sender organization (and no product/study/blind data).
	let case_id = create_case(
		&app,
		&admin_cookie,
		&format!("SR-NO-SENDER-ORG-{}", Uuid::new_v4()),
		None,
	)
	.await?;

	// Viewer has a sender scope, but the case has no sender org to match against.
	// Sender scope must behave like product/study: an absent case value is allowed,
	// not blocked. (required_scope_matches semantics, not optional_scope_matches.)
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": ["Sender Org A"] }),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		format!("/api/cases/{case_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["data"]["id"], case_id.to_string());
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_get_allows_unset_scope_even_when_case_has_values() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);

	// Case carries sender org, product brand, and study number.
	let case_id = create_case(
		&app,
		&admin_cookie,
		&format!("SR-UNSET-ALL-{}", Uuid::new_v4()),
		Some("PROD-UNSET"),
	)
	.await?;
	create_message_header(&app, &admin_cookie, case_id, "SEND-UNSET").await?;
	create_sender_information(&app, &admin_cookie, case_id, "Sender Unset Org")
		.await?;
	create_drug_with_brand(&app, &admin_cookie, case_id, "Brand Unset").await?;
	create_study(&app, &admin_cookie, case_id, "STUDY-UNSET").await?;

	// Viewer has NO scope configured at all. Unset scope means "allow all":
	// the case must be visible even though it carries sender/product/study values.
	let (status, value) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		format!("/api/cases/{case_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["data"]["id"], case_id.to_string());
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_update_requires_matching_sender_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);

	let case_a = create_case(
		&app,
		&admin_cookie,
		&format!("SR-UPD-A-{}", Uuid::new_v4()),
		None,
	)
	.await?;
	let case_b = create_case(
		&app,
		&admin_cookie,
		&format!("SR-UPD-B-{}", Uuid::new_v4()),
		None,
	)
	.await?;
	create_message_header(&app, &admin_cookie, case_a, "SEND-A").await?;
	create_message_header(&app, &admin_cookie, case_b, "SEND-B").await?;
	create_sender_information(&app, &admin_cookie, case_a, "Sender Org A").await?;
	create_sender_information(&app, &admin_cookie, case_b, "Sender Org B").await?;
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": ["Sender Org A"] }),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"PUT",
		&viewer_cookie,
		format!("/api/cases/{case_b}"),
		Some(json!({
			"data": {
				"dg_prd_key": "UNAUTHORIZED-PRODUCT-EDIT"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_get_requires_matching_product_and_study_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(
		&app,
		&admin_cookie,
		&format!("SR-SCOPE-{}", Uuid::new_v4()),
		Some("PROD-ALPHA"),
	)
	.await?;
	create_message_header(&app, &admin_cookie, case_id, "SEND-A").await?;
	create_sender_information(&app, &admin_cookie, case_id, "Sender Org A").await?;
	create_drug_with_brand(&app, &admin_cookie, case_id, "Brand Alpha").await?;
	create_study(&app, &admin_cookie, case_id, "STUDY-ALPHA").await?;

	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({
			"access_sender_ids": "[\"Sender Org A\"]",
			"access_product_ids": "[\"Brand Alpha\"]",
			"access_study_ids": "[\"STUDY-ALPHA\"]"
		}),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		format!("/api/cases/{case_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["data"]["id"], case_id.to_string());
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_get_allows_empty_product_or_study_scope_but_blocks_mismatch(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(
		&app,
		&admin_cookie,
		&format!("SR-STRICT-SCOPE-{}", Uuid::new_v4()),
		Some("PROD-STRICT"),
	)
	.await?;
	create_message_header(&app, &admin_cookie, case_id, "SEND-STRICT").await?;
	create_sender_information(&app, &admin_cookie, case_id, "Sender Strict Org")
		.await?;
	create_drug_with_brand(&app, &admin_cookie, case_id, "Brand Strict").await?;
	create_study(&app, &admin_cookie, case_id, "STUDY-STRICT").await?;

	// Sender matches; product/study left unset. Unset scope means "allow all",
	// so the case is visible even though it carries product/study values.
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": ["Sender Strict Org"] }),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		format!("/api/cases/{case_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	// A product scope that is set but does NOT match the case must still block:
	// filtering only kicks in when the user has an explicit scope value.
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({
			"access_sender_ids": ["Sender Strict Org"],
			"access_product_ids": ["Wrong Brand"]
		}),
	)
	.await?;

	let (status, _value) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		format!("/api/cases/{case_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN);

	// Matching scope on every dimension is allowed.
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({
			"access_sender_ids": ["Sender Strict Org"],
			"access_product_ids": ["Brand Strict"],
			"access_study_ids": ["STUDY-STRICT"]
		}),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		format!("/api/cases/{case_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_get_blocks_blinded_case_without_blind_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(
		&app,
		&admin_cookie,
		&format!("SR-BLIND-{}", Uuid::new_v4()),
		Some("PROD-BLIND"),
	)
	.await?;
	create_message_header(&app, &admin_cookie, case_id, "SEND-A").await?;
	create_sender_information(&app, &admin_cookie, case_id, "Sender Org A").await?;
	create_study(&app, &admin_cookie, case_id, "STUDY-BLIND").await?;
	create_drug(&app, &admin_cookie, case_id, true).await?;

	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({
			"access_sender_ids": "[\"Sender Org A\"]",
			"access_product_ids": "[\"Demo Product\"]",
			"access_study_ids": "[\"STUDY-BLIND\"]",
			"access_blind_allowed": false
		}),
	)
	.await?;

	let (status, _value) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		format!("/api/cases/{case_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_import_export_submission_histories_follow_product_scope() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let scoped_manager = insert_user(
		&mm,
		seed.org_id,
		TEST_CUSTOM_MANAGER_ROLE,
		system_user_id(),
		Some("managerpwd"),
	)
	.await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let manager_token =
		generate_web_token(&scoped_manager.email, scoped_manager.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let manager_cookie = cookie_header(&manager_token.to_string());
	let app = web_server::app(mm.clone());

	let case_allowed_number = format!("SR-HIST-A-{}", Uuid::new_v4());
	let case_hidden_number = format!("SR-HIST-B-{}", Uuid::new_v4());
	let case_allowed =
		create_case(&app, &admin_cookie, &case_allowed_number, Some("PROD-A"))
			.await?;
	let case_hidden =
		create_case(&app, &admin_cookie, &case_hidden_number, Some("PROD-B"))
			.await?;
	create_message_header(&app, &admin_cookie, case_allowed, "SEND-HIST").await?;
	create_message_header(&app, &admin_cookie, case_hidden, "SEND-HIST").await?;
	create_sender_information(&app, &admin_cookie, case_allowed, "Sender Hist Org")
		.await?;
	create_sender_information(&app, &admin_cookie, case_hidden, "Sender Hist Org")
		.await?;
	create_drug_with_brand(&app, &admin_cookie, case_allowed, "Brand A").await?;
	create_drug_with_brand(&app, &admin_cookie, case_hidden, "Brand B").await?;
	insert_history_rows_for_case(
		&mm,
		case_allowed,
		&case_allowed_number,
		seed.admin.id,
		seed.org_id,
		"allowed",
	)
	.await?;
	insert_history_rows_for_case(
		&mm,
		case_hidden,
		&case_hidden_number,
		seed.admin.id,
		seed.org_id,
		"hidden",
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		scoped_manager.id,
		json!({
			"access_sender_ids": ["Sender Hist Org"],
			"access_product_ids": ["Brand A"]
		}),
	)
	.await?;

	let (status, import_history) = request_json(
		&app,
		"GET",
		&manager_cookie,
		"/api/import/xml/history".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{import_history:?}");
	let import_items = import_history["data"]["items"]
		.as_array()
		.ok_or("missing import history items")?;
	assert!(import_items
		.iter()
		.any(|row| row["caseId"] == case_allowed.to_string()));
	assert!(!import_items
		.iter()
		.any(|row| row["caseId"] == case_hidden.to_string()));

	let (status, export_history) = request_json(
		&app,
		"GET",
		&manager_cookie,
		"/api/exports/history".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{export_history:?}");
	let export_items = export_history["data"]["items"]
		.as_array()
		.ok_or("missing export history items")?;
	assert!(export_items
		.iter()
		.any(|row| row["caseId"] == case_allowed.to_string()));
	assert!(!export_items
		.iter()
		.any(|row| row["caseId"] == case_hidden.to_string()));

	let (status, submission_history) = request_json(
		&app,
		"GET",
		&manager_cookie,
		"/api/submissions/history".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{submission_history:?}");
	let submission_items = submission_history["data"]["items"]
		.as_array()
		.ok_or("missing submission history items")?;
	assert!(
		submission_items
			.iter()
			.any(|row| row["caseId"] == case_allowed.to_string()),
		"{submission_history:?}"
	);
	assert!(
		!submission_items
			.iter()
			.any(|row| row["caseId"] == case_hidden.to_string()),
		"{submission_history:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_routing_profile_sender_options_include_info_sender_masters(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);

	create_sender_presave(
		&app,
		&admin_cookie,
		"Client A Sender Master",
		"SEND-MASTER-A",
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": ["Client A Sender Master"] }),
	)
	.await?;

	let (status, admin_profile) = request_json(
		&app,
		"GET",
		&admin_cookie,
		"/api/users/me/routing".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{admin_profile:?}");
	let admin_senders = admin_profile["data"]["availableSenders"]
		.as_array()
		.ok_or("missing admin senders")?;
	let admin_master = admin_senders
		.iter()
		.find(|row| row["senderIdentifier"] == "SEND-MASTER-A")
		.ok_or("INFO sender master missing from admin routing options")?;
	assert_eq!(admin_master["caseCount"], 0);

	let (status, viewer_profile) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		"/api/users/me/routing".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{viewer_profile:?}");
	let viewer_senders = viewer_profile["data"]["availableSenders"]
		.as_array()
		.ok_or("missing viewer senders")?;
	assert_eq!(viewer_senders.len(), 1);
	assert_eq!(viewer_senders[0]["senderIdentifier"], "SEND-MASTER-A");
	assert_eq!(viewer_senders[0]["caseCount"], 0);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_unset_sender_scope_lists_all_sender_presaves() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);

	let sender_id = create_sender_presave(
		&app,
		&admin_cookie,
		"Unset Scope Sender Master",
		"SEND-UNSET-SCOPE",
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		"/api/presaves/senders".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let rows = value["data"].as_array().ok_or("missing sender rows")?;
	assert!(
		rows.iter().any(|row| row["id"] == sender_id.to_string()),
		"unset sender scope must list all sender presaves: {value:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_company_sponsor_admin_cannot_assign_sender_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let company_admin = insert_user(
		&mm,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_COMPANY,
		system_user_id(),
		Some("companypwd"),
	)
	.await?;
	let company_token =
		generate_web_token(&company_admin.email, company_admin.token_salt)?;
	let company_cookie = cookie_header(&company_token.to_string());
	let app = web_server::app(mm);

	let (status, value) = request_json(
		&app,
		"PUT",
		&company_cookie,
		format!("/api/users/{}", seed.viewer.id),
		Some(json!({
			"data": {
				"access_sender_ids": ["SEND-A"]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_routing_profile_sender_options_follow_role_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let company_admin = insert_user(
		&mm,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_COMPANY,
		system_user_id(),
		Some("companypwd"),
	)
	.await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let company_token =
		generate_web_token(&company_admin.email, company_admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let company_cookie = cookie_header(&company_token.to_string());
	let app = web_server::app(mm);

	let case_a = create_case(
		&app,
		&admin_cookie,
		&format!("SR-ROUTE-A-{}", Uuid::new_v4()),
		None,
	)
	.await?;
	let case_b = create_case(
		&app,
		&admin_cookie,
		&format!("SR-ROUTE-B-{}", Uuid::new_v4()),
		None,
	)
	.await?;
	create_message_header(&app, &admin_cookie, case_a, "SEND-A").await?;
	create_message_header(&app, &admin_cookie, case_b, "SEND-B").await?;
	create_sender_information(&app, &admin_cookie, case_a, "Sender Org A").await?;
	create_sender_information(&app, &admin_cookie, case_b, "Sender Org B").await?;
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": ["Sender Org A"] }),
	)
	.await?;

	let (status, admin_profile) = request_json(
		&app,
		"GET",
		&admin_cookie,
		"/api/users/me/routing".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{admin_profile:?}");
	let admin_senders = admin_profile["data"]["availableSenders"]
		.as_array()
		.ok_or("missing admin senders")?;
	assert!(admin_senders
		.iter()
		.any(|row| row["senderIdentifier"] == "SEND-A"));
	assert!(admin_senders
		.iter()
		.any(|row| row["senderIdentifier"] == "SEND-B"));

	let (status, company_profile) = request_json(
		&app,
		"GET",
		&company_cookie,
		"/api/users/me/routing".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{company_profile:?}");
	let company_senders = company_profile["data"]["availableSenders"]
		.as_array()
		.ok_or("missing company senders")?;
	assert!(company_senders
		.iter()
		.any(|row| row["senderIdentifier"] == "SEND-A"));
	assert!(company_senders
		.iter()
		.any(|row| row["senderIdentifier"] == "SEND-B"));

	let (status, viewer_profile) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		"/api/users/me/routing".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{viewer_profile:?}");
	let viewer_senders = viewer_profile["data"]["availableSenders"]
		.as_array()
		.ok_or("missing viewer senders")?;
	assert_eq!(viewer_senders.len(), 1);
	assert_eq!(viewer_senders[0]["senderIdentifier"], "SEND-A");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_active_sender_selection_filters_case_list() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);

	let case_a = create_case(
		&app,
		&admin_cookie,
		&format!("SR-ACTIVE-A-{}", Uuid::new_v4()),
		None,
	)
	.await?;
	let case_b = create_case(
		&app,
		&admin_cookie,
		&format!("SR-ACTIVE-B-{}", Uuid::new_v4()),
		None,
	)
	.await?;
	create_message_header(&app, &admin_cookie, case_a, "SEND-A").await?;
	create_message_header(&app, &admin_cookie, case_b, "SEND-B").await?;
	create_sender_information(&app, &admin_cookie, case_a, "Sender Org A").await?;
	create_sender_information(&app, &admin_cookie, case_b, "Sender Org B").await?;
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": ["Sender Org A", "Sender Org B"] }),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"PUT",
		&viewer_cookie,
		"/api/users/me/routing".to_string(),
		Some(json!({ "data": { "sender_id": "SEND-A" } })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let (status, value) =
		request_json(&app, "GET", &viewer_cookie, "/api/cases".to_string(), None)
			.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let cases = value["data"].as_array().ok_or("missing cases array")?;
	assert!(cases.iter().any(|row| row["id"] == case_a.to_string()));
	assert!(!cases.iter().any(|row| row["id"] == case_b.to_string()));

	let (status, _value) = request_json(
		&app,
		"PUT",
		&viewer_cookie,
		"/api/users/me/routing".to_string(),
		Some(json!({ "data": { "active_sender_identifier": "SEND-C" } })),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN);
	Ok(())
}
