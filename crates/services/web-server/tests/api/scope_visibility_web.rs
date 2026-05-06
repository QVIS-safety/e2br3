use crate::common::{
	cookie_header, init_test_mm, insert_user, seed_org_with_users, system_user_id,
	Result,
};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use lib_auth::token::generate_web_token;
use lib_core::ctx::{
	ROLE_MANAGER, ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO,
	ROLE_SYSTEM_ADMIN,
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
				"safety_report_id": safety_report_id,
				"status": "draft",
				"dg_prd_key": dg_prd_key
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

async fn create_sender_presave_template(
	app: &Router,
	cookie: &str,
	name: &str,
	sender_identifier: &str,
) -> Result<Uuid> {
	let (status, value) = request_json(
		app,
		"POST",
		cookie,
		"/api/presave-templates".to_string(),
		Some(json!({
			"data": {
				"entity_type": "sender",
				"name": name,
				"description": "Routing source-of-truth test sender",
				"data": {
					"senderType": "2",
					"senderIdentifier": sender_identifier,
					"senderOrganization": name,
					"linkedOrganizationName": name,
					"linkedOrganizationType": "client"
				}
			}
		})),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create sender presave template failed: status={status} body={value}"
		)
		.into());
	}
	extract_id(&value)
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
				"medicinal_product": "Demo Product"
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
					validation_profile,
					exported_by
				) VALUES ($1, $2, $3, 'success', 'fda', $4)",
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

	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({
			"access_sender_ids": "[\"SEND-A\"]"
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
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": ["SEND-A"] }),
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
	create_study(&app, &admin_cookie, case_id, "STUDY-ALPHA").await?;

	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({
			"access_sender_ids": "[\"SEND-A\"]",
			"access_product_ids": "[\"PROD-ALPHA\"]",
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
async fn test_case_get_blocks_empty_product_or_study_scope_when_case_has_values(
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
	create_study(&app, &admin_cookie, case_id, "STUDY-STRICT").await?;

	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": ["SEND-STRICT"] }),
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

	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({
			"access_sender_ids": ["SEND-STRICT"],
			"access_product_ids": ["PROD-STRICT"],
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
	create_study(&app, &admin_cookie, case_id, "STUDY-BLIND").await?;
	create_drug(&app, &admin_cookie, case_id, true).await?;

	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({
			"access_sender_ids": "[\"SEND-A\"]",
			"access_product_ids": "[\"PROD-BLIND\"]",
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
		ROLE_MANAGER,
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
			"access_sender_ids": ["SEND-HIST"],
			"access_product_ids": ["PROD-A"]
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

	create_sender_presave_template(
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
		json!({ "access_sender_ids": ["SEND-MASTER-A"] }),
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
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": ["SEND-A"] }),
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
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": ["SEND-A", "SEND-B"] }),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"PUT",
		&viewer_cookie,
		"/api/users/me/routing".to_string(),
		Some(json!({ "data": { "active_sender_identifier": "SEND-A" } })),
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

#[serial]
#[tokio::test]
async fn test_role_admin_api_exposes_client_role_metadata() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let (status, value) = request_json(
		&app,
		"GET",
		&admin_cookie,
		"/api/admin/roles".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let roles = value.as_array().ok_or("roles response should be array")?;
	let system = roles
		.iter()
		.find(|role| role["canonical_role_id"] == ROLE_SYSTEM_ADMIN)
		.ok_or("missing system admin role")?;
	assert_eq!(system["is_operational"].as_bool(), Some(false));
	assert_eq!(system["is_editable"].as_bool(), Some(false));

	let sponsor = roles
		.iter()
		.find(|role| role["canonical_role_id"] == ROLE_SPONSOR_ADMIN_CRO)
		.ok_or("missing sponsor admin role")?;
	assert_eq!(sponsor["is_builtin"].as_bool(), Some(true));
	assert_eq!(sponsor["is_sponsor_admin"].as_bool(), Some(true));

	let (status, value) = request_json(
		&app,
		"GET",
		&admin_cookie,
		format!("/api/admin/roles/{ROLE_SPONSOR_ADMIN_CRO}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["canonical_role_id"], ROLE_SPONSOR_ADMIN_CRO);

	let (status, _value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		format!("/api/admin/roles/{ROLE_SPONSOR_ADMIN_CRO}"),
		Some(json!({ "data": { "display_name": "Should Not Change" } })),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_role_admin_api_defaults_visible_name_to_role_id() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let role_name = format!("qa_desc_{}", Uuid::new_v4().simple());

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/roles".to_string(),
		Some(json!({
			"data": {
				"role_name": role_name,
				"description": "Role created with description only",
				"privileges": [
					{
						"menu_key": "case",
						"can_read": true,
						"can_edit": false,
						"can_review": false,
						"can_lock": false
					}
				]
			}
		})),
	)
	.await?;

	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	assert_eq!(value["canonical_role_id"], role_name);
	assert_eq!(value["display_name"], role_name);
	assert_eq!(value["description"], "Role created with description only");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_role_admin_api_persists_menu_privileges() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let role_name = format!("qa_role_{}", Uuid::new_v4().simple());

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/roles".to_string(),
		Some(json!({
			"data": {
				"role_name": role_name,
				"display_name": "QA Role",
				"description": "Can edit cases and read audit",
				"privileges": [
					{
						"menu_key": "case",
						"can_read": true,
						"can_edit": true,
						"can_review": false,
						"can_lock": false
					},
					{
						"menu_key": "audit",
						"can_read": true,
						"can_edit": false,
						"can_review": false,
						"can_lock": false
					}
				]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	assert_eq!(value["canonical_role_id"], role_name);
	assert_eq!(value["description"], "Can edit cases and read audit");
	assert_eq!(value["can_view"].as_bool(), Some(true));
	assert_eq!(value["can_admin"].as_bool(), Some(false));
	assert_eq!(
		value["privilege_map"]["case"]["can_edit"].as_bool(),
		Some(true)
	);
	assert_eq!(
		value["privilege_map"]["audit"]["can_read"].as_bool(),
		Some(true)
	);

	let (status, value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		format!("/api/admin/roles/{role_name}"),
		Some(json!({
			"data": {
				"description": "Can lock cases",
				"privileges": [
					{
						"menu_key": "case",
						"can_read": true,
						"can_edit": true,
						"can_review": true,
						"can_lock": true
					}
				]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["description"], "Can lock cases");
	assert_eq!(value["can_review"].as_bool(), Some(true));
	assert_eq!(value["can_lock"].as_bool(), Some(true));
	assert_eq!(
		value["privilege_map"]["case"]["can_lock"].as_bool(),
		Some(true)
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_custom_admin_capable_role_can_administer_users_and_roles() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let role_name = format!("custom_admin_{}", Uuid::new_v4().simple());

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/roles".to_string(),
		Some(json!({
			"data": {
				"role_name": role_name,
				"description": "Custom sponsor-admin equivalent",
				"privileges": [
					{
						"menu_key": "admin",
						"can_read": true,
						"can_edit": true,
						"can_review": true,
						"can_lock": true
					}
				]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	assert_eq!(value["sponsor_admin_capable"].as_bool(), Some(true));

	let custom_admin = insert_user(
		&mm,
		seed.org_id,
		&role_name,
		system_user_id(),
		Some("custompwd"),
	)
	.await?;
	let custom_token =
		generate_web_token(&custom_admin.email, custom_admin.token_salt)?;
	let custom_cookie = cookie_header(&custom_token.to_string());

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/users".to_string(),
		Some(json!({
			"data": {
				"organization_id": seed.org_id,
				"email": format!("custom-admin-created-{}@example.com", Uuid::new_v4()),
				"role": "viewer"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");

	let next_role = format!("custom_admin_child_{}", Uuid::new_v4().simple());
	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/admin/roles".to_string(),
		Some(json!({
			"data": {
				"role_name": next_role,
				"privileges": [
					{
						"menu_key": "case",
						"can_read": true,
						"can_edit": false,
						"can_review": false,
						"can_lock": false
					}
				]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	Ok(())
}
