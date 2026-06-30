// Narrative sub-resources REST endpoints (H.3.r, H.5.r)

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{
	CASE_SUMMARY_CREATE, CASE_SUMMARY_DELETE, CASE_SUMMARY_LIST, CASE_SUMMARY_READ,
	CASE_SUMMARY_UPDATE, NARRATIVE_READ, SENDER_DIAGNOSIS_CREATE,
	SENDER_DIAGNOSIS_DELETE, SENDER_DIAGNOSIS_LIST, SENDER_DIAGNOSIS_READ,
	SENDER_DIAGNOSIS_UPDATE,
};
use lib_core::model::narrative::{
	CaseSummaryInformation, CaseSummaryInformationBmc, CaseSummaryInformationFilter,
	CaseSummaryInformationForCreate, CaseSummaryInformationForUpdate,
	NarrativeInformationBmc, SenderDiagnosis, SenderDiagnosisBmc,
	SenderDiagnosisFilter, SenderDiagnosisForCreate, SenderDiagnosisForUpdate,
};
use lib_core::model::patient::{PatientInformation, PatientInformationBmc};
use lib_core::model::{self, ModelManager};
use lib_core::narrative_template::{render_template, template_tokens};
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_case_write_allowed, require_permission, Result};
use lib_web::middleware::mw_auth::CtxW;
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct NarrativePreviewRequest {
	pub template: String,
}

#[derive(Debug, Serialize)]
pub struct NarrativePreviewToken {
	pub code: String,
	pub resolved: bool,
}

#[derive(Debug, Serialize)]
pub struct NarrativePreviewResponse {
	pub rendered: String,
	pub tokens: Vec<NarrativePreviewToken>,
}

fn patient_sex_display(value: &str) -> Option<&'static str> {
	match value.trim() {
		"1" => Some("남성"),
		"2" => Some("여성"),
		"0" => Some("알 수 없음"),
		_ => None,
	}
}

fn resolve_patient_template_code(
	patient: Option<&PatientInformation>,
	code: &str,
) -> Option<String> {
	let patient = patient?;
	match code {
		"D.2.2a" => patient
			.age_at_time_of_onset
			.map(|value| value.normalize().to_string()),
		"D.5" => patient
			.sex
			.as_deref()
			.and_then(patient_sex_display)
			.map(str::to_string),
		_ => None,
	}
}

async fn patient_for_case_optional(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Option<PatientInformation>> {
	match PatientInformationBmc::get_by_case(ctx, mm, case_id).await {
		Ok(patient) => Ok(Some(patient)),
		Err(model::Error::EntityUuidNotFound { .. }) => Ok(None),
		Err(err) => Err(err.into()),
	}
}

async fn narrative_id_for_case(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Uuid> {
	lib_rest_core::require_case_read_allowed(ctx, mm, case_id).await?;
	let narrative = NarrativeInformationBmc::get_by_case(ctx, mm, case_id).await?;
	Ok(narrative.id)
}

async fn ensure_narrative_scope(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	entity_narrative_id: Uuid,
	entity_id: Uuid,
	entity: &'static str,
) -> Result<()> {
	let expected_narrative_id = narrative_id_for_case(ctx, mm, case_id).await?;
	if expected_narrative_id != entity_narrative_id {
		return Err(model::Error::EntityUuidNotFound {
			entity,
			id: entity_id,
		}
		.into());
	}
	Ok(())
}

/// POST /api/cases/{case_id}/narrative/preview
pub async fn preview_narrative_template(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<NarrativePreviewRequest>,
) -> Result<(StatusCode, Json<DataRestResult<NarrativePreviewResponse>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, NARRATIVE_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let patient = patient_for_case_optional(&ctx, &mm, case_id).await?;
	let tokens = template_tokens(&params.template);
	let rendered = render_template(&params.template, |code| {
		resolve_patient_template_code(patient.as_ref(), code)
	});
	let tokens = tokens
		.into_iter()
		.map(|code| {
			let resolved =
				resolve_patient_template_code(patient.as_ref(), &code).is_some();
			NarrativePreviewToken { code, resolved }
		})
		.collect();

	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: NarrativePreviewResponse { rendered, tokens },
		}),
	))
}

// -- Sender Diagnosis (H.3.r)

/// POST /api/cases/{case_id}/narrative/sender-diagnoses
pub async fn create_sender_diagnosis(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<SenderDiagnosisForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<SenderDiagnosis>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, SENDER_DIAGNOSIS_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let narrative_id = narrative_id_for_case(&ctx, &mm, case_id).await?;

	let ParamsForCreate { data } = params;
	let mut data = data;
	data.narrative_id = narrative_id;

	let id = SenderDiagnosisBmc::create(&ctx, &mm, data).await?;
	let entity = SenderDiagnosisBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/narrative/sender-diagnoses
pub async fn list_sender_diagnoses(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<SenderDiagnosis>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, SENDER_DIAGNOSIS_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;
	let Some(narrative) =
		NarrativeInformationBmc::get_by_case_optional(&ctx, &mm, case_id).await?
	else {
		return Ok((StatusCode::OK, Json(DataRestResult { data: vec![] })));
	};

	let filter = SenderDiagnosisFilter {
		narrative_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			narrative.id.to_string()
		))])),
		..Default::default()
	};
	let entities = SenderDiagnosisBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/narrative/sender-diagnoses/{id}
pub async fn get_sender_diagnosis(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<SenderDiagnosis>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, SENDER_DIAGNOSIS_READ)?;
	let entity = SenderDiagnosisBmc::get(&ctx, &mm, id).await?;
	ensure_narrative_scope(
		&ctx,
		&mm,
		case_id,
		entity.narrative_id,
		id,
		"sender_diagnoses",
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/narrative/sender-diagnoses/{id}
pub async fn update_sender_diagnosis(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<SenderDiagnosisForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<SenderDiagnosis>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, SENDER_DIAGNOSIS_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForUpdate { data } = params;
	let entity = SenderDiagnosisBmc::get(&ctx, &mm, id).await?;
	ensure_narrative_scope(
		&ctx,
		&mm,
		case_id,
		entity.narrative_id,
		id,
		"sender_diagnoses",
	)
	.await?;
	SenderDiagnosisBmc::update(&ctx, &mm, id, data).await?;
	let entity = SenderDiagnosisBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/narrative/sender-diagnoses/{id}
pub async fn delete_sender_diagnosis(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, SENDER_DIAGNOSIS_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let entity = SenderDiagnosisBmc::get(&ctx, &mm, id).await?;
	ensure_narrative_scope(
		&ctx,
		&mm,
		case_id,
		entity.narrative_id,
		id,
		"sender_diagnoses",
	)
	.await?;
	SenderDiagnosisBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}

/// POST /api/cases/{case_id}/narrative/sender-diagnoses/{id}/restore
pub async fn restore_sender_diagnosis(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<SenderDiagnosis>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, SENDER_DIAGNOSIS_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let entity = SenderDiagnosisBmc::get(&ctx, &mm, id).await?;
	ensure_narrative_scope(
		&ctx,
		&mm,
		case_id,
		entity.narrative_id,
		id,
		"sender_diagnoses",
	)
	.await?;
	SenderDiagnosisBmc::restore(&ctx, &mm, id).await?;
	let entity = SenderDiagnosisBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

// -- Case Summary Information (H.5.r)

/// POST /api/cases/{case_id}/narrative/summaries
pub async fn create_case_summary_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<CaseSummaryInformationForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<CaseSummaryInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_SUMMARY_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let narrative_id = narrative_id_for_case(&ctx, &mm, case_id).await?;

	let ParamsForCreate { data } = params;
	let mut data = data;
	data.narrative_id = narrative_id;

	let id = CaseSummaryInformationBmc::create(&ctx, &mm, data).await?;
	let entity = CaseSummaryInformationBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/narrative/summaries
pub async fn list_case_summary_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<CaseSummaryInformation>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_SUMMARY_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;
	let Some(narrative) =
		NarrativeInformationBmc::get_by_case_optional(&ctx, &mm, case_id).await?
	else {
		return Ok((StatusCode::OK, Json(DataRestResult { data: vec![] })));
	};

	let filter = CaseSummaryInformationFilter {
		narrative_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			narrative.id.to_string()
		))])),
		..Default::default()
	};
	let entities = CaseSummaryInformationBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/narrative/summaries/{id}
pub async fn get_case_summary_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<CaseSummaryInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_SUMMARY_READ)?;
	let entity = CaseSummaryInformationBmc::get(&ctx, &mm, id).await?;
	ensure_narrative_scope(
		&ctx,
		&mm,
		case_id,
		entity.narrative_id,
		id,
		"case_summary_information",
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/narrative/summaries/{id}
pub async fn update_case_summary_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<CaseSummaryInformationForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<CaseSummaryInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_SUMMARY_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForUpdate { data } = params;
	let entity = CaseSummaryInformationBmc::get(&ctx, &mm, id).await?;
	ensure_narrative_scope(
		&ctx,
		&mm,
		case_id,
		entity.narrative_id,
		id,
		"case_summary_information",
	)
	.await?;
	CaseSummaryInformationBmc::update(&ctx, &mm, id, data).await?;
	let entity = CaseSummaryInformationBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/narrative/summaries/{id}
pub async fn delete_case_summary_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_SUMMARY_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let entity = CaseSummaryInformationBmc::get(&ctx, &mm, id).await?;
	ensure_narrative_scope(
		&ctx,
		&mm,
		case_id,
		entity.narrative_id,
		id,
		"case_summary_information",
	)
	.await?;
	CaseSummaryInformationBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}

/// POST /api/cases/{case_id}/narrative/summaries/{id}/restore
pub async fn restore_case_summary_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<CaseSummaryInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_SUMMARY_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let entity = CaseSummaryInformationBmc::get(&ctx, &mm, id).await?;
	ensure_narrative_scope(
		&ctx,
		&mm,
		case_id,
		entity.narrative_id,
		id,
		"case_summary_information",
	)
	.await?;
	CaseSummaryInformationBmc::restore(&ctx, &mm, id).await?;
	let entity = CaseSummaryInformationBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}
