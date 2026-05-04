use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use lib_auth::token::generate_web_token;
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::ModelManager;
use lib_core::validation::find_canonical_rule;
use serde_json::{json, Value};
use sqlx::types::Uuid as SqlxUuid;
use std::collections::BTreeSet;
use tokio::time::{sleep, Duration};
use tower::ServiceExt;
use uuid::Uuid;

fn parse_json_or_raw(body: &[u8]) -> Result<Value> {
	let raw = String::from_utf8_lossy(body).trim().to_string();
	if raw.is_empty() {
		return Ok(Value::Null);
	}
	Ok(serde_json::from_slice::<Value>(body)
		.unwrap_or_else(|_| json!({ "raw": raw })))
}

pub struct ValidationCtx {
	pub app: Router,
	pub cookie: String,
	pub case_id: Uuid,
	pub mm: ModelManager,
	pub admin_id: Uuid,
	pub org_id: Uuid,
}

pub async fn setup_case() -> Result<ValidationCtx> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	Ok(ValidationCtx {
		app,
		cookie,
		case_id,
		mm,
		admin_id: seed.admin.id,
		org_id: seed.org_id,
	})
}

pub async fn post_json(
	app: &Router,
	cookie: &str,
	uri: String,
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
	let value = parse_json_or_raw(&body)?;
	Ok((status, value))
}

pub async fn put_json(
	app: &Router,
	cookie: &str,
	uri: String,
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
	let value = parse_json_or_raw(&body)?;
	Ok((status, value))
}

pub async fn get_json(
	app: &Router,
	cookie: &str,
	uri: String,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("GET")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, parse_json_or_raw(&body)?))
}

pub async fn create_case(app: &Router, cookie: &str, org_id: Uuid) -> Result<Uuid> {
	let body = json!({
		"data": {
			"organization_id": org_id,
			"safety_report_id": format!("SR-VAL-{}", Uuid::new_v4()),
			"status": "draft"
		}
	});
	let (status, value) =
		post_json(app, cookie, "/api/cases".to_string(), body).await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create case failed: status={status} body={value}").into()
		);
	}
	extract_id(&value)
}

pub async fn create_safety_report(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	fulfil_expedited_criteria: bool,
) -> Result<Uuid> {
	create_safety_report_with(app, cookie, case_id, "1", fulfil_expedited_criteria)
		.await
}

pub async fn create_safety_report_with(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	report_type: &str,
	fulfil_expedited_criteria: bool,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"transmission_date": [2024, 1],
			"report_type": report_type,
			"date_first_received_from_source": [2024, 1],
			"date_of_most_recent_information": [2024, 1],
			"fulfil_expedited_criteria": fulfil_expedited_criteria
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/safety-report"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(format!(
			"create safety-report failed: status={status} body={value}"
		)
		.into());
	}
	extract_id(&value)
}

pub async fn update_safety_report(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	body: Value,
) -> Result<()> {
	let (status, value) = put_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/safety-report"),
		body,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update safety-report failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

pub async fn create_message_header(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	batch_receiver_identifier: Option<&str>,
) -> Result<Uuid> {
	create_message_header_with_receiver(
		app,
		cookie,
		case_id,
		batch_receiver_identifier,
		"RECEIVER01",
	)
	.await
}

pub async fn create_message_header_with_receiver(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	batch_receiver_identifier: Option<&str>,
	message_receiver_identifier: &str,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"message_number": format!("MSG-{case_id}"),
			"message_sender_identifier": "SENDER01",
			"message_receiver_identifier": message_receiver_identifier,
			"message_date": "20240201010101",
			"batch_receiver_identifier": batch_receiver_identifier
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/message-header"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(format!(
			"create message-header failed: status={status} body={value}"
		)
		.into());
	}
	extract_id(&value)
}

pub async fn create_study_information(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	study_name: Option<&str>,
	sponsor_study_number: Option<&str>,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"study_name": study_name,
			"sponsor_study_number": sponsor_study_number
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/safety-report/studies"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create study-information failed: status={status} body={value}"
		)
		.into());
	}
	extract_id(&value)
}

pub async fn update_study_information(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	study_id: Uuid,
	body: Value,
) -> Result<()> {
	let (status, value) = put_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/safety-report/studies/{study_id}"),
		body,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update study-information failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

pub async fn create_study_registration(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	study_id: Uuid,
	sequence_number: i32,
	registration_number: &str,
	country_code: Option<&str>,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"study_information_id": study_id,
			"sequence_number": sequence_number,
			"registration_number": registration_number,
			"country_code": country_code
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!(
			"/api/cases/{case_id}/safety-report/studies/{study_id}/registrations"
		),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create study-registration failed: status={status} body={value}"
		)
		.into());
	}
	extract_id(&value)
}

pub async fn create_sender(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	sender_type: &str,
	organization_name: &str,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sender_type": sender_type,
			"organization_name": organization_name
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/safety-report/senders"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create sender failed: status={status} body={value}").into(),
		);
	}
	extract_id(&value)
}

pub async fn create_primary_source(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	sequence_number: i32,
	qualification: Option<&str>,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": sequence_number,
			"qualification": qualification
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/safety-report/primary-sources"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create primary-source failed: status={status} body={value}"
		)
		.into());
	}
	extract_id(&value)
}

pub async fn create_other_case_identifier(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	sequence_number: i32,
	source_of_identifier: &str,
	case_identifier: &str,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": sequence_number,
			"source_of_identifier": source_of_identifier,
			"case_identifier": case_identifier
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/other-identifiers"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create other-case-identifier failed: status={status} body={value}"
		)
		.into());
	}
	extract_id(&value)
}

pub async fn update_primary_source(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	id: Uuid,
	body: Value,
) -> Result<()> {
	let (status, value) = put_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/safety-report/primary-sources/{id}"),
		body,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update primary-source failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

pub async fn create_patient(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	patient_initials: Option<&str>,
	sex: Option<&str>,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"patient_initials": patient_initials,
			"sex": sex
		}
	});
	let (status, value) =
		post_json(app, cookie, format!("/api/cases/{case_id}/patient"), body)
			.await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(
			format!("create patient failed: status={status} body={value}").into(),
		);
	}
	extract_id(&value)
}

pub async fn update_patient(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	body: Value,
) -> Result<()> {
	let (status, value) =
		put_json(app, cookie, format!("/api/cases/{case_id}/patient"), body).await?;
	if status != StatusCode::OK {
		return Err(
			format!("update patient failed: status={status} body={value}").into(),
		);
	}
	Ok(())
}

pub async fn create_past_drug_history(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	sequence_number: i32,
	drug_name: Option<&str>,
	mpid: Option<&str>,
	mpid_version: Option<&str>,
) -> Result<Uuid> {
	let (status, patient_value) =
		get_json(app, cookie, format!("/api/cases/{case_id}/patient")).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"get patient for past-drug-history failed: status={status} body={patient_value}"
		)
		.into());
	}
	let patient_id = extract_id(&patient_value)?;
	let body = json!({
		"data": {
			"patient_id": patient_id,
			"sequence_number": sequence_number,
			"drug_name": drug_name,
			"mpid": mpid,
			"mpid_version": mpid_version
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/patient/past-drugs"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create past-drug-history failed: status={status} body={value}"
		)
		.into());
	}
	extract_id(&value)
}

pub async fn create_parent_information(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	sex: Option<&str>,
) -> Result<Uuid> {
	let (status, patient_value) =
		get_json(app, cookie, format!("/api/cases/{case_id}/patient")).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"get patient for parent-information failed: status={status} body={patient_value}"
		)
		.into());
	}
	let patient_id = extract_id(&patient_value)?;
	let body = json!({
		"data": {
			"patient_id": patient_id,
			"sex": sex,
			"medical_history_text": null
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/patient/parents"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create parent-information failed: status={status} body={value}"
		)
		.into());
	}
	extract_id(&value)
}

pub async fn create_parent_past_drug_history(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	parent_id: Uuid,
	sequence_number: i32,
	drug_name: Option<&str>,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"parent_id": parent_id,
			"sequence_number": sequence_number,
			"drug_name": drug_name
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/patient/parent/{parent_id}/past-drugs"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create parent-past-drug-history failed: status={status} body={value}"
		)
		.into());
	}
	extract_id(&value)
}

pub async fn update_parent_past_drug_history(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	parent_id: Uuid,
	id: Uuid,
	body: Value,
) -> Result<()> {
	let (status, value) = put_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/patient/parent/{parent_id}/past-drugs/{id}"),
		body,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update parent-past-drug-history failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

pub async fn create_reaction(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	sequence_number: i32,
	primary_source_reaction: &str,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": sequence_number,
			"primary_source_reaction": primary_source_reaction
		}
	});
	let (status, value) =
		post_json(app, cookie, format!("/api/cases/{case_id}/reactions"), body)
			.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create reaction failed: status={status} body={value}").into(),
		);
	}
	extract_id(&value)
}

pub async fn update_reaction(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	id: Uuid,
	body: Value,
) -> Result<()> {
	let (status, value) = put_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/reactions/{id}"),
		body,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(
			format!("update reaction failed: status={status} body={value}").into(),
		);
	}
	Ok(())
}

pub async fn create_test_result(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	sequence_number: i32,
	test_name: &str,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": sequence_number,
			"test_name": test_name
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/test-results"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create test-result failed: status={status} body={value}"
		)
		.into());
	}
	let id = extract_id(&value)?;
	let (status, value) = put_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/test-results/{id}"),
		json!({"data": { "result_unstructured": "Baseline result" }}),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"prime test-result failed: status={status} body={value}"
		)
		.into());
	}
	Ok(id)
}

pub async fn create_drug(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	sequence_number: i32,
	drug_characterization: &str,
	medicinal_product: &str,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": sequence_number,
			"drug_characterization": drug_characterization,
			"medicinal_product": medicinal_product
		}
	});
	let (status, value) =
		post_json(app, cookie, format!("/api/cases/{case_id}/drugs"), body).await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create drug failed: status={status} body={value}").into()
		);
	}
	extract_id(&value)
}

pub async fn create_drug_device_characteristic(
	ctx: &ValidationCtx,
	drug_id: Uuid,
	sequence_number: i32,
	code: Option<&str>,
	value_type: Option<&str>,
	value_code: Option<&str>,
	value_value: Option<&str>,
) -> Result<Uuid> {
	ctx.mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		ctx.mm.dbx(),
		SqlxUuid::parse_str(&ctx.admin_id.to_string())?,
		SqlxUuid::parse_str(&ctx.org_id.to_string())?,
		lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	let sql = "INSERT INTO drug_device_characteristics \
		(drug_id, sequence_number, code, value_type, value_code, value_value, created_at, updated_at, created_by) \
		VALUES ($1, $2, $3, $4, $5, $6, now(), now(), $7) RETURNING id";
	let (id,) = ctx
		.mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (SqlxUuid,)>(sql)
				.bind(SqlxUuid::parse_str(&drug_id.to_string())?)
				.bind(sequence_number)
				.bind(code)
				.bind(value_type)
				.bind(value_code)
				.bind(value_value)
				.bind(SqlxUuid::parse_str(&ctx.admin_id.to_string())?),
		)
		.await?;
	ctx.mm.dbx().commit_txn().await?;
	Ok(Uuid::parse_str(&id.to_string())?)
}

pub async fn update_drug(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	id: Uuid,
	body: Value,
) -> Result<()> {
	let (status, value) = put_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/drugs/{id}"),
		body,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(
			format!("update drug failed: status={status} body={value}").into()
		);
	}
	Ok(())
}

pub async fn create_active_substance(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	drug_id: Uuid,
	sequence_number: i32,
	substance_name: Option<&str>,
	substance_termid: Option<&str>,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"drug_id": drug_id,
			"sequence_number": sequence_number,
			"substance_name": substance_name,
			"substance_termid": substance_termid
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create active-substance failed: status={status} body={value}"
		)
		.into());
	}
	extract_id(&value)
}

pub async fn create_dosage(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	drug_id: Uuid,
	sequence_number: i32,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"drug_id": drug_id,
			"sequence_number": sequence_number
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/dosages"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create dosage failed: status={status} body={value}").into(),
		);
	}
	extract_id(&value)
}

pub async fn create_drug_indication(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	drug_id: Uuid,
	sequence_number: i32,
	indication_text: Option<&str>,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"drug_id": drug_id,
			"sequence_number": sequence_number,
			"indication_text": indication_text
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/indications"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create drug-indication failed: status={status} body={value}"
		)
		.into());
	}
	extract_id(&value)
}

pub async fn create_drug_reaction_assessment(
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
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create drug-reaction-assessment failed: status={status} body={value}"
		)
		.into());
	}
	extract_id(&value)
}

pub async fn create_relatedness_assessment(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	drug_id: Uuid,
	assessment_id: Uuid,
	sequence_number: i32,
	source_of_assessment: Option<&str>,
	method_of_assessment: Option<&str>,
	result_of_assessment: Option<&str>,
	result_of_assessment_kr2: Option<&str>,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"drug_reaction_assessment_id": assessment_id,
			"sequence_number": sequence_number,
			"source_of_assessment": source_of_assessment,
			"method_of_assessment": method_of_assessment,
			"result_of_assessment": result_of_assessment,
			"result_of_assessment_kr2": result_of_assessment_kr2
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create relatedness-assessment failed: status={status} body={value}"
		)
		.into());
	}
	extract_id(&value)
}

pub async fn create_narrative(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	case_narrative: &str,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"case_narrative": case_narrative
		}
	});
	let (status, value) =
		post_json(app, cookie, format!("/api/cases/{case_id}/narrative"), body)
			.await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(format!(
			"create narrative failed: status={status} body={value}"
		)
		.into());
	}
	extract_id(&value)
}

pub async fn create_sender_diagnosis(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	narrative_id: Uuid,
	sequence_number: i32,
	diagnosis_meddra_code: Option<&str>,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"narrative_id": narrative_id,
			"sequence_number": sequence_number,
			"diagnosis_meddra_code": diagnosis_meddra_code
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/narrative/sender-diagnoses"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create sender-diagnosis failed: status={status} body={value}"
		)
		.into());
	}
	extract_id(&value)
}

pub async fn create_case_summary(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	narrative_id: Uuid,
	sequence_number: i32,
	summary_text: &str,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"narrative_id": narrative_id,
			"sequence_number": sequence_number,
			"summary_text": summary_text
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/narrative/summaries"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create case-summary failed: status={status} body={value}"
		)
		.into());
	}
	extract_id(&value)
}

#[allow(dead_code)]
pub async fn update_narrative(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	body: Value,
) -> Result<()> {
	let (status, value) =
		put_json(app, cookie, format!("/api/cases/{case_id}/narrative"), body)
			.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update narrative failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

pub async fn validate_case(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	profile: &str,
) -> Result<Value> {
	let (status, value) = get_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/validation?profile={profile}"),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(
			format!("validate case failed: status={status} body={value}").into(),
		);
	}
	Ok(value)
}

pub fn issue_codes(validation_body: &Value) -> Vec<String> {
	validation_body["data"]["issues"]
		.as_array()
		.into_iter()
		.flatten()
		.filter_map(|issue| issue["code"].as_str().map(str::to_string))
		.collect()
}

pub fn assert_has_code(validation_body: &Value, code: &str) {
	let found = issue_codes(validation_body).into_iter().any(|c| c == code);
	assert!(
		found,
		"expected code {code} in validation report; got body={validation_body}"
	);
}

pub fn assert_lacks_code(validation_body: &Value, code: &str) {
	let found = issue_codes(validation_body).into_iter().any(|c| c == code);
	assert!(
		!found,
		"expected code {code} to be absent in validation report; got body={validation_body}"
	);
}

pub async fn db_exec_case_sql(ctx: &ValidationCtx, sql: &str) -> Result<()> {
	let admin_id = SqlxUuid::parse_str(&ctx.admin_id.to_string())?;
	let org_id = SqlxUuid::parse_str(&ctx.org_id.to_string())?;
	for attempt in 0..3 {
		ctx.mm.dbx().begin_txn().await?;
		set_full_context_dbx(
			ctx.mm.dbx(),
			admin_id,
			org_id,
			lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO,
		)
		.await?;
		match ctx.mm.dbx().execute(sqlx::query(sql)).await {
			Ok(_) => {
				ctx.mm.dbx().commit_txn().await?;
				return Ok(());
			}
			Err(err) => {
				let _ = ctx.mm.dbx().rollback_txn().await;
				let is_retryable = err.to_string().contains("deadlock detected");
				if is_retryable && attempt < 2 {
					sleep(Duration::from_millis(50 * (attempt + 1) as u64)).await;
					continue;
				}
				return Err(err.into());
			}
		}
	}
	unreachable!("db_exec_case_sql retry loop exhausted without returning")
}

fn extract_id(value: &Value) -> Result<Uuid> {
	let id = value["data"]["id"]
		.as_str()
		.ok_or("missing data.id in response body")?;
	Ok(Uuid::parse_str(id)?)
}

pub fn validation_issue<'a>(validation_body: &'a Value, code: &str) -> &'a Value {
	let issues = validation_body
		.get("data")
		.and_then(|data| data.get("issues"))
		.and_then(Value::as_array)
		.unwrap_or_else(|| {
			panic!("validation response missing data.issues: {validation_body}")
		});
	let mut matches = issues
		.iter()
		.filter(|issue| issue.get("code").and_then(Value::as_str) == Some(code));
	let issue = matches.next().unwrap_or_else(|| {
		panic!("expected validation issue {code}, got {validation_body}")
	});
	assert!(
		matches.next().is_none(),
		"expected validation issue {code} exactly once, got duplicates: {validation_body}"
	);
	issue
}

pub fn assert_banner_issue(validation_body: &Value, code: &str) {
	let issue = validation_issue(validation_body, code);
	let canonical = find_canonical_rule(code)
		.unwrap_or_else(|| panic!("missing canonical rule {code}"));
	assert_eq!(
		issue.get("code").and_then(Value::as_str),
		Some(code),
		"unexpected code payload for {code}: {validation_body}"
	);
	assert_eq!(
		issue.get("message").and_then(Value::as_str),
		Some(canonical.message),
		"unexpected message for {code}: {validation_body}"
	);
	assert_eq!(
		issue.get("section").and_then(Value::as_str),
		Some(canonical.section),
		"unexpected section for {code}: {validation_body}"
	);
	assert_eq!(
		issue.get("blocking").and_then(Value::as_bool),
		Some(canonical.blocking),
		"unexpected blocking flag for {code}: {validation_body}"
	);
	let expected_field_path = expected_field_path_for_code(code)
		.unwrap_or_else(|| panic!("missing expected field path for {code}"));
	assert_eq!(
		issue.get("field_path").and_then(Value::as_str),
		Some(expected_field_path.as_str()),
		"unexpected field_path for {code}: {validation_body}"
	);
}

const KNOWN_NON_BANNER_FIELD_PATHS: &[&str] = &[
	"caseSummaryInformation.0.languageCode",
	"documentsHeldBySender.0.documentDescription",
	"drugs.0.activeSubstances.0.substanceName",
	"drugs.0.activeSubstances.0.substanceStrengthUnit",
	"drugs.0.activeSubstances.0.substanceStrengthValue",
	"drugs.0.activeSubstances.0.substanceTermId",
	"drugs.0.activeSubstances.0.substanceTermIdVersion",
	"drugs.0.dosageInformation.0.doseUnit",
	"drugs.0.dosageInformation.0.durationUnit",
	"drugs.0.dosageInformation.0.durationValue",
	"drugs.0.dosageInformation.0.frequencyUnit",
	"drugs.0.dosageInformation.0.parentRouteTermIdVersion",
	"drugs.0.dosageInformation.0.routeTermIdVersion",
	"drugs.0.dosageInformation.0.doseFormTermIdVersion",
	"drugs.0.drugReactionAssessments.0.methodOfAssessment",
	"drugs.0.drugReactionAssessments.0.resultOfAssessment",
	"drugs.0.drugReactionAssessments.0.sourceOfAssessment",
	"messageHeader.messageDate",
	"messageHeader.messageNumber",
	"messageHeader.messageReceiverIdentifier",
	"messageHeader.messageSenderIdentifier",
	"patientInformation.patientDeath.autopsyPerformed",
	"reactions.0.reactionLanguage",
];

const KNOWN_NON_EMITTED_BANNER_RULE_CODES: &[&str] = &[];

fn is_banner_capable_field_path(path: &str) -> bool {
	!KNOWN_NON_BANNER_FIELD_PATHS.contains(&path)
}

fn section_source(section_letter: char) -> &'static str {
	match section_letter {
		'C' => include_str!(
			"../../../../libs/lib-core/src/validation/case/sections/c.rs"
		),
		'D' => include_str!(
			"../../../../libs/lib-core/src/validation/case/sections/d.rs"
		),
		'E' => include_str!(
			"../../../../libs/lib-core/src/validation/case/sections/e.rs"
		),
		'F' => include_str!(
			"../../../../libs/lib-core/src/validation/case/sections/f.rs"
		),
		'G' => include_str!(
			"../../../../libs/lib-core/src/validation/case/sections/g.rs"
		),
		'H' => include_str!(
			"../../../../libs/lib-core/src/validation/case/sections/h.rs"
		),
		'N' => include_str!(
			"../../../../libs/lib-core/src/validation/case/sections/n.rs"
		),
		_ => panic!("unsupported section {section_letter}"),
	}
}

fn parse_field_path_entries(source: &str) -> Vec<(String, String)> {
	let match_start = source
		.find("match code {")
		.unwrap_or_else(|| panic!("field_path_for_rule match not found"));
	let after_match = &source[match_start + "match code {".len()..];
	let match_end = after_match
		.find("_ => None,")
		.unwrap_or_else(|| panic!("field_path_for_rule terminator not found"));
	let body = &after_match[..match_end];
	let mut out = Vec::new();
	let mut current_codes: Vec<String> = Vec::new();
	let mut waiting_for_path = false;

	for line in body.lines() {
		let trimmed = line.trim();
		if trimmed.is_empty() {
			continue;
		}
		if trimmed.contains("=>") {
			let head = trimmed
				.split_once("=>")
				.map(|(left, _)| left)
				.unwrap_or(trimmed);
			collect_rule_codes_from_fragment(head, &mut current_codes);
			if let Some(path) = extract_some_path(trimmed) {
				for code in current_codes.drain(..) {
					out.push((code, path.clone()));
				}
				waiting_for_path = false;
			} else {
				waiting_for_path = true;
			}
			continue;
		}
		if waiting_for_path {
			if let Some(path) = extract_some_path(trimmed) {
				for code in current_codes.drain(..) {
					out.push((code, path.clone()));
				}
				waiting_for_path = false;
			}
			continue;
		}
		collect_rule_codes_from_fragment(trimmed, &mut current_codes);
	}

	out
}

fn collect_rule_codes_from_fragment(fragment: &str, codes: &mut Vec<String>) {
	let mut cursor = 0usize;
	while let Some(open_rel) = fragment[cursor..].find('"') {
		let open = cursor + open_rel + 1;
		let close = fragment[open..]
			.find('"')
			.unwrap_or_else(|| panic!("unterminated quoted token in {fragment}"))
			+ open;
		let token = &fragment[open..close];
		if token.starts_with("ICH.")
			|| token.starts_with("FDA.")
			|| token.starts_with("MFDS.")
		{
			codes.push(token.to_string());
		}
		cursor = close + 1;
	}
}

fn extract_some_path(fragment: &str) -> Option<String> {
	if let Some(path_start_rel) = fragment.find("Some(\"") {
		let path_start = path_start_rel + "Some(\"".len();
		let path_end = fragment[path_start..].find('"')? + path_start;
		return Some(fragment[path_start..path_end].to_string());
	}
	if fragment.starts_with('"') {
		let path_end = fragment[1..].find('"')? + 1;
		let token = &fragment[1..path_end];
		if token.starts_with("ICH.")
			|| token.starts_with("FDA.")
			|| token.starts_with("MFDS.")
		{
			return None;
		}
		return Some(token.to_string());
	}
	None
}

fn expected_entries_for_section(section_letter: char) -> Vec<(String, String)> {
	parse_field_path_entries(section_source(section_letter))
		.into_iter()
		.filter(|(code, path)| {
			is_banner_capable_field_path(path)
				&& !KNOWN_NON_EMITTED_BANNER_RULE_CODES.contains(&code.as_str())
		})
		.collect()
}

pub fn expected_banner_rule_codes_for_section(section_letter: char) -> Vec<String> {
	expected_entries_for_section(section_letter)
		.into_iter()
		.map(|(code, _)| code)
		.collect()
}

pub fn expected_field_path_for_code(code: &str) -> Option<String> {
	for section in ['C', 'D', 'E', 'F', 'G', 'H', 'N'] {
		for (entry_code, path) in expected_entries_for_section(section) {
			if entry_code == code {
				return Some(path);
			}
		}
	}
	None
}

pub fn assert_section_rule_coverage(
	section_letter: char,
	tested_rule_codes: &[&str],
) {
	let expected = expected_banner_rule_codes_for_section(section_letter);
	let actual = tested_rule_codes
		.iter()
		.map(|code| (*code).to_string())
		.collect::<Vec<_>>();
	assert_eq!(
		actual, expected,
		"banner-capable rule coverage drift for section {section_letter}"
	);
	let actual_set = actual.into_iter().collect::<BTreeSet<_>>();
	let expected_set = expected.into_iter().collect::<BTreeSet<_>>();
	assert_eq!(
		actual_set, expected_set,
		"banner-capable rule membership drift for section {section_letter}"
	);
}
