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
	has_permission, CASE_APPROVE, CASE_CREATE, CASE_UPDATE, TERMINOLOGY_APPROVE,
	TERMINOLOGY_IMPORT, USER_CREATE, XML_EXPORT, XML_EXPORT_READ, XML_IMPORT,
	XML_IMPORT_READ,
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

async fn create_empty_custom_role(
	app: &Router,
	admin_cookie: &str,
	profile_id: &str,
) -> Result<()> {
	let _ =
		create_empty_custom_role_with_generated_id(app, admin_cookie, profile_id)
			.await?;
	Ok(())
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
				"profile_id": profile_id,
				"name": profile_id,
				"description": format!("Effective permission test role {profile_id}"),
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	Ok(value["profile_id"]
		.as_str()
		.or_else(|| value["profileId"].as_str())
		.ok_or("missing created profile_id")?
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
	assert_eq!(status, StatusCode::CREATED, "{value:?}");

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
	assert_eq!(status, StatusCode::CREATED, "{value:?}");

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
		"/api/admin/permission-profiles".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let roles = value.as_array().ok_or("roles response should be array")?;
	let system = roles
		.iter()
		.find(|role| role["profile_id"] == ROLE_SYSTEM_ADMIN)
		.ok_or("missing system permission profile")?;
	assert_eq!(system["is_operational"].as_bool(), Some(false));
	assert_eq!(system["is_editable"].as_bool(), Some(false));

	let sponsor = roles
		.iter()
		.find(|role| role["profile_id"] == ROLE_SPONSOR_ADMIN_CRO)
		.ok_or("missing sponsor permission profile")?;
	assert_eq!(sponsor["is_builtin"].as_bool(), Some(true));
	assert_eq!(sponsor["is_sponsor_admin"].as_bool(), Some(true));
	assert_eq!(sponsor["is_editable"].as_bool(), Some(false));
	let sponsor_privileges = sponsor["privileges"]
		.as_array()
		.ok_or("sponsor privileges should be an array")?;
	for menu_key in [
		"case",
		"info",
		"import",
		"export_submission",
		"users",
		"roles",
		"settings",
		"audit",
		"data",
	] {
		let privilege = sponsor_privileges
			.iter()
			.find(|row| row["menu_key"] == menu_key)
			.ok_or_else(|| format!("missing sponsor privilege for {menu_key}"))?;
		assert_eq!(privilege["can_read"].as_bool(), Some(true), "{menu_key}");
		assert_eq!(privilege["can_edit"].as_bool(), Some(true), "{menu_key}");
	}

	let system_privileges = system["privileges"]
		.as_array()
		.ok_or("system privileges should be an array")?;
	assert!(
		system_privileges.is_empty(),
		"system admin should not receive Safety DB working menu privileges"
	);

	let (status, value) = request_json(
		&app,
		"GET",
		&admin_cookie,
		format!("/api/admin/permission-profiles/{ROLE_SPONSOR_ADMIN_CRO}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["profile_id"], ROLE_SPONSOR_ADMIN_CRO);

	let (status, _value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		format!("/api/admin/permission-profiles/{ROLE_SPONSOR_ADMIN_CRO}"),
		Some(json!({ "data": { "name": "Should Not Change" } })),
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
	let profile_id = format!("qa_desc_{}", Uuid::new_v4().simple());

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"profile_id": profile_id,
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
	assert_eq!(value["profile_id"], profile_id);
	assert_eq!(value["name"], profile_id);
	assert_eq!(value["description"], "Role created with description only");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_role_admin_api_allows_new_role_without_privileges() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let profile_id = format!("qa_empty_{}", Uuid::new_v4().simple());

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"profile_id": profile_id,
				"name": "QA empty privilege role",
				"description": "Created before privileges are assigned",
				"privileges": []
			}
		})),
	)
	.await?;

	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	assert_eq!(value["profile_id"], profile_id);
	assert_eq!(value["privileges"].as_array().map(Vec::len), Some(0));
	assert_eq!(value["can_view"].as_bool(), Some(false));
	assert_eq!(value["can_admin"].as_bool(), Some(false));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_role_admin_api_preserves_description_equal_to_name() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let profile_id = format!("qa_same_desc_{}", Uuid::new_v4().simple());
	let role_name = "QA Same Description Role";

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"profile_id": profile_id,
				"name": role_name,
				"description": role_name,
				"privileges": []
			}
		})),
	)
	.await?;

	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	assert_eq!(value["profile_id"], profile_id);
	assert_eq!(value["name"], role_name);
	assert_eq!(value["description"], role_name);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_role_admin_api_rejects_duplicate_role_name_in_same_org() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let first_profile_id = format!("qa_dup_a_{}", Uuid::new_v4().simple());
	let second_profile_id = format!("qa_dup_b_{}", Uuid::new_v4().simple());

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"profile_id": first_profile_id,
				"name": "Duplicate Role",
				"description": "First duplicate role",
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"profile_id": second_profile_id,
				"name": " duplicate role ",
				"description": "Second duplicate role",
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert_eq!(value["error"]["message"], "SERVICE_ERROR");
	assert_eq!(
		value["error"]["data"]["detail"],
		"role name already exists in this organization"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_role_admin_api_rejects_rename_to_duplicate_role_name_in_same_org(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let first_profile_id = format!("qa_dup_rename_a_{}", Uuid::new_v4().simple());
	let second_profile_id = format!("qa_dup_rename_b_{}", Uuid::new_v4().simple());

	for (profile_id, name) in [
		(first_profile_id.as_str(), "Original Role"),
		(second_profile_id.as_str(), "Other Role"),
	] {
		let (status, value) = request_json(
			&app,
			"POST",
			&admin_cookie,
			"/api/admin/permission-profiles".to_string(),
			Some(json!({
				"data": {
					"profile_id": profile_id,
					"name": name,
					"privileges": []
				}
			})),
		)
		.await?;
		assert_eq!(status, StatusCode::CREATED, "{value:?}");
	}

	let (status, value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		format!("/api/admin/permission-profiles/{second_profile_id}"),
		Some(json!({
			"data": {
				"name": " original role "
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert_eq!(value["error"]["message"], "SERVICE_ERROR");
	assert_eq!(
		value["error"]["data"]["detail"],
		"role name already exists in this organization"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_role_admin_api_rejects_overlong_name_and_description() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let overlong_name = "R".repeat(129);
	let overlong_description = "D".repeat(513);

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"profile_id": format!("qa_long_name_{}", Uuid::new_v4().simple()),
				"name": overlong_name,
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert_eq!(
		value["error"]["data"]["detail"],
		"role name must be 128 characters or fewer"
	);

	let profile_id = format!("qa_long_desc_{}", Uuid::new_v4().simple());
	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"profile_id": profile_id,
				"name": "Description Limit Role",
				"description": overlong_description,
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert_eq!(
		value["error"]["data"]["detail"],
		"role description must be 512 characters or fewer"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_role_admin_api_rejects_overlong_name_and_description_on_update(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let profile_id = format!("qa_update_limits_{}", Uuid::new_v4().simple());

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"profile_id": profile_id,
				"name": "Update Limit Role",
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");

	for (payload, expected_detail) in [
		(
			json!({ "data": { "name": "R".repeat(129) } }),
			"role name must be 128 characters or fewer",
		),
		(
			json!({ "data": { "description": "D".repeat(513) } }),
			"role description must be 512 characters or fewer",
		),
	] {
		let (status, value) = request_json(
			&app,
			"PUT",
			&admin_cookie,
			format!("/api/admin/permission-profiles/{profile_id}"),
			Some(payload),
		)
		.await?;
		assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
		assert_eq!(value["error"]["data"]["detail"], expected_detail);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_role_admin_api_does_not_fallback_to_old_boolean_privileges(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let profile_id = format!("qa_old_bool_{}", Uuid::new_v4().simple());

	mm.dbx()
		.execute(
			sqlx::query(
				r#"
				INSERT INTO permission_profiles
					(organization_id, profile_id, name, description, can_view, can_review,
					 can_lock, can_admin, privileges_json, active, built_in,
					 editable, sponsor_admin_capable)
				VALUES ($1, $2, $3, 'old boolean row', true, true, true, true,
				        '[]'::jsonb, true, false, true, true)
				"#,
			)
			.bind(seed.org_id)
			.bind(&profile_id)
			.bind(format!("Old Boolean {profile_id}")),
		)
		.await?;

	let app = web_server::app(mm);
	let (status, value) = request_json(
		&app,
		"GET",
		&admin_cookie,
		format!("/api/admin/permission-profiles/{profile_id}"),
		None,
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["privileges"].as_array().map(Vec::len), Some(0));
	assert_eq!(value["can_view"].as_bool(), Some(false));
	assert_eq!(value["can_review"].as_bool(), Some(false));
	assert_eq!(value["can_lock"].as_bool(), Some(false));
	assert_eq!(value["can_admin"].as_bool(), Some(false));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_role_admin_api_persists_privilege_matrix_menu_keys() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let profile_id = format!("qa_matrix_{}", Uuid::new_v4().simple());

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"profile_id": profile_id,
				"name": "QA matrix role",
				"description": "Created before privilege matrix toggles",
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");

	let matrix_privileges = json!([
		{
			"menu_key": "home_notice",
			"can_read": true,
			"can_edit": true,
			"can_review": false,
			"can_lock": false
		},
		{
			"menu_key": "home_workflow",
			"can_read": true,
			"can_edit": false,
			"can_review": true,
			"can_lock": false
		},
		{
			"menu_key": "monitoring",
			"can_read": true,
			"can_edit": false,
			"can_review": false,
			"can_lock": true
		},
		{
			"menu_key": "sync",
			"can_read": true,
			"can_edit": true,
			"can_review": true,
			"can_lock": false
		},
		{
			"menu_key": "sync_mapping",
			"can_read": true,
			"can_edit": false,
			"can_review": true,
			"can_lock": true
		},
		{
			"menu_key": "report_due_mail",
			"can_read": true,
			"can_edit": true,
			"can_review": false,
			"can_lock": false
		}
	]);

	let (status, value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		format!("/api/admin/permission-profiles/{profile_id}"),
		Some(json!({ "data": { "privileges": matrix_privileges } })),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{value:?}");

	let (status, value) = request_json(
		&app,
		"GET",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let roles = value.as_array().ok_or("roles response should be array")?;
	let persisted_role = roles
		.iter()
		.find(|role| role["profile_id"] == profile_id)
		.ok_or("missing persisted matrix role")?;
	let privileges = persisted_role["privileges"]
		.as_array()
		.ok_or("persisted role privileges should be an array")?;
	for (menu_key, can_read, can_edit, can_review, can_lock) in [
		("home_notice", true, true, false, false),
		("home_workflow", true, false, true, false),
		("monitoring", true, false, false, true),
		("sync", true, true, true, false),
		("sync_mapping", true, false, true, true),
		("report_due_mail", true, true, false, false),
	] {
		let row = privileges
			.iter()
			.find(|row| row["menu_key"] == menu_key)
			.ok_or_else(|| format!("missing persisted privilege for {menu_key}"))?;
		assert_eq!(row["can_read"].as_bool(), Some(can_read), "{menu_key}");
		assert_eq!(row["can_edit"].as_bool(), Some(can_edit), "{menu_key}");
		assert_eq!(row["can_review"].as_bool(), Some(can_review), "{menu_key}");
		assert_eq!(row["can_lock"].as_bool(), Some(can_lock), "{menu_key}");
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_role_privilege_matrix_update_grants_effective_case_access(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let profile_id = format!("qa_effective_{}", Uuid::new_v4().simple());

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"profile_id": profile_id,
				"name": "QA effective role",
				"description": "Starts without effective case permissions",
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");

	let custom_user = insert_user(
		&mm,
		seed.org_id,
		&profile_id,
		system_user_id(),
		Some("custompwd"),
	)
	.await?;
	let custom_token =
		generate_web_token(&custom_user.email, custom_user.token_salt)?;
	let custom_cookie = cookie_header(&custom_token.to_string());

	let (status, _value) =
		request_json(&app, "GET", &custom_cookie, "/api/cases".to_string(), None)
			.await?;
	assert_eq!(status, StatusCode::FORBIDDEN);

	let (status, value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		format!("/api/admin/permission-profiles/{profile_id}"),
		Some(json!({
			"data": {
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
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let (status, value) =
		request_json(&app, "GET", &custom_cookie, "/api/cases".to_string(), None)
			.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_matrix_privileges_grant_effective_case_permissions() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let read_profile_id = format!("qacmr_{}", Uuid::new_v4().simple());
	let edit_profile_id = format!("qacme_{}", Uuid::new_v4().simple());
	let review_profile_id = format!("qacmv_{}", Uuid::new_v4().simple());
	let lock_profile_id = format!("qacml_{}", Uuid::new_v4().simple());

	create_empty_custom_role(&app, &admin_cookie, &read_profile_id).await?;
	create_empty_custom_role(&app, &admin_cookie, &edit_profile_id).await?;
	create_empty_custom_role(&app, &admin_cookie, &review_profile_id).await?;
	create_empty_custom_role(&app, &admin_cookie, &lock_profile_id).await?;
	let (read_user_id, read_cookie) =
		custom_role_user(&mm, seed.org_id, &read_profile_id).await?;
	let (edit_user_id, edit_cookie) =
		custom_role_user(&mm, seed.org_id, &edit_profile_id).await?;
	let (review_user_id, review_cookie) =
		custom_role_user(&mm, seed.org_id, &review_profile_id).await?;
	let (lock_user_id, lock_cookie) =
		custom_role_user(&mm, seed.org_id, &lock_profile_id).await?;
	let case_id = create_case(
		&app,
		&admin_cookie,
		&format!("CASE-MATRIX-SEED-{}", Uuid::new_v4().simple()),
		None,
	)
	.await?;
	create_message_header(&app, &admin_cookie, case_id, "CASE-MATRIX-SENDER")
		.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		read_user_id,
		json!({ "access_sender_ids": ["CASE-MATRIX-SENDER"] }),
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		edit_user_id,
		json!({ "access_sender_ids": ["CASE-MATRIX-SENDER"] }),
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		review_user_id,
		json!({ "access_sender_ids": ["CASE-MATRIX-SENDER"] }),
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		lock_user_id,
		json!({ "access_sender_ids": ["CASE-MATRIX-SENDER"] }),
	)
	.await?;

	assert_get_status(&app, &read_cookie, "/api/cases", StatusCode::FORBIDDEN)
		.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&read_profile_id,
		json!([
			{
				"menu_key": "case",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_get_status(&app, &read_cookie, "/api/cases", StatusCode::OK).await?;
	assert_get_status(
		&app,
		&read_cookie,
		&format!("/api/cases/{case_id}"),
		StatusCode::OK,
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"POST",
		&read_cookie,
		"/api/cases".to_string(),
		Some(json!({
			"data": {
				"safety_report_id": format!("CASE-MATRIX-{}", Uuid::new_v4().simple()),
				"status": "draft"
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"read-only case role should not create cases: {value:?}"
	);

	let (status, value) = request_json(
		&app,
		"PUT",
		&read_cookie,
		format!("/api/cases/{case_id}"),
		Some(json!({
			"data": {
				"report_year": "2026"
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"read-only case role should not update cases: {value:?}"
	);

	update_role_privileges(
		&app,
		&admin_cookie,
		&edit_profile_id,
		json!([
			{
				"menu_key": "case",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert!(
		has_permission(&edit_profile_id, CASE_UPDATE),
		"edit role should grant CASE_UPDATE"
	);
	assert!(
		!has_permission(&edit_profile_id, CASE_APPROVE),
		"edit role should not grant CASE_APPROVE"
	);

	let (status, value) = request_json(
		&app,
		"POST",
		&edit_cookie,
		"/api/cases".to_string(),
		Some(json!({
			"data": {
				"safety_report_id": format!("CASE-MATRIX-{}", Uuid::new_v4().simple()),
				"status": "draft"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");

	let (status, value) = request_json(
		&app,
		"PUT",
		&edit_cookie,
		format!("/api/cases/{case_id}"),
		Some(json!({
			"data": {
				"report_year": "2026"
			}
		})),
	)
	.await?;
	assert_ne!(
		status,
		StatusCode::FORBIDDEN,
		"case.can_edit should pass PUT /api/cases/{{id}} CASE_UPDATE gate: {value:?}"
	);

	let (status, value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		"/api/admin/settings".to_string(),
		Some(json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"description": "Default authoring state",
							"due_days": 0,
							"allowed_roles": [review_profile_id, lock_profile_id]
						}
					]
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	update_role_privileges(
		&app,
		&admin_cookie,
		&review_profile_id,
		json!([
			{
				"menu_key": "case",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_workflow_assign_status(
		&app,
		&review_cookie,
		case_id,
		&review_profile_id,
		StatusCode::FORBIDDEN,
	)
	.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&review_profile_id,
		json!([
			{
				"menu_key": "case",
				"can_read": true,
				"can_edit": false,
				"can_review": true,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert!(
		has_permission(&review_profile_id, CASE_UPDATE),
		"review role should grant CASE_UPDATE"
	);
	// CASE_APPROVE has no current web route enforcement point; keep this as a
	// mapping assertion until an approve-specific case endpoint exists.
	assert!(
		has_permission(&review_profile_id, CASE_APPROVE),
		"review role should grant CASE_APPROVE"
	);
	assert_workflow_assign_status(
		&app,
		&review_cookie,
		case_id,
		&review_profile_id,
		StatusCode::OK,
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"POST",
		&review_cookie,
		"/api/cases".to_string(),
		Some(json!({
			"data": {
				"safety_report_id": format!("CASE-MATRIX-{}", Uuid::new_v4().simple()),
				"status": "draft"
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"review alone should not grant case create: {value:?}"
	);

	update_role_privileges(
		&app,
		&admin_cookie,
		&lock_profile_id,
		json!([
			{
				"menu_key": "case",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_workflow_assign_status(
		&app,
		&lock_cookie,
		case_id,
		&lock_profile_id,
		StatusCode::FORBIDDEN,
	)
	.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&lock_profile_id,
		json!([
			{
				"menu_key": "case",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": true
			}
		]),
	)
	.await?;
	assert!(
		has_permission(&lock_profile_id, CASE_UPDATE),
		"lock role should grant CASE_UPDATE"
	);
	// CASE_APPROVE has no current web route enforcement point; keep this as a
	// mapping assertion until an approve-specific case endpoint exists.
	assert!(
		has_permission(&lock_profile_id, CASE_APPROVE),
		"lock role should grant CASE_APPROVE"
	);
	assert_workflow_assign_status(
		&app,
		&lock_cookie,
		case_id,
		&lock_profile_id,
		StatusCode::OK,
	)
	.await?;

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_info_matrix_privileges_grant_effective_presave_permissions(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let profile_id = format!("qa_info_matrix_{}", Uuid::new_v4().simple());

	create_empty_custom_role(&app, &admin_cookie, &profile_id).await?;
	let (custom_user_id, custom_cookie) =
		custom_role_user(&mm, seed.org_id, &profile_id).await?;
	let template_id = create_sender_presave_template(
		&app,
		&admin_cookie,
		&format!("Info Matrix Seed {}", Uuid::new_v4().simple()),
		"INFO-MATRIX-SEED",
	)
	.await?;
	let editable_template_id = create_sender_presave_template(
		&app,
		&admin_cookie,
		&format!("Info Matrix Editable {}", Uuid::new_v4().simple()),
		"INFO-MATRIX-EDITABLE",
	)
	.await?;
	let deletable_template_id = create_sender_presave_template(
		&app,
		&admin_cookie,
		&format!("Info Matrix Deletable {}", Uuid::new_v4().simple()),
		"INFO-MATRIX-DELETABLE",
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		custom_user_id,
		json!({
			"access_sender_ids": [
				"INFO-MATRIX-SEED",
				"INFO-MATRIX-EDITABLE",
				"INFO-MATRIX-DELETABLE",
				"INFO-MATRIX-EDIT"
			]
		}),
	)
	.await?;

	assert_get_status(
		&app,
		&custom_cookie,
		"/api/presave-templates",
		StatusCode::FORBIDDEN,
	)
	.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "info",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_get_status(
		&app,
		&custom_cookie,
		"/api/presave-templates",
		StatusCode::OK,
	)
	.await?;
	assert_get_status(
		&app,
		&custom_cookie,
		&format!("/api/presave-templates/{template_id}"),
		StatusCode::OK,
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/presave-templates".to_string(),
		Some(json!({
			"data": {
				"entity_type": "sender",
				"name": "Info Matrix Sender",
				"description": "Should require info edit",
				"data": {
					"senderType": "2",
					"senderIdentifier": "INFO-MATRIX",
					"senderOrganization": "Info Matrix Sender",
					"linkedOrganizationName": "Info Matrix Sender",
					"linkedOrganizationType": "client"
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) = request_json(
		&app,
		"PATCH",
		&custom_cookie,
		format!("/api/presave-templates/{editable_template_id}"),
		Some(json!({
			"data": {
				"name": "Info Matrix Readonly Patch",
				"description": "Read-only info should not update templates"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) = request_json(
		&app,
		"DELETE",
		&custom_cookie,
		format!("/api/presave-templates/{deletable_template_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "info",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/presave-templates".to_string(),
		Some(json!({
			"data": {
				"entity_type": "sender",
				"name": format!("Info Matrix Sender {}", Uuid::new_v4().simple()),
				"description": "Info edit should allow creation",
				"data": {
					"senderType": "2",
					"senderIdentifier": "INFO-MATRIX-EDIT",
					"senderOrganization": "Info Matrix Sender Edit",
					"linkedOrganizationName": "Info Matrix Sender Edit",
					"linkedOrganizationType": "client"
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let created_template_id = extract_id(&value)?;

	let (status, value) = request_json(
		&app,
		"PATCH",
		&custom_cookie,
		format!("/api/presave-templates/{editable_template_id}"),
		Some(json!({
			"data": {
				"name": "Info Matrix Editable Updated",
				"description": "Info edit should allow updates",
				"data": {
					"senderType": "2",
					"senderIdentifier": "INFO-MATRIX-EDITABLE",
					"senderOrganization": "Info Matrix Editable Updated",
					"linkedOrganizationName": "Info Matrix Editable Updated",
					"linkedOrganizationType": "client"
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(
		value["data"]["name"].as_str(),
		Some("Info Matrix Editable Updated"),
		"{value:?}"
	);

	let (status, value) = request_json(
		&app,
		"DELETE",
		&custom_cookie,
		format!("/api/presave-templates/{deletable_template_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::NO_CONTENT, "{value:?}");
	assert_get_status(
		&app,
		&custom_cookie,
		&format!("/api/presave-templates/{created_template_id}"),
		StatusCode::OK,
	)
	.await?;

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_data_matrix_privileges_grant_effective_terminology_permissions(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let profile_id = format!("qa_data_matrix_{}", Uuid::new_v4().simple());

	create_empty_custom_role(&app, &admin_cookie, &profile_id).await?;
	let (_custom_user_id, custom_cookie) =
		custom_role_user(&mm, seed.org_id, &profile_id).await?;

	assert_get_status(
		&app,
		&custom_cookie,
		"/api/terminology/countries",
		StatusCode::FORBIDDEN,
	)
	.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "data",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_get_status(
		&app,
		&custom_cookie,
		"/api/terminology/countries",
		StatusCode::OK,
	)
	.await?;
	assert!(
		!has_permission(&profile_id, TERMINOLOGY_IMPORT),
		"read-only DATA must not grant terminology import permission"
	);
	assert!(
		!has_permission(&profile_id, TERMINOLOGY_APPROVE),
		"read-only DATA must not grant terminology approve permission"
	);

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/import/meddra?version=27.1&language=en")
		.header("cookie", custom_cookie.clone())
		.header("content-type", "multipart/form-data; boundary=----boundary")
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(
		res.status(),
		StatusCode::FORBIDDEN,
		"read-only DATA must not import terminology"
	);

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/terminology/releases/meddra/TEST/approve".to_string(),
		None,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"read-only DATA must not approve terminology releases: {value:?}"
	);

	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "data",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert!(
		has_permission(&profile_id, TERMINOLOGY_IMPORT),
		"editable DATA must grant terminology import permission"
	);
	assert!(
		has_permission(&profile_id, TERMINOLOGY_APPROVE),
		"editable DATA must grant terminology approve permission"
	);

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/import/meddra?version=27.1&language=en")
		.header("cookie", custom_cookie.clone())
		.header("content-type", "multipart/form-data; boundary=----boundary")
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_ne!(
		res.status(),
		StatusCode::FORBIDDEN,
		"editable DATA should pass terminology import permission check"
	);

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/terminology/releases/meddra/TEST/approve".to_string(),
		None,
	)
	.await?;
	assert_ne!(
		status,
		StatusCode::FORBIDDEN,
		"editable DATA should pass terminology approve permission check: {value:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_export_submission_matrix_privileges_grant_effective_xml_export_permission(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let no_export_profile_name =
		format!("qa_export_none_{}", Uuid::new_v4().simple());
	let read_profile_name = format!("qa_export_read_{}", Uuid::new_v4().simple());
	let edit_profile_name = format!("qa_export_edit_{}", Uuid::new_v4().simple());

	let no_export_profile_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&no_export_profile_name,
	)
	.await?;
	let read_profile_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&read_profile_name,
	)
	.await?;
	let edit_profile_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&edit_profile_name,
	)
	.await?;
	let (_no_export_user_id, no_export_cookie) =
		custom_role_user(&mm, seed.org_id, &no_export_profile_id).await?;
	let (_read_user_id, read_cookie) =
		custom_role_user(&mm, seed.org_id, &read_profile_id).await?;
	let (_edit_user_id, edit_cookie) =
		custom_role_user(&mm, seed.org_id, &edit_profile_id).await?;

	assert!(
		!has_permission(&no_export_profile_id, XML_EXPORT),
		"empty custom role must not grant XML_EXPORT"
	);
	assert!(
		!has_permission(&no_export_profile_id, XML_EXPORT_READ),
		"empty custom role must not grant XML_EXPORT_READ"
	);
	assert_get_status(
		&app,
		&no_export_cookie,
		"/api/exports/history",
		StatusCode::FORBIDDEN,
	)
	.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&read_profile_id,
		json!([
			{
				"menu_key": "export_submission",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert!(
		has_permission(&read_profile_id, XML_EXPORT_READ),
		"export_submission.can_read must grant XML_EXPORT_READ"
	);
	assert!(
		!has_permission(&read_profile_id, XML_EXPORT),
		"export_submission.can_read must not grant XML_EXPORT"
	);
	assert_get_not_status(
		&app,
		&read_cookie,
		"/api/exports/history",
		StatusCode::FORBIDDEN,
	)
	.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&edit_profile_id,
		json!([
			{
				"menu_key": "export_submission",
				"can_read": false,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert!(
		has_permission(&edit_profile_id, XML_EXPORT),
		"export_submission.can_edit must independently grant XML_EXPORT"
	);
	assert!(
		!has_permission(&edit_profile_id, XML_EXPORT_READ),
		"export_submission.can_edit must not grant history read without can_read"
	);
	assert_get_status(
		&app,
		&edit_cookie,
		"/api/exports/history",
		StatusCode::FORBIDDEN,
	)
	.await?;

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_import_matrix_privileges_split_files_edit_from_history_read(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let read_profile_name = format!("qa_import_read_{}", Uuid::new_v4().simple());
	let edit_profile_name = format!("qa_import_edit_{}", Uuid::new_v4().simple());

	let read_profile_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&read_profile_name,
	)
	.await?;
	let edit_profile_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&edit_profile_name,
	)
	.await?;
	let (_read_user_id, read_cookie) =
		custom_role_user(&mm, seed.org_id, &read_profile_id).await?;
	let (_edit_user_id, edit_cookie) =
		custom_role_user(&mm, seed.org_id, &edit_profile_id).await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&read_profile_id,
		json!([
			{
				"menu_key": "import",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert!(has_permission(&read_profile_id, XML_IMPORT_READ));
	assert!(!has_permission(&read_profile_id, XML_IMPORT));
	assert_get_status(
		&app,
		&read_cookie,
		"/api/import/xml/history",
		StatusCode::OK,
	)
	.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&edit_profile_id,
		json!([
			{
				"menu_key": "import",
				"can_read": false,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert!(has_permission(&edit_profile_id, XML_IMPORT));
	assert!(!has_permission(&edit_profile_id, XML_IMPORT_READ));
	assert_get_status(
		&app,
		&edit_cookie,
		"/api/import/xml/history",
		StatusCode::FORBIDDEN,
	)
	.await?;

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
	let profile_id = format!("qa_role_{}", Uuid::new_v4().simple());

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"profile_id": profile_id,
				"name": "QA Role",
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
	assert_eq!(value["profile_id"], profile_id);
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
		format!("/api/admin/permission-profiles/{profile_id}"),
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
async fn test_permission_profile_admin_privilege_does_not_grant_permission_profile(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let profile_id = format!("custom_admin_{}", Uuid::new_v4().simple());

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"profile_id": profile_id,
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
		&profile_id,
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
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let next_role = format!("custom_admin_child_{}", Uuid::new_v4().simple());
	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"profile_id": next_role,
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
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_settings_admin_matrix_does_not_grant_admin_route_access() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let profile_id = format!("qa_settings_admin_{}", Uuid::new_v4().simple());

	create_empty_custom_role(&app, &admin_cookie, &profile_id).await?;
	let (_custom_user_id, custom_cookie) =
		custom_role_user(&mm, seed.org_id, &profile_id).await?;

	assert_get_status(&app, &custom_cookie, "/api/users", StatusCode::FORBIDDEN)
		.await?;
	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/users".to_string(),
		Some(json!({
			"data": {
				"organization_id": seed.org_id,
				"email": format!("settings-admin-empty-{}@example.com", Uuid::new_v4()),
				"role": "viewer"
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"empty settings role must not create users: {value:?}"
	);

	let value = update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "settings",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_eq!(
		value["sponsor_admin_capable"].as_bool(),
		Some(false),
		"settings.can_read alone must not make the role Safety DB admin capable: {value:?}"
	);
	assert!(
		!has_permission(&profile_id, CASE_CREATE),
		"settings.can_read alone must not grant raw CASE_CREATE permission"
	);
	assert!(
		!has_permission(&profile_id, USER_CREATE),
		"settings.can_read alone must not grant raw USER_CREATE permission"
	);
	assert_get_status(&app, &custom_cookie, "/api/users", StatusCode::FORBIDDEN)
		.await?;
	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/cases".to_string(),
		Some(json!({
			"data": {
				"safety_report_id": format!("SETTINGS-READ-{}", Uuid::new_v4().simple()),
				"status": "draft"
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"settings.can_read alone must not create cases via raw permissions: {value:?}"
	);
	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/users".to_string(),
		Some(json!({
			"data": {
				"organization_id": seed.org_id,
				"email": format!("settings-admin-read-{}@example.com", Uuid::new_v4()),
				"role": "viewer"
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"settings.can_read alone must not create users: {value:?}"
	);

	let value = update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "settings",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_eq!(
		value["sponsor_admin_capable"].as_bool(),
		Some(true),
			"settings.can_edit may expose legacy metadata but must not grant permission profile access: {value:?}"
	);
	assert_get_status(&app, &custom_cookie, "/api/users", StatusCode::FORBIDDEN)
		.await?;
	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/users".to_string(),
		Some(json!({
			"data": {
				"organization_id": seed.org_id,
				"email": format!("settings-admin-edit-{}@example.com", Uuid::new_v4()),
				"role": "viewer"
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"permission profiles must not create users through POST /api/users: {value:?}"
	);

	Ok(())
}
