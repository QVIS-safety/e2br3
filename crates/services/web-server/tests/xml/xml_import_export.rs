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

fn extract_data_id(body: &[u8]) -> Result<String> {
	let value: Value = serde_json::from_slice(body)?;
	let id = value
		.get("data")
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing data.id")?;
	Ok(id.to_string())
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
		.and_then(|v| v.get("case_id"))
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

	let update_body = serde_json::json!({
		"data": {
			"status": "validated"
		}
	});
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/cases/{case_id}"))
		.header("content-type", "application/json")
		.header("cookie", cookie.clone())
		.body(Body::from(update_body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

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
		.and_then(|v| v.get("case_id"))
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

	let (status, body) = request_json(
		&app,
		&cookie,
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
	let reactions_value: Value = serde_json::from_slice(&body)?;
	let reaction_id = reactions_value
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing first reaction id")?
		.to_string();

	// DG sentinels for assertions.
	let sentinel_indication = format!("RTDG15-{}", Uuid::new_v4().simple());
	let sentinel_substance = format!("RTDG21-{}", Uuid::new_v4().simple());
	let sentinel_batch = format!("RTDG32-{}", Uuid::new_v4().simple());
	let sentinel_source = "PHARMACEUTICAL COMPANY";
	let sentinel_method = "Global Introspection";
	let sentinel_result = "Not Suspected";

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
				"indication_meddra_version": "25.0"
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

	// Ensure reaction assessment exists for first reaction.
	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list reaction-assessments status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let assessment_rows = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.cloned()
		.unwrap_or_default();
	let mut assessment_id = assessment_rows
		.iter()
		.find(|v| {
			v.get("reaction_id").and_then(|x| x.as_str())
				== Some(reaction_id.as_str())
		})
		.and_then(|v| v.get("id").and_then(|x| x.as_str()))
		.map(|s| s.to_string());
	if assessment_id.is_none() {
		let (status, body) = request_json(
			&app,
			&cookie,
			"POST",
			format!("/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments"),
			Some(serde_json::json!({
				"data": { "drug_id": drug_id, "reaction_id": reaction_id }
			})),
		)
		.await?;
		if status != StatusCode::CREATED {
			return Err(format!(
				"create reaction-assessment status {} body {}",
				status,
				String::from_utf8_lossy(&body)
			)
			.into());
		}
		assessment_id = Some(extract_data_id(&body)?);
	}
	let assessment_id = assessment_id.ok_or("missing assessment id")?;

	// Upsert relatedness row #1.
	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!(
			"/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness"
		),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list relatedness status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let existing_relatedness = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.cloned()
		.unwrap_or_default();
	let relatedness_id = existing_relatedness
		.iter()
		.find(|v| v.get("sequence_number").and_then(|x| x.as_i64()) == Some(1))
		.and_then(|v| v.get("id").and_then(|x| x.as_str()))
		.map(|s| s.to_string());

	let relatedness_payload = serde_json::json!({
		"data": {
			"sequence_number": 1,
			"source_of_assessment": sentinel_source,
			"method_of_assessment": sentinel_method,
			"result_of_assessment": sentinel_result
		}
	});
	let (status, body) = if let Some(id) = relatedness_id {
		request_json(
			&app,
			&cookie,
			"PUT",
			format!(
				"/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness/{id}"
			),
			Some(relatedness_payload),
		)
		.await?
	} else {
		request_json(
			&app,
			&cookie,
			"POST",
			format!(
				"/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness"
			),
			Some(relatedness_payload),
		)
		.await?
	};
	if status != StatusCode::OK && status != StatusCode::CREATED {
		return Err(format!(
			"upsert relatedness status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	// Set validated status for export gate (avoids validator token requirement).
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}"),
		Some(serde_json::json!({
			"data": { "status": "validated" }
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update case status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

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
		sentinel_source,
		sentinel_method,
		sentinel_result,
	] {
		assert!(
			xml.contains(expected),
			"expected DG export XML to contain `{expected}`"
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
