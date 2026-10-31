pub(super) use crate::common::{
	cookie_header, create_test_import_product, init_test_env, init_test_mm,
	prepare_test_fda_export_case, seed_org_with_users, unique_safety_report_id_xml,
	Result,
};
pub(super) use axum::body::{to_bytes, Body};
pub(super) use axum::http::{Request, StatusCode};
pub(super) use lib_auth::token::generate_web_token;
pub(super) use lib_core::ctx::{Ctx, ROLE_SPONSOR_ADMIN_CRO};
pub(super) use lib_core::model::drug::FdaDeviceInformationBmc;
pub(super) use lib_core::model::patient::{
	AutopsyCauseOfDeathBmc, AutopsyCauseOfDeathForCreate,
	PatientDeathInformationBmc, PatientDeathInformationForCreate,
	PatientInformationBmc,
};
pub(super) use lib_core::model::ModelManager;
pub(super) use serde_json::Value;
pub(super) use serial_test::serial;
pub(super) use tower::ServiceExt;
pub(super) use uuid::Uuid;

pub(super) fn outbound_header(
	authority: lib_core::regulatory::RegulatoryAuthority,
) -> xml::OutboundMessageHeader {
	let (batch_receiver_identifier, message_receiver_identifier) = match authority {
		lib_core::regulatory::RegulatoryAuthority::Ich => ("TEST-ICH", "TEST-ICH"),
		lib_core::regulatory::RegulatoryAuthority::Fda => ("ZZFDA", "CDER"),
		lib_core::regulatory::RegulatoryAuthority::Mfds => {
			("MFDS-O-KR", "MFDS-O-KR")
		}
	};
	xml::OutboundMessageHeader {
		batch_number: "TEST-BATCH".to_string(),
		batch_sender_identifier: "TEST-SENDER".to_string(),
		batch_receiver_identifier: batch_receiver_identifier.to_string(),
		batch_transmission_date: time::OffsetDateTime::now_utc(),
		message_sender_identifier: "TEST-SENDER".to_string(),
		message_receiver_identifier: message_receiver_identifier.to_string(),
	}
}

pub(super) fn workspace_root() -> std::path::PathBuf {
	std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../..")
		.canonicalize()
		.expect("workspace root")
}

pub(super) fn resolve_from_workspace(
	path: std::path::PathBuf,
) -> std::path::PathBuf {
	if path.is_absolute() {
		path
	} else {
		workspace_root().join(path)
	}
}

pub(super) fn default_xsd_path() -> std::path::PathBuf {
	workspace_root()
		.join("docs/exporter/schema/multicacheschemas/MCCI_IN200100UV01.xsd")
}

pub(super) fn resolved_xsd_path() -> std::path::PathBuf {
	match std::env::var("E2BR3_XSD_PATH") {
		Ok(value) => resolve_from_workspace(std::path::PathBuf::from(value)),
		Err(_) => default_xsd_path(),
	}
}

pub(super) async fn request_json(
	app: &axum::Router,
	cookie: &str,
	method: &str,
	uri: String,
	body: Option<serde_json::Value>,
) -> Result<(StatusCode, Vec<u8>)> {
	let mut builder = Request::builder()
		.method(method)
		.uri(uri)
		.header("cookie", cookie);
	if body.is_some() {
		builder = builder.header("content-type", "application/json");
	}
	let req =
		builder.body(Body::from(body.map(|b| b.to_string()).unwrap_or_default()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?.to_vec();
	Ok((status, body))
}

pub(super) fn extract_data_id(body: &[u8]) -> Result<String> {
	let value: Value = serde_json::from_slice(body)?;
	let id = value
		.get("data")
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing data.id")?;
	Ok(id.to_string())
}

pub(super) type ImportedCaseSetup =
	(axum::Router, String, String, ModelManager, Ctx);

pub(super) async fn setup_imported_case() -> Result<ImportedCaseSetup> {
	setup_imported_case_from("FAERS2022Scenario2.xml").await
}

pub(super) async fn setup_imported_case_from(
	fixture_name: &str,
) -> Result<ImportedCaseSetup> {
	init_test_env().await?;
	let Some(examples_dir) = std::env::var("E2BR3_EXAMPLES_DIR")
		.ok()
		.map(std::path::PathBuf::from)
		.map(resolve_from_workspace)
	else {
		return Err("E2BR3_EXAMPLES_DIR not set".into());
	};
	std::env::set_var("E2BR3_XSD_PATH", resolved_xsd_path());

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "admin_pwd", "viewer_pwd").await?;
	let product_presave_id =
		create_test_import_product(&mm, seed.org_id, seed.admin.id).await?;
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let xml_path = examples_dir.join(fixture_name);
	let xml = unique_safety_report_id_xml(std::fs::read_to_string(xml_path)?);
	let boundary = "X-BOUNDARY-XML-IMPORT-REMAINING";
	let body = format!(
		"--{boundary}\r\nContent-Disposition: form-data; name=\"format\"\r\n\r\nfda\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"productPresaveId\"\r\n\r\n{product_presave_id}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"case.xml\"\r\nContent-Type: application/xml\r\n\r\n{xml}\r\n--{boundary}--\r\n"
	);
	let req = Request::builder()
		.method("POST")
		.uri("/api/import/xml")
		.header(
			"content-type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header("cookie", cookie.clone())
		.body(Body::from(body))?;
	let res = app.clone().oneshot(req).await?;
	let import_status = res.status();
	let import_body = to_bytes(res.into_body(), usize::MAX).await?;
	if import_status != StatusCode::OK {
		return Err(format!(
			"import status {} body {}",
			import_status,
			String::from_utf8_lossy(&import_body)
		)
		.into());
	}
	let import_value: Value = serde_json::from_slice(&import_body)?;
	let case_id = import_value
		.get("data")
		.and_then(|v| v.get("caseId"))
		.and_then(|v| v.as_str())
		.ok_or_else(|| {
			format!("missing data.caseId in import response: {import_value}")
		})?
		.to_string();
	prepare_test_fda_export_case(
		&mm,
		seed.org_id,
		seed.admin.id,
		Uuid::parse_str(&case_id)?,
	)
	.await?;
	ensure_reaction_language(&app, &cookie, &case_id).await?;
	ensure_batch_transmission_date(&app, &cookie, &case_id).await?;
	ensure_fda_device_characteristics(&app, &cookie, &case_id).await?;

	Ok((app, cookie, case_id, mm, ctx))
}

pub(super) async fn ensure_reaction_language(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<()> {
	let (status, body) = request_json(
		app,
		cookie,
		"GET",
		format!("/api/cases/{case_id}/reactions"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list reactions status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let Some(reactions) = value.get("data").and_then(Value::as_array) else {
		return Ok(());
	};
	for reaction in reactions {
		let Some(reaction_id) = reaction.get("id").and_then(Value::as_str) else {
			continue;
		};
		let has_text = reaction
			.get("primary_source_reaction")
			.and_then(Value::as_str)
			.map(|v| !v.trim().is_empty())
			.unwrap_or(false);
		let has_language = reaction
			.get("reaction_language")
			.and_then(Value::as_str)
			.map(|v| !v.trim().is_empty())
			.unwrap_or(false);
		if has_text && !has_language {
			let mut last_failure = None;
			for _attempt in 0..3 {
				let (status, body) = request_json(
					app,
					cookie,
					"PUT",
					format!("/api/cases/{case_id}/reactions/{reaction_id}"),
					Some(serde_json::json!({
						"data": { "reaction_language": "eng" }
					})),
				)
				.await?;
				if status == StatusCode::OK {
					last_failure = None;
					break;
				}
				let body_text = String::from_utf8_lossy(&body).to_string();
				if !body_text.contains("Audit trail logging failed")
					&& !body_text.contains("deadlock detected")
				{
					return Err(format!(
						"update reaction language status {} body {}",
						status, body_text
					)
					.into());
				}
				last_failure = Some((status, body_text));
			}
			if let Some((status, body_text)) = last_failure {
				return Err(format!(
					"update reaction language status {} body {}",
					status, body_text
				)
				.into());
			}
		}
	}
	Ok(())
}

pub(super) async fn ensure_batch_transmission_date(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<()> {
	let (status, body) = request_json(
		app,
		cookie,
		"GET",
		format!("/api/cases/{case_id}/message-header"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"get message-header status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let has_batch_transmission_date = value
		.get("data")
		.and_then(|v| v.get("batch_transmission_date"))
		.and_then(Value::as_array)
		.map(|v| !v.is_empty())
		.unwrap_or(false);
	if has_batch_transmission_date {
		return Ok(());
	}
	let (status, body) = request_json(
		app,
		cookie,
		"PUT",
		format!("/api/cases/{case_id}/message-header"),
		Some(serde_json::json!({
			"data": {
				"batch_transmission_date": [2024, 32, 1, 1, 1, 0, 0, 0, 0]
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update batch_transmission_date status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	Ok(())
}

pub(super) async fn ensure_fda_device_characteristics(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<()> {
	let (status, body) = request_json(
		app,
		cookie,
		"GET",
		format!("/api/cases/{case_id}/drugs"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list drugs status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let Some(drugs) = value.get("data").and_then(Value::as_array) else {
		return Err(format!("missing drug items: {value}").into());
	};
	for (drug_index, drug) in drugs.iter().enumerate() {
		if drug_index != 0 {
			continue;
		}
		let Some(drug_id) = drug.get("id").and_then(Value::as_str) else {
			continue;
		};
		let (status, body) = request_json(
			app,
			cookie,
			"PUT",
			format!("/api/cases/{case_id}/drugs/{drug_id}/devices/replace"),
			Some(serde_json::json!({
				"data": { "devices": [{
					"malfunction": true,
					"device_problem_codes": [{ "value_code": "1" }]
				}] }
			})),
		)
		.await?;
		if status != StatusCode::OK {
			return Err(format!(
				"replace FDA devices status {} body {}",
				status,
				String::from_utf8_lossy(&body)
			)
			.into());
		}
		let value: Value = serde_json::from_slice(&body)?;
		assert_eq!(value["data"].as_array().map(Vec::len), Some(1));
	}
	Ok(())
}

pub(super) async fn set_validated(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<()> {
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	let req = Request::builder()
		.method("POST")
		.uri(format!("/api/cases/{case_id}/validator/mark-validated"))
		.header("cookie", cookie)
		.header("x-validator-token", "validator-secret")
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::OK {
		if String::from_utf8_lossy(&body).contains("validation issue(s) remain") {
			return Ok(());
		}
		let (validation_status, validation_body) = request_json(
			app,
			cookie,
			"GET",
			format!("/api/cases/{case_id}/validation?authority=fda"),
			None,
		)
		.await?;
		return Err(format!(
			"mark validated status {} body {} validation_status {} validation_body {}",
			status,
			String::from_utf8_lossy(&body),
			validation_status,
			String::from_utf8_lossy(&validation_body)
		)
		.into());
	}
	Ok(())
}

pub(super) async fn export_xml(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<String> {
	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{case_id}/export/xml?authority=fda"))
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"export status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	Ok(String::from_utf8_lossy(&body).to_string())
}
