use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::store::set_full_context_dbx;
use serde_json::{json, Value};
use serial_test::serial;
use time::Date;
use tower::ServiceExt;
use uuid::Uuid;

fn parse_json_or_raw(body: &[u8]) -> Value {
	let raw = String::from_utf8_lossy(body).trim().to_string();
	if raw.is_empty() {
		return json!({});
	}
	serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({ "raw": raw }))
}

async fn post_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: serde_json::Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("POST")
		.uri(uri)
		.header("content-type", "application/json")
		.header("cookie", cookie)
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = parse_json_or_raw(&body);
	Ok((status, value))
}

async fn put_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: serde_json::Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("PUT")
		.uri(uri)
		.header("content-type", "application/json")
		.header("cookie", cookie)
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = parse_json_or_raw(&body);
	Ok((status, value))
}

async fn get_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("GET")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = parse_json_or_raw(&body);
	Ok((status, value))
}

async fn get_bytes(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
) -> Result<(StatusCode, Vec<u8>)> {
	let req = Request::builder()
		.method("GET")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, body.to_vec()))
}

async fn create_narrative(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<()> {
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/narrative"),
		json!({
			"data": {
				"case_id": case_id,
				"case_narrative": "test narrative",
				"additional_information": "test sponsor information"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	Ok(())
}

async fn create_sender(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
	authority: &str,
) -> Result<()> {
	let (status, body) = post_json(
		app,
		cookie,
		"/api/presaves/senders",
		json!({
			"data": { "rows": {
				"sender": {
					"senderType": "1",
					"organizationName": "Test Sender"
				},
				"gateways": [{
					"sequenceNumber": 1,
					"gatewayAuthority": authority,
					"senderIdentifier": "TEST-SENDER",
					"isDefaultForAuthority": true
				}],
				"responsiblePersons": []
			} }
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let sender_presave_id = body["data"]["rows"]["sender"]["id"]
		.as_str()
		.ok_or("missing sender presave id")?;
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/safety-report/senders"),
		json!({
			"data": {
				"case_id": case_id,
				"sender_type": "1",
				"organization_name": "Test Sender",
				"source_sender_presave_id": sender_presave_id
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	Ok(())
}

async fn create_unlinked_sender(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<()> {
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/safety-report/senders"),
		json!({
			"data": {
				"case_id": case_id,
				"sender_type": "1",
				"organization_name": "Test Sender"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	Ok(())
}

async fn set_reaction_outcome(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<()> {
	let (status, body) =
		get_json(app, cookie, &format!("/api/cases/{case_id}/reactions")).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let reaction_id = body["data"][0]["id"]
		.as_str()
		.ok_or("missing intake reaction id")?;
	let (status, body) = put_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/reactions/{reaction_id}"),
		json!({ "data": { "outcome": "1" } }),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	Ok(())
}

async fn create_suspect_drug(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<()> {
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/drugs"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"drug_characterization": "1",
				"medicinal_product": "Test Drug"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	Ok(())
}

fn extract_case_id(body: &Value) -> Result<String> {
	Ok(body["data"]["case_id"]
		.as_str()
		.ok_or("missing case_id")?
		.to_string())
}

fn intake_basis(
	safety_report_id: &str,
	day_of_year: u32,
	report_type: &str,
) -> Value {
	let date = Date::from_ordinal_date(2024, day_of_year as u16)
		.expect("valid test ordinal date");
	let date = format!(
		"{:04}{:02}{:02}",
		date.year(),
		u8::from(date.month()),
		date.day()
	);
	json!({
		"authority": "ich",
		"safety_report_id": safety_report_id,
		"date_of_most_recent_information": date,
		"report_type": report_type,
		"patient_initials": intake_patient_initials(safety_report_id),
		"age_d2_2a": "41",
		"sex_d5": "2",
		"dg_prd_key": format!("DG-{}", intake_patient_initials(safety_report_id)),
		"reaction_meddra_version": "27.0",
		"reaction_meddra_code": "10019211",
		"ae_start_date": date.clone()
	})
}

fn intake_patient_initials(safety_report_id: &str) -> String {
	let suffix: String = safety_report_id
		.chars()
		.filter(|c| c.is_ascii_alphanumeric())
		.rev()
		.take(6)
		.collect::<Vec<_>>()
		.into_iter()
		.rev()
		.collect();
	format!("P{}", suffix.to_ascii_uppercase())
}

fn follow_up_pages(authority: &str) -> Value {
	let direct = || json!({ "authorities": [authority], "rows": {} });
	json!({
		"ci": direct(),
		"rp": direct(),
		"sd": direct(),
		"si": direct(),
		"dm": direct(),
		"nr": direct(),
		"lr": [],
		"dh": [],
		"ae": [],
		"lb": [],
		"dg": []
	})
}

#[serial]
#[tokio::test]
async fn test_follow_up_creation_preserves_scope_and_allocates_versions(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let safety_report_id = format!("FOLLOW-UP-{}", Uuid::new_v4());
	let product_id = format!("PRODUCT-{}", Uuid::new_v4());

	let (status, source_body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"status": "draft",
				"dgPrdKey": product_id,
				"safetyReportIdentification": {
					"safetyReportId": safety_report_id
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{source_body:?}");
	let source_case_id = Uuid::parse_str(
		source_body["data"]["id"]
			.as_str()
			.ok_or("missing source case id")?,
	)?;

	let (status, header) = post_json(&app, &cookie, &format!("/api/cases/{source_case_id}/message-header"), json!({"data": {
		"case_id": source_case_id, "message_number": format!("MSG-{source_case_id}"),
		"message_sender_identifier": "FOLLOW-UP-SENDER", "message_receiver_identifier": "KR",
		"batch_sender_identifier": "FOLLOW-UP-SENDER", "batch_receiver_identifier": "KR",
		"message_date": "20261019000000"
	}})).await?;
	assert_eq!(status, StatusCode::CREATED, "{header:?}");

	let mut invalid_pages = follow_up_pages("mfds");
	invalid_pages["ae"] = json!([
		{
			"clientRowId": "index-0",
			"request": {
				"authorities": ["mfds"],
				"rows": { "reaction": {
					"sequenceNumber": 1,
					"primarySourceReaction": "Headache",
					"meddraVersion": "27.0",
					"meddraCode": "10019211"
				} }
			}
		},
		{
			"clientRowId": "index-0",
			"request": {
				"authorities": ["mfds"],
				"rows": { "reaction": {
					"sequenceNumber": 2,
					"primarySourceReaction": "Nausea",
					"meddraVersion": "27.0",
					"meddraCode": "10028813"
				} }
			}
		}
	]);
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{source_case_id}/follow-up"),
		invalid_pages,
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");

	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	let (case_count,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (i64,)>(
				"SELECT count(*) FROM safety_report_identification WHERE safety_report_id = $1",
			)
			.bind(&safety_report_id),
		)
		.await?;
	mm.dbx().commit_txn().await?;
	assert_eq!(
		case_count, 1,
		"failed follow-up must roll back the case shell"
	);

	let mut follow_up_ids = Vec::new();
	for expected_version in [2, 3] {
		let (status, body) = post_json(
			&app,
			&cookie,
			&format!("/api/cases/{source_case_id}/follow-up"),
			follow_up_pages("mfds"),
		)
		.await?;
		assert_eq!(status, StatusCode::CREATED, "{body:?}");
		assert_eq!(body["data"]["safetyReportVersion"], expected_version);
		let created_id = body["data"]["caseId"]
			.as_str()
			.ok_or("missing follow-up id")?;
		let (shell_status, shell) = get_json(
			&app,
			&cookie,
			&format!("/api/cases/{created_id}/editor/shell"),
		)
		.await?;
		assert_eq!(shell_status, StatusCode::OK, "{shell:?}");
		let (header_status, header) = get_json(
			&app,
			&cookie,
			&format!("/api/cases/{created_id}/message-header"),
		)
		.await?;
		assert_eq!(header_status, StatusCode::OK, "{header:?}");
		assert_eq!(
			header["data"]["message_sender_identifier"],
			"FOLLOW-UP-SENDER"
		);
		assert_eq!(header["data"]["message_receiver_identifier"], "KR");
		assert_eq!(
			header["data"]["message_number"],
			format!("MSG-{created_id}")
		);
		follow_up_ids.push(Uuid::parse_str(
			body["data"]["caseId"]
				.as_str()
				.ok_or("missing follow-up case id")?,
		)?);
	}

	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	let rows = mm
		.dbx()
		.fetch_all(
			sqlx::query_as::<_, (Uuid, Option<String>, i32)>(
				"SELECT c.id, c.dg_prd_key, s.version
				   FROM cases c
				   JOIN safety_report_identification s ON s.case_id = c.id
				  WHERE s.safety_report_id = $1
				  ORDER BY s.version",
			)
			.bind(&safety_report_id),
		)
		.await?;
	mm.dbx().commit_txn().await?;

	assert_eq!(rows.len(), 3);
	assert_eq!(rows[1].0, follow_up_ids[0]);
	assert_eq!(rows[2].0, follow_up_ids[1]);
	for (_, product, _) in &rows[1..] {
		assert_eq!(product.as_deref(), Some(product_id.as_str()));
	}

	Ok(())
}

fn intake_data(
	safety_report_id: &str,
	day_of_year: u32,
	report_type: &str,
	extra: Value,
) -> Value {
	let mut base = intake_basis(safety_report_id, day_of_year, report_type);
	let base_map = base
		.as_object_mut()
		.expect("intake basis should be a JSON object");
	let extra_map = extra
		.as_object()
		.expect("intake extra should be a JSON object");
	for (key, value) in extra_map {
		base_map.insert(key.clone(), value.clone());
	}
	base
}

#[serial]
#[tokio::test]
async fn test_case_intake_duplicate_check_and_create() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let safety_report_id = format!("INTAKE-{}", Uuid::new_v4());

	let intake_body = json!({
		"data": intake_data(&safety_report_id, 120, "1", json!({ "authority": "ich" }))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", intake_body).await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let case_id = extract_case_id(&body)?;

	let dup_check = json!({
		"data": intake_data(&safety_report_id, 120, "1", json!({}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", dup_check).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], true);
	assert_eq!(body["data"]["basis_complete"], true);
	assert!(body["data"]["matches"].as_array().is_some());
	assert!(!body["data"]["matches"]
		.as_array()
		.ok_or("matches should be array")?
		.is_empty());

	let (status, value) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/safety-report"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["data"]["report_type"], "1");
	let (status, header_body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/message-header"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{header_body:?}");
	assert_eq!(
		header_body["data"]["case_id"].as_str(),
		Some(case_id.as_str())
	);
	let expected_sender = std::env::var("E2BR3_DEFAULT_MESSAGE_SENDER")?;
	let expected_receiver = std::env::var("E2BR3_DEFAULT_MESSAGE_RECEIVER_ICH")?;
	assert_eq!(
		header_body["data"]["batch_sender_identifier"].as_str(),
		Some(expected_sender.as_str())
	);
	assert_eq!(
		header_body["data"]["message_sender_identifier"].as_str(),
		Some(expected_sender.as_str())
	);
	assert_eq!(
		header_body["data"]["batch_receiver_identifier"].as_str(),
		Some(expected_receiver.as_str())
	);
	assert_eq!(
		header_body["data"]["message_receiver_identifier"].as_str(),
		Some(expected_receiver.as_str())
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_ich_export_without_required_narrative_still_downloads() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let safety_report_id = format!("INTAKE-NO-NARRATIVE-{}", Uuid::new_v4());
	let intake_body = json!({
		"data": intake_data(&safety_report_id, 121, "1", json!({ "authority": "ich" }))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", intake_body).await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let case_id = extract_case_id(&body)?;
	create_unlinked_sender(&app, &cookie, &case_id).await?;

	let (status, body) = get_bytes(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/export/xml?authority=ich"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	assert!(String::from_utf8(body)?.contains("<MCCI_IN200100UV01"));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_from_intake_creates_batch_number_for_export() -> Result<()> {
	std::env::set_var("E2BR3_EXPORT_VALIDATE", "0");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let safety_report_id = format!("INTAKE-BATCH-{}", Uuid::new_v4());
	let intake_body = json!({
		"data": intake_data(&safety_report_id, 124, "1", json!({
			"authority": "fda",
			"fda_report_type": "1"
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", intake_body).await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let case_id = extract_case_id(&body)?;
	create_sender(&app, &cookie, &case_id, "fda").await?;
	set_reaction_outcome(&app, &cookie, &case_id).await?;
	create_suspect_drug(&app, &cookie, &case_id).await?;

	let (status, header_body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/message-header"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{header_body:?}");
	let expected_batch_number = format!("BATCH-{case_id}");
	assert_eq!(
		header_body["data"]["batch_number"].as_str(),
		Some(expected_batch_number.as_str())
	);
	create_narrative(&app, &cookie, &case_id).await?;

	let update_body = json!({
		"data": {
			"batch_receiver_identifier": "FDA"
		}
	});
	let (status, update_response) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/message-header"),
		update_body,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{update_response:?}");
	let (status, workflow_response) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/review/toggle"),
		json!({"data":{"expected_status":"draft"}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{workflow_response:?}");

	let (status, xml) = get_bytes(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/export/xml?authority=fda"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&xml));
	let xml = String::from_utf8(xml)?;
	assert!(
		xml.contains(&format!("extension=\"{expected_batch_number}\"")),
		"N.1.2 batch number missing from export: {xml}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_from_intake_persists_distinct_c_1_dates() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", Uuid::new_v4());
	let intake_body = json!({
		"data": intake_data(&safety_report_id, 123, "1", json!({
			"transmission_date": "2024-04-30",
			"date_first_received_from_source": "20240501",
			"date_of_most_recent_information": "20240502"
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", intake_body).await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let case_id = extract_case_id(&body)?;

	let (status, value) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/safety-report"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["data"]["transmission_date"], "20240430000000");
	assert_eq!(value["data"]["date_first_received_from_source"], "20240501");
	assert_eq!(value["data"]["date_of_most_recent_information"], "20240502");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_from_intake_allows_duplicate_override() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", Uuid::new_v4());
	let intake_body = json!({
		"data": intake_data(&safety_report_id, 121, "1", json!({}))
	});
	let (status, _) =
		post_json(&app, &cookie, "/api/cases/from-intake", intake_body.clone())
			.await?;
	assert_eq!(status, StatusCode::CREATED);

	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", intake_body).await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(body["error"]["data"]["detail"]
		.as_str()
		.unwrap_or_default()
		.contains("duplicate case detected"));

	let override_body = json!({
		"data": intake_data(&safety_report_id, 121, "1", json!({
			"allow_duplicate_override": true
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", override_body).await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_intake_duplicate_check_requires_all_active_fields_to_match(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", Uuid::new_v4());
	let create_body = json!({
		"data": intake_data(&safety_report_id, 122, "1", json!({
			"dg_prd_key": "DG-A",
			"allow_duplicate_override": true
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", create_body).await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");

	let same_key_check = json!({
		"data": intake_data(&safety_report_id, 122, "1", json!({
			"dg_prd_key": "DG-A"
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", same_key_check).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], true, "{body:?}");

	let different_key_check = json!({
		"data": intake_data(&safety_report_id, 122, "1", json!({
			"dg_prd_key": "DG-B"
		}))
	});
	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/cases/intake-check",
		different_key_check,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], false, "{body:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_intake_duplicate_check_warns_for_missing_active_fields(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", Uuid::new_v4());
	let check_body = json!({
		"data": intake_data(&safety_report_id, 140, "1", json!({
			"reaction_meddra_version": null
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", check_body).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], false, "{body:?}");
	assert_eq!(body["data"]["basis_complete"], false, "{body:?}");
	assert!(!body["data"]["warnings"]
		.as_array()
		.ok_or("warnings should be array")?
		.is_empty());

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_intake_duplicate_check_rejects_missing_product_and_basis_fields(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", Uuid::new_v4());
	let check_body = json!({
		"data": intake_data(&safety_report_id, 141, "1", json!({
			"patient_initials": null,
			"age_d2_2a": null,
			"sex_d5": null,
			"dg_prd_key": null,
			"reaction_meddra_version": null,
			"reaction_meddra_code": null,
			"ae_start_date": null
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", check_body).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], false, "{body:?}");
	assert_eq!(body["data"]["basis_complete"], false, "{body:?}");
	assert!(!body["data"]["warnings"]
		.as_array()
		.ok_or("warnings should be array")?
		.is_empty());

	let create_body = json!({
		"data": intake_data(&safety_report_id, 141, "1", json!({
			"patient_initials": null,
			"age_d2_2a": null,
			"sex_d5": null,
			"dg_prd_key": null,
			"reaction_meddra_version": null,
			"reaction_meddra_code": null,
			"ae_start_date": null
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", create_body).await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_from_intake_requires_product_id() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", Uuid::new_v4());
	let intake_body = json!({
		"data": intake_data(&safety_report_id, 141, "1", json!({
			"patient_initials": null,
			"reaction_meddra_version": null,
			"dg_prd_key": null
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", intake_body.clone())
			.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");

	let override_body = json!({
		"data": intake_data(&safety_report_id, 142, "1", json!({
			"patient_initials": null,
			"reaction_meddra_version": null,
			"dg_prd_key": null,
			"allow_duplicate_override": true
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", override_body).await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_from_intake_rolls_back_shell_when_patient_create_fails(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let safety_report_id = format!("INTAKE-{}", Uuid::new_v4());
	let dg_prd_key = format!("ROLLBACK-{safety_report_id}");

	let body = json!({
		"data": intake_data(&safety_report_id, 143, "1", json!({
			"allow_duplicate_override": true,
			"dg_prd_key": dg_prd_key,
			"sex_d5": "9"
		}))
	});
	let (status, _) =
		post_json(&app, &cookie, "/api/cases/from-intake", body).await?;
	assert_ne!(status, StatusCode::CREATED);

	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	let (count,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (i64,)>(
				"SELECT COUNT(*) FROM cases WHERE dg_prd_key = $1",
			)
			.bind(&dg_prd_key),
		)
		.await?;
	mm.dbx().rollback_txn().await?;
	assert_eq!(count, 0, "failed intake must not leave a case shell");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_intake_duplicate_check_accepts_null_flavor_codes_as_optional(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", Uuid::new_v4());
	let check_body = json!({
		"data": intake_data(&safety_report_id, 142, "1", json!({
			"patient_initials": null,
			"patient_initials_null_flavor": "UNK"
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", check_body).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["basis_complete"], true, "{body:?}");
	assert_eq!(
		body["data"]["warnings"]
			.as_array()
			.ok_or("warnings should be array")?
			.len(),
		0,
		"{body:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_intake_duplicate_check_requires_active_fields_by_report_type(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let spontaneous_id = format!("INTAKE-{}", Uuid::new_v4());
	let spontaneous_missing_patient = json!({
		"data": intake_data(&spontaneous_id, 143, "1", json!({
			"patient_initials": null,
			"age_d2_2a": null,
			"sex_d5": null
		}))
	});
	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/cases/intake-check",
		spontaneous_missing_patient,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["basis_complete"], false, "{body:?}");
	assert!(!body["data"]["warnings"]
		.as_array()
		.ok_or("warnings should be array")?
		.is_empty());

	let study_id = format!("INTAKE-{}", Uuid::new_v4());
	let study_missing_investigation = json!({
		"data": intake_data(&study_id, 144, "2", json!({
			"reporter_organization": "Seoul Hospital",
			"sponsor_study_number": "STUDY-123",
			"investigation_number": null
		}))
	});
	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/cases/intake-check",
		study_missing_investigation,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["basis_complete"], false, "{body:?}");
	assert!(!body["data"]["warnings"]
		.as_array()
		.ok_or("warnings should be array")?
		.is_empty());

	let unknown_id = format!("INTAKE-{}", Uuid::new_v4());
	let unknown_complete = json!({
		"data": intake_data(&unknown_id, 145, "4", json!({
			"patient_initials": intake_patient_initials(&unknown_id),
			"age_d2_2a": "41",
			"sex_d5": "2"
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", unknown_complete)
			.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["basis_complete"], true, "{body:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_intake_duplicate_check_requires_all_active_fields() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", Uuid::new_v4());
	let create_body = json!({
		"data": intake_data(&safety_report_id, 123, "1", json!({
			"allow_duplicate_override": true
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", create_body).await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let case_id = extract_case_id(&body)?;
	let expected_initials = intake_patient_initials(&safety_report_id);

	let (status, patient_body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/patient")).await?;
	assert_eq!(status, StatusCode::OK, "{patient_body:?}");

	let (status, patient_update_body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/patient"),
		json!({
			"data": {
				"patient_initials": expected_initials,
				"age_at_time_of_onset": 0.0,
				"sex": "1"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{patient_update_body:?}");

	let base_match = json!({
		"data": intake_data(&safety_report_id, 123, "1", json!({}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", base_match).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], false, "{body:?}");

	let d1_match = json!({
		"data": intake_data(&safety_report_id, 123, "1", json!({
			"patient_initials": expected_initials,
			"dg_prd_key": null,
			"reaction_meddra_version": null
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", d1_match).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], false, "{body:?}");

	let d1_mismatch = json!({
		"data": intake_data(&safety_report_id, 123, "1", json!({
			"patient_initials": "ZZ",
			"dg_prd_key": null,
			"reaction_meddra_version": null
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", d1_mismatch).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], false, "{body:?}");

	let d5_match = json!({
		"data": intake_data(&safety_report_id, 123, "1", json!({
			"patient_initials": null,
			"dg_prd_key": null,
			"reaction_meddra_version": null,
			"age_d2_2a": "0.0",
			"sex_d5": "1"
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", d5_match).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], false, "{body:?}");

	let d5_mismatch = json!({
		"data": intake_data(&safety_report_id, 123, "1", json!({
			"patient_initials": null,
			"dg_prd_key": null,
			"reaction_meddra_version": null,
			"age_d2_2a": "0.0",
			"sex_d5": "2"
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", d5_mismatch).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], false, "{body:?}");

	let e_i_2_1_b_match = json!({
		"data": intake_data(&safety_report_id, 123, "1", json!({
			"reaction_meddra_code": "10019211"
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", e_i_2_1_b_match).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], false, "{body:?}");

	let e_i_2_1_b_mismatch = json!({
		"data": intake_data(&safety_report_id, 123, "1", json!({
			"reaction_meddra_code": "99999999"
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", e_i_2_1_b_mismatch)
			.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], false, "{body:?}");

	let e_i_4_match = json!({
		"data": intake_data(&safety_report_id, 123, "1", json!({
			"ae_start_date": "20240502"
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", e_i_4_match).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], false, "{body:?}");

	let e_i_4_mismatch = json!({
		"data": intake_data(&safety_report_id, 123, "1", json!({
			"ae_start_date": "20240503"
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", e_i_4_mismatch).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], false, "{body:?}");

	Ok(())
}
