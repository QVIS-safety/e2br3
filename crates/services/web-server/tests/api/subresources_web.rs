use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

fn extract_id(body: &[u8]) -> Result<Uuid> {
	let value: Value = serde_json::from_slice(body)?;
	let id = value
		.get("data")
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing data.id")?;
	Ok(Uuid::parse_str(id)?)
}

async fn post_json(
	app: &Router,
	cookie: &str,
	uri: String,
	body: Value,
) -> Result<(StatusCode, Vec<u8>)> {
	let req = Request::builder()
		.method("POST")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?.to_vec();
	Ok((status, body))
}

async fn get_json(
	app: &Router,
	cookie: &str,
	uri: String,
) -> Result<(StatusCode, Vec<u8>)> {
	let req = Request::builder()
		.method("GET")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?.to_vec();
	Ok((status, body))
}

async fn create_case(app: &Router, cookie: &str, org_id: Uuid) -> Result<Uuid> {
	let body = json!({
		"data": {
			"organization_id": org_id,
			"safety_report_id": format!("SR-{}", Uuid::new_v4()),
			"status": "draft"
		}
	});
	let (status, body) =
		post_json(app, cookie, "/api/cases".to_string(), body).await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create case status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	extract_id(&body)
}

async fn create_patient(app: &Router, cookie: &str, case_id: Uuid) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"patient_initials": "AB",
			"sex": "1"
		}
	});
	let (status, body) =
		post_json(app, cookie, format!("/api/cases/{case_id}/patient"), body)
			.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create patient status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	extract_id(&body)
}

async fn create_narrative(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"case_narrative": "test narrative"
		}
	});
	let (status, body) =
		post_json(app, cookie, format!("/api/cases/{case_id}/narrative"), body)
			.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create narrative status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	extract_id(&body)
}

async fn create_receiver(app: &Router, cookie: &str, case_id: Uuid) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"receiver_type": "1",
			"organization_name": "Receiver Org A"
		}
	});
	let (status, body) =
		post_json(app, cookie, format!("/api/cases/{case_id}/receiver"), body)
			.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create receiver status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	extract_id(&body)
}

async fn create_safety_report(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"transmission_date": [2024, 1],
			"report_type": "1",
			"date_first_received_from_source": [2024, 1],
			"date_of_most_recent_information": [2024, 1],
			"fulfil_expedited_criteria": false
		}
	});
	let (status, body) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/safety-report"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create safety report status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	extract_id(&body)
}

#[serial]
#[tokio::test]
async fn test_narrative_rejects_legacy_additional_fields() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let body = json!({"data": {
		"case_id": case_id,
		"case_narrative": "test narrative",
		"case_summary": "legacy additional narrative field"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::UNPROCESSABLE_ENTITY,
		"status={status} body={}",
		String::from_utf8_lossy(&body)
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_singleton_post_endpoints_are_idempotent() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let msg_a = format!("MSG-A-{case_id}");
	let msg_b = format!("MSG-B-{case_id}");

	// message header
	let body = json!({"data": {
		"case_id": case_id,
		"message_number": msg_a,
		"message_sender_identifier": "SEND-A",
		"message_receiver_identifier": "RECV-A",
		"message_date": "20240201010101"
	}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/message-header"),
		body,
	)
	.await?;
	assert!(
		status == StatusCode::CREATED || status == StatusCode::OK,
		"status={status}"
	);
	let body = json!({"data": {
		"case_id": case_id,
		"message_number": msg_b,
		"message_sender_identifier": "SEND-B",
		"message_receiver_identifier": "RECV-B",
		"message_date": "20240202020202"
	}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/message-header"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/message-header"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	let value: Value = serde_json::from_slice(&body)?;
	assert!(value["data"]["id"].as_str().is_some(), "{value:?}");

	// patient
	create_patient(&app, &cookie, case_id).await?;
	let body = json!({"data": {
		"case_id": case_id,
		"patient_initials": "CD",
		"sex": "2"
	}});
	let (status, _) =
		post_json(&app, &cookie, format!("/api/cases/{case_id}/patient"), body)
			.await?;
	assert_eq!(status, StatusCode::OK);
	let (status, body) =
		get_json(&app, &cookie, format!("/api/cases/{case_id}/patient")).await?;
	assert_eq!(status, StatusCode::OK);
	let value: Value = serde_json::from_slice(&body)?;
	assert!(value["data"]["id"].as_str().is_some(), "{value:?}");

	// receiver
	create_receiver(&app, &cookie, case_id).await?;
	let body = json!({"data": {
		"case_id": case_id,
		"receiver_type": "2",
		"organization_name": "Receiver Org B"
	}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/receiver"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	let (status, body) =
		get_json(&app, &cookie, format!("/api/cases/{case_id}/receiver")).await?;
	assert_eq!(status, StatusCode::OK);
	let value: Value = serde_json::from_slice(&body)?;
	assert!(value["data"]["id"].as_str().is_some(), "{value:?}");

	// narrative
	create_narrative(&app, &cookie, case_id).await?;
	let body = json!({"data": {
		"case_id": case_id,
		"case_narrative": "updated narrative"
	}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	let (status, body) =
		get_json(&app, &cookie, format!("/api/cases/{case_id}/narrative")).await?;
	assert_eq!(status, StatusCode::OK);
	let value: Value = serde_json::from_slice(&body)?;
	assert!(value["data"]["id"].as_str().is_some(), "{value:?}");

	// safety report
	create_safety_report(&app, &cookie, case_id).await?;
	let body = json!({"data": {
		"case_id": case_id,
		"transmission_date": [2025, 1],
		"report_type": "2",
		"date_first_received_from_source": [2025, 1],
		"date_of_most_recent_information": [2025, 1],
		"fulfil_expedited_criteria": true
	}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	let (status, body) =
		get_json(&app, &cookie, format!("/api/cases/{case_id}/safety-report"))
			.await?;
	assert_eq!(status, StatusCode::OK);
	let value: Value = serde_json::from_slice(&body)?;
	assert!(value["data"]["id"].as_str().is_some(), "{value:?}");

	Ok(())
}

async fn create_drug(app: &Router, cookie: &str, case_id: Uuid) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"drug_characterization": "1",
			"medicinal_product": "Test Drug"
		}
	});
	let (status, body) =
		post_json(app, cookie, format!("/api/cases/{case_id}/drugs"), body).await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create drug status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	extract_id(&body)
}

async fn create_reaction(app: &Router, cookie: &str, case_id: Uuid) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"primary_source_reaction": "Headache"
		}
	});
	let (status, body) =
		post_json(app, cookie, format!("/api/cases/{case_id}/reactions"), body)
			.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create reaction status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	extract_id(&body)
}

async fn create_reaction_assessment(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	drug_id: Uuid,
	reaction_id: Uuid,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"drug_id": drug_id,
			"reaction_id": reaction_id
		}
	});
	let (status, body) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create reaction assessment status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	extract_id(&body)
}

#[serial]
#[tokio::test]
async fn test_patient_subresources_endpoints_ok() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let patient_id = create_patient(&app, &cookie, case_id).await?;

	// Medical history episode
	let body = json!({"data": {"patient_id": patient_id, "sequence_number": 1, "meddra_code": "100"}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/medical-history"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/medical-history"),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"medical-history list status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let data = value
		.get("data")
		.and_then(|v| v.as_array())
		.ok_or("missing data array")?;
	assert!(!data.is_empty());

	// Past drug history
	let body = json!({"data": {"patient_id": patient_id, "sequence_number": 1, "drug_name": "Test"}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/past-drugs"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/past-drugs"),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"past-drugs list status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let data = value
		.get("data")
		.and_then(|v| v.as_array())
		.ok_or("missing data array")?;
	assert!(!data.is_empty());

	// Death info
	let body =
		json!({"data": {"patient_id": patient_id, "date_of_death": [2024, 1]}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/death-info"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let death_info_id = extract_id(&body)?;

	// Reported cause of death
	let body = json!({"data": {"death_info_id": death_info_id, "sequence_number": 1, "meddra_code": "100"}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);

	// Autopsy cause of death
	let body = json!({"data": {"death_info_id": death_info_id, "sequence_number": 1, "meddra_code": "100"}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!(
			"/api/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes"
		),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);

	// Parent info
	let body = json!({"data": {"patient_id": patient_id, "sex": "2", "medical_history_text": "none"}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/parents"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_drug_subresources_endpoints_ok() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let drug_id = create_drug(&app, &cookie, case_id).await?;

	let body = json!({"data": {"drug_id": drug_id, "sequence_number": 1, "substance_name": "Substance"}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);

	let body = json!({"data": {"drug_id": drug_id, "sequence_number": 1}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/dosages"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);

	let body = json!({"data": {"drug_id": drug_id, "sequence_number": 1, "indication_text": "test"}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/indications"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);

	let body = json!({"data": {
		"drug_id": drug_id,
		"sequence_number": 1,
		"code": "FDA.G.k.12.r.3",
		"value_code": "1"
	}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/device-characteristics"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_narrative_subresources_endpoints_ok() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let narrative_id = create_narrative(&app, &cookie, case_id).await?;

	let body = json!({"data": {"narrative_id": narrative_id, "sequence_number": 1, "diagnosis_meddra_code": "100"}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/sender-diagnoses"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);

	let body = json!({"data": {"narrative_id": narrative_id, "sequence_number": 1, "summary_text": "summary"}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/summaries"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_safety_report_subresources_endpoints_ok() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let body = json!({"data": {"case_id": case_id, "sender_type": "1", "organization_name": "Org"}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/senders"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);

	let body = json!({"data": {"case_id": case_id, "sequence_number": 1, "qualification": "1"}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/primary-sources"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);

	let body = json!({"data": {"case_id": case_id, "sequence_number": 1, "reference_text": "ref"}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/literature"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);

	let body = json!({"data": {"case_id": case_id, "study_name": "Study", "sponsor_study_number": "S-1"}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/studies"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let study_id = extract_id(&body)?;

	let body = json!({"data": {"study_information_id": study_id, "registration_number": "REG-1", "sequence_number": 1}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!(
			"/api/cases/{case_id}/safety-report/studies/{study_id}/registrations"
		),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_study_create_accepts_study_type_reaction() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let body = json!({
		"data": {
			"case_id": case_id,
			"study_name": "Study",
			"sponsor_study_number": "S-1",
			"study_type_reaction": "3"
		}
	});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/studies"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"study create status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["study_type_reaction"].as_str(), Some("3"));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_relatedness_assessment_endpoints_ok() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let drug_id = create_drug(&app, &cookie, case_id).await?;
	let reaction_id = create_reaction(&app, &cookie, case_id).await?;
	let assessment_id =
		create_reaction_assessment(&app, &cookie, case_id, drug_id, reaction_id)
			.await?;

	let body = json!({"data": {"drug_reaction_assessment_id": assessment_id, "sequence_number": 1, "result_of_assessment": "1"}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!(
			"/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness"
		),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);

	Ok(())
}
