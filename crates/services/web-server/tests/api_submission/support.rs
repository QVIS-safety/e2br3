pub(super) use crate::common::{
	cookie_header, init_test_mm, seed_active_test_meddra_term, Result,
};
pub(super) use axum::body::{to_bytes, Body};
pub(super) use axum::extract::State;
pub(super) use axum::http::{Request, StatusCode};
pub(super) use axum::routing::post;
pub(super) use axum::{Json, Router};
pub(super) use lib_auth::token::generate_web_token;
pub(super) use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
pub(super) use lib_core::model::store::set_full_context_dbx;
pub(super) use serde_json::{json, Value};
pub(super) use serial_test::serial;
pub(super) use std::sync::Arc;
pub(super) use tokio::net::TcpListener;
pub(super) use tokio::sync::Mutex;
pub(super) use tokio::time::{sleep, Duration};
pub(super) use tower::ServiceExt;
pub(super) use uuid::Uuid;

const PROFILE_TEST_KEY: &str = "test-only-profile-key-32-characters";

pub(super) fn profile_input(
) -> lib_core::model::organization_as2_profile::OrganizationAs2ProfileInput {
	lib_core::model::organization_as2_profile::OrganizationAs2ProfileInput {
		endpoint_url: "http://127.0.0.1:9/as2".into(),
		as2_to: "TEST-PARTNER".into(),
		expected_ack_as2_from: "TEST-PARTNER".into(),
		extra_headers: json!({}),
		signing_pkcs12_base64: include_str!("../fixtures/as2/test-only.p12.b64")
			.trim()
			.into(),
		signing_pkcs12_password: "changeit".into(),
		signing_key_alias: None,
		partner_certificate_pem: include_str!(
			"../fixtures/as2/test-only-partner.pem"
		)
		.into(),
		async_mdn_url: None,
	}
}

pub(super) async fn seed_org_with_users(
	mm: &lib_core::model::ModelManager,
	admin_pwd: &str,
	viewer_pwd: &str,
) -> Result<crate::common::SeedOrgUsers> {
	let seed = crate::common::seed_org_with_users(mm, admin_pwd, viewer_pwd).await?;
	let previous_key = std::env::var("AS2_PROFILE_ENCRYPTION_KEY").ok();
	std::env::set_var("AS2_PROFILE_ENCRYPTION_KEY", PROFILE_TEST_KEY);
	let ctx = lib_core::ctx::Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	for authority in ["fda", "mfds"] {
		lib_core::model::organization_as2_profile::OrganizationAs2ProfileBmc::upsert(
			&ctx, mm, seed.org_id, authority, profile_input(),
		).await?;
	}
	match previous_key {
		Some(key) => std::env::set_var("AS2_PROFILE_ENCRYPTION_KEY", key),
		None => std::env::remove_var("AS2_PROFILE_ENCRYPTION_KEY"),
	}
	Ok(seed)
}

pub(super) fn parse_json_or_raw(body: &[u8]) -> Value {
	let raw = String::from_utf8_lossy(body).trim().to_string();
	if raw.is_empty() {
		return json!({});
	}
	serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({ "raw": raw }))
}

pub(super) fn clear_submission_env() {
	std::env::remove_var("AS2_SUBMITTER_URL");
	std::env::remove_var("AS2_SUBMITTER_TOKEN");
	std::env::remove_var("AS2_SUBMITTER_TIMEOUT_SECS");
	std::env::remove_var("AS2_ACK_CALLBACK_URL");
	std::env::remove_var("AS2_CALLBACK_TOKEN");
	std::env::remove_var("AS2_PROFILE_ENCRYPTION_KEY");
	std::env::remove_var("SUBMISSION_RECONCILE_TOKEN");
}

pub(super) fn configure_as2_test_endpoint(url: impl AsRef<str>) {
	std::env::set_var("AS2_SUBMITTER_URL", url.as_ref());
	std::env::set_var("AS2_SUBMITTER_TOKEN", "test-token");
	std::env::set_var("AS2_SUBMITTER_TIMEOUT_SECS", "5");
	std::env::set_var(
		"AS2_ACK_CALLBACK_URL",
		"http://127.0.0.1:9/internal/submissions/callbacks/ack",
	);
	std::env::set_var("AS2_CALLBACK_TOKEN", "callback-secret");
	std::env::set_var("AS2_PROFILE_ENCRYPTION_KEY", PROFILE_TEST_KEY);
}

pub(super) fn valid_compliance_payload() -> Value {
	json!({
		"reason_for_change": "submit case to FDA gateway",
		"e_signature": {
			"meaning": "submit case",
			"password": "adminpwd"
		}
	})
}

pub(super) async fn post_json(
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
	let value = parse_json_or_raw(&body);
	Ok((status, value))
}

pub(super) async fn post_json_with_headers(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
	extra_headers: &[(&str, &str)],
) -> Result<(StatusCode, Value)> {
	let mut req = Request::builder()
		.method("POST")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json");
	for (k, v) in extra_headers {
		req = req.header(*k, *v);
	}
	let req = req.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = parse_json_or_raw(&body);
	Ok((status, value))
}

pub(super) async fn put_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("PUT")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = parse_json_or_raw(&body);
	Ok((status, value))
}

pub(super) async fn get_json(
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

#[derive(Clone)]
pub(super) struct MockSubmitterState {
	received: Arc<Mutex<Vec<Value>>>,
}

pub(super) async fn start_mock_submitter() -> Result<(String, Arc<Mutex<Vec<Value>>>)>
{
	let received = Arc::new(Mutex::new(Vec::<Value>::new()));
	let state = MockSubmitterState {
		received: received.clone(),
	};
	let app = Router::new()
		.route("/submit", post(mock_submitter_submit))
		.with_state(state);
	let listener = TcpListener::bind("127.0.0.1:0").await?;
	let addr = listener.local_addr()?;
	tokio::spawn(async move {
		let _ = axum::serve(listener, app).await;
	});
	Ok((format!("http://{addr}"), received))
}

pub(super) async fn mock_submitter_submit(
	State(state): State<MockSubmitterState>,
	Json(payload): Json<Value>,
) -> (StatusCode, Json<Value>) {
	state.received.lock().await.push(payload);
	(
		StatusCode::OK,
		Json(json!({
			"remote_submission_id": format!("AS2-MOCK-{}", Uuid::new_v4().simple().to_string().to_uppercase()),
			"ack": { "level": 1, "success": true, "code": "ACK1" },
		})),
	)
}

pub(super) async fn configure_mock_as2_transport() -> Result<()> {
	let (submitter_url, _received) = start_mock_submitter().await?;
	configure_as2_test_endpoint(submitter_url);
	Ok(())
}

pub(super) async fn create_case(
	app: &axum::Router,
	cookie: &str,
	org_id: Uuid,
) -> Result<Uuid> {
	create_case_with_profile(app, cookie, org_id, "fda").await
}

pub(super) async fn create_case_with_profile(
	app: &axum::Router,
	cookie: &str,
	_org_id: Uuid,
	_appendix: &str,
) -> Result<Uuid> {
	let initial_report_id = format!("US-SENDER-{}", Uuid::new_v4().simple());
	let body = json!({
		"data": {
			"status": "draft",
			"fdaReportType": "1",
			"mfdsReportType": "4",
			"safetyReportIdentification": {
				"safetyReportId": initial_report_id
			}
		}
	});
	let (status, value) = post_json(app, cookie, "/api/cases", body).await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create case failed: status={status} body={value}").into()
		);
	}
	let id = value["data"]["id"].as_str().ok_or("missing case id")?;
	Ok(Uuid::parse_str(id)?)
}

pub(super) async fn create_safety_report(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"safety_report_id": format!("US-SENDER-{}", case_id.simple()),
			"transmission_date": "20241001000000",
				"report_type": "1",
				"date_first_received_from_source": "20241001",
				"date_of_most_recent_information": "20241001",
				"fulfil_expedited_criteria": false,
				"additional_documents_available": false,
				"worldwide_unique_id": format!("US-SENDER-{}", case_id.simple()),
				"first_sender_type": "1",
				"local_criteria_report_type": "2",
				"combination_product_report_indicator": "false",
				"other_case_identifiers_exist": null,
				"other_case_identifiers_exist_null_flavor": "NI"
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/safety-report"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create safety report failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

pub(super) async fn create_sender(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let gateways = ["ich", "fda", "mfds"]
		.into_iter()
		.enumerate()
		.map(|(index, authority)| {
			json!({
				"sequenceNumber": index + 1,
				"gatewayAuthority": authority,
				"senderIdentifier": format!("TEST-{}", authority.to_ascii_uppercase()),
				"routingIdentifier": format!("ROUTE-{case_id}-{authority}"),
				"isDefaultForAuthority": true
			})
		})
		.collect::<Vec<_>>();
	let (status, value) = post_json(
		app,
		cookie,
		"/api/presaves/senders",
		json!({
			"data": { "rows": {
				"sender": {
					"senderType": "1",
					"organizationName": format!("Sender Org {case_id}")
				},
				"gateways": gateways,
				"responsiblePersons": []
			} }
		}),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create sender presave failed: status={status} body={value}"
		)
		.into());
	}
	let sender_presave_id = value["data"]["rows"]["sender"]["id"]
		.as_str()
		.ok_or("missing sender presave id")?;
	let (status, receiver) = post_json(
		app,
		cookie,
		"/api/presaves/receivers",
		json!({"data": {"rows": {
			"receiver": {
				"receiverType": "Regulatory Authority",
				"organizationName": format!("Test Receiver {case_id}")
			},
			"consignees": [],
			"routes": [
				{
					"sequenceNumber": 1, "authority": "ich", "receiverLabel": "ICH",
					"batchReceiverIdentifier": "ICH-BATCH", "messageReceiverIdentifier": "ICH-RECEIVER",
					"conditionPage": "CI", "conditionFieldCode": "C.1.3", "conditionOperator": "Equal",
					"conditionValueCode": "1", "conditionValueLabel": "Spontaneous"
				},
				{
					"sequenceNumber": 2, "authority": "fda", "receiverLabel": "FDA(CDER IND)",
					"batchReceiverIdentifier": "ZZFDA_PREMKT", "messageReceiverIdentifier": "CDER_IND",
					"conditionPage": "CI", "conditionFieldCode": "FDA_REPORT_TYPE", "conditionOperator": "Equal",
					"conditionValueCode": "1", "conditionValueLabel": "CDER IND"
				},
				{
					"sequenceNumber": 3, "authority": "mfds", "receiverLabel": "MFDS(FR)",
					"batchReceiverIdentifier": "MFDS-O-FR", "messageReceiverIdentifier": "MFDS-O-FR",
					"conditionPage": "CI", "conditionFieldCode": "MFDS_REPORT_TYPE", "conditionOperator": "Equal",
					"conditionValueCode": "4", "conditionValueLabel": "FR"
				}
			]
		}}}),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create receiver presave failed: status={status} body={receiver}"
		)
		.into());
	}
	let receiver_presave_id = receiver["data"]["rows"]["receiver"]["id"]
		.as_str()
		.ok_or("missing receiver presave id")?;
	let product_id = format!("TEST-PRODUCT-{}", case_id.simple());
	let (status, product) = post_json(
		app,
		cookie,
		"/api/presaves/products",
		json!({"data": {"rows": {
			"product": {
				"senderPresaveId": sender_presave_id,
				"receiverPresaveId": receiver_presave_id,
				"productId": product_id,
				"medicinalProduct": "Test Product"
			},
			"activeSubstances": []
		}}}),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create product presave failed: status={status} body={product}"
		)
		.into());
	}
	let (status, case) = put_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {"dg_prd_key": product_id},
			"reason_for_change": "test fixture product assignment"
		}),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"link case product failed: status={status} body={case}"
		)
		.into());
	}
	let body = json!({
		"data": {
			"case_id": case_id,
			"sender_type": "1",
			"organization_name": "Sender Org",
			"department": "Drug Safety",
			"person_title": "Dr",
			"person_given_name": "Test",
			"person_family_name": "Sender",
			"street_address": "1 Test Street",
			"city": "Boston",
			"state": "MA",
			"postcode": "02108",
			"country_code": "US",
			"telephone": "+1-202-555-0100",
			"fax": "+1-202-555-0101",
			"email": "sender@example.test",
			"source_sender_presave_id": sender_presave_id
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/safety-report/senders"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create sender failed: status={status} body={value}").into(),
		);
	}
	Ok(())
}

pub(super) async fn create_primary_source(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let body = json!({
		"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"qualification": "1",
				"country_code": "US",
				"email": "reporter@example.com",
				"primary_source_regulatory": "1"
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/safety-report/primary-sources"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create primary source failed: status={status} body={value}"
		)
		.into());
	}
	let primary_source_id = value["data"]["id"]
		.as_str()
		.ok_or("missing primary source id")?;
	let req = Request::builder()
		.method("PUT")
		.uri(format!(
			"/api/cases/{case_id}/safety-report/primary-sources/{primary_source_id}"
		))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(
			json!({"data": {
				"country_code": "US",
				"email": "reporter@example.com",
				"primary_source_regulatory": "1"
			}})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	if status != StatusCode::OK {
		return Err(format!(
			"update primary source failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

pub(super) async fn create_fda_ind_details(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let (status, study) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/safety-report/studies"),
		json!({"data": {
			"case_id": case_id,
			"fda_ind_number_occurred": "123456"
		}}),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create FDA IND details failed: status={status} body={study}"
		)
		.into());
	}
	let study_id = study["data"]["id"].as_str().ok_or("missing study id")?;
	let (status, cross_reported) = post_json(
		app,
		cookie,
		&format!(
			"/api/cases/{case_id}/safety-report/studies/{study_id}/fda-cross-reported-inds"
		),
		json!({"data": {
			"study_information_id": study_id,
			"sequence_number": 1,
			"ind_number_null_flavor": "NA"
		}}),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create FDA cross-reported IND failed: status={status} body={cross_reported}"
		)
		.into());
	}
	Ok(())
}

pub(super) async fn create_patient(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"patient_initials": "AB",
			"sex": "1"
		}
	});
	let (status, value) =
		post_json(app, cookie, &format!("/api/cases/{case_id}/patient"), body)
			.await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(
			format!("create patient failed: status={status} body={value}").into(),
		);
	}
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/cases/{case_id}/patient"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(
			json!({
				"data": {
					"race_codes": ["C41260"],
					"ethnicity_code": "C41222",
					"medical_history_text": "No relevant medical history."
				}
			})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	if status != StatusCode::OK {
		return Err(
			format!("update patient failed: status={status} body={value}").into(),
		);
	}
	Ok(())
}

pub(super) async fn create_reaction(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"primary_source_reaction": "Headache"
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/reactions"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create reaction failed: status={status} body={value}").into(),
		);
	}
	let reaction_id = value["data"]["id"].as_str().ok_or("missing reaction id")?;
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/cases/{case_id}/reactions/{reaction_id}"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(
			json!({ "data": {
					"reaction_meddra_version": "26.0",
					"reaction_meddra_code": "10000001",
				"outcome": "1",
				"reaction_language": "eng",
				"serious": false,
				"criteria_death_null_flavor": "NI",
				"criteria_life_threatening_null_flavor": "NI",
				"criteria_hospitalization_null_flavor": "NI",
				"criteria_disabling_null_flavor": "NI",
				"criteria_congenital_anomaly_null_flavor": "NI",
				"criteria_other_medically_important_null_flavor": "NI",
				"required_intervention_null_flavor": "NI"
			}})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	if status != StatusCode::OK {
		return Err(
			format!("update reaction failed: status={status} body={value}").into(),
		);
	}
	Ok(())
}

pub(super) async fn create_drug(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"drug_characterization": "1",
			"medicinal_product": "Drug A",
			"mpid": "MPID-TEST",
			"mpid_version": "1",
			"mfds_mpid": "KR-MPID",
			"mfds_mpid_version": "KR-V1"
		}
	});
	let (status, value) =
		post_json(app, cookie, &format!("/api/cases/{case_id}/drugs"), body).await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create drug failed: status={status} body={value}").into()
		);
	}
	Ok(())
}

pub(super) async fn create_narrative(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"case_narrative": "Case narrative for submission validator fixture."
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/narrative"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(format!(
			"create narrative failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

pub(super) async fn seed_rule_clean_case(
	mm: &lib_core::model::ModelManager,
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	seed_active_test_meddra_term(mm).await?;
	create_safety_report(app, cookie, case_id).await?;
	create_fda_ind_details(app, cookie, case_id).await?;
	create_sender(app, cookie, case_id).await?;
	create_primary_source(app, cookie, case_id).await?;
	create_patient(app, cookie, case_id).await?;
	create_reaction(app, cookie, case_id).await?;
	create_drug(app, cookie, case_id).await?;
	create_narrative(app, cookie, case_id).await?;
	Ok(())
}
