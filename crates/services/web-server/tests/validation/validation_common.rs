use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use lib_auth::token::generate_web_token;
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::ModelManager;
use serde_json::{json, Value};
use sqlx::types::Uuid as SqlxUuid;
use tower::ServiceExt;
use uuid::Uuid;

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
	Ok((status, serde_json::from_slice::<Value>(&body)?))
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
	Ok((status, serde_json::from_slice::<Value>(&body)?))
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
	Ok((status, serde_json::from_slice::<Value>(&body)?))
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
	extract_id(&value)
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
		lib_core::ctx::ROLE_ADMIN,
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
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"drug_reaction_assessment_id": assessment_id,
			"sequence_number": sequence_number,
			"source_of_assessment": source_of_assessment,
			"method_of_assessment": method_of_assessment,
			"result_of_assessment": result_of_assessment
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

pub async fn db_exec_case_sql(ctx: &ValidationCtx, sql: &str) -> Result<()> {
	ctx.mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		ctx.mm.dbx(),
		SqlxUuid::parse_str(&ctx.admin_id.to_string())?,
		SqlxUuid::parse_str(&ctx.org_id.to_string())?,
		lib_core::ctx::ROLE_ADMIN,
	)
	.await?;
	ctx.mm.dbx().execute(sqlx::query(sql)).await?;
	ctx.mm.dbx().commit_txn().await?;
	Ok(())
}

fn extract_id(value: &Value) -> Result<Uuid> {
	let id = value["data"]["id"]
		.as_str()
		.ok_or("missing data.id in response body")?;
	Ok(Uuid::parse_str(id)?)
}
