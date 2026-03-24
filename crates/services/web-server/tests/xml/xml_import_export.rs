use crate::common::{
	cookie_header, init_test_env, init_test_mm, seed_org_with_users, Result,
};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::Value;
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

fn workspace_root() -> std::path::PathBuf {
	std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../..")
		.canonicalize()
		.expect("workspace root")
}

fn resolve_from_workspace(path: std::path::PathBuf) -> std::path::PathBuf {
	if path.is_absolute() {
		path
	} else {
		workspace_root().join(path)
	}
}

fn default_xsd_path() -> std::path::PathBuf {
	workspace_root()
		.join("deploy/ec2/schemas/multicacheschemas/MCCI_IN200100UV01.xsd")
}

fn resolved_xsd_path() -> std::path::PathBuf {
	match std::env::var("E2BR3_XSD_PATH") {
		Ok(value) => resolve_from_workspace(std::path::PathBuf::from(value)),
		Err(_) => default_xsd_path(),
	}
}

async fn request_json(
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

async fn ensure_reaction_language(
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
						"data": { "reaction_language": "en" }
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

async fn ensure_batch_transmission_date(
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

async fn ensure_fda_device_characteristics(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<()> {
	let (status, body) = request_json(
		app,
		cookie,
		"GET",
		format!("/api/cases/{case_id}/validation?profile=fda"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"validation precheck status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let target_drug_indexes: std::collections::BTreeSet<usize> = value
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
			format!("/api/cases/{case_id}/drugs/{drug_id}/device-characteristics"),
			None,
		)
		.await?;
		if status != StatusCode::OK {
			return Err(format!(
				"list device characteristics status {} body {}",
				status,
				String::from_utf8_lossy(&body)
			)
			.into());
		}
		let value: Value = serde_json::from_slice(&body)?;
		let Some(chars) = value.get("data").and_then(Value::as_array) else {
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
				format!(
					"/api/cases/{case_id}/drugs/{drug_id}/device-characteristics"
				),
				Some(serde_json::json!({
					"data": {
						"drug_id": drug_id,
						"sequence_number": next_sequence_number,
						"code": "FDA.G.k.12.r.3",
						"value_code": "1"
					}
				})),
			)
			.await?;
			if status != StatusCode::CREATED {
				return Err(format!(
					"create gk12r3 status {} body {}",
					status,
					String::from_utf8_lossy(&body)
				)
				.into());
			}
			let (status, body) = request_json(
				app,
				cookie,
				"GET",
				format!(
					"/api/cases/{case_id}/drugs/{drug_id}/device-characteristics"
				),
				None,
			)
			.await?;
			if status != StatusCode::OK {
				return Err(format!(
					"relist device characteristics status {} body {}",
					status,
					String::from_utf8_lossy(&body)
				)
				.into());
			}
			let value: Value = serde_json::from_slice(&body)?;
			let found = value
				.get("data")
				.and_then(Value::as_array)
				.map(|rows| {
					rows.iter().any(|row| {
						row.get("code")
							.and_then(Value::as_str)
							.map(|code| code.eq_ignore_ascii_case("FDA.G.k.12.r.3"))
							.unwrap_or(false)
					})
				})
				.unwrap_or(false);
			if !found {
				return Err(format!(
					"gk12r3 create persisted unexpected rows: {value}"
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
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::OK {
		let (validation_status, validation_body) = request_json(
			app,
			cookie,
			"GET",
			format!("/api/cases/{case_id}/validation?profile=fda"),
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

#[serial]
#[tokio::test]
async fn test_import_then_export_xml() -> Result<()> {
	init_test_env().await;
	let Some(examples_dir) = std::env::var("E2BR3_EXAMPLES_DIR")
		.ok()
		.map(std::path::PathBuf::from)
		.map(resolve_from_workspace)
	else {
		eprintln!("E2BR3_EXAMPLES_DIR not set; skipping xml import/export test");
		return Ok(());
	};
	std::env::set_var("E2BR3_XSD_PATH", resolved_xsd_path());

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "admin_pwd", "viewer_pwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let xml_path = examples_dir.join("FAERS2022Scenario2.xml");
	let mut xml = std::fs::read_to_string(xml_path)?;
	let unique_safety_report_id = format!("DSJP-TEST-{}", uuid::Uuid::new_v4());
	let marker = "extension=\"US-APHARMA-8744554B-UPDATE-TESTING222\"";
	if xml.contains(marker) {
		xml =
			xml.replace(marker, &format!("extension=\"{unique_safety_report_id}\""));
	} else {
		return Err("failed to locate safety_report_id marker in example XML".into());
	}

	let boundary = "X-BOUNDARY-XML-IMPORT";
	let body = format!(
		"--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"case.xml\"\r\nContent-Type: application/xml\r\n\r\n{xml}\r\n--{boundary}--\r\n"
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
	let value: Value = serde_json::from_slice(&body)?;
	let case_id = value
		.get("data")
		.and_then(|v| v.get("case_id").or_else(|| v.get("caseId")))
		.and_then(|v| v.as_str())
		.ok_or("missing case_id in import response")?;

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{case_id}/message-header"))
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"message header status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let message_number = value
		.get("data")
		.and_then(|v| v.get("message_number"))
		.and_then(|v| v.as_str())
		.unwrap_or_default();
	assert!(
		!message_number.is_empty(),
		"imported message header should include message_number"
	);

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{case_id}/patient"))
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"patient status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let patient_initials = value
		.get("data")
		.and_then(|v| v.get("patient_initials"))
		.and_then(|v| v.as_str())
		.unwrap_or_default();
	assert!(
		!patient_initials.is_empty(),
		"imported patient should include patient_initials"
	);

	ensure_reaction_language(&app, &cookie, case_id).await?;
	ensure_batch_transmission_date(&app, &cookie, case_id).await?;
	ensure_fda_device_characteristics(&app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id).await?;

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{case_id}/export/xml"))
		.header("cookie", cookie)
		.body(Body::empty())?;

	let res = app.oneshot(req).await?;
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
	let xml = String::from_utf8_lossy(&body);
	assert!(xml.contains("<MCCI_IN200100UV01"));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_import_update_dg_fields_then_export_contains_updates() -> Result<()> {
	init_test_env().await;
	let Some(examples_dir) = std::env::var("E2BR3_EXAMPLES_DIR")
		.ok()
		.map(std::path::PathBuf::from)
		.map(resolve_from_workspace)
	else {
		eprintln!(
			"E2BR3_EXAMPLES_DIR not set; skipping DG import/update/export test"
		);
		return Ok(());
	};
	std::env::set_var("E2BR3_XSD_PATH", resolved_xsd_path());

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "admin_pwd", "viewer_pwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let xml_path = examples_dir.join("FAERS2022Scenario2.xml");
	let xml = std::fs::read_to_string(xml_path)?;
	let boundary = "X-BOUNDARY-XML-IMPORT-DG";
	let body = format!(
		"--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"case.xml\"\r\nContent-Type: application/xml\r\n\r\n{xml}\r\n--{boundary}--\r\n"
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
		.and_then(|v| v.get("case_id").or_else(|| v.get("caseId")))
		.and_then(|v| v.as_str())
		.ok_or("missing case_id in import response")?
		.to_string();

	// Resolve primary drug/reaction IDs from imported case.
	let (status, body) = request_json(
		&app,
		&cookie,
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
	let drugs_value: Value = serde_json::from_slice(&body)?;
	let drug_id = drugs_value
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing first drug id")?
		.to_string();

	// DG sentinels for assertions.
	let sentinel_indication = format!("RTDG15-{}", Uuid::new_v4().simple());
	let sentinel_substance = format!("RTDG21-{}", Uuid::new_v4().simple());
	let sentinel_batch = format!("RTDG32-{}", Uuid::new_v4().simple());

	// Upsert active substance row #1.
	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list active-substances status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let substance_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing active substance id")?
		.to_string();
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!(
			"/api/cases/{case_id}/drugs/{drug_id}/active-substances/{substance_id}"
		),
		Some(serde_json::json!({
			"data": {
				"substance_name": sentinel_substance,
				"strength_value": 5,
				"strength_unit": "mg"
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update active-substance status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	// Upsert dosage row #1.
	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/drugs/{drug_id}/dosages"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list dosages status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let dosage_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing dosage id")?
		.to_string();
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/drugs/{drug_id}/dosages/{dosage_id}"),
		Some(serde_json::json!({
			"data": {
				"dose_value": 10.5,
				"dose_unit": "mg",
				"first_administration_date": "2024-01-02",
				"last_administration_date": "2024-01-02",
				"duration_value": 4,
				"duration_unit": "801",
				"batch_lot_number": sentinel_batch
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update dosage status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	// Upsert indication row #1.
	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/drugs/{drug_id}/indications"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list indications status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let indication_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing indication id")?
		.to_string();
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/drugs/{drug_id}/indications/{indication_id}"),
		Some(serde_json::json!({
			"data": {
				"indication_text": sentinel_indication,
				"indication_meddra_version": "25.0",
				"indication_meddra_code": "10019211"
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update indication status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	ensure_reaction_language(&app, &cookie, &case_id).await?;
	ensure_batch_transmission_date(&app, &cookie, &case_id).await?;
	ensure_fda_device_characteristics(&app, &cookie, &case_id).await?;
	mark_case_validated(&app, &cookie, &case_id).await?;

	// Export and assert updated DG values are present in XML.
	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{case_id}/export/xml"))
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
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
	let xml = String::from_utf8_lossy(&body);
	for expected in [
		sentinel_indication.as_str(),
		sentinel_substance.as_str(),
		"mg",
		"20240102",
		"801",
		sentinel_batch.as_str(),
	] {
		assert!(
			xml.contains(expected),
			"expected DG export XML to contain `{expected}`"
		);
	}
	assert!(
		xml.contains(drug_id.as_str()),
		"expected DG export XML to contain current drug id root"
	);
	assert!(
		!xml.contains("68d6f5ce-3b3b-45c7-92dd-69e06730c3a9"),
		"expected DG export XML to exclude stale template product root ids"
	);
	for stale in [
		"68d6f5ce-3b3b-45c7-92dd-69e06730c3a9",
		"59d6f5ce-3b3b-45c7-92dd-69e06730c2b7",
		"40d6f5ce-3b3b-45c7-92dd-69e06730c2b6",
		"154eb889-958b-45f2-a02f-42d4d6f4657f",
		"154eb889-958b-45f2-a02f-42d4d6f4555f",
	] {
		assert!(
			!xml.contains(stale),
			"expected DG export XML to exclude stale template root id `{stale}`"
		);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_fda_export_always_validates_even_when_env_unset() -> Result<()> {
	let original = std::env::var("E2BR3_EXPORT_VALIDATE").ok();
	std::env::remove_var("E2BR3_EXPORT_VALIDATE");

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "admin_pwd", "viewer_pwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	// Create a bare FDA case with minimal data so XML export requires validation gate.
	let create_body = serde_json::json!({
		"data": {
			"organization_id": seed.org_id,
			"safety_report_id": format!("SR-{}", Uuid::new_v4()),
			"status": "draft",
			"validation_profile": "fda"
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri("/api/cases")
		.header("content-type", "application/json")
		.header("cookie", cookie.clone())
		.body(Body::from(create_body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let case_id = value
		.get("data")
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing data.id in case create response")?;

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{case_id}/export/xml"))
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	assert_eq!(
		status,
		StatusCode::BAD_REQUEST,
		"expected FDA export to fail validation, got status {} body {}",
		status,
		String::from_utf8_lossy(&body)
	);

	match original {
		Some(v) => std::env::set_var("E2BR3_EXPORT_VALIDATE", v),
		None => std::env::remove_var("E2BR3_EXPORT_VALIDATE"),
	}
	Ok(())
}
