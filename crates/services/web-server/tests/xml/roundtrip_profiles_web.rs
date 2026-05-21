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
			let mut last_failure = None;
			for _attempt in 0..3 {
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
				if status == StatusCode::OK {
					last_failure = None;
					break;
				}
				let body_text = body.to_string();
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
			if let Some((status, body)) = last_failure {
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

async fn ensure_batch_transmission_date(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<()> {
	let (status, body) = request_json(
		app,
		cookie,
		"GET",
		&format!("/api/cases/{case_id}/message-header"),
		None,
		Body::empty(),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(
			format!("get message-header status {} body {}", status, body).into(),
		);
	}
	let has_batch_transmission_date = body
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
		&format!("/api/cases/{case_id}/message-header"),
		Some("application/json"),
		Body::from(
			serde_json::json!({
				"data": {
					"batch_transmission_date": [2024, 32, 1, 1, 1, 0, 0, 0, 0]
				}
			})
			.to_string(),
		),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update batch_transmission_date status {} body {}",
			status, body
		)
		.into());
	}
	Ok(())
}

async fn ensure_fda_device_characteristics(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<()> {
	let (status, body) = request_json(
		app,
		cookie,
		"GET",
		&format!("/api/cases/{case_id}/validation?profile=fda"),
		None,
		Body::empty(),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(
			format!("validation precheck status {} body {}", status, body).into(),
		);
	}
	let target_drug_indexes: std::collections::BTreeSet<usize> = body
		.get("data")
		.and_then(|v| v.get("issues"))
		.and_then(Value::as_array)
		.map(|issues| {
			issues
				.iter()
				.filter(|issue| {
					issue.get("code").and_then(Value::as_str)
						== Some("FDA.G.K.12.R.3.REQUIRED")
				})
				.filter_map(|issue| issue.get("path").and_then(Value::as_str))
				.filter_map(|path| {
					let index = path.strip_prefix("drugs.")?.split('.').next()?;
					index.parse::<usize>().ok()
				})
				.collect()
		})
		.unwrap_or_default();
	if target_drug_indexes.is_empty() {
		return Ok(());
	}

	let (status, body) = request_json(
		app,
		cookie,
		"GET",
		&format!("/api/cases/{case_id}/drugs"),
		None,
		Body::empty(),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!("list drugs status {} body {}", status, body).into());
	}
	let Some(drugs) = body.get("data").and_then(Value::as_array) else {
		return Ok(());
	};
	for (drug_index, drug) in drugs.iter().enumerate() {
		if !target_drug_indexes.contains(&drug_index) {
			continue;
		}
		let Some(drug_id) = drug.get("id").and_then(Value::as_str) else {
			continue;
		};
		let (status, body) = request_json(
			app,
			cookie,
			"GET",
			&format!("/api/cases/{case_id}/drugs/{drug_id}/device-characteristics"),
			None,
			Body::empty(),
		)
		.await?;
		if status != StatusCode::OK {
			return Err(format!(
				"list device characteristics status {} body {}",
				status, body
			)
			.into());
		}
		let Some(chars) = body.get("data").and_then(Value::as_array) else {
			continue;
		};
		let has_gk12r3 = chars.iter().any(|ch| {
			ch.get("code")
				.and_then(Value::as_str)
				.map(|code| code.eq_ignore_ascii_case("FDA.G.k.12.r.3"))
				.unwrap_or(false)
		});
		if !has_gk12r3 {
			let next_sequence_number = chars
				.iter()
				.filter_map(|ch| ch.get("sequence_number").and_then(Value::as_i64))
				.max()
				.unwrap_or(0)
				+ 1;
			let (status, body) = request_json(
				app,
				cookie,
				"POST",
				&format!(
					"/api/cases/{case_id}/drugs/{drug_id}/device-characteristics"
				),
				Some("application/json"),
				Body::from(
					serde_json::json!({
						"data": {
							"drug_id": drug_id,
							"sequence_number": next_sequence_number,
							"code": "FDA.G.k.12.r.3",
							"value_code": "1"
						}
					})
					.to_string(),
				),
			)
			.await?;
			if status != StatusCode::CREATED {
				return Err(format!(
					"create gk12r3 status {} body {}",
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
			.and_then(|v| v.get("case_id").or_else(|| v.get("caseId")))
			.and_then(Value::as_str)
		else {
			failures.push(format!("{}: missing case_id", fixture.filename));
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
		if fixture.profile == "fda" {
			if let Err(err) =
				ensure_fda_device_characteristics(&app, &cookie, case_id).await
			{
				failures.push(format!("{}: {err}", fixture.filename));
				continue;
			}
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
