#![allow(unused_imports, dead_code)]

use crate::common::{
	cookie_header, init_test_mm, insert_user, seed_company_org_with_users,
	seed_org_with_users, system_user_id, Result, TEST_CUSTOM_MANAGER_ROLE,
};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use lib_auth::token::generate_web_token;
use lib_core::ctx::{
	ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN,
};
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::ModelManager;
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

fn extract_id(value: &Value) -> Result<Uuid> {
	let data = &value["data"];
	let id = data["id"]
		.as_str()
		.or_else(|| data["rows"]["sender"]["id"].as_str())
		.or_else(|| data["rows"]["product"]["id"].as_str())
		.or_else(|| data["rows"]["study"]["id"].as_str())
		.ok_or("missing presave id")?;
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
			"data": { "rows": {
				"sender": {
					"senderType": "2",
					"organizationName": name,
					"email": format!("{sender_identifier}@example.test")
				},
				"gateways": [{
					"sequenceNumber": 1,
					"gatewayAuthority": "fda",
					"senderIdentifier": sender_identifier,
					"isDefaultForAuthority": true
				}],
				"responsiblePersons": [{
					"sequenceNumber": 1,
					"personGivenName": "Safety"
				}]
			} }
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
	Ok(id)
}

async fn create_product_presave(
	app: &Router,
	cookie: &str,
	sender_presave_id: Uuid,
	medicinal_product: &str,
) -> Result<Uuid> {
	let (status, value) = request_json(
		app,
		"POST",
		cookie,
		"/api/presaves/products".to_string(),
		Some(json!({
			"data": { "rows": {
				"product": {
					"senderPresaveId": sender_presave_id,
					"productId": format!("SCOPE-PRODUCT-{}", Uuid::new_v4()),
					"medicinalProduct": medicinal_product
				},
				"activeSubstances": []
			} }
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	extract_id(&value)
}

async fn create_study_presave(
	app: &Router,
	cookie: &str,
	product_presave_id: Uuid,
	study_name: &str,
) -> Result<Uuid> {
	let (status, value) = request_json(
		app,
		"POST",
		cookie,
		"/api/presaves/studies".to_string(),
		Some(json!({
			"data": { "rows": {
				"study": {
					"productPresaveId": product_presave_id,
					"studyName": study_name,
					"sponsorStudyNumber": format!("SCOPE-STUDY-{}", Uuid::new_v4()),
					"studyTypeReaction": "1"
				},
				"products": [],
				"reporters": [],
				"registrationNumbers": [],
				"fdaCrossReportedInds": []
			} }
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	extract_id(&value)
}

async fn link_case_presave_sources(
	mm: &ModelManager,
	case_id: Uuid,
	user_id: Uuid,
	org_id: Uuid,
	sender_id: Option<Uuid>,
	product_id: Option<Uuid>,
	study_id: Option<Uuid>,
) -> Result<()> {
	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	set_full_context_dbx(dbx, user_id, org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	if let Some(sender_id) = sender_id {
		dbx.execute(
			sqlx::query(
				"UPDATE sender_information
				 SET source_sender_presave_id = $1
				 WHERE case_id = $2",
			)
			.bind(sender_id)
			.bind(case_id),
		)
		.await?;
	}
	if let Some(product_id) = product_id {
		dbx.execute(
			sqlx::query(
				"UPDATE drug_information
				 SET source_product_presave_id = $1
				 WHERE case_id = $2",
			)
			.bind(product_id)
			.bind(case_id),
		)
		.await?;
	}
	if let Some(study_id) = study_id {
		dbx.execute(
			sqlx::query(
				"UPDATE study_information
				 SET source_study_presave_id = $1
				 WHERE case_id = $2",
			)
			.bind(study_id)
			.bind(case_id),
		)
		.await?;
	}
	dbx.commit_txn().await?;
	Ok(())
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
	_brand_name: &str,
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
	let app = web_server::app(mm.clone());
	let sender_a =
		create_sender_presave(&app, &admin_cookie, "Sender Org A", "SEND-A").await?;
	let sender_b =
		create_sender_presave(&app, &admin_cookie, "Sender Org B", "SEND-B").await?;

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
	link_case_presave_sources(
		&mm,
		case_a,
		seed.admin.id,
		seed.org_id,
		Some(sender_a),
		None,
		None,
	)
	.await?;
	link_case_presave_sources(
		&mm,
		case_b,
		seed.admin.id,
		seed.org_id,
		Some(sender_b),
		None,
		None,
	)
	.await?;
	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	set_full_context_dbx(dbx, seed.admin.id, seed.org_id, ROLE_SPONSOR_ADMIN_CRO)
		.await?;
	dbx.execute(
		sqlx::query(
			"UPDATE cases SET created_at = NOW() + INTERVAL '1 hour' WHERE id = $1",
		)
		.bind(case_b),
	)
	.await?;
	dbx.commit_txn().await?;

	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": [sender_a.to_string()] }),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		"/api/cases?list_options%5Blimit%5D=1&list_options%5Boffset%5D=0"
			.to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let cases = value["data"].as_array().ok_or("missing cases array")?;
	assert_eq!(cases.len(), 1, "{value:?}");
	assert_eq!(cases[0]["id"], case_a.to_string());
	assert_eq!(cases[0]["can_act_on_workflow"], false);
	assert_eq!(cases[0]["workflow_block_reason"], "workflow_not_enabled");
	assert!(cases[0]["case_write_block_reason"].is_null());

	let (status, list_view) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		"/api/cases/list-view?list_options%5Blimit%5D=1&list_options%5Boffset%5D=0&list_options%5Border_bys%5D=%21created_at".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{list_view:?}");
	let rows = list_view["data"]["items"]
		.as_array()
		.ok_or("missing list-view items")?;
	assert_eq!(rows.len(), 1, "{list_view:?}");
	assert_eq!(rows[0]["caseId"], case_a.to_string());
	assert_eq!(rows[0]["no"], 1);

	let (status, query) = request_json(
		&app,
		"POST",
		&viewer_cookie,
		"/api/cases/query".to_string(),
		Some(json!({
			"conditions": [],
			"resultPages": ["CASE"],
			"limit": 1,
			"offset": 0
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{query:?}");
	assert_eq!(query["data"]["total"], 1);
	assert_eq!(query["data"]["caseIds"][0], case_a.to_string());
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sender_scope_follows_product_relationship() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm.clone());
	let sender_id = create_sender_presave(
		&app,
		&admin_cookie,
		"Product-linked Sender",
		"SEND-PRODUCT-LINK",
	)
	.await?;
	let product_id = create_product_presave(
		&app,
		&admin_cookie,
		sender_id,
		"Product-linked Brand",
	)
	.await?;
	let case_id = create_case(
		&app,
		&admin_cookie,
		&format!("SR-PRODUCT-SENDER-{}", Uuid::new_v4()),
		Some("PRODUCT-LINK"),
	)
	.await?;
	create_drug_with_brand(&app, &admin_cookie, case_id, "Product-linked Brand")
		.await?;
	link_case_presave_sources(
		&mm,
		case_id,
		seed.admin.id,
		seed.org_id,
		None,
		Some(product_id),
		None,
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": [sender_id.to_string()] }),
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
async fn test_user_scope_assignment_rejects_cross_sender_product() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let sender_a =
		create_sender_presave(&app, &admin_cookie, "Scope Sender A", "SCOPE-A")
			.await?;
	let sender_b =
		create_sender_presave(&app, &admin_cookie, "Scope Sender B", "SCOPE-B")
			.await?;
	let product_b =
		create_product_presave(&app, &admin_cookie, sender_b, "Scope Product B")
			.await?;
	let study_b =
		create_study_presave(&app, &admin_cookie, product_b, "SCOPE-STUDY-B")
			.await?;

	let (status, value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		format!("/api/users/{}", seed.viewer.id),
		Some(json!({
			"data": {
				"access_sender_ids": [sender_a.to_string()],
				"access_product_ids": [product_b.to_string()]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert!(value.to_string().contains("access_product_ids"));

	let (status, value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		format!("/api/users/{}", seed.viewer.id),
		Some(json!({
			"data": {
				"access_sender_ids": [sender_a.to_string()],
				"access_study_ids": [study_b.to_string()]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert!(value.to_string().contains("access_study_ids"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_user_scope_empty_array_clears_existing_assignment() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let sender_id =
		create_sender_presave(&app, &admin_cookie, "Clear Sender", "CLEAR").await?;

	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": [sender_id.to_string()] }),
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": [] }),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"GET",
		&admin_cookie,
		format!("/api/users/{}", seed.viewer.id),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["data"]["scope"]["assignedSenderIds"], json!([]));
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
	let app = web_server::app(mm.clone());
	let scoped_sender =
		create_sender_presave(&app, &admin_cookie, "Scoped Sender", "SCOPED")
			.await?;
	let case_sender =
		create_sender_presave(&app, &admin_cookie, "Case Sender", "CASE-SENDER")
			.await?;
	let case_id = create_case(
		&app,
		&admin_cookie,
		&format!("SR-SENDER-HEADER-ONLY-{}", Uuid::new_v4()),
		None,
	)
	.await?;
	create_message_header(&app, &admin_cookie, case_id, "MSG-ONLY").await?;
	create_sender_information(&app, &admin_cookie, case_id, "Sender Org B").await?;
	link_case_presave_sources(
		&mm,
		case_id,
		seed.admin.id,
		seed.org_id,
		Some(case_sender),
		None,
		None,
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": [scoped_sender.to_string()] }),
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
async fn test_case_get_allows_case_without_source_when_user_has_sender_scope(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);
	let scoped_sender = create_sender_presave(
		&app,
		&admin_cookie,
		"Scoped Sender",
		"SCOPED-NO-SOURCE",
	)
	.await?;

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
		json!({ "access_sender_ids": [scoped_sender.to_string()] }),
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
	let app = web_server::app(mm.clone());
	let sender_a =
		create_sender_presave(&app, &admin_cookie, "Sender Org A", "SEND-UPD-A")
			.await?;
	let sender_b =
		create_sender_presave(&app, &admin_cookie, "Sender Org B", "SEND-UPD-B")
			.await?;
	let product_a =
		create_product_presave(&app, &admin_cookie, sender_a, "Product A").await?;
	let product_b =
		create_product_presave(&app, &admin_cookie, sender_b, "Product B").await?;
	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	set_full_context_dbx(dbx, seed.admin.id, seed.org_id, ROLE_SPONSOR_ADMIN_CRO)
		.await?;
	let product_b_key = dbx
		.fetch_one(
			sqlx::query_as::<_, (String,)>(
				"SELECT product_id FROM product_presaves WHERE id = $1",
			)
			.bind(product_b),
		)
		.await?
		.0;
	dbx.commit_txn().await?;

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
	link_case_presave_sources(
		&mm,
		case_a,
		seed.admin.id,
		seed.org_id,
		Some(sender_a),
		None,
		None,
	)
	.await?;
	link_case_presave_sources(
		&mm,
		case_b,
		seed.admin.id,
		seed.org_id,
		Some(sender_b),
		None,
		None,
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({
			"access_sender_ids": [sender_a.to_string()],
			"access_product_ids": [product_a.to_string()]
		}),
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

	let (status, value) = request_json(
		&app,
		"PUT",
		&viewer_cookie,
		format!("/api/cases/{case_a}"),
		Some(json!({ "data": { "dg_prd_key": product_b_key } })),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");
	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	set_full_context_dbx(dbx, seed.admin.id, seed.org_id, ROLE_SPONSOR_ADMIN_CRO)
		.await?;
	let stored_product_key = dbx
		.fetch_one(
			sqlx::query_as::<_, (Option<String>,)>(
				"SELECT dg_prd_key FROM cases WHERE id = $1",
			)
			.bind(case_a),
		)
		.await?
		.0;
	dbx.commit_txn().await?;
	assert_eq!(stored_product_key, None);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_editor_can_create_assigned_dg_but_denies_unassigned_product(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	let profile_id = create_empty_custom_role(
		&app,
		&admin_cookie,
		&format!("qa_scoped_dg_editor_{}", Uuid::new_v4().simple()),
	)
	.await?;
	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([{
			"menu_key": "case",
			"can_read": true,
			"can_edit": true,
			"can_review": false,
			"can_lock": false
		}]),
	)
	.await?;
	let (editor_id, editor_cookie) =
		custom_role_user(&mm, seed.org_id, &profile_id).await?;

	let sender_id =
		create_sender_presave(&app, &admin_cookie, "DG Sender", "DG-SCOPE").await?;
	let assigned_product =
		create_product_presave(&app, &admin_cookie, sender_id, "Assigned Drug")
			.await?;
	let unassigned_product =
		create_product_presave(&app, &admin_cookie, sender_id, "Unassigned Drug")
			.await?;

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	set_full_context_dbx(dbx, seed.admin.id, seed.org_id, ROLE_SPONSOR_ADMIN_CRO)
		.await?;
	let assigned_product_key = dbx
		.fetch_one(
			sqlx::query_as::<_, (String,)>(
				"SELECT product_id FROM product_presaves WHERE id = $1",
			)
			.bind(assigned_product),
		)
		.await?
		.0;
	let unassigned_product_key = dbx
		.fetch_one(
			sqlx::query_as::<_, (String,)>(
				"SELECT product_id FROM product_presaves WHERE id = $1",
			)
			.bind(unassigned_product),
		)
		.await?
		.0;
	dbx.commit_txn().await?;

	let assigned_case = create_case(
		&app,
		&admin_cookie,
		&format!("SR-DG-ASSIGNED-{}", Uuid::new_v4()),
		Some(&assigned_product_key),
	)
	.await?;
	let unassigned_case = create_case(
		&app,
		&admin_cookie,
		&format!("SR-DG-UNASSIGNED-{}", Uuid::new_v4()),
		Some(&unassigned_product_key),
	)
	.await?;
	let initially_unscoped_case = create_case(
		&app,
		&admin_cookie,
		&format!("SR-DG-UNSCOPED-{}", Uuid::new_v4()),
		None,
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		editor_id,
		json!({
			"access_sender_ids": [sender_id.to_string()],
			"access_product_ids": [assigned_product.to_string()]
		}),
	)
	.await?;

	let editor_case = create_case(
		&app,
		&editor_cookie,
		&format!("SR-DG-EDITOR-{}", Uuid::new_v4()),
		Some(&assigned_product_key),
	)
	.await?;
	let (status, value) = request_json(
		&app,
		"GET",
		&editor_cookie,
		format!("/api/cases/{editor_case}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let (status, value) = request_json(
		&app,
		"POST",
		&editor_cookie,
		"/api/cases".to_string(),
		Some(json!({
			"data": {
				"safetyReportIdentification": {
					"safetyReportId": format!("SR-DG-DENIED-{}", Uuid::new_v4())
				},
				"status": "draft",
				"dgPrdKey": unassigned_product_key
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let dg_row = |source_product_presave_id: Uuid, medicinal_product: &str| {
		json!({
			"authorities": ["ich"],
			"rows": { "drug": {
				"sourceProductPresaveId": source_product_presave_id,
				"sequenceNumber": 1,
				"drugCharacterization": "1",
				"medicinalProduct": medicinal_product
			} }
		})
	};
	let (status, value) = request_json(
		&app,
		"POST",
		&editor_cookie,
		format!("/api/cases/{assigned_case}/editor/pages/DG/rows"),
		Some(dg_row(assigned_product, "Assigned Drug")),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");

	let (status, value) = request_json(
		&app,
		"POST",
		&editor_cookie,
		format!("/api/cases/{unassigned_case}/editor/pages/DG/rows"),
		Some(dg_row(unassigned_product, "Unassigned Drug")),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) = request_json(
		&app,
		"POST",
		&editor_cookie,
		format!("/api/cases/{initially_unscoped_case}/editor/pages/DG/rows"),
		Some(dg_row(unassigned_product, "Unassigned Drug")),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");
	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	set_full_context_dbx(dbx, seed.admin.id, seed.org_id, ROLE_SPONSOR_ADMIN_CRO)
		.await?;
	let stored_rows = dbx
		.fetch_one(
			sqlx::query_as::<_, (i64,)>(
				"SELECT count(*) FROM drug_information WHERE case_id = $1",
			)
			.bind(initially_unscoped_case),
		)
		.await?
		.0;
	dbx.commit_txn().await?;
	assert_eq!(stored_rows, 0);
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
	let app = web_server::app(mm.clone());
	let sender_id =
		create_sender_presave(&app, &admin_cookie, "Sender Org A", "SEND-SCOPE")
			.await?;
	let product_id =
		create_product_presave(&app, &admin_cookie, sender_id, "Brand Alpha")
			.await?;
	let study_id =
		create_study_presave(&app, &admin_cookie, product_id, "STUDY-ALPHA").await?;

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
	link_case_presave_sources(
		&mm,
		case_id,
		seed.admin.id,
		seed.org_id,
		Some(sender_id),
		Some(product_id),
		Some(study_id),
	)
	.await?;

	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({
			"access_sender_ids": [sender_id.to_string()],
			"access_product_ids": [product_id.to_string()],
			"access_study_ids": [study_id.to_string()]
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
	let app = web_server::app(mm.clone());
	let sender_id = create_sender_presave(
		&app,
		&admin_cookie,
		"Sender Strict Org",
		"SEND-STRICT",
	)
	.await?;
	let product_id =
		create_product_presave(&app, &admin_cookie, sender_id, "Brand Strict")
			.await?;
	let study_id =
		create_study_presave(&app, &admin_cookie, product_id, "STUDY-STRICT")
			.await?;
	let other_product_id =
		create_product_presave(&app, &admin_cookie, sender_id, "Brand Other")
			.await?;

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
	link_case_presave_sources(
		&mm,
		case_id,
		seed.admin.id,
		seed.org_id,
		Some(sender_id),
		Some(product_id),
		Some(study_id),
	)
	.await?;

	// Sender matches; product/study left unset. Unset scope means "allow all",
	// so the case is visible even though it carries product/study values.
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": [sender_id.to_string()] }),
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
			"access_sender_ids": [sender_id.to_string()],
			"access_product_ids": [other_product_id.to_string()]
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
			"access_sender_ids": [sender_id.to_string()],
			"access_product_ids": [product_id.to_string()],
			"access_study_ids": [study_id.to_string()]
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
async fn test_case_get_does_not_treat_e2b_blinded_field_as_user_scope() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm.clone());
	let sender_id =
		create_sender_presave(&app, &admin_cookie, "Sender Org A", "SEND-BLIND")
			.await?;
	let product_id =
		create_product_presave(&app, &admin_cookie, sender_id, "Demo Product")
			.await?;
	let study_id =
		create_study_presave(&app, &admin_cookie, product_id, "STUDY-BLIND").await?;

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
	link_case_presave_sources(
		&mm,
		case_id,
		seed.admin.id,
		seed.org_id,
		Some(sender_id),
		Some(product_id),
		Some(study_id),
	)
	.await?;

	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({
			"access_sender_ids": [sender_id.to_string()],
			"access_product_ids": [product_id.to_string()],
			"access_study_ids": [study_id.to_string()],
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
	assert_eq!(status, StatusCode::OK);
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
	let sender_id =
		create_sender_presave(&app, &admin_cookie, "Sender Hist Org", "SEND-HIST")
			.await?;
	let product_allowed =
		create_product_presave(&app, &admin_cookie, sender_id, "Brand A").await?;
	let product_hidden =
		create_product_presave(&app, &admin_cookie, sender_id, "Brand B").await?;

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
	link_case_presave_sources(
		&mm,
		case_allowed,
		seed.admin.id,
		seed.org_id,
		Some(sender_id),
		Some(product_allowed),
		None,
	)
	.await?;
	link_case_presave_sources(
		&mm,
		case_hidden,
		seed.admin.id,
		seed.org_id,
		Some(sender_id),
		Some(product_hidden),
		None,
	)
	.await?;
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
			"access_sender_ids": [sender_id.to_string()],
			"access_product_ids": [product_allowed.to_string()]
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
	assert!(
		export_items.iter().all(|row| row["exportedAt"].is_string()),
		"exportedAt must be an RFC 3339 string: {export_history:?}"
	);
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

	let sender_id = create_sender_presave(
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
		json!({ "access_sender_ids": [sender_id.to_string()] }),
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
		.find(|row| row["senderIdentifier"] == sender_id.to_string())
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
	assert_eq!(viewer_senders[0]["senderIdentifier"], sender_id.to_string());
	assert_eq!(viewer_senders[0]["caseCount"], 0);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_unset_sender_scope_lists_all_sender_presaves() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let info_user = insert_user(
		&mm,
		seed.org_id,
		TEST_CUSTOM_MANAGER_ROLE,
		system_user_id(),
		Some("viewpwd"),
	)
	.await?;
	let viewer_token = generate_web_token(&info_user.email, info_user.token_salt)?;
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
	let rows = value["data"]
		.as_array()
		.ok_or("missing sender rows")?
		.iter()
		.filter_map(|row| row["rows"]["sender"].as_object())
		.collect::<Vec<_>>();
	assert!(
		rows.iter().any(|row| row["id"] == sender_id.to_string()),
		"unset sender scope must list all sender presaves: {value:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sender_uuid_scope_lists_matching_sender_presave() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let info_user = insert_user(
		&mm,
		seed.org_id,
		TEST_CUSTOM_MANAGER_ROLE,
		system_user_id(),
		Some("viewpwd"),
	)
	.await?;
	let viewer_token = generate_web_token(&info_user.email, info_user.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);

	let allowed_id = create_sender_presave(
		&app,
		&admin_cookie,
		"UUID Scope Sender A",
		"SEND-UUID-A",
	)
	.await?;
	let denied_id = create_sender_presave(
		&app,
		&admin_cookie,
		"UUID Scope Sender B",
		"SEND-UUID-B",
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		info_user.id,
		json!({ "access_sender_ids": [allowed_id.to_string()] }),
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
	let rows = value["data"]
		.as_array()
		.ok_or("missing sender rows")?
		.iter()
		.filter_map(|row| row["rows"]["sender"].as_object())
		.collect::<Vec<_>>();
	assert!(rows.iter().any(|row| row["id"] == allowed_id.to_string()));
	assert!(!rows.iter().any(|row| row["id"] == denied_id.to_string()));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_company_sponsor_admin_can_assign_sender_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_company_org_with_users(&mm, "companypwd", "viewpwd").await?;
	let company_admin = seed.admin;
	let company_token =
		generate_web_token(&company_admin.email, company_admin.token_salt)?;
	let company_cookie = cookie_header(&company_token.to_string());
	let app = web_server::app(mm);
	let sender_id = create_sender_presave(
		&app,
		&company_cookie,
		"Company Sender Scope",
		"COMPANY-SCOPE",
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"PUT",
		&company_cookie,
		format!("/api/users/{}", seed.viewer.id),
		Some(json!({
			"data": {
				"access_sender_ids": [sender_id.to_string()]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_routing_profile_sender_options_follow_role_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm.clone());
	let sender_a =
		create_sender_presave(&app, &admin_cookie, "Sender Org A", "SEND-ROUTE-A")
			.await?;
	let sender_b =
		create_sender_presave(&app, &admin_cookie, "Sender Org B", "SEND-ROUTE-B")
			.await?;

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
	link_case_presave_sources(
		&mm,
		case_a,
		seed.admin.id,
		seed.org_id,
		Some(sender_a),
		None,
		None,
	)
	.await?;
	link_case_presave_sources(
		&mm,
		case_b,
		seed.admin.id,
		seed.org_id,
		Some(sender_b),
		None,
		None,
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": [sender_a.to_string()] }),
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
		.any(|row| row["senderIdentifier"] == sender_a.to_string()));
	assert!(admin_senders
		.iter()
		.any(|row| row["senderIdentifier"] == sender_b.to_string()));

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
	assert_eq!(viewer_senders[0]["senderIdentifier"], sender_a.to_string());

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_active_sender_selection_does_not_filter_case_list() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm.clone());
	let sender_a =
		create_sender_presave(&app, &admin_cookie, "Sender Org A", "SEND-ACTIVE-A")
			.await?;
	let sender_b =
		create_sender_presave(&app, &admin_cookie, "Sender Org B", "SEND-ACTIVE-B")
			.await?;

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
	link_case_presave_sources(
		&mm,
		case_a,
		seed.admin.id,
		seed.org_id,
		Some(sender_a),
		None,
		None,
	)
	.await?;
	link_case_presave_sources(
		&mm,
		case_b,
		seed.admin.id,
		seed.org_id,
		Some(sender_b),
		None,
		None,
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({
			"access_sender_ids": [sender_a.to_string(), sender_b.to_string()]
		}),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"PUT",
		&viewer_cookie,
		"/api/users/me/routing".to_string(),
		Some(json!({ "data": { "sender_id": sender_a.to_string() } })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let (status, value) =
		request_json(&app, "GET", &viewer_cookie, "/api/cases".to_string(), None)
			.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let cases = value["data"].as_array().ok_or("missing cases array")?;
	assert!(cases.iter().any(|row| row["id"] == case_a.to_string()));
	assert!(cases.iter().any(|row| row["id"] == case_b.to_string()));

	let (status, _value) = request_json(
		&app,
		"PUT",
		&viewer_cookie,
		"/api/users/me/routing".to_string(),
		Some(json!({
			"data": { "active_sender_identifier": Uuid::new_v4().to_string() }
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN);
	Ok(())
}
