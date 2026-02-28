use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;

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

#[serial]
#[tokio::test]
async fn test_case_intake_duplicate_check_and_create() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", uuid::Uuid::new_v4());

	let intake_body = json!({
		"data": {
			"safety_report_id": safety_report_id,
			"date_of_most_recent_information": [2024, 120],
			"report_type": "1",
			"validation_profile": "fda"
		}
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", intake_body).await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let case_id = extract_case_id(&body)?;

	let dup_check = json!({
		"data": {
			"safety_report_id": safety_report_id,
			"date_of_most_recent_information": [2024, 120],
			"report_type": "1"
		}
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", dup_check).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], true);
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
async fn test_case_from_intake_blocks_duplicates_without_override() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", uuid::Uuid::new_v4());
	let intake_body = json!({
		"data": {
			"safety_report_id": safety_report_id,
			"date_of_most_recent_information": [2024, 121],
			"report_type": "1",
			"validation_profile": "ich"
		}
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
		"data": {
			"safety_report_id": safety_report_id,
			"date_of_most_recent_information": [2024, 121],
			"report_type": "1",
			"validation_profile": "ich",
			"allow_duplicate_override": true
		}
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", override_body).await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	assert_eq!(body["data"]["version"].as_i64(), Some(2), "{body:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_lifecycle_lists_versions_for_same_safety_report_id() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", uuid::Uuid::new_v4());
	let first = json!({
		"data": {
			"safety_report_id": safety_report_id,
			"date_of_most_recent_information": [2024, 200],
			"report_type": "1",
			"validation_profile": "fda"
		}
	});
	let (status, first_body) =
		post_json(&app, &cookie, "/api/cases/from-intake", first).await?;
	assert_eq!(status, StatusCode::CREATED, "{first_body:?}");
	let first_case_id = extract_case_id(&first_body)?;

	let second = json!({
		"data": {
			"safety_report_id": safety_report_id,
			"date_of_most_recent_information": [2024, 200],
			"report_type": "1",
			"validation_profile": "fda",
			"allow_duplicate_override": true
		}
	});
	let (status, second_body) =
		post_json(&app, &cookie, "/api/cases/from-intake", second).await?;
	assert_eq!(status, StatusCode::CREATED, "{second_body:?}");
	let second_case_id = extract_case_id(&second_body)?;

	let (status, lifecycle) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{second_case_id}/lifecycle"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{lifecycle:?}");
	assert_eq!(lifecycle["data"]["safety_report_id"], safety_report_id);
	let items = lifecycle["data"]["items"]
		.as_array()
		.ok_or("missing lifecycle items array")?;
	assert_eq!(items.len(), 2, "{lifecycle:?}");
	assert_eq!(items[0]["version"].as_i64(), Some(1), "{lifecycle:?}");
	assert_eq!(items[0]["case_id"].as_str(), Some(first_case_id.as_str()));
	assert_eq!(items[1]["version"].as_i64(), Some(2), "{lifecycle:?}");
	assert_eq!(items[1]["case_id"].as_str(), Some(second_case_id.as_str()));
	assert_eq!(
		items[1]["is_current"].as_bool(),
		Some(true),
		"{lifecycle:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_intake_duplicate_check_respects_dg_prd_key_filter() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", uuid::Uuid::new_v4());
	let create_body = json!({
		"data": {
			"safety_report_id": safety_report_id,
			"date_of_most_recent_information": [2024, 122],
			"report_type": "1",
			"validation_profile": "fda",
			"dg_prd_key": "DG-A"
		}
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", create_body).await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");

	let same_key_check = json!({
		"data": {
			"safety_report_id": safety_report_id,
			"date_of_most_recent_information": [2024, 122],
			"report_type": "1",
			"dg_prd_key": "DG-A"
		}
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", same_key_check).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], true, "{body:?}");

	let different_key_check = json!({
		"data": {
			"safety_report_id": safety_report_id,
			"date_of_most_recent_information": [2024, 122],
			"report_type": "1",
			"dg_prd_key": "DG-B"
		}
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
async fn test_case_intake_duplicate_check_respects_patient_and_reaction_fields(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let safety_report_id = format!("INTAKE-{}", uuid::Uuid::new_v4());
	let create_body = json!({
		"data": {
			"safety_report_id": safety_report_id,
			"date_of_most_recent_information": [2024, 123],
			"report_type": "1",
			"validation_profile": "ich"
		}
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/from-intake", create_body).await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let case_id = extract_case_id(&body)?;

	let (status, patient_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/patient"),
		json!({
			"data": {
				"case_id": case_id,
				"patient_initials": "AB",
				"sex": "1"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{patient_body:?}");

	let (status, reaction_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/reactions"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"primary_source_reaction": "Headache"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{reaction_body:?}");
	let reaction_id = reaction_body["data"]["id"]
		.as_str()
		.ok_or("missing reaction id")?
		.to_string();

	let (status, reaction_update_body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/reactions/{reaction_id}"),
		json!({
			"data": {
				"reaction_meddra_version": "27.0",
				"reaction_meddra_code": "10019211",
				"start_date": [2024, 123]
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{reaction_update_body:?}");

	let base_match = json!({
		"data": {
			"safety_report_id": safety_report_id,
			"date_of_most_recent_information": [2024, 123],
			"report_type": "1"
		}
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", base_match).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], true, "{body:?}");

	let d1_match = json!({
		"data": {
			"safety_report_id": safety_report_id,
			"date_of_most_recent_information": [2024, 123],
			"report_type": "1",
			"patient_initials": "AB"
		}
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", d1_match).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], true, "{body:?}");

	let d1_mismatch = json!({
		"data": {
			"safety_report_id": safety_report_id,
			"date_of_most_recent_information": [2024, 123],
			"report_type": "1",
			"patient_initials": "ZZ"
		}
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", d1_mismatch).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], false, "{body:?}");

	let d5_match = json!({
		"data": {
			"safety_report_id": safety_report_id,
			"date_of_most_recent_information": [2024, 123],
			"report_type": "1",
			"sex_d5": "1"
		}
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", d5_match).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], true, "{body:?}");

	let d5_mismatch = json!({
		"data": {
			"safety_report_id": safety_report_id,
			"date_of_most_recent_information": [2024, 123],
			"report_type": "1",
			"sex_d5": "2"
		}
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", d5_mismatch).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], false, "{body:?}");

	let e_i_2_1_b_match = json!({
		"data": {
			"safety_report_id": safety_report_id,
			"date_of_most_recent_information": [2024, 123],
			"report_type": "1",
			"reaction_meddra_code": "10019211"
		}
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", e_i_2_1_b_match).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], true, "{body:?}");

	let e_i_2_1_b_mismatch = json!({
		"data": {
			"safety_report_id": safety_report_id,
			"date_of_most_recent_information": [2024, 123],
			"report_type": "1",
			"reaction_meddra_code": "99999999"
		}
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", e_i_2_1_b_mismatch)
			.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], false, "{body:?}");

	let e_i_4_match = json!({
		"data": {
			"safety_report_id": safety_report_id,
			"date_of_most_recent_information": [2024, 123],
			"report_type": "1",
			"ae_start_date": [2024, 123]
		}
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", e_i_4_match).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], true, "{body:?}");

	let e_i_4_mismatch = json!({
		"data": {
			"safety_report_id": safety_report_id,
			"date_of_most_recent_information": [2024, 123],
			"report_type": "1",
			"ae_start_date": [2024, 124]
		}
	});
	let (status, body) =
		post_json(&app, &cookie, "/api/cases/intake-check", e_i_4_mismatch).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["duplicate"], false, "{body:?}");

	Ok(())
}
