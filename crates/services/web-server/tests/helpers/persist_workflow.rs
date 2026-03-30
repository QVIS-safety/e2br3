use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_ADMIN;
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::ModelManager;
use serde_json::{json, Value};
use tower::ServiceExt;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

pub struct PersistTestCtx {
	pub mm: ModelManager,
	pub app: Router,
	pub cookie: String,
	pub org_id: Uuid,
	pub admin_id: Uuid,
}

pub async fn setup() -> Result<PersistTestCtx> {
	let mm = init_test_mm().await?;
	let seed = loop {
		match seed_org_with_users(&mm, "adminpwd", "viewpwd").await {
			Ok(seed) => break seed,
			Err(err)
				if err
					.to_string()
					.to_ascii_lowercase()
					.contains("deadlock detected") =>
			{
				sleep(Duration::from_millis(100)).await;
			}
			Err(err) => return Err(err),
		}
	};
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	Ok(PersistTestCtx {
		mm,
		app,
		cookie,
		org_id: seed.org_id,
		admin_id: seed.admin.id,
	})
}

pub fn disable_export_validation_for_test() {
	std::env::set_var("E2BR3_EXPORT_VALIDATE_FDA", "0");
}

fn extract_id(body: &Value) -> Result<Uuid> {
	let id = body
		.get("data")
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing data.id")?;
	Ok(Uuid::parse_str(id)?)
}

pub async fn request_json(
	app: &Router,
	cookie: &str,
	method: &str,
	uri: String,
	body: Option<Value>,
) -> Result<(StatusCode, Value)> {
	let mut builder = Request::builder()
		.method(method)
		.uri(uri)
		.header("cookie", cookie);
	if body.is_some() {
		builder = builder.header("content-type", "application/json");
	}
	let req =
		builder.body(Body::from(body.map(|v| v.to_string()).unwrap_or_default()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let bytes = to_bytes(res.into_body(), usize::MAX).await?;
	let value = serde_json::from_slice::<Value>(&bytes).unwrap_or_else(|_| {
		json!({
			"raw": String::from_utf8_lossy(&bytes).to_string()
		})
	});
	Ok((status, value))
}

pub async fn request_xml(
	app: &Router,
	cookie: &str,
	uri: String,
) -> Result<(StatusCode, String)> {
	let req = Request::builder()
		.method("GET")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let bytes = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, String::from_utf8_lossy(&bytes).to_string()))
}

pub async fn create_case(ctx: &PersistTestCtx) -> Result<Uuid> {
	let body = json!({
		"data": {
			"organization_id": ctx.org_id,
			"safety_report_id": format!("PERSIST-{}", Uuid::new_v4()),
			"status": "draft",
			"validation_profile": "fda"
		}
	});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		"/api/cases".to_string(),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create case failed: status={status} body={value}").into()
		);
	}
	extract_id(&value)
}

pub async fn save_case(ctx: &PersistTestCtx, case_id: Uuid) -> Result<()> {
	let body = json!({"data": {"status": "draft"}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"PUT",
		format!("/api/cases/{case_id}"),
		Some(body),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!("save case failed: status={status} body={value}").into());
	}
	Ok(())
}

#[allow(dead_code)]
pub async fn set_case_status(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	status_value: &str,
) -> Result<()> {
	let body = json!({"data": {"status": status_value}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"PUT",
		format!("/api/cases/{case_id}"),
		Some(body),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"set case status to {status_value} failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

pub async fn force_case_validated_for_export(
	ctx: &PersistTestCtx,
	case_id: Uuid,
) -> Result<()> {
	ctx.mm.dbx().begin_txn().await?;
	set_full_context_dbx(ctx.mm.dbx(), ctx.admin_id, ctx.org_id, ROLE_ADMIN).await?;
	ctx.mm
		.dbx()
		.execute(
			sqlx::query(
				"UPDATE cases
				 SET status = 'validated', updated_at = now(), updated_by = $2
				 WHERE id = $1",
			)
			.bind(case_id)
			.bind(ctx.admin_id),
		)
		.await?;
	ctx.mm.dbx().commit_txn().await?;
	Ok(())
}

pub async fn validate_case_fda(ctx: &PersistTestCtx, case_id: Uuid) -> Result<()> {
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"GET",
		format!("/api/cases/{case_id}/validation?profile=fda"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!("validate failed: status={status} body={value}").into());
	}
	Ok(())
}

pub async fn export_case_xml(ctx: &PersistTestCtx, case_id: Uuid) -> Result<String> {
	let (status, xml) = request_xml(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{case_id}/export/xml"),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!("export failed: status={status} body={xml}").into());
	}
	Ok(xml)
}

pub async fn fill_section_c(ctx: &PersistTestCtx, case_id: Uuid) -> Result<()> {
	let body = json!({"data": {
		"case_id": case_id,
		"message_number": format!("MSG-C-{case_id}"),
		"message_sender_identifier": "SENDER-C",
		"message_receiver_identifier": "RECEIVER-C",
		"message_date": "20240201010101"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/message-header"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(
			format!("message-header failed: status={status} body={value}").into(),
		);
	}

	let body = json!({"data": {
		"case_id": case_id,
		"receiver_type": "1",
		"organization_name": "Receiver Persist Org"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/receiver"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(format!("receiver failed: status={status} body={value}").into());
	}

	let body = json!({"data": {
		"case_id": case_id,
		"transmission_date": [2024, 1],
		"report_type": "1",
		"date_first_received_from_source": [2024, 1],
		"date_of_most_recent_information": [2024, 1],
		"fulfil_expedited_criteria": false,
		"first_sender_type": "2",
		"additional_documents_available": true
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/safety-report"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(
			format!("safety-report failed: status={status} body={value}").into(),
		);
	}

	let body = json!({"data": {
		"case_id": case_id,
		"sender_type": "1",
		"organization_name": "Sender Persist Org"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/safety-report/senders"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"safety-report sender failed: status={status} body={value}"
		)
		.into());
	}

	let body = json!({"data": {
		"case_id": case_id,
		"sequence_number": 1,
		"qualification": "1"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/safety-report/primary-sources"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("primary-source failed: status={status} body={value}").into(),
		);
	}

	let body = json!({"data": {
		"case_id": case_id,
		"sequence_number": 1,
		"reference_text": "Persist literature"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/safety-report/literature"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("literature failed: status={status} body={value}").into()
		);
	}

	let body = json!({"data": {
		"case_id": case_id,
		"study_name": "Persist Study",
		"sponsor_study_number": "P-STUDY-1"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/safety-report/studies"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!("study failed: status={status} body={value}").into());
	}
	let study_id = extract_id(&value)?;

	let body = json!({"data": {
		"study_information_id": study_id,
		"registration_number": "PERSIST-REG-1",
		"sequence_number": 1
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!(
			"/api/cases/{case_id}/safety-report/studies/{study_id}/registrations"
		),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"study registration failed: status={status} body={value}"
		)
		.into());
	}

	Ok(())
}

pub async fn fill_section_d(ctx: &PersistTestCtx, case_id: Uuid) -> Result<()> {
	let body = json!({"data": {
		"case_id": case_id,
		"patient_initials": "PD",
		"sex": "1"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/patient"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(format!("patient failed: status={status} body={value}").into());
	}
	let patient_id = extract_id(&value)?;

	let body = json!({"data": {"patient_id": patient_id, "sequence_number": 1, "meddra_code": "100"}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/patient/medical-history"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("medical-history failed: status={status} body={value}").into(),
		);
	}

	let body = json!({"data": {"patient_id": patient_id, "sequence_number": 1, "drug_name": "Past Drug Persist"}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/patient/past-drugs"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("past-drugs failed: status={status} body={value}").into()
		);
	}

	let body =
		json!({"data": {"patient_id": patient_id, "date_of_death": [2024, 1]}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/patient/death-info"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("death-info failed: status={status} body={value}").into()
		);
	}
	let death_info_id = extract_id(&value)?;

	let body = json!({"data": {"death_info_id": death_info_id, "sequence_number": 1, "meddra_code": "100"}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("reported-causes failed: status={status} body={value}").into(),
		);
	}

	let body = json!({"data": {"death_info_id": death_info_id, "sequence_number": 1, "meddra_code": "100"}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!(
			"/api/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes"
		),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("autopsy-causes failed: status={status} body={value}").into(),
		);
	}

	let body = json!({"data": {"patient_id": patient_id, "sex": "2", "medical_history_text": "none"}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/patient/parents"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!("parents failed: status={status} body={value}").into());
	}

	Ok(())
}

pub async fn fill_section_e(ctx: &PersistTestCtx, case_id: Uuid) -> Result<Uuid> {
	let body = json!({"data": {
		"case_id": case_id,
		"sequence_number": 1,
		"primary_source_reaction": "Persist Reaction Headache"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/reactions"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!("reaction failed: status={status} body={value}").into());
	}
	let reaction_id = extract_id(&value)?;

	let body = json!({"data": {
		"outcome": "3",
		"reaction_meddra_code": "10019211"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"PUT",
		format!("/api/cases/{case_id}/reactions/{reaction_id}"),
		Some(body),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(
			format!("reaction update failed: status={status} body={value}").into(),
		);
	}

	Ok(reaction_id)
}

pub async fn fill_section_f(ctx: &PersistTestCtx, case_id: Uuid) -> Result<()> {
	let body = json!({"data": {
		"case_id": case_id,
		"sequence_number": 1,
		"test_name": "Persist Blood Test"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/test-results"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("test-result failed: status={status} body={value}").into()
		);
	}
	Ok(())
}

pub async fn fill_section_g(ctx: &PersistTestCtx, case_id: Uuid) -> Result<()> {
	let body = json!({"data": {
		"case_id": case_id,
		"sequence_number": 1,
		"drug_characterization": "1",
		"medicinal_product": "Persist Drug"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/drugs"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!("drug failed: status={status} body={value}").into());
	}
	let drug_id = extract_id(&value)?;

	let additional_info_codes =
		json!([{"value_code": "ADD-CODE-1"}, {"value_code": "ADD-CODE-2"}]);
	let fda_device_info = json!({
		"malfunction": true,
		"follow_up_types": [{"value_code": "FU-1"}],
		"device_problem_codes": [{"value_code": "DP-1"}],
		"device_brand_name": "Persist Device Brand",
		"common_device_name": "Persist Common Device",
		"device_product_code": "PDCODE",
		"manufacturer_name": "Persist Device Mfr",
		"manufacturer_address": "1 Device Way",
		"manufacturer_city": "Seoul",
		"manufacturer_state": "Seoul",
		"manufacturer_country": "KR",
		"device_usage": "1",
		"device_lot_number": "DL-001",
		"operator_of_device": "2",
		"remedial_actions": [{"value_code": "RA-1"}]
	});
	let body = json!({
		"data": {
			"medicinal_product": "Persist Drug",
			"drug_characterization": "1",
			"brand_name": "Persist Brand",
			"drug_generic_name": "Persist Generic",
			"drug_authorization_number": "AUTH-123",
			"manufacturer_name": "Persist Manufacturer",
			"manufacturer_country": "US",
			"batch_lot_number": "LOT-123",
			"cumulative_dose_first_reaction_value": "12.5",
			"cumulative_dose_first_reaction_unit": "mg",
			"gestation_period_exposure_value": "2",
			"gestation_period_exposure_unit": "week",
			"dosage_text": "Persist legacy dosage text",
			"action_taken": "1",
			"rechallenge": "1",
			"investigational_product_blinded": false,
			"mpid": "Persist-MPID",
			"mpid_version": "2026.03",
			"phpid": "Persist-PhPID",
			"phpid_version": "2026.04",
			"obtain_drug_country": "KR",
			"parent_route": "Oral parent route",
			"parent_route_termid": "PARENT-ROUTE-ID",
			"parent_route_termid_version": "1.0",
			"parent_dosage_text": "Persist parent dosage",
			"fda_additional_info_coded": "FDA-ADD-1",
			"drug_additional_info_codes_json": additional_info_codes,
			"fda_specialized_product_category": "COMBINATION",
			"fda_device_info_json": fda_device_info
		}
	});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"PUT",
		format!("/api/cases/{case_id}/drugs/{drug_id}"),
		Some(body),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(
			format!("drug update failed: status={status} body={value}").into()
		);
	}

	let body = json!({"data": {
		"drug_id": drug_id,
		"sequence_number": 1,
		"substance_name": "Persist Substance",
		"substance_termid": "SUB-TERM-1",
		"substance_termid_version": "26.1",
		"strength_value": "10.0",
		"strength_unit": "mg"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"active-substance failed: status={status} body={value}"
		)
		.into());
	}
	let active_substance_id = extract_id(&value)?;

	let body = json!({"data": {
		"substance_name": "Persist Substance Updated",
		"substance_termid": "SUB-TERM-2",
		"substance_termid_version": "27.0",
		"strength_value": "12.5",
		"strength_unit": "mL"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"PUT",
		format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances/{active_substance_id}"),
		Some(body),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"active-substance update failed: status={status} body={value}"
		)
		.into());
	}

	let body = json!({"data": {
		"drug_id": drug_id,
		"sequence_number": 1,
		"dose_value": "1.5",
		"dose_unit": "tablet",
		"number_of_units": 2,
		"frequency_value": "3",
		"frequency_unit": "day",
		"first_administration_date": "2026-03-01",
		"last_administration_date": "2026-03-05",
		"duration_value": "4",
		"duration_unit": "804",
		"batch_lot_number": "DOSAGE-LOT-1",
		"dosage_text": "Persist dosage detail",
		"dose_form": "Tablet",
		"dose_form_termid": "DF-1",
		"dose_form_termid_version": "2026.1",
		"route_of_administration": "048",
		"route_termid_version": "2026.2",
		"parent_route": "Parent oral",
		"parent_route_termid": "PR-1",
		"parent_route_termid_version": "2026.3"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/drugs/{drug_id}/dosages"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!("dosage failed: status={status} body={value}").into());
	}
	let dosage_id = extract_id(&value)?;

	let body = json!({"data": {
		"dosage_text": "Persist dosage detail updated",
		"dose_form_termid": "DF-2",
		"dose_form_termid_version": "2027.1",
		"route_of_administration": "061",
		"route_termid_version": "2027.2",
		"parent_route_termid": "PR-2",
		"parent_route_termid_version": "2027.3"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"PUT",
		format!("/api/cases/{case_id}/drugs/{drug_id}/dosages/{dosage_id}"),
		Some(body),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(
			format!("dosage update failed: status={status} body={value}").into(),
		);
	}

	let body = json!({"data": {
		"drug_id": drug_id,
		"sequence_number": 1,
		"indication_text": "Persist indication",
		"indication_meddra_version": "26.0",
		"indication_meddra_code": "135790"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/drugs/{drug_id}/indications"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("indication failed: status={status} body={value}").into()
		);
	}
	let indication_id = extract_id(&value)?;

	let body = json!({"data": {
		"indication_text": "Persist indication updated",
		"indication_meddra_version": "27.0",
		"indication_meddra_code": "246810"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"PUT",
		format!("/api/cases/{case_id}/drugs/{drug_id}/indications/{indication_id}"),
		Some(body),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"indication update failed: status={status} body={value}"
		)
		.into());
	}

	let body = json!({"data": {
		"drug_id": drug_id,
		"sequence_number": 1,
		"code": "FDA.G.k.12.r.3",
		"value_code": "1"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/drugs/{drug_id}/device-characteristics"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"device-characteristic failed: status={status} body={value}"
		)
		.into());
	}

	let body = json!({"data": {"drug_id": drug_id, "sequence_number": 1}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/drugs/{drug_id}/recurrences"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("recurrence failed: status={status} body={value}").into()
		);
	}
	let recurrence_id = extract_id(&value)?;

	let body = json!({"data": {
		"rechallenge_action": "1",
		"reaction_meddra_version": "26.0",
		"reaction_meddra_code": "10012345",
		"reaction_recurred": "2"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"PUT",
		format!("/api/cases/{case_id}/drugs/{drug_id}/recurrences/{recurrence_id}"),
		Some(body),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"recurrence update failed: status={status} body={value}"
		)
		.into());
	}

	let reaction_id = fill_section_e(ctx, case_id).await?;

	let body = json!({"data": {"drug_id": drug_id, "reaction_id": reaction_id}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"reaction-assessment failed: status={status} body={value}"
		)
		.into());
	}
	let assessment_id = extract_id(&value)?;

	let body = json!({"data": {
		"administration_start_interval_value": "5",
		"administration_start_interval_unit": "804",
		"last_dose_interval_value": "2",
		"last_dose_interval_unit": "804",
		"recurrence_action": "1",
		"recurrence_meddra_version": "27.0",
		"recurrence_meddra_code": "10054321",
		"reaction_recurred": "1"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"PUT",
		format!("/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}"),
		Some(body),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"reaction-assessment update failed: status={status} body={value}"
		)
		.into());
	}

	let body = json!({"data": {
		"drug_reaction_assessment_id": assessment_id,
		"sequence_number": 1,
		"source_of_assessment": "1",
		"method_of_assessment": "1",
		"result_of_assessment": "1",
		"result_of_assessment_kr2": "Persist KR2"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("relatedness failed: status={status} body={value}").into()
		);
	}

	Ok(())
}

pub async fn db_fetch_json_by_case(
	ctx: &PersistTestCtx,
	sql: &str,
	case_id: Uuid,
) -> Result<Value> {
	ctx.mm.dbx().begin_txn().await?;
	set_full_context_dbx(ctx.mm.dbx(), ctx.admin_id, ctx.org_id, ROLE_ADMIN).await?;
	let (value,) = ctx
		.mm
		.dbx()
		.fetch_one(sqlx::query_as::<_, (Value,)>(sql).bind(case_id))
		.await?;
	ctx.mm.dbx().commit_txn().await?;
	Ok(value)
}

pub async fn fill_section_h(ctx: &PersistTestCtx, case_id: Uuid) -> Result<()> {
	let body = json!({"data": {
		"case_id": case_id,
		"case_narrative": "Persist narrative text"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/narrative"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(format!("narrative failed: status={status} body={value}").into());
	}
	let narrative_id = extract_id(&value)?;

	let body = json!({"data": {
		"narrative_id": narrative_id,
		"sequence_number": 1,
		"diagnosis_meddra_code": "100"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/narrative/sender-diagnoses"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"sender-diagnosis failed: status={status} body={value}"
		)
		.into());
	}

	let body = json!({"data": {
		"narrative_id": narrative_id,
		"sequence_number": 1,
		"summary_text": "Persist summary"
	}});
	let (status, value) = request_json(
		&ctx.app,
		&ctx.cookie,
		"POST",
		format!("/api/cases/{case_id}/narrative/summaries"),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("case-summary failed: status={status} body={value}").into()
		);
	}

	Ok(())
}

pub async fn db_count_by_case(
	ctx: &PersistTestCtx,
	table: &str,
	case_id: Uuid,
) -> Result<i64> {
	ctx.mm.dbx().begin_txn().await?;
	set_full_context_dbx(ctx.mm.dbx(), ctx.admin_id, ctx.org_id, ROLE_ADMIN).await?;
	let sql = format!("SELECT COUNT(*)::bigint FROM {table} WHERE case_id = $1");
	let (count,) = ctx
		.mm
		.dbx()
		.fetch_one(sqlx::query_as::<_, (i64,)>(&sql).bind(case_id))
		.await?;
	ctx.mm.dbx().commit_txn().await?;
	Ok(count)
}
