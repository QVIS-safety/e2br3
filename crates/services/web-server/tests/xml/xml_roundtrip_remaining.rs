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

async fn setup_imported_case() -> Result<(axum::Router, String, String)> {
	init_test_env().await;
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
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let xml_path = examples_dir.join("FAERS2022Scenario2.xml");
	let xml = std::fs::read_to_string(xml_path)?;
	let boundary = "X-BOUNDARY-XML-IMPORT-REMAINING";
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
	ensure_reaction_language(&app, &cookie, &case_id).await?;

	Ok((app, cookie, case_id))
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
			if status != StatusCode::OK {
				return Err(format!(
					"update reaction language status {} body {}",
					status,
					String::from_utf8_lossy(&body)
				)
				.into());
			}
		}
	}
	Ok(())
}

async fn set_validated(
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
		return Err(format!(
			"mark validated status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	Ok(())
}

async fn export_xml(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<String> {
	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{case_id}/export/xml"))
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

#[serial]
#[tokio::test]
async fn test_roundtrip_dm_dh_remaining_fields() -> Result<()> {
	let (app, cookie, case_id) = match setup_imported_case().await {
		Ok(v) => v,
		Err(err) => {
			eprintln!("skipping DM/DH remaining test: {err}");
			return Ok(());
		}
	};

	// DM.D.6
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/patient"),
		Some(serde_json::json!({
			"data": { "last_menstrual_period_date": "2024-02-03" }
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update patient status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	// DM.D.10.2.1 + D.10.3
	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/patient/parents"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list parents status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let parent_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.map(|s| s.to_string());

	let parent_id = if let Some(id) = parent_id {
		id
	} else {
		let (status, body) = request_json(
			&app,
			&cookie,
			"POST",
			format!("/api/cases/{case_id}/patient/parents"),
			Some(serde_json::json!({
				"data": {
					"patient_id": Uuid::nil(),
					"sex": "2",
					"medical_history_text": "RTDM-PARENT"
				}
			})),
		)
		.await?;
		if status != StatusCode::CREATED {
			return Err(format!(
				"create parent status {} body {}",
				status,
				String::from_utf8_lossy(&body)
			)
			.into());
		}
		extract_data_id(&body)?
	};

	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/patient/parents/{parent_id}"),
		Some(serde_json::json!({
			"data": {
				"parent_age": 37.7,
				"parent_age_unit": "801",
				"parent_birth_date": "1988-11-23"
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update parent status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	// DH.D.8.r.4 + D.8.r.5
	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/patient/past-drugs"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list past-drugs status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let past_drug_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.map(|s| s.to_string());
	let past_drug_id = if let Some(id) = past_drug_id {
		id
	} else {
		let (status, body) = request_json(
			&app,
			&cookie,
			"POST",
			format!("/api/cases/{case_id}/patient/past-drugs"),
			Some(serde_json::json!({
				"data": {
					"patient_id": Uuid::nil(),
					"sequence_number": 1,
					"drug_name": format!("RTDH-{}", Uuid::new_v4().simple())
				}
			})),
		)
		.await?;
		if status != StatusCode::CREATED {
			return Err(format!(
				"create past-drug status {} body {}",
				status,
				String::from_utf8_lossy(&body)
			)
			.into());
		}
		extract_data_id(&body)?
	};

	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/patient/past-drugs/{past_drug_id}"),
		Some(serde_json::json!({
			"data": {
				"start_date": "2024-03-04",
				"end_date": "2024-03-05"
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update past-drug status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	set_validated(&app, &cookie, &case_id).await?;
	let xml = export_xml(&app, &cookie, &case_id).await?;
	for expected in ["20240203", "37.7", "19881123", "20240304", "20240305"] {
		assert!(
			xml.contains(expected),
			"expected DM/DH export XML to contain `{expected}`"
		);
	}
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_roundtrip_ae_remaining_fields() -> Result<()> {
	let (app, cookie, case_id) = match setup_imported_case().await {
		Ok(v) => v,
		Err(err) => {
			eprintln!("skipping AE remaining test: {err}");
			return Ok(());
		}
	};

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
	let reaction_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.map(|s| s.to_string());
	let reaction_id = if let Some(id) = reaction_id {
		id
	} else {
		let sentinel = format!("RTAE1-{}", Uuid::new_v4().simple());
		let (status, body) = request_json(
			&app,
			&cookie,
			"POST",
			format!("/api/cases/{case_id}/reactions"),
			Some(serde_json::json!({
				"data": { "sequence_number": 1, "primary_source_reaction": sentinel }
			})),
		)
		.await?;
		if status != StatusCode::CREATED {
			return Err(format!(
				"create reaction status {} body {}",
				status,
				String::from_utf8_lossy(&body)
			)
			.into());
		}
		extract_data_id(&body)?
	};

	let sentinel_text = format!("RTAE1-{}", Uuid::new_v4().simple());
	let sentinel_required_intervention = "false";
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/reactions/{reaction_id}"),
		Some(serde_json::json!({
			"data": {
				"primary_source_reaction": sentinel_text,
				"reaction_language": "en",
				"reaction_meddra_code": "10012345",
				"reaction_meddra_version": "27.0",
				"required_intervention": sentinel_required_intervention,
				"start_date": "2024-04-06",
				"end_date": "2024-04-07",
				"outcome": "2"
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update reaction status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	set_validated(&app, &cookie, &case_id).await?;
	let xml = export_xml(&app, &cookie, &case_id).await?;
	for expected in [
		sentinel_text.as_str(),
		"10012345",
		"27.0",
		"20240406",
		"20240407",
	] {
		assert!(
			xml.contains(expected),
			"expected AE export XML to contain `{expected}`"
		);
	}
	let ri_code = "code=\"7\" codeSystem=\"2.16.840.1.113883.3.989.5.1.2.2.1.3\"";
	let ri_pos = xml
		.find(ri_code)
		.ok_or("missing required intervention code node")?;
	let ri_end = core::cmp::min(ri_pos + 280, xml.len());
	let ri_window = &xml[ri_pos..ri_end];
	assert!(
		ri_window.contains("value=\"false\""),
		"expected required intervention node window to contain value=\"false\", got: {ri_window}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_roundtrip_dg_remaining_14_fields() -> Result<()> {
	let (app, cookie, case_id) = match setup_imported_case().await {
		Ok(v) => v,
		Err(err) => {
			eprintln!("skipping DG remaining test: {err}");
			return Ok(());
		}
	};

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
	let drug_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing first drug id")?
		.to_string();

	let sentinel_indication_text = format!("RTDG15-{}", Uuid::new_v4().simple());
	let sentinel_substance = format!("RTDG21-{}", Uuid::new_v4().simple());
	let sentinel_batch = format!("RTDG32-{}", Uuid::new_v4().simple());

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
				"indication_text": sentinel_indication_text,
				"indication_meddra_version": "26.1",
				"indication_meddra_code": "10054321"
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
				"strength_unit": "g"
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
				"dose_unit": "ml",
				"first_administration_date": "2024-01-02",
				"last_administration_date": "2024-01-03",
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

	set_validated(&app, &cookie, &case_id).await?;
	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{case_id}/export/xml"))
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
	let xml = String::from_utf8_lossy(&body);
	for expected in [
		sentinel_indication_text.as_str(),
		"26.1",
		"10054321",
		sentinel_substance.as_str(),
		"g",
		"ml",
		"20240102",
		"20240103",
		"801",
		sentinel_batch.as_str(),
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
async fn test_roundtrip_ci_si_fields() -> Result<()> {
	let (app, cookie, case_id) = setup_imported_case().await?;

	let sentinel_wuid = format!("RTCIW-{}", Uuid::new_v4().simple());
	let sentinel_null_reason = format!("RTCINULL-{}", Uuid::new_v4().simple());
	let sentinel_batch = format!("RTSI-BATCH-{}", Uuid::new_v4().simple());
	let sentinel_batch_sender = format!("RTSI-BS-{}", Uuid::new_v4().simple());
	let sentinel_batch_receiver = format!("RTSI-BR-{}", Uuid::new_v4().simple());
	let sentinel_msg_number = format!("RTSI-MSG-{}", Uuid::new_v4().simple());
	let sentinel_msg_sender = format!("RTSI-MS-{}", Uuid::new_v4().simple());
	let sentinel_msg_receiver = format!("RTSI-MR-{}", Uuid::new_v4().simple());

	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/safety-report"),
		Some(serde_json::json!({
			"data": {
				"transmission_date": "2024-06-01",
				"report_type": "2",
				"date_first_received_from_source": "2024-05-01",
				"date_of_most_recent_information": "2024-06-01",
				"fulfil_expedited_criteria": true,
				"local_criteria_report_type": "1",
				"combination_product_report_indicator": "2",
				"worldwide_unique_id": sentinel_wuid,
				"nullification_reason": sentinel_null_reason
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update safety-report status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/message-header"),
		Some(serde_json::json!({
			"data": {
				"batch_number": sentinel_batch,
				"batch_sender_identifier": sentinel_batch_sender,
				"batch_receiver_identifier": sentinel_batch_receiver,
				"message_number": sentinel_msg_number,
				"message_sender_identifier": sentinel_msg_sender,
				"message_receiver_identifier": sentinel_msg_receiver,
				"message_date": "20240601112233"
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update message-header status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	set_validated(&app, &cookie, &case_id).await?;
	let xml = export_xml(&app, &cookie, &case_id).await?;
	for expected in [
		sentinel_wuid.as_str(),
		sentinel_null_reason.as_str(),
		sentinel_batch.as_str(),
		sentinel_batch_sender.as_str(),
		sentinel_batch_receiver.as_str(),
		sentinel_msg_number.as_str(),
		sentinel_msg_sender.as_str(),
		sentinel_msg_receiver.as_str(),
		"20240601112233",
	] {
		assert!(
			xml.contains(expected),
			"expected CI/SI export XML to contain `{expected}`"
		);
	}
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_roundtrip_rp_sd_fields() -> Result<()> {
	let (app, cookie, case_id) = setup_imported_case().await?;

	let sentinel_rp_given = format!("RTRP-G-{}", Uuid::new_v4().simple());
	let sentinel_rp_family = format!("RTRP-F-{}", Uuid::new_v4().simple());
	let sentinel_rp_org = format!("RTRP-O-{}", Uuid::new_v4().simple());
	let sentinel_sd_org = format!("RTSD-O-{}", Uuid::new_v4().simple());
	let sentinel_sd_given = format!("RTSD-G-{}", Uuid::new_v4().simple());
	let sentinel_sd_email = format!("{}@example.com", Uuid::new_v4().simple());

	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/safety-report/primary-sources"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list primary-sources status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let primary_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing primary source id")?
		.to_string();

	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/safety-report/primary-sources/{primary_id}"),
		Some(serde_json::json!({
			"data": {
				"reporter_given_name": sentinel_rp_given,
				"reporter_family_name": sentinel_rp_family,
				"organization": sentinel_rp_org,
				"street": "RP Street 11",
				"city": "RP City",
				"state": "RP State",
				"postcode": "12345",
				"telephone": "1234567890",
				"email": "rp@example.com",
				"country_code": "KR",
				"qualification": "1",
				"primary_source_regulatory": "1"
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update primary-source status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/safety-report/senders"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list senders status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let sender_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing sender id")?
		.to_string();
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/safety-report/senders/{sender_id}"),
		Some(serde_json::json!({
			"data": {
				"sender_type": "2",
				"organization_name": sentinel_sd_org,
				"person_given_name": sentinel_sd_given,
				"person_family_name": "SenderFamily",
				"street_address": "Sender Street 22",
				"city": "Sender City",
				"state": "Sender State",
				"postcode": "54321",
				"country_code": "US",
				"telephone": "3334445555",
				"email": sentinel_sd_email
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update sender status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	set_validated(&app, &cookie, &case_id).await?;
	let xml = export_xml(&app, &cookie, &case_id).await?;
	for expected in [
		sentinel_rp_given.as_str(),
		sentinel_rp_family.as_str(),
		sentinel_rp_org.as_str(),
		sentinel_sd_org.as_str(),
		sentinel_sd_given.as_str(),
		sentinel_sd_email.as_str(),
	] {
		assert!(
			xml.contains(expected),
			"expected RP/SD export XML to contain `{expected}`"
		);
	}
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_roundtrip_nr_fields() -> Result<()> {
	let (app, cookie, case_id) = setup_imported_case().await?;

	let sentinel_h1 = format!("RTNR-H1-{}", Uuid::new_v4().simple());
	let sentinel_h2 = format!("RTNR-H2-{}", Uuid::new_v4().simple());
	let sentinel_h4 = format!("RTNR-H4-{}", Uuid::new_v4().simple());
	let sentinel_h53 = format!("RTNR-H53-{}", Uuid::new_v4().simple());

	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/narrative"),
		Some(serde_json::json!({
			"data": {
				"case_narrative": sentinel_h1,
				"reporter_comments": sentinel_h2,
				"sender_comments": sentinel_h4
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update narrative status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/narrative/summaries"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list summaries status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let summary_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.map(|s| s.to_string());
	let summary_id = if let Some(id) = summary_id {
		id
	} else {
		let (status, body) = request_json(
			&app,
			&cookie,
			"POST",
			format!("/api/cases/{case_id}/narrative/summaries"),
			Some(serde_json::json!({
				"data": {
					"narrative_id": Uuid::nil(),
					"sequence_number": 1,
					"summary_text": sentinel_h53
				}
			})),
		)
		.await?;
		if status != StatusCode::CREATED {
			return Err(format!(
				"create summary status {} body {}",
				status,
				String::from_utf8_lossy(&body)
			)
			.into());
		}
		extract_data_id(&body)?
	};
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/narrative/summaries/{summary_id}"),
		Some(serde_json::json!({
			"data": {
				"summary_type": "2",
				"language_code": "en",
				"summary_text": sentinel_h53
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update summary status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	set_validated(&app, &cookie, &case_id).await?;
	let xml = export_xml(&app, &cookie, &case_id).await?;
	for expected in [
		sentinel_h1.as_str(),
		sentinel_h2.as_str(),
		sentinel_h4.as_str(),
		sentinel_h53.as_str(),
	] {
		assert!(
			xml.contains(expected),
			"expected NR export XML to contain `{expected}`"
		);
	}
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_api_persistence_ae_sd_all_fields() -> Result<()> {
	let (app, cookie, case_id) = match setup_imported_case().await {
		Ok(v) => v,
		Err(err) => {
			eprintln!("skipping AE/SD API persistence test: {err}");
			return Ok(());
		}
	};

	// AE: ensure there is a reaction, then update all AE fields via API.
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
	let reaction_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.map(|s| s.to_string());
	let reaction_id = if let Some(id) = reaction_id {
		id
	} else {
		let sentinel = format!("RTAE-CREATE-{}", Uuid::new_v4().simple());
		let (status, body) = request_json(
			&app,
			&cookie,
			"POST",
			format!("/api/cases/{case_id}/reactions"),
			Some(serde_json::json!({
				"data": { "sequence_number": 1, "primary_source_reaction": sentinel }
			})),
		)
		.await?;
		if status != StatusCode::CREATED {
			return Err(format!(
				"create reaction status {} body {}",
				status,
				String::from_utf8_lossy(&body)
			)
			.into());
		}
		extract_data_id(&body)?
	};

	let ae_primary = format!("RTAE-PSR-{}", Uuid::new_v4().simple());
	let ae_translation = format!("RTAE-TR-{}", Uuid::new_v4().simple());
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/reactions/{reaction_id}"),
		Some(serde_json::json!({
			"data": {
				"primary_source_reaction": ae_primary,
				"primary_source_reaction_translation": ae_translation,
				"reaction_language": "fr",
				"reaction_meddra_code": "10012345",
				"reaction_meddra_version": "27.0",
				"term_highlighted": true,
				"serious": true,
				"criteria_death": true,
				"criteria_life_threatening": false,
				"criteria_hospitalization": true,
				"criteria_disabling": true,
				"criteria_congenital_anomaly": false,
				"criteria_other_medically_important": true,
				"required_intervention": "false",
				"start_date": "2024-04-06",
				"end_date": "2024-04-07",
				"duration_value": 5.5,
				"duration_unit": "804",
				"outcome": "2",
				"medical_confirmation": true,
				"country_code": "US"
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update reaction status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/reactions/{reaction_id}"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"get reaction status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let reaction = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.cloned()
		.ok_or("missing reaction data")?;
	let mut ae_mismatches: Vec<String> = Vec::new();
	let reaction_str = |key: &str| -> String {
		reaction
			.get(key)
			.and_then(|v| v.as_str())
			.unwrap_or_default()
			.to_string()
	};
	let reaction_bool =
		|key: &str| -> Option<bool> { reaction.get(key).and_then(|v| v.as_bool()) };
	let check_date = |field: &str,
	                  primary: &str,
	                  fallback: &str,
	                  expected_str: &str,
	                  expected_year: i64,
	                  expected_ordinal: i64,
	                  out: &mut Vec<String>| {
		let value = reaction.get(primary).or_else(|| reaction.get(fallback));
		let ok = match value {
			Some(v) if v.as_str() == Some(expected_str) => true,
			Some(v) => v
				.as_array()
				.and_then(|arr| {
					let y = arr.first()?.as_i64()?;
					let ord = arr.get(1)?.as_i64()?;
					Some(y == expected_year && ord == expected_ordinal)
				})
				.unwrap_or(false),
			None => false,
		};
		if !ok {
			out.push(format!(
					"{field}: expected `{expected_str}` or [{expected_year},{expected_ordinal}], got `{:?}`",
					value
				));
		}
	};
	let check_str =
		|field: &str, actual: String, expected: &str, out: &mut Vec<String>| {
			if actual != expected {
				out.push(format!("{field}: expected `{expected}`, got `{actual}`"));
			}
		};
	let check_bool = |field: &str,
	                  actual: Option<bool>,
	                  expected: bool,
	                  out: &mut Vec<String>| {
		if actual != Some(expected) {
			out.push(format!(
				"{field}: expected `{expected}`, got `{:?}`",
				actual
			));
		}
	};

	check_str(
		"primary_source_reaction",
		reaction_str("primary_source_reaction"),
		&ae_primary,
		&mut ae_mismatches,
	);
	check_str(
		"reaction_language",
		reaction_str("reaction_language"),
		"fr",
		&mut ae_mismatches,
	);
	check_str(
		"primary_source_reaction_translation",
		reaction_str("primary_source_reaction_translation"),
		&ae_translation,
		&mut ae_mismatches,
	);
	check_str(
		"reaction_meddra_code",
		reaction_str("reaction_meddra_code"),
		"10012345",
		&mut ae_mismatches,
	);
	check_str(
		"reaction_meddra_version",
		reaction_str("reaction_meddra_version"),
		"27.0",
		&mut ae_mismatches,
	);
	check_bool(
		"term_highlighted",
		reaction_bool("term_highlighted"),
		true,
		&mut ae_mismatches,
	);
	check_bool(
		"serious",
		reaction_bool("serious"),
		true,
		&mut ae_mismatches,
	);
	check_bool(
		"criteria_death",
		reaction_bool("criteria_death"),
		true,
		&mut ae_mismatches,
	);
	check_bool(
		"criteria_life_threatening",
		reaction_bool("criteria_life_threatening"),
		false,
		&mut ae_mismatches,
	);
	check_bool(
		"criteria_hospitalization",
		reaction_bool("criteria_hospitalization"),
		true,
		&mut ae_mismatches,
	);
	check_bool(
		"criteria_disabling",
		reaction_bool("criteria_disabling"),
		true,
		&mut ae_mismatches,
	);
	check_bool(
		"criteria_congenital_anomaly",
		reaction_bool("criteria_congenital_anomaly"),
		false,
		&mut ae_mismatches,
	);
	check_bool(
		"criteria_other_medically_important",
		reaction_bool("criteria_other_medically_important"),
		true,
		&mut ae_mismatches,
	);
	check_str(
		"required_intervention",
		reaction_str("required_intervention"),
		"false",
		&mut ae_mismatches,
	);
	check_date(
		"start_date",
		"start_date",
		"reaction_start_date",
		"2024-04-06",
		2024,
		97,
		&mut ae_mismatches,
	);
	check_date(
		"end_date",
		"end_date",
		"reaction_end_date",
		"2024-04-07",
		2024,
		98,
		&mut ae_mismatches,
	);
	check_str(
		"duration_unit",
		reaction_str("duration_unit"),
		"804",
		&mut ae_mismatches,
	);
	check_str("outcome", reaction_str("outcome"), "2", &mut ae_mismatches);
	check_bool(
		"medical_confirmation",
		reaction_bool("medical_confirmation"),
		true,
		&mut ae_mismatches,
	);
	check_str(
		"country_code",
		reaction_str("country_code"),
		"US",
		&mut ae_mismatches,
	);

	// SD: update all sender fields via API, then verify persisted values from GET.
	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/safety-report/senders"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list senders status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let sender_id = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.map(|s| s.to_string());
	let sender_id = if let Some(id) = sender_id {
		id
	} else {
		let (status, body) = request_json(
			&app,
			&cookie,
			"POST",
			format!("/api/cases/{case_id}/safety-report/senders"),
			Some(serde_json::json!({
				"data": {
					"case_id": case_id,
					"sender_type": "2",
					"organization_name": "RTSD-CREATE"
				}
			})),
		)
		.await?;
		if status != StatusCode::CREATED {
			return Err(format!(
				"create sender status {} body {}",
				status,
				String::from_utf8_lossy(&body)
			)
			.into());
		}
		extract_data_id(&body)?
	};

	let sd_org = format!("RTSD-ORG-{}", Uuid::new_v4().simple());
	let sd_dept = format!("RTSD-DEPT-{}", Uuid::new_v4().simple());
	let sd_title = format!("RTSD-TITLE-{}", Uuid::new_v4().simple());
	let sd_given = format!("RTSD-GIVEN-{}", Uuid::new_v4().simple());
	let sd_middle = format!("RTSD-MID-{}", Uuid::new_v4().simple());
	let sd_family = format!("RTSD-FAM-{}", Uuid::new_v4().simple());
	let sd_street = format!("RTSD-STREET-{}", Uuid::new_v4().simple());
	let sd_city = format!("RTSD-CITY-{}", Uuid::new_v4().simple());
	let sd_email = format!("{}@example.com", Uuid::new_v4().simple());
	let (status, body) = request_json(
		&app,
		&cookie,
		"PUT",
		format!("/api/cases/{case_id}/safety-report/senders/{sender_id}"),
		Some(serde_json::json!({
			"data": {
				"sender_type": "2",
				"organization_name": sd_org,
				"department": sd_dept,
				"street_address": sd_street,
				"city": sd_city,
				"state": "CA",
				"postcode": "90210",
				"country_code": "US",
				"person_title": sd_title,
				"person_given_name": sd_given,
				"person_middle_name": sd_middle,
				"person_family_name": sd_family,
				"telephone": "15550101234",
				"fax": "15550109999",
				"email": sd_email
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update sender status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	let (status, body) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/safety-report/senders/{sender_id}"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"get sender status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let sender = serde_json::from_slice::<Value>(&body)?
		.get("data")
		.cloned()
		.ok_or("missing sender data")?;
	let mut sd_mismatches: Vec<String> = Vec::new();
	let sender_str = |key: &str| -> String {
		sender
			.get(key)
			.and_then(|v| v.as_str())
			.unwrap_or_default()
			.to_string()
	};
	check_str(
		"sender_type",
		sender_str("sender_type"),
		"2",
		&mut sd_mismatches,
	);
	check_str(
		"organization_name",
		sender_str("organization_name"),
		&sd_org,
		&mut sd_mismatches,
	);
	check_str(
		"department",
		sender_str("department"),
		&sd_dept,
		&mut sd_mismatches,
	);
	check_str(
		"street_address",
		sender_str("street_address"),
		&sd_street,
		&mut sd_mismatches,
	);
	check_str("city", sender_str("city"), &sd_city, &mut sd_mismatches);
	check_str("state", sender_str("state"), "CA", &mut sd_mismatches);
	check_str(
		"postcode",
		sender_str("postcode"),
		"90210",
		&mut sd_mismatches,
	);
	check_str(
		"country_code",
		sender_str("country_code"),
		"US",
		&mut sd_mismatches,
	);
	check_str(
		"person_title",
		sender_str("person_title"),
		&sd_title,
		&mut sd_mismatches,
	);
	check_str(
		"person_given_name",
		sender_str("person_given_name"),
		&sd_given,
		&mut sd_mismatches,
	);
	check_str(
		"person_middle_name",
		sender_str("person_middle_name"),
		&sd_middle,
		&mut sd_mismatches,
	);
	check_str(
		"person_family_name",
		sender_str("person_family_name"),
		&sd_family,
		&mut sd_mismatches,
	);
	check_str(
		"telephone",
		sender_str("telephone"),
		"15550101234",
		&mut sd_mismatches,
	);
	check_str("fax", sender_str("fax"), "15550109999", &mut sd_mismatches);
	check_str("email", sender_str("email"), &sd_email, &mut sd_mismatches);

	if !ae_mismatches.is_empty() || !sd_mismatches.is_empty() {
		return Err(format!(
			"AE mismatches: {:?}; SD mismatches: {:?}; reaction={}; sender={}",
			ae_mismatches, sd_mismatches, reaction, sender
		)
		.into());
	}

	// Also verify API-persisted AE/SD values are reflected in exported XML.
	set_validated(&app, &cookie, &case_id).await?;
	let xml = export_xml(&app, &cookie, &case_id).await?;
	for expected in [
		ae_primary.as_str(),
		ae_translation.as_str(),
		"10012345",
		"27.0",
		"20240406",
		"20240407",
		sd_org.as_str(),
		sd_dept.as_str(),
		sd_street.as_str(),
		sd_city.as_str(),
		sd_title.as_str(),
		sd_given.as_str(),
		sd_middle.as_str(),
		sd_family.as_str(),
		sd_email.as_str(),
	] {
		assert!(
			xml.contains(expected),
			"expected AE/SD export XML to contain `{expected}`"
		);
	}

	Ok(())
}
