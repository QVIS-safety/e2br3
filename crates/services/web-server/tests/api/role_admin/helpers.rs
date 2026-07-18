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
use std::collections::HashSet;
use tower::ServiceExt;
use uuid::Uuid;

pub(super) fn extract_id(value: &Value) -> Result<Uuid> {
	let id = value["data"]["id"].as_str().ok_or("missing data.id")?;
	Ok(Uuid::parse_str(id)?)
}

pub(super) async fn request_json(
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

pub(super) async fn request_raw_status(
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

pub(super) async fn create_empty_custom_role(
	app: &Router,
	admin_cookie: &str,
	profile_id: &str,
) -> Result<String> {
	create_empty_custom_role_with_generated_id(app, admin_cookie, profile_id).await
}

pub(super) async fn create_empty_custom_role_with_generated_id(
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

pub(super) async fn update_role_privileges(
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

pub(super) async fn custom_role_user(
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

pub(super) async fn assert_get_status(
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

pub(super) async fn assert_get_not_status(
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

pub(super) async fn assert_profile_access(
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
	assert!(profile["data"].get("capabilities").is_none(), "{profile:?}");
	let permissions = profile["data"]["permissions"]
		.as_array()
		.ok_or("missing permissions")?
		.iter()
		.filter_map(Value::as_str)
		.collect::<HashSet<_>>();
	for (module, action, expected) in expected {
		let required: &[&str] = match (*module, *action) {
			("case", "read") => &["Case.Read", "Case.List"],
			("case", "create") => &["Case.Create"],
			("case", "update") => &["Case.Update"],
			("case", "delete") => &["Case.Delete"],
			("case", "review" | "lock") => &["Case.Approve"],
			("import", "read") => &["XmlImport.Read"],
			("import", "execute") => &["XmlImport.Import"],
			("exportSubmission", "read") => &["XmlExport.Read"],
			("exportSubmission", "execute") => &["XmlExport.Export"],
			("data", "read") => &["Terminology.Read"],
			("data", "import") => &["Terminology.Import"],
			("data", "approve") => &["Terminology.Approve"],
			("users", "read") => &["User.Read", "User.List"],
			("users", "create") => &["User.Create"],
			("users", "update") => &["User.Update"],
			("users", "delete") => &["User.Delete"],
			("settings", "read") => &["Settings.Read"],
			("settings", "update") => &["Settings.Update"],
			("homeNotice", "read") => &["DashboardNotice.Read"],
			("homeNotice", "update") => &["DashboardNotice.Update"],
			("admin" | "roles", _) => &["User.Create"],
			_ => {
				return Err(
					format!("unknown access assertion {module}.{action}").into()
				)
			}
		};
		let actual = required
			.iter()
			.any(|permission| permissions.contains(permission));
		assert_eq!(
			actual, *expected,
			"{module}.{action} permission mismatch: {profile:?}"
		);
	}
	Ok(profile)
}

pub(super) async fn assert_profile_permissions(
	app: &Router,
	cookie: &str,
	present: &[&str],
	absent: &[&str],
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
	assert!(profile["data"].get("capabilities").is_none(), "{profile:?}");
	let permissions = profile["data"]["permissions"]
		.as_array()
		.ok_or("missing permissions")?
		.iter()
		.filter_map(Value::as_str)
		.collect::<HashSet<_>>();
	for permission in present {
		assert!(permissions.contains(permission), "missing {permission}");
	}
	for permission in absent {
		assert!(!permissions.contains(permission), "unexpected {permission}");
	}
	Ok(profile)
}

pub(super) async fn assert_workflow_assign_status(
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

pub(super) async fn create_case(
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

pub(super) async fn create_message_header(
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

pub(super) async fn create_sender_information(
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

pub(super) async fn create_sender_presave(
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

pub(super) async fn create_study(
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

pub(super) async fn create_drug(
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

pub(super) async fn create_drug_with_brand(
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

pub(super) async fn update_user_scope(
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

pub(super) async fn insert_history_rows_for_case(
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

pub(super) async fn set_org_type(
	mm: &ModelManager,
	org_id: Uuid,
	org_type: &str,
) -> Result<()> {
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		system_user_id(),
		crate::common::system_org_id(),
		ROLE_SYSTEM_ADMIN,
	)
	.await?;
	mm.dbx()
		.execute(
			sqlx::query("UPDATE organizations SET org_type = $1 WHERE id = $2")
				.bind(org_type)
				.bind(org_id),
		)
		.await?;
	mm.dbx().commit_txn().await?;
	Ok(())
}
