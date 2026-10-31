pub(super) use crate::common::{
	cookie_header, create_test_import_product, init_test_env, init_test_mm,
	prepare_test_fda_export_case, seed_org_with_users, unique_safety_report_id_xml,
	Result,
};
pub(super) use axum::body::{to_bytes, Body};
pub(super) use axum::http::{Request, StatusCode};
pub(super) use lib_auth::token::generate_web_token;
pub(super) use serde_json::Value;
pub(super) use serial_test::serial;
pub(super) use std::io::{Cursor, Read};
pub(super) use tower::ServiceExt;
pub(super) use uuid::Uuid;

pub(super) use crate::xml_support::{
	ensure_batch_transmission_date, ensure_reaction_language, request_json,
	resolve_from_workspace, resolved_xsd_path, workspace_root,
};

pub(super) fn fixture_xml(filename: &str) -> Result<String> {
	Ok(std::fs::read_to_string(
		workspace_root().join("docs/exporter/fda").join(filename),
	)?)
}

pub(super) fn scenario6_xml() -> Result<String> {
	let xml = fixture_xml("FAERS2022Scenario6.xml")?;
	let invalid_attachment_tail = "\n.\n\n\t\t\t\t\t\t\t</text>";
	let updated =
		xml.replacen(invalid_attachment_tail, "\n\n\t\t\t\t\t\t\t</text>", 1);
	if updated == xml {
		return Err("Scenario 6 invalid attachment marker was not found".into());
	}
	Ok(updated)
}

pub(super) fn scenario6_with_first_test_date_null_flavor() -> Result<String> {
	let xml = scenario6_xml()?;
	let original = "<originalText>Calcium Level</originalText>\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t<!--  F.r.2.1 Test Name (free text) #1 -->\n\t\t\t\t\t\t\t\t\t\t\t\t\t</code>\n\t\t\t\t\t\t\t\t\t\t\t\t\t<effectiveTime value=\"20090101\"/>";
	let replacement = "<originalText>Calcium Level</originalText>\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t<!--  F.r.2.1 Test Name (free text) #1 -->\n\t\t\t\t\t\t\t\t\t\t\t\t\t</code>\n\t\t\t\t\t\t\t\t\t\t\t\t\t<effectiveTime nullFlavor=\"UNK\"/>";
	let updated = xml.replacen(original, replacement, 1);
	if updated == xml {
		return Err("fixture patch should replace one F.r.1 date".into());
	}
	Ok(updated)
}

pub(super) async fn import_xml_string(
	app: &axum::Router,
	cookie: &str,
	filename: &str,
	xml: &str,
	product_presave_id: Uuid,
) -> Result<Value> {
	let original_xml = xml;
	let xml = unique_safety_report_id_xml(original_xml.to_owned());
	if xml == original_xml {
		return Err(format!(
			"fixture has no safety report ID to make unique: {filename}"
		)
		.into());
	}
	let boundary = "X-BOUNDARY-XML-IMPORT-NULLFLAVOR";
	let body = format!(
		"--{boundary}\r\nContent-Disposition: form-data; name=\"format\"\r\n\r\nfda\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"productPresaveId\"\r\n\r\n{product_presave_id}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: application/xml\r\n\r\n{xml}\r\n--{boundary}--\r\n"
	);
	let req = Request::builder()
		.method("POST")
		.uri("/api/import/xml")
		.header(
			"content-type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header("cookie", cookie)
		.body(Body::from(body))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"import status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	Ok(serde_json::from_slice(&body)?)
}

pub(super) fn import_case_id(import_value: &Value) -> Result<&str> {
	import_value
		.get("data")
		.and_then(|v| v.get("caseId"))
		.and_then(Value::as_str)
		.ok_or_else(|| {
			format!("missing data.caseId in import response: {import_value}").into()
		})
}
