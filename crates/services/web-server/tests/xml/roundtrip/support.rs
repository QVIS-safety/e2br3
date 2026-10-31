pub(super) use crate::common::{
	cookie_header, create_test_import_product, init_test_env, init_test_mm,
	prepare_test_fda_export_case, seed_org_with_users, unique_safety_report_id_xml,
	Result,
};
pub(super) use crate::xml_support::{
	ensure_batch_transmission_date, ensure_reaction_language, request_json,
	resolve_from_workspace, resolved_xsd_path,
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

pub(super) fn extract_data_id(body: &[u8]) -> Result<String> {
	let value: Value = serde_json::from_slice(body)?;
	let id = value
		.get("data")
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or_else(|| format!("response missing data.id string: {value}"))?;
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
	let examples_dir = resolve_from_workspace(std::path::PathBuf::from(
		std::env::var("E2BR3_EXAMPLES_DIR")
			.map_err(|err| format!("E2BR3_EXAMPLES_DIR is required: {err}"))?,
	));
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
	let fixture_xml = std::fs::read_to_string(&xml_path)?;
	let xml = unique_safety_report_id_xml(fixture_xml.clone());
	if xml == fixture_xml {
		return Err(format!(
			"XML fixture does not contain the expected US safety report identifier to make unique: {}",
			xml_path.display()
		)
		.into());
	}
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
	let drugs = value
		.get("data")
		.and_then(Value::as_array)
		.ok_or_else(|| format!("list drugs response missing data array: {value}"))?;
	let drug = drugs
		.first()
		.ok_or_else(|| format!("list drugs response has no first drug: {value}"))?;
	let drug_id = drug
		.get("id")
		.and_then(Value::as_str)
		.ok_or_else(|| format!("first drug response item missing id: {drug}"))?;
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
	let devices = value.get("data").and_then(Value::as_array).ok_or_else(|| {
		format!("replace FDA devices response missing data array: {value}")
	})?;
	if devices.len() != 1 {
		return Err(format!(
			"replace FDA devices response expected one device, got {}: {value}",
			devices.len()
		)
		.into());
	}
	if devices[0].get("malfunction").and_then(Value::as_bool) != Some(true) {
		return Err(format!(
			"replace FDA devices response did not contain malfunction=true: {value}"
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
	Ok(String::from_utf8(body.to_vec())?)
}
