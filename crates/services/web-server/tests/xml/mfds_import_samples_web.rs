use crate::common::{
	cookie_header, create_test_import_product, init_test_mm, seed_org_with_users,
	Result,
};
use crate::xml_support::{resolve_from_workspace, resolved_xsd_path};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::Value;
use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use tower::ServiceExt;
use uuid::Uuid;
use xml::import_sections::c_safety_report::parse_c_safety_report;
use xml::import_sections::d_patient::parse_d_patient;
use xml::import_sections::e_reaction::parse_e_reactions;
use xml::import_sections::f_test_result::parse_f_test_results;
use xml::import_sections::g_drug::parse_g_drugs;
use xml::import_sections::h_narrative::parse_h_narrative;
use xml::validation::validate_e2b_xml;

#[derive(Debug)]
struct ExpectedCounts {
	has_safety_report: bool,
	has_patient: bool,
	has_narrative: bool,
	reactions: usize,
	drugs: usize,
	test_results: usize,
}

fn mfds_examples_dir() -> PathBuf {
	if let Ok(value) = std::env::var("E2BR3_MFDS_EXAMPLES_DIR") {
		return resolve_from_workspace(PathBuf::from(value));
	}
	PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/xml/fixtures/mfds")
}

fn list_xml_files(dir: &Path) -> Result<Vec<PathBuf>> {
	let mut files = Vec::new();
	for entry in fs::read_dir(dir)? {
		let entry = entry?;
		let path = entry.path();
		if path
			.extension()
			.and_then(|ext| ext.to_str())
			.map(|ext| ext.eq_ignore_ascii_case("xml"))
			.unwrap_or(false)
		{
			files.push(path);
		}
	}
	files.sort();
	Ok(files)
}

fn expected_from_xml(xml: &[u8]) -> Result<ExpectedCounts> {
	Ok(ExpectedCounts {
		has_safety_report: parse_c_safety_report(xml)?.is_some(),
		has_patient: parse_d_patient(xml)?.is_some(),
		has_narrative: parse_h_narrative(xml)?.is_some(),
		reactions: parse_e_reactions(xml)?.len(),
		drugs: parse_g_drugs(xml)?.len(),
		test_results: parse_f_test_results(xml)?.len(),
	})
}

fn build_multipart(
	xml: &[u8],
	filename: &str,
	product_presave_id: Uuid,
) -> (String, Vec<u8>) {
	let boundary = "X-BOUNDARY-MFDS-IMPORT";
	let mut body = Vec::new();
	body.extend_from_slice(
		format!(
			"--{boundary}\r\nContent-Disposition: form-data; name=\"format\"\r\n\r\nmfds\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"productPresaveId\"\r\n\r\n{product_presave_id}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: application/xml\r\n\r\n"
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

fn get_data_array_len(body: &Value) -> Result<usize> {
	Ok(body
		.get("data")
		.and_then(Value::as_array)
		.ok_or("missing data array")?
		.len())
}

fn import_case_id(body: &Value) -> Option<&str> {
	body.get("data")
		.and_then(|v| v.get("caseId"))
		.and_then(Value::as_str)
}

#[serial]
#[tokio::test]
async fn test_mfds_samples_import_and_validate() -> Result<()> {
	let dir = mfds_examples_dir();
	if !dir.is_dir() {
		return Err(format!(
			"MFDS sample fixture directory is required: {}",
			dir.display()
		)
		.into());
	}
	std::env::set_var("E2BR3_XSD_PATH", resolved_xsd_path());
	// Always exercise real schema validation in this integration suite.

	let xml_files = list_xml_files(&dir)?;
	if xml_files.is_empty() {
		return Err(format!(
			"MFDS sample fixture directory contains no XML files: {}",
			dir.display()
		)
		.into());
	}

	// Align test expectations with importer section parsers.
	std::env::set_var("XML_V2_IMPORT_D", "1");
	std::env::set_var("XML_V2_IMPORT_E", "1");
	std::env::set_var("XML_V2_IMPORT_F", "1");
	std::env::set_var("XML_V2_IMPORT_G", "1");
	std::env::set_var("XML_V2_IMPORT_H", "1");

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "admin_pwd", "viewer_pwd").await?;
	let product_presave_id =
		create_test_import_product(&mm, seed.org_id, seed.admin.id).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let mut failures: Vec<String> = Vec::new();
	for xml_path in xml_files {
		let filename = xml_path
			.file_name()
			.and_then(|n| n.to_str())
			.ok_or_else(|| {
				format!(
					"MFDS XML fixture has a non-UTF-8 filename: {}",
					xml_path.display()
				)
			})?
			.to_string();
		let xml = fs::read(&xml_path)?;
		let expected = expected_from_xml(&xml)?;
		let (boundary, multipart) =
			build_multipart(&xml, &filename, product_presave_id);

		let (status, body) = request_json(
			&app,
			&cookie,
			"POST",
			"/api/import/xml",
			Some(&format!("multipart/form-data; boundary={boundary}")),
			Body::from(multipart),
		)
		.await?;
		if status != StatusCode::OK {
			failures.push(format!(
				"{filename}: import failed with status {status}, body={body}"
			));
			continue;
		}

		let Some(case_id) = import_case_id(&body) else {
			failures.push(format!("{filename}: missing case_id in import response"));
			continue;
		};

		let (case_status, case_body) = request_json(
			&app,
			&cookie,
			"GET",
			&format!("/api/cases/{case_id}"),
			None,
			Body::empty(),
		)
		.await?;
		if case_status != StatusCode::OK {
			failures.push(format!(
				"{filename}: GET case failed with status {case_status}, body={case_body}"
			));
			continue;
		}

		let section_checks = [
			(
				"safety-report",
				format!("/api/cases/{case_id}/safety-report"),
				expected.has_safety_report,
				StatusCode::NOT_FOUND,
			),
			(
				"patient",
				format!("/api/cases/{case_id}/patient"),
				expected.has_patient,
				StatusCode::NOT_FOUND,
			),
			(
				"narrative",
				format!("/api/cases/{case_id}/narrative"),
				expected.has_narrative,
				StatusCode::OK,
			),
		];
		for (name, uri, should_exist, absent_status) in section_checks {
			let (s, b) =
				request_json(&app, &cookie, "GET", &uri, None, Body::empty())
					.await?;
			let expected_status = if should_exist {
				StatusCode::OK
			} else {
				absent_status
			};
			if s != expected_status {
				failures.push(format!(
					"{filename}: {name} expected status {expected_status}, got {s}, body={b}"
				));
				continue;
			}
			if s == StatusCode::OK {
				let Some(data) = b.get("data") else {
					failures.push(format!(
						"{filename}: {name} response missing data, body={b}"
					));
					continue;
				};
				if data.is_null() == should_exist {
					failures.push(format!(
						"{filename}: {name} presence mismatch expected={should_exist}, body={b}"
					));
				}
			}
		}

		let list_checks = [
			(
				"reactions",
				format!("/api/cases/{case_id}/reactions"),
				expected.reactions,
			),
			(
				"drugs",
				format!("/api/cases/{case_id}/drugs"),
				expected.drugs,
			),
			(
				"test-results",
				format!("/api/cases/{case_id}/test-results"),
				expected.test_results,
			),
		];
		for (name, uri, expected_count) in list_checks {
			let (s, b) =
				request_json(&app, &cookie, "GET", &uri, None, Body::empty())
					.await?;
			if s != StatusCode::OK {
				failures.push(format!(
					"{filename}: GET {name} failed with status {s}, body={b}"
				));
				continue;
			}
			let actual = match get_data_array_len(&b) {
				Ok(value) => value,
				Err(err) => {
					failures.push(format!(
						"{filename}: {name} response parse error: {err}, body={b}"
					));
					continue;
				}
			};
			if actual != expected_count {
				failures.push(format!(
					"{filename}: {name} count mismatch imported={actual} parsed={expected_count}"
				));
			}
		}

		let (validation_status, validation_body) = request_json(
			&app,
			&cookie,
			"GET",
			&format!("/api/cases/{case_id}/validation?authority=mfds"),
			None,
			Body::empty(),
		)
		.await?;
		if validation_status != StatusCode::OK {
			failures.push(format!(
				"{filename}: validation call failed with status {validation_status}, body={validation_body}"
			));
			continue;
		}
		let Some(ok) = validation_body
			.get("data")
			.and_then(|v| v.get("ok"))
			.and_then(Value::as_bool)
		else {
			failures.push(format!(
				"{filename}: validation response missing data.ok boolean, body={validation_body}"
			));
			continue;
		};
		if !ok {
			failures.push(format!(
				"{filename}: MFDS validation returned ok=false, body={validation_body}"
			));
			continue;
		}

		let (export_status, export_bytes) = request_raw(
			&app,
			&cookie,
			"GET",
			&format!("/api/cases/{case_id}/export/xml?authority=mfds"),
			None,
			Body::empty(),
		)
		.await?;
		if export_status != StatusCode::OK {
			failures.push(format!(
				"{filename}: export failed with status {export_status}, body={}",
				String::from_utf8_lossy(&export_bytes)
			));
			continue;
		}
		let export_report = validate_e2b_xml(&export_bytes, None)?;
		if !export_report.ok {
			failures.push(format!(
				"{filename}: exported XML failed validation: {:?}",
				export_report.errors
			));
		}
	}

	if !failures.is_empty() {
		return Err(format!(
			"MFDS sample import validation failures ({}):\n{}",
			failures.len(),
			failures.join("\n")
		)
		.into());
	}

	Ok(())
}
