use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

async fn post_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("POST")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice::<Value>(&body)?))
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
	let body = serde_json::from_slice::<Value>(&body).unwrap_or(Value::Null);
	Ok((status, body))
}

async fn create_case(
	app: &axum::Router,
	cookie: &str,
	safety_report_prefix: &str,
) -> Result<String> {
	let safety_report_id = format!("{safety_report_prefix}-{}", Uuid::new_v4());
	let (status, body) = post_json(
		app,
		cookie,
		"/api/cases",
		json!({
			"data": {
				"safety_report_id": safety_report_id,
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	Ok(body["data"]["id"]
		.as_str()
		.ok_or("missing created case id")?
		.to_string())
}

#[serial]
#[tokio::test]
async fn editor_shell_returns_only_case_header_workflow_and_permissions(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let safety_report_id = format!("EDITOR-SHELL-{}", Uuid::new_v4());

	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safety_report_id": safety_report_id,
				"status": "draft",
				"dg_prd_key": "DG-EDITOR-SHELL"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let case_id = body["data"]["id"]
		.as_str()
		.ok_or("missing created case id")?
		.to_string();

	let (status, body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/editor/shell"))
			.await?;

	assert_eq!(status, StatusCode::OK);
	assert_eq!(body["id"], case_id);
	assert!(body.get("status").is_some());
	assert!(body.get("appendices").is_some());
	assert!(body.get("canActOnWorkflow").is_some());
	assert!(body.get("reactions").is_none());
	assert!(body.get("testResults").is_none());
	assert!(body.get("drugs").is_none());
	assert!(body.get("patientInformation").is_none());
	assert!(body.get("messageHeader").is_none());
	assert!(body.get("safetyReportIdentification").is_none());

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_ae_list_returns_reaction_rows_without_detail_fanout() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-AE-LIST").await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/reactions"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"primary_source_reaction": "Headache",
				"primary_source_reaction_translation": "Head pain",
				"reaction_meddra_version": "27.1",
				"reaction_meddra_code": "10019211",
				"serious": true,
				"outcome": "1"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/AE/list"),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	let rows = body["rows"].as_array().ok_or("missing rows array")?;
	assert!(!rows.is_empty(), "{body}");
	let row = &rows[0];
	assert!(row.get("id").is_some(), "{row}");
	assert_eq!(row["sequenceNumber"], 1);
	assert_eq!(row["reactionPrimarySourceNative"], "Headache");
	assert_eq!(row["reactionPrimarySourceTranslation"], "Head pain");
	assert_eq!(row["meddraVersion"], "27.1");
	assert_eq!(row["meddraCode"], "10019211");
	assert!(row.get("seriousness").is_some(), "{row}");
	assert!(row.get("outcome").is_none(), "{row}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_ae_detail_returns_one_reaction_by_uuid() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-AE-DETAIL").await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/reactions"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"primary_source_reaction": "Headache",
				"primary_source_reaction_translation": "Head pain",
				"reaction_meddra_version": "27.1",
				"reaction_meddra_code": "10019211",
				"serious": true,
				"outcome": "1"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let reaction_id = body["data"]["id"]
		.as_str()
		.ok_or("missing reaction id")?
		.to_string();

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/AE/{reaction_id}"),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	assert_eq!(body["rowId"], reaction_id);
	let reactions = body["data"]["reactions"]
		.as_array()
		.ok_or("missing reactions array")?;
	assert_eq!(reactions.len(), 1, "{body}");
	assert_eq!(reactions[0]["id"], reaction_id);
	assert!(reactions[0].get("primary_source_reaction").is_some());

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_lb_list_returns_test_rows_without_detail_fanout() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-LB-LIST").await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/test-results"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"test_name": "ALT",
				"test_result_value": "42",
				"test_result_unit": "U/L"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/LB/list"),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	let rows = body["rows"].as_array().ok_or("missing rows array")?;
	assert!(!rows.is_empty(), "{body}");
	let row = &rows[0];
	assert!(row.get("id").is_some(), "{row}");
	assert_eq!(row["sequenceNumber"], 1);
	assert_eq!(row["testName"], "ALT");
	assert_eq!(row["resultValue"], "42");
	assert_eq!(row["resultUnit"], "U/L");

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_lb_detail_returns_one_test_result_by_uuid() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-LB-DETAIL").await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/test-results"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"test_name": "ALT",
				"test_result_value": "42",
				"test_result_unit": "U/L"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let test_result_id = body["data"]["id"]
		.as_str()
		.ok_or("missing test result id")?
		.to_string();

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/LB/{test_result_id}"),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	assert_eq!(body["rowId"], test_result_id);
	let test_results = body["data"]["testResults"]
		.as_array()
		.ok_or("missing testResults array")?;
	assert_eq!(test_results.len(), 1, "{body}");
	assert_eq!(test_results[0]["id"], test_result_id);
	assert_eq!(test_results[0]["test_name"], "ALT");

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_dg_list_returns_drug_rows_without_nested_drug_children() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-DG-LIST").await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"drug_characterization": "1",
				"medicinal_product": "Example Product",
				"action_taken": "1"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/DG/list"),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	let rows = body["rows"].as_array().ok_or("missing rows array")?;
	assert!(!rows.is_empty(), "{body}");
	let row = &rows[0];
	assert!(row.get("id").is_some(), "{row}");
	assert_eq!(row["sequenceNumber"], 1);
	assert_eq!(row["drugRole"], "1");
	assert_eq!(row["medicinalProduct"], "Example Product");
	assert_eq!(row["actionTaken"], "1");
	assert!(row.get("dosageInformation").is_none(), "{row}");
	assert!(row.get("drugReactionAssessments").is_none(), "{row}");
	assert!(row.get("activeSubstances").is_none(), "{row}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_dg_detail_returns_one_drug_with_nested_children() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-DG-DETAIL").await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"drug_characterization": "1",
				"medicinal_product": "Example Product",
				"action_taken": "1"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let drug_id = body["data"]["id"]
		.as_str()
		.ok_or("missing drug id")?
		.to_string();

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs/{drug_id}/dosages"),
		json!({
			"data": {
				"drug_id": drug_id,
				"sequence_number": 1,
				"dose_value": 10,
				"dose_unit": "mg",
				"dosage_text": "10 mg daily"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");

	let (status, body) = post_json(
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
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let reaction_id = body["data"]["id"]
		.as_str()
		.ok_or("missing reaction id")?
		.to_string();

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments"),
		json!({
			"data": {
				"drug_id": drug_id,
				"reaction_id": reaction_id
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/DG/{drug_id}"),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	assert_eq!(body["rowId"], drug_id);
	let drugs = body["data"]["drugs"]
		.as_array()
		.ok_or("missing drugs array")?;
	assert_eq!(drugs.len(), 1, "{body}");
	let drug = &drugs[0];
	assert_eq!(drug["id"], drug_id);
	assert!(!drug["dosageInformation"]
		.as_array()
		.ok_or("missing dosageInformation array")?
		.is_empty());
	assert!(!drug["drugReactionAssessments"]
		.as_array()
		.ok_or("missing drugReactionAssessments array")?
		.is_empty());

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_row_detail_rejects_numeric_row_position_as_identifier() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-NUMERIC-DETAIL").await?;

	let (status, body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/editor/AE/1"))
			.await?;

	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_dh_list_returns_past_drug_rows() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-DH-LIST").await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/patient"),
		json!({
			"data": {
				"case_id": case_id,
				"patient_initials": "ABC"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let patient_id = body["data"]["id"].as_str().ok_or("missing patient id")?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/patient/past-drugs"),
		json!({
			"data": {
				"patient_id": patient_id,
				"sequence_number": 1,
				"drug_name": "Prior Drug",
				"indication_meddra_code": "10012345",
				"start_date": "2024-01-02",
				"end_date": "2024-02-03"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/DH/list"),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	let rows = body["rows"].as_array().ok_or("missing rows array")?;
	assert!(!rows.is_empty(), "{body}");
	let row = &rows[0];
	assert!(row.get("id").is_some(), "{row}");
	assert_eq!(row["sequenceNumber"], 1);
	assert_eq!(row["drugName"], "Prior Drug");
	assert_eq!(row["indication"], "10012345");
	assert_eq!(row["startDate"], "2024-01-02");
	assert_eq!(row["endDate"], "2024-02-03");

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_dh_detail_returns_one_past_drug_history_by_uuid() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-DH-DETAIL").await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/patient"),
		json!({
			"data": {
				"case_id": case_id,
				"patient_initials": "ABC"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let patient_id = body["data"]["id"].as_str().ok_or("missing patient id")?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/patient/past-drugs"),
		json!({
			"data": {
				"patient_id": patient_id,
				"sequence_number": 1,
				"drug_name": "Prior Drug",
				"indication_meddra_code": "10012345"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let past_drug_id = body["data"]["id"]
		.as_str()
		.ok_or("missing past drug id")?
		.to_string();

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/DH/{past_drug_id}"),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	assert_eq!(body["rowId"], past_drug_id);
	let past_drug_history = body["data"]["patientInformation"]["pastDrugHistory"]
		.as_array()
		.ok_or("missing patientInformation.pastDrugHistory array")?;
	assert_eq!(past_drug_history.len(), 1, "{body}");
	assert_eq!(past_drug_history[0]["id"], past_drug_id);
	assert_eq!(past_drug_history[0]["drug_name"], "Prior Drug");

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_dh_list_returns_empty_rows_when_patient_missing() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-DH-MISSING-PATIENT").await?;

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/DH/list"),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	let rows = body["rows"].as_array().ok_or("missing rows array")?;
	assert!(rows.is_empty(), "{body}");

	Ok(())
}
