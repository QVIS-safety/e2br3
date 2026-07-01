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

async fn put_json_with_audit_reason(
	app: &Router,
	cookie: &str,
	uri: String,
	body: Value,
	reason: &str,
) -> Result<(StatusCode, Vec<u8>)> {
	let req = Request::builder()
		.method("PUT")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.header("x-e2br3-reason-for-change", reason)
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

async fn delete_json(
	app: &Router,
	cookie: &str,
	uri: String,
) -> Result<(StatusCode, Vec<u8>)> {
	let req = Request::builder()
		.method("DELETE")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?.to_vec();
	Ok((status, body))
}

async fn create_case(app: &Router, cookie: &str, _org_id: Uuid) -> Result<Uuid> {
	let body = json!({
		"data": {
			"safetyReportIdentification": {
				"safetyReportId": format!("SR-{}", Uuid::new_v4())
			},
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

#[serial]
#[tokio::test]
async fn patient_physical_fields_preserve_null_flavors_and_ucum_units() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, seed.org_id).await?;

	let body = json!({
		"data": {
			"case_id": case_id,
			"age_at_time_of_onset": 7,
			"age_unit": "10.a",
			"gestation_period": 2,
			"gestation_period_unit": "{Trimester}",
			"weight_kg_null_flavor": "NI",
			"height_cm_null_flavor": "UNK"
		}
	});
	let (status, body) =
		post_json(&app, &cookie, format!("/api/cases/{case_id}/patient"), body)
			.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"create patient body {}",
		String::from_utf8_lossy(&body)
	);
	let payload: Value = serde_json::from_slice(&body)?;
	assert_eq!(payload["data"]["age_unit"], "10.a");
	assert_eq!(payload["data"]["gestation_period_unit"], "{Trimester}");
	assert_eq!(payload["data"]["weight_kg"], Value::Null);
	assert_eq!(payload["data"]["height_cm"], Value::Null);
	assert_eq!(payload["data"]["weight_kg_null_flavor"], "NI");
	assert_eq!(payload["data"]["height_cm_null_flavor"], "UNK");

	Ok(())
}

#[serial]
#[tokio::test]
async fn parent_information_preserves_dob_null_flavor_and_decade_unit() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let patient_id = create_patient(&app, &cookie, case_id).await?;

	let body = json!({
		"data": {
			"patient_id": patient_id,
			"parent_birth_date_null_flavor": "UNK",
			"parent_age": 4,
			"parent_age_unit": "10.a"
		}
	});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/parents"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"create parent body {}",
		String::from_utf8_lossy(&body)
	);
	let payload: Value = serde_json::from_slice(&body)?;
	assert_eq!(payload["data"]["parent_birth_date"], Value::Null);
	assert_eq!(payload["data"]["parent_birth_date_null_flavor"], "UNK");
	assert_eq!(payload["data"]["parent_age"], "4.00");
	assert_eq!(payload["data"]["parent_age_unit"], "10.a");

	Ok(())
}

async fn create_patient_with_narrative_preview_values(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"patient_initials": "CD",
			"age_at_time_of_onset": 30,
			"sex": "2"
		}
	});
	let (status, body) =
		post_json(app, cookie, format!("/api/cases/{case_id}/patient"), body)
			.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create patient for narrative preview status {} body {}",
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
			"case_narrative": "test narrative",
			"additional_information": "test sponsor information"
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
async fn safety_report_preserves_c1_boolean_null_flavors() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, seed.org_id).await?;

	let body = json!({
		"data": {
			"case_id": case_id,
			"safety_report_id": format!("SR-NF-{}", Uuid::new_v4()),
			"transmission_date": "20260630010101",
			"report_type": "1",
			"date_first_received_from_source": [2026, 6, 30],
			"date_of_most_recent_information": [2026, 6, 30],
			"fulfil_expedited_criteria": null,
			"fulfil_expedited_criteria_null_flavor": "NI",
			"other_case_identifiers_exist": null,
			"other_case_identifiers_exist_null_flavor": "NI"
		}
	});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);

	let value: Value = serde_json::from_slice(&body)?;
	assert!(value["data"]["fulfil_expedited_criteria"].is_null());
	assert_eq!(value["data"]["fulfil_expedited_criteria_null_flavor"], "NI");
	assert!(value["data"]["other_case_identifiers_exist"].is_null());
	assert_eq!(
		value["data"]["other_case_identifiers_exist_null_flavor"],
		"NI"
	);

	let update = json!({
		"data": {
			"fulfil_expedited_criteria": true,
			"other_case_identifiers_exist": true
		}
	});
	let (status, body) = put_json_with_audit_reason(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report"),
		update,
		"set boolean values",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["fulfil_expedited_criteria"], true);
	assert!(value["data"]["fulfil_expedited_criteria_null_flavor"].is_null());
	assert_eq!(value["data"]["other_case_identifiers_exist"], true);
	assert!(value["data"]["other_case_identifiers_exist_null_flavor"].is_null());

	Ok(())
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
		"case_narrative": "updated narrative",
		"additional_information": "updated sponsor information"
	}});
	let (status, _) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	let body = json!({"data": {
		"case_narrative": "updated narrative",
		"additional_information": "updated sponsor information"
	}});
	let (status, _) = put_json_with_audit_reason(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative"),
		body,
		"update narrative sponsor information",
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	let (status, body) =
		get_json(&app, &cookie, format!("/api/cases/{case_id}/narrative")).await?;
	assert_eq!(status, StatusCode::OK);
	let value: Value = serde_json::from_slice(&body)?;
	assert!(value["data"]["id"].as_str().is_some(), "{value:?}");
	assert_eq!(
		value["data"]["additional_information"],
		"updated sponsor information"
	);

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
async fn dg_kr_substance_fields_create_read_and_update_independently() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let drug_id = create_drug(&app, &cookie, case_id).await?;

	let substance_payload = json!({
		"data": {
			"drug_id": drug_id,
			"sequence_number": 1,
			"substance_name": "Substance A",
			"substance_termid": "BASE-SUB",
			"substance_termid_version": "BASE-SV1",
			"mfds_id": "KR-SUB",
			"mfds_version": "KR-SV1"
		}
	});

	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances"),
		substance_payload,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let created: Value = serde_json::from_slice(&body)?;
	let substance_id = extract_id(&body)?;
	assert_eq!(created["data"]["substance_termid"], "BASE-SUB");
	assert_eq!(created["data"]["substance_termid_version"], "BASE-SV1");
	assert_eq!(created["data"]["mfds_id"], "KR-SUB");
	assert_eq!(created["data"]["mfds_version"], "KR-SV1");

	let update_payload = json!({
		"data": {
			"mfds_id": "KR-SUB-2",
			"mfds_version": "KR-SV2"
		}
	});
	let (status, body) = put_json_with_audit_reason(
		&app,
		&cookie,
		format!(
			"/api/cases/{case_id}/drugs/{drug_id}/active-substances/{substance_id}"
		),
		update_payload,
		"update KR substance fields",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let updated: Value = serde_json::from_slice(&body)?;
	assert_eq!(updated["data"]["substance_termid"], "BASE-SUB");
	assert_eq!(updated["data"]["substance_termid_version"], "BASE-SV1");
	assert_eq!(updated["data"]["mfds_id"], "KR-SUB-2");
	assert_eq!(updated["data"]["mfds_version"], "KR-SV2");

	let (status, body) = get_json(
		&app,
		&cookie,
		format!(
			"/api/cases/{case_id}/drugs/{drug_id}/active-substances/{substance_id}"
		),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let fetched: Value = serde_json::from_slice(&body)?;
	assert_eq!(fetched["data"]["substance_termid"], "BASE-SUB");
	assert_eq!(fetched["data"]["substance_termid_version"], "BASE-SV1");
	assert_eq!(fetched["data"]["mfds_id"], "KR-SUB-2");
	assert_eq!(fetched["data"]["mfds_version"], "KR-SV2");

	Ok(())
}

#[serial]
#[tokio::test]
async fn dg_kr_product_fields_create_read_and_update_independently() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let create_payload = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"drug_characterization": "1",
			"medicinal_product": "Base Product",
			"mpid": "BASE-MPID",
			"mpid_version": "BASE-V1",
			"mfds_mpid": "KR-MPID",
			"mfds_mpid_version": "KR-V1"
		}
	});

	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs"),
		create_payload,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let created: Value = serde_json::from_slice(&body)?;
	assert_eq!(created["data"]["mpid"], "BASE-MPID");
	assert_eq!(created["data"]["mpid_version"], "BASE-V1");
	assert_eq!(created["data"]["mfds_mpid"], "KR-MPID");
	assert_eq!(created["data"]["mfds_mpid_version"], "KR-V1");
	let drug_id = created["data"]["id"].as_str().ok_or("missing drug id")?;

	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let fetched: Value = serde_json::from_slice(&body)?;
	assert_eq!(fetched["data"]["mpid"], "BASE-MPID");
	assert_eq!(fetched["data"]["mpid_version"], "BASE-V1");
	assert_eq!(fetched["data"]["mfds_mpid"], "KR-MPID");
	assert_eq!(fetched["data"]["mfds_mpid_version"], "KR-V1");

	let update_payload = json!({
		"data": {
			"mfds_mpid": "KR-MPID-2",
			"mfds_mpid_version": "KR-V2"
		}
	});
	let (status, body) = put_json_with_audit_reason(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}"),
		update_payload,
		"update KR product fields",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let updated: Value = serde_json::from_slice(&body)?;
	assert_eq!(updated["data"]["mpid"], "BASE-MPID");
	assert_eq!(updated["data"]["mpid_version"], "BASE-V1");
	assert_eq!(updated["data"]["mfds_mpid"], "KR-MPID-2");
	assert_eq!(updated["data"]["mfds_mpid_version"], "KR-V2");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_reaction_supports_ae_common_and_mfds_device_fields() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;

	let body = json!({"data": {
		"case_id": case_id,
		"sequence_number": 1,
		"primary_source_reaction": "Device site pain",
		"included_in_ema_ime_list": true,
		"expectedness": "2",
		"severity": "severe",
		"mfds_device_ae_classification": "0",
		"mfds_device_ae_outcome": "10",
		"mfds_device_cause_medical_device": true,
		"mfds_device_cause_procedure_issue": false,
		"mfds_device_cause_patient_condition": true,
		"mfds_device_cause_unable_to_assess": false,
		"mfds_device_cause_other": "Other device cause",
		"mfds_device_action_reason": "Action reason text",
		"mfds_device_action_recall": true,
		"mfds_device_action_repair": true,
		"mfds_device_action_inspection": false,
		"mfds_device_action_replacement": true,
		"mfds_device_action_improvement": false,
		"mfds_device_action_monitoring": true,
		"mfds_device_action_notification": true,
		"mfds_device_action_label_change": false,
		"mfds_device_action_other": "Other action text"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/reactions"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let created: Value = serde_json::from_slice(&body)?;
	let reaction_id = created["data"]["id"]
		.as_str()
		.ok_or("missing reaction id")?;
	assert_eq!(created["data"]["expectedness"], "2");
	assert_eq!(created["data"]["mfds_device_ae_outcome"], "10");
	assert_eq!(
		created["data"]["mfds_device_action_reason"],
		"Action reason text"
	);

	let body = json!({"data": {
		"expectedness": "1",
		"severity": "moderate",
		"mfds_device_ae_classification": "1",
		"mfds_device_action_other": "Updated other action"
	}});
	let (status, body) = put_json_with_audit_reason(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/reactions/{reaction_id}"),
		body,
		"update AE common and MFDS device fields",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let updated: Value = serde_json::from_slice(&body)?;
	assert_eq!(updated["data"]["expectedness"], "1");
	assert_eq!(updated["data"]["severity"], "moderate");
	assert_eq!(updated["data"]["mfds_device_ae_classification"], "1");
	assert_eq!(
		updated["data"]["mfds_device_action_other"],
		"Updated other action"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_test_results_support_50_character_normal_ranges() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let low = "L".repeat(50);
	let high = "H".repeat(50);

	let body = json!({"data": {
		"case_id": case_id,
		"sequence_number": 1,
		"test_name": "ALT",
		"normal_low_value": low,
		"normal_high_value": high
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/test-results"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["normal_low_value"], low);
	assert_eq!(value["data"]["normal_high_value"], high);
	let test_result_id = value["data"]["id"]
		.as_str()
		.ok_or("missing test result id")?;

	let updated_low = "A".repeat(50);
	let updated_high = "B".repeat(50);
	let body = json!({"data": {
		"normal_low_value": updated_low,
		"normal_high_value": updated_high
	}});
	let (status, body) = put_json_with_audit_reason(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/test-results/{test_result_id}"),
		body,
		"test normal range widening",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["normal_low_value"], updated_low);
	assert_eq!(value["data"]["normal_high_value"], updated_high);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_audit_reason_header_records_normal_patient_update_reason() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let patient_id = create_patient(&app, &cookie, case_id).await?;

	let reason = "Edited Data: Corrected patient initials";
	let body = json!({"data": {
		"case_id": case_id,
		"patient_initials": "CD",
		"sex": "2"
	}});
	let (status, body) = put_json_with_audit_reason(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient"),
		body,
		reason,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::OK,
		"update patient status {} body {}",
		status,
		String::from_utf8_lossy(&body)
	);

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	dbx.execute(sqlx::query("SET ROLE e2br3_auditor_role"))
		.await?;
	let recorded_reason = dbx
		.fetch_optional(
			sqlx::query_as::<_, (Option<String>,)>(
				r#"
				SELECT reason_for_change
				FROM audit_logs
				WHERE table_name = 'patient_information'
				  AND record_id = $1
				  AND action = 'UPDATE'
				  AND changed_fields ? 'patient_initials'
				ORDER BY id DESC
				LIMIT 1
				"#,
			)
			.bind(patient_id),
		)
		.await?;
	dbx.rollback_txn().await?;

	assert_eq!(
		recorded_reason.and_then(|(value,)| value),
		Some(reason.to_string())
	);

	Ok(())
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
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/parents"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let parent_id = extract_id(&body)?;

	// Parent past drug history
	let body = json!({"data": {
		"parent_id": parent_id,
		"sequence_number": 1,
		"drug_name_null_flavor": "NA",
		"mfds_medicinal_product_version": "MFDS-V1",
		"mfds_medicinal_product_id": "MFDS-ID",
		"mpid_version": "MPID-V1",
		"mpid": "MPID-123",
		"phpid_version": "PHPID-V1",
		"phpid": "PHPID-123",
		"start_date_null_flavor": "ASKU",
			"end_date_null_flavor": "NASK",
		"indication_meddra_version": "27.1",
		"indication_meddra_code": "10012345",
		"reaction_meddra_version": "27.1",
		"reaction_meddra_code": "10054321"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/parent/{parent_id}/past-drugs"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["drug_name"], Value::Null);
	assert_eq!(value["data"]["drug_name_null_flavor"], "NA");
	assert_eq!(value["data"]["mfds_medicinal_product_version"], "MFDS-V1");
	assert_eq!(value["data"]["mfds_medicinal_product_id"], "MFDS-ID");
	assert_eq!(value["data"]["mpid_version"], "MPID-V1");
	assert_eq!(value["data"]["mpid"], "MPID-123");
	assert_eq!(value["data"]["phpid_version"], "PHPID-V1");
	assert_eq!(value["data"]["phpid"], "PHPID-123");
	assert_eq!(value["data"]["start_date_null_flavor"], "ASKU");
	assert_eq!(value["data"]["end_date_null_flavor"], "NASK");
	assert_eq!(value["data"]["indication_meddra_version"], "27.1");
	assert_eq!(value["data"]["indication_meddra_code"], "10012345");
	assert_eq!(value["data"]["reaction_meddra_version"], "27.1");
	assert_eq!(value["data"]["reaction_meddra_code"], "10054321");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_past_drugs_support_mfds_product_fields_and_200_char_ids() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let patient_id = create_patient(&app, &cookie, case_id).await?;
	let long_mpid = "M".repeat(200);
	let long_phpid = "P".repeat(200);

	let body = json!({"data": {
		"patient_id": patient_id,
		"sequence_number": 1,
		"drug_name": "Past drug",
		"mfds_medicinal_product_version": "KR-VERSION-123456",
		"mfds_medicinal_product_id": "KRPROD1234",
		"mpid_version": "MPID-V1",
		"mpid": long_mpid,
		"phpid_version": "PHPID-V1",
		"phpid": long_phpid
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/patient/past-drugs"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"past-drug create status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let data = &value["data"];
	assert_eq!(data["mfds_medicinal_product_version"], "KR-VERSION-123456");
	assert_eq!(data["mfds_medicinal_product_id"], "KRPROD1234");
	assert_eq!(data["mpid"], long_mpid);
	assert_eq!(data["phpid"], long_phpid);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_drug_active_substance_soft_delete_restore() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let drug_id = create_drug(&app, &cookie, case_id).await?;

	let body = json!({"data": {
		"drug_id": drug_id,
		"sequence_number": 1,
		"substance_name": "Soft delete substance"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let row_id = extract_id(&body)?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["deleted"], false);

	let (status, body) = delete_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances/{row_id}"),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::NO_CONTENT,
		"{}",
		String::from_utf8_lossy(&body)
	);

	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	let rows = value["data"].as_array().ok_or("data is not an array")?;
	assert!(
		rows.iter()
			.all(|row| row["id"].as_str() != Some(&row_id.to_string())),
		"soft-deleted row should be excluded from default list"
	);

	let (status, body) = post_json(
		&app,
		&cookie,
		format!(
			"/api/cases/{case_id}/drugs/{drug_id}/active-substances/{row_id}/restore"
		),
		json!({}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["id"], row_id.to_string());
	assert_eq!(value["data"]["deleted"], false);

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

	let body = json!({"data": {
		"drug_id": drug_id,
		"sequence_number": 1,
		"substance_name": "Substance",
		"substance_termid": "TERM-1",
		"substance_termid_version": "2026-01",
		"strength_value": 12.5,
		"strength_unit": "mg"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["substance_name"], "Substance");
	assert_eq!(value["data"]["substance_termid"], "TERM-1");
	assert_eq!(value["data"]["substance_termid_version"], "2026-01");
	assert_eq!(value["data"]["strength_unit"], "mg");

	let body = json!({"data": {
		"drug_id": drug_id,
		"sequence_number": 1,
		"dose_value": 10,
		"dose_unit": "mg",
		"number_of_units": 2,
		"frequency_value": 1,
		"frequency_unit": "d",
		"duration_value": 3,
		"duration_unit": "800",
		"batch_lot_number": "LOT-1",
		"dosage_text": "10 mg daily",
		"dose_form": "tablet",
		"dose_form_termid": "DF-1",
		"dose_form_termid_version": "2026-01",
		"route_of_administration": "001",
		"route_termid": "ROUTE-1",
		"route_termid_version": "2026-02",
		"parent_route": "parent oral",
		"parent_route_termid": "PROUTE-1",
		"parent_route_termid_version": "2026-03"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/dosages"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["dose_unit"], "mg");
	assert_eq!(value["data"]["frequency_unit"], "d");
	assert_eq!(value["data"]["duration_unit"], "800");
	assert_eq!(value["data"]["batch_lot_number"], "LOT-1");
	assert_eq!(value["data"]["dosage_text"], "10 mg daily");
	assert_eq!(value["data"]["dose_form_termid"], "DF-1");
	assert_eq!(value["data"]["route_termid"], "ROUTE-1");
	assert_eq!(value["data"]["route_of_administration"], "001");
	assert_eq!(value["data"]["parent_route_termid"], "PROUTE-1");

	let body = json!({"data": {
		"drug_id": drug_id,
		"sequence_number": 1,
		"indication_text": "test",
		"indication_meddra_version": "27.1",
		"indication_meddra_code": "10000001"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/indications"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["indication_text"], "test");
	assert_eq!(value["data"]["indication_meddra_version"], "27.1");
	assert_eq!(value["data"]["indication_meddra_code"], "10000001");

	let body = json!({"data": {
		"drug_id": drug_id,
		"sequence_number": 1,
		"code": "FDA.G.k.12.r.3",
		"code_system": "2.16.840.1.113883.3.989.2.1.1.19",
		"code_display_name": "Problem Code",
		"value_type": "CE",
		"value_code": "1",
		"value_code_system": "FDA",
		"value_display_name": "Problem"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/device-characteristics"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["code"], "FDA.G.k.12.r.3");
	assert_eq!(
		value["data"]["code_system"],
		"2.16.840.1.113883.3.989.2.1.1.19"
	);
	assert_eq!(value["data"]["value_type"], "CE");
	assert_eq!(value["data"]["value_code"], "1");
	assert_eq!(value["data"]["value_code_system"], "FDA");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_narrative_preview_resolves_patient_tokens() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_patient_with_narrative_preview_values(&app, &cookie, case_id).await?;

	let body = json!({
		"template": "{D.2.2a}세의 {D.5} 환자 {UNKNOWN.CODE}"
	});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/preview"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["rendered"], "30세의 여성 환자 {UNKNOWN.CODE}");
	assert_eq!(value["data"]["tokens"][0]["code"], "D.2.2a");
	assert_eq!(value["data"]["tokens"][0]["resolved"], true);
	assert_eq!(value["data"]["tokens"][1]["code"], "D.5");
	assert_eq!(value["data"]["tokens"][1]["resolved"], true);
	assert_eq!(value["data"]["tokens"][2]["code"], "UNKNOWN.CODE");
	assert_eq!(value["data"]["tokens"][2]["resolved"], false);

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

	let body = json!({"data": {"narrative_id": narrative_id, "sequence_number": 1, "diagnosis_meddra_version": "27.1", "diagnosis_meddra_code": "100"}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/sender-diagnoses"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["diagnosis_meddra_version"], "27.1");
	assert_eq!(value["data"]["diagnosis_meddra_code"], "100");

	let body = json!({"data": {"narrative_id": narrative_id, "sequence_number": 1, "summary_type": "01", "language_code": "en", "summary_text": "summary"}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/summaries"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["summary_type"], "01");
	assert_eq!(value["data"]["language_code"], "en");
	assert_eq!(value["data"]["summary_text"], "summary");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_narrative_child_lists_return_empty_when_parent_narrative_missing(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;

	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/sender-diagnoses"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"].as_array().map(|rows| rows.len()), Some(0));

	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/summaries"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"].as_array().map(|rows| rows.len()), Some(0));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sender_diagnosis_soft_delete_restore() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let narrative_id = create_narrative(&app, &cookie, case_id).await?;

	let body = json!({
		"data": {
			"narrative_id": narrative_id,
			"sequence_number": 1,
			"diagnosis_meddra_version": "27.1",
			"diagnosis_meddra_code": "100"
		}
	});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/sender-diagnoses"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let value: Value = serde_json::from_slice(&body)?;
	let row_id = value["data"]["id"].as_str().expect("sender diagnosis id");
	assert_eq!(value["data"]["deleted"], false);

	let (status, body) = delete_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/sender-diagnoses/{row_id}"),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::NO_CONTENT,
		"{}",
		String::from_utf8_lossy(&body)
	);

	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/sender-diagnoses"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"].as_array().map(|rows| rows.len()), Some(0));

	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/sender-diagnoses/{row_id}/restore"),
		json!({}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["deleted"], false);

	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/narrative/sender-diagnoses"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"].as_array().map(|rows| rows.len()), Some(1));

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

	let body = json!({"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"reference_text": "Smith J. Case literature 2026.",
			"reference_text_null_flavor": "ASKU",
			"document_base64": "cGRm",
			"media_type": "application/pdf",
			"representation": "B64",
			"compression": "DF"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/literature"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(
		value["data"]["reference_text"],
		"Smith J. Case literature 2026."
	);
	assert_eq!(value["data"]["reference_text_null_flavor"], "ASKU");
	assert_eq!(value["data"]["document_base64"], "cGRm");
	assert_eq!(value["data"]["media_type"], "application/pdf");
	assert_eq!(value["data"]["representation"], "B64");
	assert_eq!(value["data"]["compression"], "DF");

	let body = json!({"data": {
		"case_id": case_id,
		"study_name": "Study",
		"study_name_null_flavor": "NASK",
		"sponsor_study_number": "S-1",
		"sponsor_study_number_null_flavor": "ASKU",
		"fda_ind_number_occurred": "1234567890",
		"fda_pre_anda_number_occurred": "9876543210"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/studies"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["study_name_null_flavor"], "NASK");
	assert_eq!(value["data"]["sponsor_study_number_null_flavor"], "ASKU");
	assert_eq!(value["data"]["fda_ind_number_occurred"], "1234567890");
	assert_eq!(value["data"]["fda_pre_anda_number_occurred"], "9876543210");
	let study_id = extract_id(&body)?;

	let body = json!({"data": {
		"study_information_id": study_id,
		"registration_number": "REG-1",
		"registration_number_null_flavor": "ASKU",
		"country_code": "KR",
		"country_code_null_flavor": "NASK",
		"sequence_number": 1
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!(
			"/api/cases/{case_id}/safety-report/studies/{study_id}/registrations"
		),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["registration_number_null_flavor"], "ASKU");
	assert_eq!(value["data"]["country_code_null_flavor"], "NASK");

	let body = json!({"data": {"study_information_id": study_id, "ind_number": "IND-123", "sequence_number": 1}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!(
			"/api/cases/{case_id}/safety-report/studies/{study_id}/fda-cross-reported-inds"
		),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["ind_number"], "IND-123");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_other_case_identifier_soft_delete_restore() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;

	let body = json!({"data": {
		"case_id": case_id,
		"sequence_number": 1,
		"source_of_identifier": "Regulator",
		"case_identifier": "REG-CASE-1"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/other-identifiers"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let row_id = extract_id(&body)?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["deleted"], false);

	let (status, body) = delete_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/other-identifiers/{row_id}"),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::NO_CONTENT,
		"{}",
		String::from_utf8_lossy(&body)
	);

	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/other-identifiers"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	let rows = value["data"].as_array().ok_or("data is not an array")?;
	assert!(
		rows.iter()
			.all(|row| row["id"].as_str() != Some(&row_id.to_string())),
		"soft-deleted row should be excluded from default list"
	);

	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/other-identifiers/{row_id}/restore"),
		json!({}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["id"], row_id.to_string());
	assert_eq!(value["data"]["deleted"], false);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_primary_source_supports_regional_rp_fields() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let body = json!({"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"reporter_given_name": "Mina",
			"reporter_name_null_flavor": "MSK",
			"organization": "Seoul General Hospital",
			"reporter_address_null_flavor": "ASKU",
			"country_code": "KR",
			"email": "mina.initial@example.test",
			"qualification": "3",
			"qualification_null_flavor": "UNK",
			"qualification_kr1": "1",
			"primary_source_regulatory": "1"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/primary-sources"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["reporter_given_name"], "Mina");
	assert_eq!(value["data"]["reporter_name_null_flavor"], "MSK");
	assert_eq!(value["data"]["organization"], "Seoul General Hospital");
	assert_eq!(value["data"]["reporter_address_null_flavor"], "ASKU");
	assert_eq!(value["data"]["country_code"], "KR");
	assert_eq!(value["data"]["email"], "mina.initial@example.test");
	assert_eq!(value["data"]["qualification"], "3");
	assert_eq!(value["data"]["qualification_null_flavor"], "UNK");
	assert_eq!(value["data"]["qualification_kr1"], "1");
	assert_eq!(value["data"]["primary_source_regulatory"], "1");
	let primary_source_id = extract_id(&body)?;

	let body = json!({"data": {
		"case_id": case_id,
		"sequence_number": 2,
		"reporter_given_name": "Backup",
		"organization": "Backup Reporter Org",
		"country_code": "KR",
		"qualification": "3",
		"primary_source_regulatory": "1"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/primary-sources"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);

	let body = json!({"data": {
		"email": "mina.updated@example.test",
		"reporter_name_null_flavor": "NASK",
		"reporter_address_null_flavor": "MSK",
		"qualification_kr1": "2",
		"qualification_null_flavor": "UNK",
		"primary_source_regulatory": "2"
	}});
	let (status, body) = put_json_with_audit_reason(
		&app,
		&cookie,
		format!(
			"/api/cases/{case_id}/safety-report/primary-sources/{primary_source_id}"
		),
		body,
		"update primary source regional fields",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["email"], "mina.updated@example.test");
	assert_eq!(value["data"]["reporter_name_null_flavor"], "NASK");
	assert_eq!(value["data"]["reporter_address_null_flavor"], "MSK");
	assert_eq!(value["data"]["qualification_kr1"], "2");
	assert_eq!(value["data"]["qualification_null_flavor"], "UNK");
	assert_eq!(value["data"]["primary_source_regulatory"], "2");

	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/primary-sources"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	let sources = value["data"]
		.as_array()
		.ok_or("missing primary source list")?;
	let primary_source_id_str = primary_source_id.to_string();
	let saved = sources
		.iter()
		.find(|source| source["id"].as_str() == Some(primary_source_id_str.as_str()))
		.ok_or("missing saved primary source")?;
	assert_eq!(saved["reporter_name_null_flavor"], "NASK");
	assert_eq!(saved["reporter_address_null_flavor"], "MSK");
	assert_eq!(saved["qualification_null_flavor"], "UNK");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sender_information_supports_mfds_health_professional_type_kr1(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let body = json!({"data": {
		"case_id": case_id,
		"sender_type": "3",
		"organization_name": "KR Sender",
		"health_professional_type_kr1": "4"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/senders"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let sender_id = extract_id(&body)?;

	let body = json!({"data": {
		"sender_type": "3",
		"organization_name": "KR Sender",
		"health_professional_type_kr1": "2"
	}});
	let (status, body) = put_json_with_audit_reason(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/senders/{sender_id}"),
		body,
		"update sender KR.1",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));

	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/senders/{sender_id}"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["health_professional_type_kr1"], "2");

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

	let body = json!({"data": {
		"drug_reaction_assessment_id": assessment_id,
		"sequence_number": 1,
		"source_of_assessment": "Sponsor",
		"method_of_assessment": "Algorithm",
		"result_of_assessment": "1",
		"result_of_assessment_kr2": "KR result"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!(
			"/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness"
		),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(
		value["data"]["drug_reaction_assessment_id"],
		assessment_id.to_string()
	);
	assert_eq!(value["data"]["source_of_assessment"], "Sponsor");
	assert_eq!(value["data"]["method_of_assessment"], "Algorithm");
	assert_eq!(value["data"]["result_of_assessment"], "1");
	assert_eq!(value["data"]["result_of_assessment_kr2"], "KR result");

	Ok(())
}
