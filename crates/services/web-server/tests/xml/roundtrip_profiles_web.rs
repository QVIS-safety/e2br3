use crate::common::{
	cookie_header, create_test_import_product, init_test_mm, seed_org_with_users,
	unique_safety_report_id_xml, Result,
};
use crate::xml_support::{
	default_xsd_path as xsd_path, ensure_batch_transmission_date,
	ensure_reaction_language, workspace_root,
};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::regulatory::RegulatoryAuthority;
use serde_json::Value;
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use tower::ServiceExt;
use uuid::Uuid;
use xml::validation::{validate_e2b_xml, validate_export_rules, XmlValidatorConfig};

#[derive(Clone, Copy)]
struct RoundtripFixture {
	filename: &'static str,
	authority: &'static str,
}

fn examples_dir() -> PathBuf {
	workspace_root().join("docs/exporter/fda")
}

fn build_multipart(
	xml: &[u8],
	filename: &str,
	product_presave_id: Uuid,
	authority: &str,
) -> (String, Vec<u8>) {
	let boundary = "X-BOUNDARY-ROUNDTRIP";
	let mut body = Vec::new();
	body.extend_from_slice(
		format!(
			"--{boundary}\r\nContent-Disposition: form-data; name=\"format\"\r\n\r\n{authority}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"productPresaveId\"\r\n\r\n{product_presave_id}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: application/xml\r\n\r\n"
		)
		.as_bytes(),
	);
	body.extend_from_slice(xml);
	body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
	(boundary.to_string(), body)
}

async fn request_json(
	app: &axum::Router,
	cookie: &str,
	method: &str,
	uri: &str,
	content_type: Option<&str>,
	body: Body,
) -> Result<(StatusCode, Value)> {
	let mut builder = Request::builder()
		.method(method)
		.uri(uri)
		.header("cookie", cookie);
	if let Some(ct) = content_type {
		builder = builder.header("content-type", ct);
	}
	let req = builder.body(body)?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let bytes = to_bytes(res.into_body(), usize::MAX).await?;
	let json = serde_json::from_slice::<Value>(&bytes)?;
	Ok((status, json))
}

async fn request_raw(
	app: &axum::Router,
	cookie: &str,
	method: &str,
	uri: &str,
	content_type: Option<&str>,
	body: Body,
) -> Result<(StatusCode, Vec<u8>)> {
	let mut builder = Request::builder()
		.method(method)
		.uri(uri)
		.header("cookie", cookie);
	if let Some(ct) = content_type {
		builder = builder.header("content-type", ct);
	}
	let req = builder.body(body)?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let bytes = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, bytes.to_vec()))
}

#[serial]
#[tokio::test]
async fn test_roundtrip_fixtures_import_validate_export_revalidate() -> Result<()> {
	std::env::set_var("E2BR3_XSD_PATH", xsd_path());

	let fixtures = [
		RoundtripFixture {
			filename: "FAERS2022Scenario1.xml",
			authority: "ich",
		},
		RoundtripFixture {
			filename: "FAERS2022Scenario2.xml",
			authority: "fda",
		},
		RoundtripFixture {
			filename: "FAERS2022Scenario3.xml",
			authority: "mfds",
		},
	];

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "admin_pwd", "viewer_pwd").await?;
	let product_presave_id =
		create_test_import_product(&mm, seed.org_id, seed.admin.id).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let mut failures = Vec::new();

	for fixture in fixtures {
		let fixture_path = examples_dir().join(fixture.filename);
		let original_xml = fs::read_to_string(&fixture_path)?;
		let xml = unique_safety_report_id_xml(original_xml.clone());
		if xml == original_xml {
			return Err(format!(
				"fixture has no safety report ID to make unique: {}",
				fixture.filename
			)
			.into());
		}
		let (boundary, multipart) = build_multipart(
			xml.as_bytes(),
			fixture.filename,
			product_presave_id,
			fixture.authority,
		);

		let (import_status, import_body) = request_json(
			&app,
			&cookie,
			"POST",
			"/api/import/xml",
			Some(&format!("multipart/form-data; boundary={boundary}")),
			Body::from(multipart),
		)
		.await?;
		if import_status != StatusCode::OK {
			failures.push(format!(
				"{}: import failed {} {}",
				fixture.filename, import_status, import_body
			));
			continue;
		}
		let Some(case_id) = import_body
			.get("data")
			.and_then(|v| v.get("caseId"))
			.and_then(Value::as_str)
		else {
			failures.push(format!(
				"{}: missing case_id in import response: {}",
				fixture.filename, import_body
			));
			continue;
		};
		if let Err(err) = ensure_reaction_language(&app, &cookie, case_id).await {
			failures.push(format!("{}: {err}", fixture.filename));
			continue;
		}
		if let Err(err) =
			ensure_batch_transmission_date(&app, &cookie, case_id).await
		{
			failures.push(format!("{}: {err}", fixture.filename));
			continue;
		}
		let (validation_status, validation_body) = request_json(
			&app,
			&cookie,
			"GET",
			&format!(
				"/api/cases/{case_id}/validation?authority={}",
				fixture.authority
			),
			None,
			Body::empty(),
		)
		.await?;
		if validation_status != StatusCode::OK {
			failures.push(format!(
				"{}: validation {} {}",
				fixture.filename, validation_status, validation_body
			));
			continue;
		}
		if validation_body
			.get("data")
			.and_then(|v| v.get("ok"))
			.and_then(Value::as_bool)
			.is_none()
		{
			failures.push(format!(
				"{}: validation response missing data.ok boolean: {}",
				fixture.filename, validation_body
			));
			continue;
		}
		// Manual export is independent of workflow/business-validation status.
		crate::xml_support::set_export_receivers(
			&app,
			&cookie,
			case_id,
			fixture.authority,
		)
		.await?;

		let (export_status, export_bytes) = request_raw(
			&app,
			&cookie,
			"GET",
			&format!(
				"/api/cases/{case_id}/export/xml?authority={}",
				fixture.authority
			),
			None,
			Body::empty(),
		)
		.await?;
		if export_status != StatusCode::OK {
			failures.push(format!(
				"{}: export failed {} {}",
				fixture.filename,
				export_status,
				String::from_utf8_lossy(&export_bytes)
			));
			continue;
		}

		let config = XmlValidatorConfig {
			xsd_path: Some(xsd_path()),
			..XmlValidatorConfig::default()
		};
		let schema_report = validate_e2b_xml(&export_bytes, Some(config.clone()))?;
		if !schema_report.ok {
			failures.push(format!(
				"{}: exported schema invalid: {:?}",
				fixture.filename, schema_report.errors
			));
			continue;
		}
		let authority =
			RegulatoryAuthority::parse(fixture.authority).ok_or_else(|| {
				format!("invalid fixture authority {}", fixture.authority)
			})?;
		let export_errors = validate_export_rules(&export_bytes, authority)?;
		if !export_errors.is_empty() {
			failures.push(format!(
				"{}: exported rule invalid: {:?}",
				fixture.filename, export_errors
			));
		}
	}

	if !failures.is_empty() {
		return Err(format!(
			"roundtrip fixture failures ({}):\n{}",
			failures.len(),
			failures.join("\n")
		)
		.into());
	}

	Ok(())
}
