use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use serial_test::serial;
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
	json!({
		"safety_report_id": safety_report_id,
		"date_of_most_recent_information": [2024, day_of_year],
		"report_type": report_type,
		"patient_initials": intake_patient_initials(safety_report_id),
		"dg_prd_key": format!("DG-{}", intake_patient_initials(safety_report_id)),
		"reaction_meddra_version": "27.0",
		"reaction_meddra_code": "10019211",
		"ae_start_date": [2024, day_of_year]
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
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", Uuid::new_v4());

	let intake_body = json!({
		"data": intake_data(&safety_report_id, 120, "1", json!({
			"validation_profile": "fda"
		}))
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
	assert!(header_body["data"]["message_sender_identifier"]
		.as_str()
		.map(|v| !v.trim().is_empty())
		.unwrap_or(false));
	assert!(header_body["data"]["message_receiver_identifier"]
		.as_str()
		.map(|v| !v.trim().is_empty())
		.unwrap_or(false));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_from_intake_derives_profile_from_appendices() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", Uuid::new_v4());
	let intake_body = json!({
		"data": intake_data(&safety_report_id, 124, "1", json!({
			"appendices_json": "[\"mfds\",\"fda\"]"
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", intake_body).await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let case_id = extract_case_id(&body)?;

	let (status, case_body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}")).await?;
	assert_eq!(status, StatusCode::OK, "{case_body:?}");
	assert_eq!(
		case_body["data"]["validation_profile"], "mfds",
		"{case_body:?}"
	);
	assert_eq!(
		case_body["data"]["appendices_json"], "[\"mfds\",\"fda\"]",
		"{case_body:?}"
	);

	let (status, header_body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/message-header"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{header_body:?}");
	assert_eq!(
		header_body["data"]["message_receiver_identifier"], "KR",
		"{header_body:?}"
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
			"transmission_date": [2024, 121],
			"date_first_received_from_source": [2024, 122],
			"date_of_most_recent_information": [2024, 123],
			"validation_profile": "ich"
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
	assert_eq!(value["data"]["transmission_date"], json!([2024, 121]));
	assert_eq!(
		value["data"]["date_first_received_from_source"],
		json!([2024, 122])
	);
	assert_eq!(
		value["data"]["date_of_most_recent_information"],
		json!([2024, 123])
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_from_intake_blocks_duplicates_even_with_override() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", Uuid::new_v4());
	let intake_body = json!({
		"data": intake_data(&safety_report_id, 121, "1", json!({
			"validation_profile": "ich"
		}))
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
			"validation_profile": "ich",
			"allow_duplicate_override": true
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", override_body).await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(body["error"]["data"]["detail"]
		.as_str()
		.unwrap_or_default()
		.contains("duplicate case detected"));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_intake_duplicate_check_uses_patient_signature_over_product_mismatch(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", Uuid::new_v4());
	let create_body = json!({
		"data": intake_data(&safety_report_id, 122, "1", json!({
			"validation_profile": "fda",
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
	assert_eq!(body["data"]["duplicate"], true, "{body:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_intake_duplicate_check_surfaces_incomplete_basis_as_warning(
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
	assert_eq!(body["data"]["basis_complete"], true, "{body:?}");
	assert!(!body["data"]["warnings"]
		.as_array()
		.ok_or("warnings should be array")?
		.is_empty());
	assert!(body["data"]["warnings"]
		.as_array()
		.map(|warnings| warnings.iter().any(|value| value
			.as_str()
			.unwrap_or_default()
			.contains("Reaction MedDRA version is missing")))
		.unwrap_or(false));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_from_intake_requires_override_when_duplicate_basis_is_incomplete(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", Uuid::new_v4());
	let intake_body = json!({
		"data": intake_data(&safety_report_id, 141, "1", json!({
			"validation_profile": "ich",
			"patient_initials": null,
			"reaction_meddra_version": null,
			"dg_prd_key": null
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", intake_body.clone())
			.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(body["error"]["data"]["detail"]
		.as_str()
		.unwrap_or_default()
		.contains("duplicate check basis is incomplete"));

	let override_body = json!({
		"data": intake_data(&safety_report_id, 141, "1", json!({
			"validation_profile": "ich",
			"patient_initials": null,
			"reaction_meddra_version": null,
			"dg_prd_key": null,
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
async fn test_case_intake_duplicate_check_treats_null_flavor_codes_as_missing(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", Uuid::new_v4());
	let check_body = json!({
		"data": intake_data(&safety_report_id, 142, "1", json!({
			"patient_initials": "UNK",
			"reaction_meddra_version": "UNK"
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", check_body).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["basis_complete"], false, "{body:?}");
	assert!(body["data"]["warnings"]
		.as_array()
		.map(|warnings| warnings.iter().any(|value| value
			.as_str()
			.unwrap_or_default()
			.contains("Reaction MedDRA version is missing")))
		.unwrap_or(false));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_intake_duplicate_check_respects_patient_and_reaction_fields(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", Uuid::new_v4());
	let create_body = json!({
		"data": intake_data(&safety_report_id, 123, "1", json!({
			"validation_profile": "ich",
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
	assert_eq!(body["data"]["duplicate"], true, "{body:?}");

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
	assert_eq!(body["data"]["duplicate"], true, "{body:?}");

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
	assert_eq!(body["data"]["duplicate"], true, "{body:?}");

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
	assert_eq!(body["data"]["duplicate"], true, "{body:?}");

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
			"ae_start_date": [2024, 123]
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", e_i_4_match).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], true, "{body:?}");

	let e_i_4_mismatch = json!({
		"data": intake_data(&safety_report_id, 123, "1", json!({
			"ae_start_date": [2024, 124]
		}))
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", e_i_4_mismatch).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], false, "{body:?}");

	Ok(())
}
