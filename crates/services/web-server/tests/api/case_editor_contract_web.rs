use crate::common::{
	cookie_header, init_test_mm, insert_user, seed_org_with_users, system_user_id,
	Result,
};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::acs::{
	upsert_dynamic_role_permissions, Permission, CASE_IDENTIFIER_LIST, CASE_LIST,
	CASE_READ, CASE_SUMMARY_LIST, DEATH_CAUSE_LIST, DRUG_INDICATION_LIST,
	DRUG_REACTION_ASSESSMENT_LIST, DRUG_READ, DRUG_RECURRENCE_LIST,
	DRUG_SUBSTANCE_LIST, MEDICAL_HISTORY_LIST, MESSAGE_HEADER_READ, NARRATIVE_READ,
	PARENT_INFORMATION_LIST, PARENT_MEDICAL_HISTORY_LIST, PARENT_PAST_DRUG_LIST,
	PATIENT_DEATH_LIST, PATIENT_IDENTIFIER_LIST, PATIENT_READ, RECEIVER_READ,
	SAFETY_REPORT_READ, SENDER_DIAGNOSIS_LIST, SENDER_INFORMATION_LIST,
	STUDY_INFORMATION_LIST, STUDY_REGISTRATION_LIST,
};
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::ModelManager;
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

async fn patch_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("PATCH")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let body = serde_json::from_slice::<Value>(&body).unwrap_or(Value::Null);
	Ok((status, body))
}

async fn delete_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("DELETE")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let body = serde_json::from_slice::<Value>(&body).unwrap_or(Value::Null);
	Ok((status, body))
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

async fn stale_validation_summary_count(
	mm: &ModelManager,
	user_id: Uuid,
	org_id: Uuid,
	case_id: &str,
) -> Result<i64> {
	let case_uuid = Uuid::parse_str(case_id)?;
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(mm.dbx(), user_id, org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	let count = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (i64,)>(
				"SELECT COUNT(*)::bigint
				   FROM case_validation_summaries
				  WHERE case_id = $1
				    AND stale = true",
			)
			.bind(case_uuid),
		)
		.await?
		.0;
	mm.dbx().commit_txn().await?;
	Ok(count)
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

async fn create_case_with_appendices(
	app: &axum::Router,
	cookie: &str,
	safety_report_prefix: &str,
	_appendices: &[&str],
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

async fn create_reaction_fixture(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<String> {
	let (status, body) = post_json(
		app,
		cookie,
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
	Ok(body["data"]["id"]
		.as_str()
		.ok_or("missing reaction id")?
		.to_string())
}

async fn create_test_result_fixture(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<String> {
	let (status, body) = post_json(
		app,
		cookie,
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
	Ok(body["data"]["id"]
		.as_str()
		.ok_or("missing test result id")?
		.to_string())
}

async fn create_drug_fixture(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<String> {
	let (status, body) = post_json(
		app,
		cookie,
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
	Ok(body["data"]["id"]
		.as_str()
		.ok_or("missing drug id")?
		.to_string())
}

async fn create_patient_fixture(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<String> {
	let (status, body) = post_json(
		app,
		cookie,
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
	Ok(body["data"]["id"]
		.as_str()
		.ok_or("missing patient id")?
		.to_string())
}

async fn create_past_drug_history_fixture(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<String> {
	let patient_id = create_patient_fixture(app, cookie, case_id).await?;
	let (status, body) = post_json(
		app,
		cookie,
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
	Ok(body["data"]["id"]
		.as_str()
		.ok_or("missing past drug id")?
		.to_string())
}

async fn create_safety_report(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
	report_type: &str,
	fulfil_expedited_criteria: bool,
) -> Result<()> {
	create_safety_report_with_local_criteria(
		app,
		cookie,
		case_id,
		report_type,
		fulfil_expedited_criteria,
		None,
	)
	.await
}

async fn create_safety_report_with_local_criteria(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
	report_type: &str,
	fulfil_expedited_criteria: bool,
	local_criteria_report_type: Option<&str>,
) -> Result<()> {
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/safety-report"),
		json!({
			"data": {
				"case_id": case_id,
				"transmission_date": [2024, 1],
				"report_type": report_type,
				"date_first_received_from_source": [2024, 1],
				"date_of_most_recent_information": [2024, 1],
				"fulfil_expedited_criteria": fulfil_expedited_criteria,
				"local_criteria_report_type": local_criteria_report_type
			}
		}),
	)
	.await?;
	assert!(
		status == StatusCode::CREATED || status == StatusCode::OK,
		"{body}"
	);
	Ok(())
}

fn assert_no_ae_lb_dg_payload(data: &Value) {
	assert!(data.get("reactions").is_none(), "{data}");
	assert!(data.get("testResults").is_none(), "{data}");
	assert!(data.get("drugs").is_none(), "{data}");
}

async fn limited_cookie(
	mm: &ModelManager,
	org_id: Uuid,
	permissions: Vec<Permission>,
) -> Result<String> {
	let limited_role = format!("editor_direct_limited_{}", Uuid::new_v4());
	upsert_dynamic_role_permissions(&limited_role, permissions);
	let limited_user = insert_user(
		mm,
		org_id,
		&limited_role,
		system_user_id(),
		Some("limitedpwd"),
	)
	.await?;
	let token = generate_web_token(&limited_user.email, limited_user.token_salt)?;
	Ok(cookie_header(&token.to_string()))
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
	assert!(body.get("appendices").is_none());
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
async fn editor_ci_returns_ci_payload_only() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-CI").await?;

	let (status, body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/editor/CI")).await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	let data = body.get("data").ok_or("missing data")?;
	let safety_report = data
		.get("safetyReportIdentification")
		.ok_or("missing safetyReportIdentification")?;
	assert!(
		data["receiverInfo"].is_null() || data["receiverInfo"].is_object(),
		"{body}"
	);
	assert!(data.get("receiverInformation").is_none(), "{body}");
	assert!(data.get("receiver").is_none(), "{body}");
	assert!(data["otherCaseIdentifiers"].is_array(), "{body}");
	assert!(data["linkedReports"].is_array(), "{body}");
	assert!(data["documentsHeldBySender"].is_array(), "{body}");
	assert!(
		safety_report.get("otherCaseIdentifiers").is_none(),
		"{body}"
	);
	assert!(safety_report.get("linkedReports").is_none(), "{body}");
	assert!(
		safety_report.get("documentsHeldBySender").is_none(),
		"{body}"
	);
	assert!(data.get("messageHeader").is_some(), "{body}");
	assert_no_ae_lb_dg_payload(data);

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_ci_page_projection_returns_appendix_aware_field_envelopes(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case_with_appendices(
		&app,
		&cookie,
		"EDITOR-CI-PROJECTION",
		&["ich", "fda"],
	)
	.await?;
	create_safety_report(&app, &cookie, &case_id, "2", true).await?;

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/CI?appendix=fda"),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	assert_eq!(body["pageId"], "CI");
	assert_eq!(body["focusedAppendix"], "fda");
	assert!(body.get("appendices").is_none(), "{body}");
	assert!(body["saved"].as_bool().is_some(), "{body}");
	assert!(body["requiredCount"].as_u64().is_some(), "{body}");
	assert!(
		body["rows"]["safetyReportIdentification"].is_object(),
		"{body}"
	);
	assert!(body["rows"]["messageHeader"].is_null(), "{body}");
	assert!(body["rows"]["receiverInfo"].is_null(), "{body}");
	assert!(body["rows"]["otherCaseIdentifiers"].is_array(), "{body}");
	assert!(body["rows"]["linkedReports"].is_array(), "{body}");
	assert!(body["rows"]["documentsHeldBySender"].is_array(), "{body}");

	let report_type = &body["fields"]["reportType"];
	assert_eq!(report_type["fieldId"], "CASE_RPT_TYPE");
	assert_eq!(report_type["path"], "safetyReportIdentification.reportType");
	assert_eq!(report_type["value"], "2");
	assert_eq!(report_type["display"], "Report from study");
	assert_eq!(report_type["visible"], true);
	assert_eq!(report_type["editable"], true);
	assert_eq!(report_type["empty"], false);
	assert_eq!(report_type["requiredEmpty"], false);

	let local_criteria = &body["fields"]["localCriteriaReportType"];
	assert_eq!(local_criteria["fieldId"], "CASEU_LOC_REPORT_TYPE");
	assert_eq!(
		local_criteria["path"],
		"safetyReportIdentification.localCriteriaReportType"
	);
	assert_eq!(local_criteria["visible"], true);
	assert_eq!(local_criteria["requiredEmpty"], true);
	assert!(
		local_criteria["issues"]
			.as_array()
			.ok_or("missing localCriteriaReportType issues")?
			.iter()
			.any(|issue| issue["code"] == "FDA.C.1.7.1.REQUIRED"),
		"{body}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_ci_page_projection_accepts_multiple_profiles() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-CI-MULTI-PROFILE").await?;
	create_safety_report(&app, &cookie, &case_id, "2", true).await?;

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/CI?profiles=fda,mfds"),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	assert_eq!(body["pageId"], "CI");
	assert_eq!(body["profiles"], json!(["fda", "mfds"]));
	assert!(body.get("focusedAppendix").is_none(), "{body}");
	assert!(body["fields"].is_object(), "{body}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_ci_page_projection_uses_all_profiles_for_visibility() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-CI-USKR-FIELDS").await?;
	create_safety_report(&app, &cookie, &case_id, "2", true).await?;

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/CI?profiles=mfds,fda"),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["profiles"], json!(["mfds", "fda"]));
	assert_eq!(body["fields"]["localCriteriaReportType"]["visible"], true);
	assert_eq!(
		body["fields"]["combinationProductReportIndicator"]["visible"],
		true
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_ci_page_projection_preserves_legacy_null_focused_appendix(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-CI-LEGACY-FOCUS").await?;
	create_safety_report(&app, &cookie, &case_id, "2", true).await?;

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/CI"),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert!(
		body.get("focusedAppendix").is_some(),
		"legacy focusedAppendix key should be present: {body}"
	);
	assert_eq!(body["focusedAppendix"], Value::Null, "{body}");
	assert_eq!(body["profiles"], json!(["ich"]));

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_ci_page_patch_updates_only_report_type_and_returns_projection(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_with_appendices(&app, &cookie, "EDITOR-CI-PATCH", &["ich"])
			.await?;
	create_safety_report(&app, &cookie, &case_id, "1", false).await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/CI"),
		json!({
			"changes": {
				"reportType": { "value": "3" }
			}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["fields"]["reportType"]["value"], "3");
	assert_eq!(body["fields"]["reportType"]["display"], "Other");
	assert_eq!(body["fields"]["fulfilExpeditedCriteria"]["value"], false);

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/safety-report"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["report_type"], "3");
	assert_eq!(body["data"]["fulfil_expedited_criteria"], false);

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_ci_page_patch_accepts_profiles() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-CI-PATCH-PROFILES").await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/CI"),
		json!({
			"profiles": ["fda", "mfds"],
			"changes": {},
			"rows": {}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["profiles"], json!(["fda", "mfds"]));
	assert!(body.get("focusedAppendix").is_none(), "{body}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_ci_page_patch_can_clear_appendix_specific_field() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_with_appendices(&app, &cookie, "EDITOR-CI-CLEAR", &["fda"])
			.await?;
	create_safety_report_with_local_criteria(
		&app,
		&cookie,
		&case_id,
		"2",
		true,
		Some("1"),
	)
	.await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/CI"),
		json!({
			"appendix": "fda",
			"changes": {
				"localCriteriaReportType": { "value": null }
			}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["focusedAppendix"], "fda");
	assert_eq!(
		body["fields"]["localCriteriaReportType"]["value"],
		Value::Null
	);
	assert_eq!(
		body["fields"]["localCriteriaReportType"]["requiredEmpty"],
		true
	);

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/safety-report"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["local_criteria_report_type"], Value::Null);

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_ci_page_projection_uses_request_appendix_as_validation_context(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case_with_appendices(
		&app,
		&cookie,
		"EDITOR-CI-REQUEST-APPENDIX",
		&["fda"],
	)
	.await?;
	create_safety_report(&app, &cookie, &case_id, "2", true).await?;

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/CI?appendix=ich"),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["focusedAppendix"], "ich");
	assert!(body.get("appendices").is_none(), "{body}");
	assert_eq!(body["fields"]["localCriteriaReportType"]["visible"], false);
	assert_eq!(
		body["fields"]["localCriteriaReportType"]["requiredEmpty"],
		false
	);
	assert!(
		body["fields"]["localCriteriaReportType"]["issues"]
			.as_array()
			.ok_or("missing local criteria issues")?
			.is_empty(),
		"{body}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_page_projection_rejects_unknown_appendix_context() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_with_appendices(&app, &cookie, "EDITOR-BAD-APPENDIX", &["ich"])
			.await?;

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/CI?appendix=unknown"),
	)
	.await?;

	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_remaining_direct_pages_have_projection_routes() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_with_appendices(&app, &cookie, "EDITOR-PAGES", &["ich"]).await?;

	for (section, expected_key) in [
		("RP", "primarySources"),
		("SD", "senderInformation"),
		("LR", "literatureReferences"),
		("SI", "studyInformation"),
		("DM", "patientInformation"),
		("NR", "narrative"),
	] {
		let (status, body) = get_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/editor/pages/{section}?appendix=fda"),
		)
		.await?;

		assert_eq!(status, StatusCode::OK, "{section}: {body}");
		assert_eq!(body["caseId"], case_id);
		assert_eq!(body["pageId"], section);
		assert_eq!(body["focusedAppendix"], "fda");
		assert!(body.get("appendices").is_none(), "{section}: {body}");
		assert!(body["saved"].as_bool().is_some(), "{section}: {body}");
		assert!(
			body["requiredCount"].as_u64().is_some(),
			"{section}: {body}"
		);
		assert!(body["fields"].is_object(), "{section}: {body}");
		assert!(body["rows"].is_object(), "{section}: {body}");
		assert!(
			body["rows"].get(expected_key).is_some(),
			"{section}: {body}"
		);
		if matches!(section, "DM" | "NR" | "SI") {
			assert_eq!(body["saved"], false, "{section}: {body}");
		}
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_remaining_direct_pages_accept_page_patch_with_appendix() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_with_appendices(&app, &cookie, "EDITOR-PAGES-PATCH", &["ich"])
			.await?;

	for section in ["RP", "SD", "LR", "SI", "DM", "NR"] {
		let (status, body) = patch_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/editor/pages/{section}"),
			json!({
				"appendix": "fda",
				"changes": {}
			}),
		)
		.await?;

		assert_eq!(status, StatusCode::OK, "{section}: {body}");
		assert_eq!(body["focusedAppendix"], "fda", "{section}");
		assert!(body.get("appendices").is_none(), "{section}: {body}");
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_direct_page_patch_rejects_unknown_field() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_with_appendices(&app, &cookie, "EDITOR-PATCH-UNKNOWN", &["ich"])
			.await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/RP"),
		json!({
			"appendix": "fda",
			"changes": {
				"notAReporterField": { "value": "x" }
			}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_direct_page_patch_rejects_unknown_appendix() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case_with_appendices(
		&app,
		&cookie,
		"EDITOR-PATCH-BAD-APPENDIX",
		&["ich"],
	)
	.await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/NR"),
		json!({
			"appendix": "unknown",
			"changes": {}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_nr_page_patch_persists_narrative_row() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_with_appendices(&app, &cookie, "EDITOR-NR-PATCH", &["ich"])
			.await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/NR"),
		json!({
			"appendix": "fda",
			"rows": {
				"narrative": {
					"caseNarrative": "Narrative saved through page patch"
				}
			}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["focusedAppendix"], "fda");
	assert_eq!(
		body["rows"]["narrative"]["case_narrative"],
		"Narrative saved through page patch"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_rp_page_patch_persists_primary_source_row() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_with_appendices(&app, &cookie, "EDITOR-RP-PATCH", &["ich"])
			.await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/RP"),
		json!({
			"appendix": "fda",
			"rows": {
				"primarySources": [{
					"sequenceNumber": 1,
					"qualification": "1"
				}]
			}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["rows"]["primarySources"][0]["qualification"], "1");

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_sd_page_patch_persists_sender_information_row() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_with_appendices(&app, &cookie, "EDITOR-SD-PATCH", &["ich"])
			.await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/SD"),
		json!({
			"appendix": "fda",
			"rows": {
				"senderInformation": {
					"organizationName": "Sender Org"
				}
			}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["rows"]["senderInformation"][0]["organization_name"],
		"Sender Org"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_lr_page_patch_persists_literature_reference_row() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_with_appendices(&app, &cookie, "EDITOR-LR-PATCH", &["ich"])
			.await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/LR"),
		json!({
			"appendix": "fda",
			"rows": {
				"literatureReferences": [{
					"sequenceNumber": 1,
					"referenceText": "Smith 2026"
				}]
			}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["rows"]["literatureReferences"][0]["reference_text"],
		"Smith 2026"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_si_page_patch_persists_study_information_row() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_with_appendices(&app, &cookie, "EDITOR-SI-PATCH", &["ich"])
			.await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/SI"),
		json!({
			"appendix": "fda",
			"rows": {
				"studyInformation": {
					"studyName": "Study 001"
				}
			}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["rows"]["studyInformation"]["study_name"], "Study 001");

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_dm_page_patch_persists_patient_information_row() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_with_appendices(&app, &cookie, "EDITOR-DM-PATCH", &["ich"])
			.await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/DM"),
		json!({
			"appendix": "fda",
			"rows": {
				"patientInformation": {
					"patientInitials": "ABC"
				}
			}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["rows"]["patientInformation"]["patient_initials"],
		"ABC"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_dm_returns_patient_payload_without_dh_list_rows() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-DM").await?;

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
				"drug_name": "Prior Drug"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");

	let (status, body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/editor/DM")).await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	let data = body.get("data").ok_or("missing data")?;
	let patient_information = data
		.get("patientInformation")
		.ok_or("missing patientInformation")?;
	assert!(
		patient_information.get("pastDrugHistory").is_none(),
		"{body}"
	);
	assert!(patient_information.get("patientDeath").is_none(), "{body}");
	assert!(data["patientIdentifiers"].is_array(), "{body}");
	assert!(data["medicalHistoryEpisodes"].is_array(), "{body}");
	assert!(data.get("deathInfo").is_some(), "{body}");
	assert!(data["reportedCauses"].is_array(), "{body}");
	assert!(data["autopsyCauses"].is_array(), "{body}");
	assert!(data.get("parentInfo").is_some(), "{body}");
	assert!(data["parentMedicalHistory"].is_array(), "{body}");
	assert!(data["parentPastDrugs"].is_array(), "{body}");
	assert!(data.get("pastDrugHistory").is_none(), "{body}");
	assert_no_ae_lb_dg_payload(data);

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_nr_returns_narrative_payload_only() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-NR").await?;

	let (status, body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/editor/NR")).await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	let data = body.get("data").ok_or("missing data")?;
	let narrative = data.get("narrative").ok_or("missing narrative")?;
	assert!(narrative.get("senderDiagnoses").is_none(), "{body}");
	assert!(data["senderDiagnoses"].is_array(), "{body}");
	assert!(data["caseSummaryInformation"].is_array(), "{body}");
	assert_no_ae_lb_dg_payload(data);

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_remaining_direct_sections_return_only_their_payloads() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-DIRECT-SECTIONS").await?;

	for (section, expected_key) in [
		("RP", "primarySources"),
		("SD", "senderInformation"),
		("LR", "literatureReferences"),
		("SI", "studyInformation"),
	] {
		let (status, body) = get_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/editor/{section}"),
		)
		.await?;

		assert_eq!(status, StatusCode::OK, "{section}: {body}");
		assert_eq!(body["caseId"], case_id);
		let data = body.get("data").ok_or("missing data")?;
		assert!(data.get(expected_key).is_some(), "{section}: {body}");
		if section == "SI" {
			assert!(
				data["studyRegistrationNumbers"].is_array(),
				"{section}: {body}"
			);
			assert!(
				data["studyInformation"]
					.get("studyRegistrationNumbers")
					.is_none(),
				"{section}: {body}"
			);
		}
		assert_no_ae_lb_dg_payload(data);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_direct_sections_reject_missing_child_permissions() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_case(&app, &admin_cookie, "EDITOR-DIRECT-ACL").await?;

	let cases = [
		(
			"CI missing SAFETY_REPORT_READ",
			"CI",
			vec![
				CASE_READ,
				CASE_LIST,
				MESSAGE_HEADER_READ,
				RECEIVER_READ,
				CASE_IDENTIFIER_LIST,
			],
		),
		(
			"CI missing MESSAGE_HEADER_READ",
			"CI",
			vec![
				CASE_READ,
				CASE_LIST,
				SAFETY_REPORT_READ,
				RECEIVER_READ,
				CASE_IDENTIFIER_LIST,
			],
		),
		(
			"CI missing CASE_IDENTIFIER_LIST",
			"CI",
			vec![
				CASE_READ,
				CASE_LIST,
				SAFETY_REPORT_READ,
				MESSAGE_HEADER_READ,
				RECEIVER_READ,
			],
		),
		(
			"CI missing RECEIVER_READ",
			"CI",
			vec![
				CASE_READ,
				CASE_LIST,
				SAFETY_REPORT_READ,
				MESSAGE_HEADER_READ,
				CASE_IDENTIFIER_LIST,
			],
		),
		(
			"RP missing PRIMARY_SOURCE_LIST",
			"RP",
			vec![CASE_READ, CASE_LIST],
		),
		(
			"SD missing SAFETY_REPORT_READ",
			"SD",
			vec![CASE_READ, CASE_LIST, SENDER_INFORMATION_LIST],
		),
		(
			"SD missing SENDER_INFORMATION_LIST",
			"SD",
			vec![CASE_READ, CASE_LIST, SAFETY_REPORT_READ],
		),
		(
			"LR missing LITERATURE_REFERENCE_LIST",
			"LR",
			vec![CASE_READ, CASE_LIST],
		),
		(
			"SI missing STUDY_INFORMATION_LIST",
			"SI",
			vec![CASE_READ, CASE_LIST, STUDY_REGISTRATION_LIST],
		),
		(
			"SI missing STUDY_REGISTRATION_LIST",
			"SI",
			vec![CASE_READ, CASE_LIST, STUDY_INFORMATION_LIST],
		),
		(
			"DM missing PATIENT_READ",
			"DM",
			vec![
				CASE_READ,
				CASE_LIST,
				PATIENT_IDENTIFIER_LIST,
				MEDICAL_HISTORY_LIST,
				PATIENT_DEATH_LIST,
				DEATH_CAUSE_LIST,
				PARENT_INFORMATION_LIST,
				PARENT_MEDICAL_HISTORY_LIST,
				PARENT_PAST_DRUG_LIST,
			],
		),
		(
			"DM missing PATIENT_IDENTIFIER_LIST",
			"DM",
			vec![
				CASE_READ,
				CASE_LIST,
				PATIENT_READ,
				MEDICAL_HISTORY_LIST,
				PATIENT_DEATH_LIST,
				DEATH_CAUSE_LIST,
				PARENT_INFORMATION_LIST,
				PARENT_MEDICAL_HISTORY_LIST,
				PARENT_PAST_DRUG_LIST,
			],
		),
		(
			"DM missing MEDICAL_HISTORY_LIST",
			"DM",
			vec![
				CASE_READ,
				CASE_LIST,
				PATIENT_READ,
				PATIENT_IDENTIFIER_LIST,
				PATIENT_DEATH_LIST,
				DEATH_CAUSE_LIST,
				PARENT_INFORMATION_LIST,
				PARENT_MEDICAL_HISTORY_LIST,
				PARENT_PAST_DRUG_LIST,
			],
		),
		(
			"DM missing PATIENT_DEATH_LIST",
			"DM",
			vec![
				CASE_READ,
				CASE_LIST,
				PATIENT_READ,
				PATIENT_IDENTIFIER_LIST,
				MEDICAL_HISTORY_LIST,
				DEATH_CAUSE_LIST,
				PARENT_INFORMATION_LIST,
				PARENT_MEDICAL_HISTORY_LIST,
				PARENT_PAST_DRUG_LIST,
			],
		),
		(
			"DM missing DEATH_CAUSE_LIST",
			"DM",
			vec![
				CASE_READ,
				CASE_LIST,
				PATIENT_READ,
				PATIENT_IDENTIFIER_LIST,
				MEDICAL_HISTORY_LIST,
				PATIENT_DEATH_LIST,
				PARENT_INFORMATION_LIST,
				PARENT_MEDICAL_HISTORY_LIST,
				PARENT_PAST_DRUG_LIST,
			],
		),
		(
			"DM missing PARENT_INFORMATION_LIST",
			"DM",
			vec![
				CASE_READ,
				CASE_LIST,
				PATIENT_READ,
				PATIENT_IDENTIFIER_LIST,
				MEDICAL_HISTORY_LIST,
				PATIENT_DEATH_LIST,
				DEATH_CAUSE_LIST,
				PARENT_MEDICAL_HISTORY_LIST,
				PARENT_PAST_DRUG_LIST,
			],
		),
		(
			"DM missing PARENT_MEDICAL_HISTORY_LIST",
			"DM",
			vec![
				CASE_READ,
				CASE_LIST,
				PATIENT_READ,
				PATIENT_IDENTIFIER_LIST,
				MEDICAL_HISTORY_LIST,
				PATIENT_DEATH_LIST,
				DEATH_CAUSE_LIST,
				PARENT_INFORMATION_LIST,
				PARENT_PAST_DRUG_LIST,
			],
		),
		(
			"DM missing PARENT_PAST_DRUG_LIST",
			"DM",
			vec![
				CASE_READ,
				CASE_LIST,
				PATIENT_READ,
				PATIENT_IDENTIFIER_LIST,
				MEDICAL_HISTORY_LIST,
				PATIENT_DEATH_LIST,
				DEATH_CAUSE_LIST,
				PARENT_INFORMATION_LIST,
				PARENT_MEDICAL_HISTORY_LIST,
			],
		),
		(
			"NR missing NARRATIVE_READ",
			"NR",
			vec![
				CASE_READ,
				CASE_LIST,
				SENDER_DIAGNOSIS_LIST,
				CASE_SUMMARY_LIST,
			],
		),
		(
			"NR missing SENDER_DIAGNOSIS_LIST",
			"NR",
			vec![CASE_READ, CASE_LIST, NARRATIVE_READ, CASE_SUMMARY_LIST],
		),
		(
			"NR missing CASE_SUMMARY_LIST",
			"NR",
			vec![CASE_READ, CASE_LIST, NARRATIVE_READ, SENDER_DIAGNOSIS_LIST],
		),
	];

	for (label, section, permissions) in cases {
		let limited_cookie = limited_cookie(&mm, seed.org_id, permissions).await?;
		let (status, body) = get_json(
			&app,
			&limited_cookie,
			&format!("/api/cases/{case_id}/editor/{section}"),
		)
		.await?;
		assert_eq!(status, StatusCode::FORBIDDEN, "{label}: {body}");
	}

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
async fn editor_repeatable_pages_have_list_projection_routes() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-REPEATABLE-PAGES").await?;

	for (section, expected_key) in [
		("DH", "rows"),
		("AE", "rows"),
		("LB", "rows"),
		("DG", "rows"),
	] {
		let (status, body) = get_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/editor/pages/{section}?appendix=fda"),
		)
		.await?;

		assert_eq!(status, StatusCode::OK, "{section}: {body}");
		assert_eq!(body["caseId"], case_id);
		assert_eq!(body["pageId"], section);
		assert_eq!(body["focusedAppendix"], "fda");
		assert!(body.get("appendices").is_none(), "{section}: {body}");
		assert!(body["rows"][expected_key].is_array(), "{section}: {body}");
	}

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

	let reaction_id = create_reaction_fixture(&app, &cookie, &case_id).await?;

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
async fn editor_repeatable_page_rows_return_row_detail_by_uuid() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-REPEATABLE-ROWS").await?;
	let reaction_id = create_reaction_fixture(&app, &cookie, &case_id).await?;

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!(
			"/api/cases/{case_id}/editor/pages/AE/rows/{reaction_id}?appendix=fda"
		),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	assert_eq!(body["section"], "AE");
	assert_eq!(body["rowId"], reaction_id);
	assert!(body.get("appendices").is_none(), "{body}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_ae_page_row_patch_updates_one_reaction() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-AE-ROW-PATCH").await?;
	let reaction_id = create_reaction_fixture(&app, &cookie, &case_id).await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/AE/rows/{reaction_id}"),
		json!({
			"appendix": "fda",
			"rows": {
				"reaction": {
					"reactionPrimarySourceNative": "Updated reaction"
				}
			}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["section"], "AE");
	assert_eq!(body["rowId"], reaction_id);
	assert_eq!(
		body["data"]["reaction"]["primary_source_reaction"],
		"Updated reaction"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_lb_page_row_patch_updates_one_test_result() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-LB-ROW-PATCH").await?;
	let test_result_id = create_test_result_fixture(&app, &cookie, &case_id).await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/LB/rows/{test_result_id}"),
		json!({
			"appendix": "fda",
			"rows": {
				"testResult": {
					"testName": "Updated lab"
				}
			}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["section"], "LB");
	assert_eq!(body["rowId"], test_result_id);
	assert_eq!(body["data"]["testResult"]["test_name"], "Updated lab");

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_dg_page_row_patch_updates_one_drug() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-DG-ROW-PATCH").await?;
	let drug_id = create_drug_fixture(&app, &cookie, &case_id).await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/DG/rows/{drug_id}"),
		json!({
			"appendix": "fda",
			"rows": {
				"drug": {
					"medicinalProduct": "Updated product"
				}
			}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["section"], "DG");
	assert_eq!(body["rowId"], drug_id);
	assert_eq!(body["data"]["drug"]["medicinal_product"], "Updated product");

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_dh_page_row_patch_updates_one_drug_history() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-DH-ROW-PATCH").await?;
	let past_drug_id =
		create_past_drug_history_fixture(&app, &cookie, &case_id).await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/DH/rows/{past_drug_id}"),
		json!({
			"appendix": "fda",
			"rows": {
				"pastDrugHistory": {
					"drugName": "Updated prior drug"
				}
			}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["section"], "DH");
	assert_eq!(body["rowId"], past_drug_id);
	assert_eq!(
		body["data"]["pastDrugHistory"]["drug_name"],
		"Updated prior drug"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_repeatable_page_row_create_and_delete_routes_work_for_all_sections(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case(&app, &cookie, "EDITOR-REPEATABLE-ROW-CREATE-DELETE").await?;
	create_patient_fixture(&app, &cookie, &case_id).await?;

	let create_requests = [
		(
			"AE",
			json!({
				"appendix": "fda",
				"rows": {
					"reaction": {
						"reactionPrimarySourceNative": "Created reaction"
					}
				}
			}),
			"reaction",
		),
		(
			"LB",
			json!({
				"appendix": "fda",
				"rows": {
					"testResult": {
						"testName": "Created lab"
					}
				}
			}),
			"testResult",
		),
		(
			"DG",
			json!({
				"appendix": "fda",
				"rows": {
					"drug": {
						"medicinalProduct": "Created product"
					}
				}
			}),
			"drug",
		),
		(
			"DH",
			json!({
				"appendix": "fda",
				"rows": {
					"pastDrugHistory": {
						"drugName": "Created prior drug"
					}
				}
			}),
			"pastDrugHistory",
		),
	];

	for (section, request, response_key) in create_requests {
		let (status, body) = post_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/editor/pages/{section}/rows"),
			request,
		)
		.await?;
		assert_eq!(status, StatusCode::CREATED, "{section}: {body}");
		assert_eq!(body["section"], section);
		assert_eq!(body["focusedAppendix"], "fda");
		assert!(body["data"][response_key].is_object(), "{section}: {body}");
		let row_id = body["rowId"]
			.as_str()
			.ok_or("missing created page row id")?;

		let (status, body) = delete_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/editor/pages/{section}/rows/{row_id}"),
		)
		.await?;
		assert_eq!(status, StatusCode::NO_CONTENT, "{section}: {body}");
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn editor_repeatable_page_row_create_and_delete_mark_validation_cache_stale(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_case(&app, &cookie, "EDITOR-ROW-STALE").await?;

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/validation/all?profiles=fda"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		stale_validation_summary_count(&mm, seed.admin.id, seed.org_id, &case_id)
			.await?,
		0
	);

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/AE/rows"),
		json!({
			"appendix": "fda",
			"rows": {
				"reaction": {
					"reactionPrimarySourceNative": "Created stale reaction"
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let reaction_id = body["rowId"]
		.as_str()
		.ok_or("missing created reaction row id")?
		.to_string();
	assert!(
		stale_validation_summary_count(&mm, seed.admin.id, seed.org_id, &case_id)
			.await? > 0,
		"row create should mark cached validation summaries stale"
	);

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/validation/all?profiles=fda"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		stale_validation_summary_count(&mm, seed.admin.id, seed.org_id, &case_id)
			.await?,
		0
	);

	let (status, body) = delete_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/AE/rows/{reaction_id}"),
	)
	.await?;
	assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
	assert!(
		stale_validation_summary_count(&mm, seed.admin.id, seed.org_id, &case_id)
			.await? > 0,
		"row delete should mark cached validation summaries stale"
	);

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
async fn editor_dg_detail_requires_child_list_permissions() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let limited_role = format!("dg_detail_limited_{}", Uuid::new_v4());
	upsert_dynamic_role_permissions(
		&limited_role,
		vec![
			CASE_READ,
			CASE_LIST,
			DRUG_READ,
			DRUG_SUBSTANCE_LIST,
			DRUG_INDICATION_LIST,
			DRUG_REACTION_ASSESSMENT_LIST,
			DRUG_RECURRENCE_LIST,
		],
	);
	let limited_user = insert_user(
		&mm,
		seed.org_id,
		&limited_role,
		system_user_id(),
		Some("limitedpwd"),
	)
	.await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let limited_token =
		generate_web_token(&limited_user.email, limited_user.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let limited_cookie = cookie_header(&limited_token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &admin_cookie, "EDITOR-DG-ACL").await?;

	let (status, body) = post_json(
		&app,
		&admin_cookie,
		&format!("/api/cases/{case_id}/drugs"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"drug_characterization": "1",
				"medicinal_product": " "
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let drug_id = body["data"]["id"].as_str().ok_or("missing drug id")?;

	let (status, body) = get_json(
		&app,
		&limited_cookie,
		&format!("/api/cases/{case_id}/editor/DG/{drug_id}"),
	)
	.await?;

	assert_eq!(status, StatusCode::FORBIDDEN, "{body}");

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
