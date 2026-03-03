use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::xml::{
	validate_e2b_xml, validate_e2b_xml_business, XmlValidatorConfig,
};
use serde_json::Value;
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Clone, Copy)]
struct RoundtripFixture {
	filename: &'static str,
	profile: &'static str,
	require_ok: bool,
}

fn workspace_root() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../..")
		.canonicalize()
		.expect("workspace root")
}

fn examples_dir() -> PathBuf {
	workspace_root().join("docs/refs/instances")
}

fn xsd_path() -> PathBuf {
	workspace_root()
		.join("deploy/ec2/schemas/multicacheschemas/MCCI_IN200100UV01.xsd")
}

fn build_multipart(xml: &[u8], filename: &str) -> (String, Vec<u8>) {
	let boundary = "X-BOUNDARY-ROUNDTRIP";
	let mut body = Vec::new();
	body.extend_from_slice(
		format!(
			"--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: application/xml\r\n\r\n"
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

async fn ensure_reaction_language(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<()> {
	let (status, body) = request_json(
		app,
		cookie,
		"GET",
		&format!("/api/cases/{case_id}/reactions"),
		None,
		Body::empty(),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!("list reactions status {} body {}", status, body).into());
	}
	let Some(reactions) = body.get("data").and_then(Value::as_array) else {
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
			let (status, body) = request_json(
				app,
				cookie,
				"PUT",
				&format!("/api/cases/{case_id}/reactions/{reaction_id}"),
				Some("application/json"),
				Body::from(
					serde_json::json!({
						"data": { "reaction_language": "en" }
					})
					.to_string(),
				),
			)
			.await?;
			if status != StatusCode::OK {
				return Err(format!(
					"update reaction language status {} body {}",
					status, body
				)
				.into());
			}
		}
	}
	Ok(())
}

async fn mark_case_validated(
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
	let bytes = to_bytes(res.into_body(), usize::MAX).await?;
	let body = serde_json::from_slice::<Value>(&bytes)?;
	if status != StatusCode::OK {
		return Err(format!("mark validated status {} body {}", status, body).into());
	}
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_roundtrip_fixtures_import_validate_export_revalidate() -> Result<()> {
	std::env::set_var("E2BR3_SKIP_XML_VALIDATE", "0");
	std::env::set_var("E2BR3_XSD_PATH", xsd_path());

	let fixtures = [
		RoundtripFixture {
			filename: "FAERS2022Scenario1.xml",
			profile: "ich",
			require_ok: true,
		},
		RoundtripFixture {
			filename: "FAERS2022Scenario2.xml",
			profile: "fda",
			require_ok: true,
		},
		RoundtripFixture {
			filename: "FAERS2022Scenario3.xml",
			profile: "mfds",
			require_ok: false,
		},
	];

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "admin_pwd", "viewer_pwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let mut failures = Vec::new();

	for fixture in fixtures {
		let fixture_path = examples_dir().join(fixture.filename);
		let mut xml = fs::read_to_string(&fixture_path)?;
		let unique_safety_report_id =
			format!("RT-{}-{}", fixture.profile, Uuid::new_v4());
		if let Some(start) = xml.find("extension=\"US-") {
			if let Some(end_rel) = xml[start + 11..].find('"') {
				let end = start + 11 + end_rel;
				xml.replace_range(start + 11..end, &unique_safety_report_id);
			}
		}
		let (boundary, multipart) =
			build_multipart(xml.as_bytes(), fixture.filename);

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
			.and_then(|v| v.get("case_id"))
			.and_then(Value::as_str)
		else {
			failures.push(format!("{}: missing case_id", fixture.filename));
			continue;
		};
		if let Err(err) = ensure_reaction_language(&app, &cookie, case_id).await {
			failures.push(format!("{}: {err}", fixture.filename));
			continue;
		}
		let (update_status, update_body) = request_json(
			&app,
			&cookie,
			"PUT",
			&format!("/api/cases/{case_id}"),
			Some("application/json"),
			Body::from(
				serde_json::json!({
					"data": { "validation_profile": fixture.profile }
				})
				.to_string(),
			),
		)
		.await?;
		if update_status != StatusCode::OK {
			failures.push(format!(
				"{}: failed to set validation_profile {} body={}",
				fixture.filename, update_status, update_body
			));
			continue;
		}

		let (validation_status, validation_body) = request_json(
			&app,
			&cookie,
			"GET",
			&format!(
				"/api/cases/{case_id}/validation?profile={}",
				fixture.profile
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
		let ok = validation_body
			.get("data")
			.and_then(|v| v.get("ok"))
			.and_then(Value::as_bool)
			.unwrap_or(false);
		if fixture.require_ok && !ok {
			failures.push(format!(
				"{}: expected ok=true for profile {}, body={}",
				fixture.filename, fixture.profile, validation_body
			));
			continue;
		}
		if !ok {
			continue;
		}
		if let Err(err) = mark_case_validated(&app, &cookie, case_id).await {
			failures.push(format!("{}: {err}", fixture.filename));
			continue;
		}

		let (export_status, export_bytes) = request_raw(
			&app,
			&cookie,
			"GET",
			&format!("/api/cases/{case_id}/export/xml"),
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
		let business_report =
			validate_e2b_xml_business(&export_bytes, Some(config))?;
		if !business_report.ok {
			failures.push(format!(
				"{}: exported business invalid: {:?}",
				fixture.filename, business_report.errors
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
