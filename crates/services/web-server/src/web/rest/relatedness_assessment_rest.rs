// Relatedness Assessment REST endpoints (G.k.9.i.2.r)

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{
	RELATEDNESS_ASSESSMENT_CREATE, RELATEDNESS_ASSESSMENT_DELETE,
	RELATEDNESS_ASSESSMENT_LIST, RELATEDNESS_ASSESSMENT_READ,
	RELATEDNESS_ASSESSMENT_UPDATE,
};
use lib_core::model::drug_reaction_assessment::{
	RelatednessAssessment, RelatednessAssessmentBmc, RelatednessAssessmentFilter,
	RelatednessAssessmentForCreate, RelatednessAssessmentForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_case_write_allowed, require_permission, Result};
use lib_web::middleware::mw_auth::CtxW;
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde_json::json;
use uuid::Uuid;

/// POST /api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness
pub async fn create_relatedness_assessment(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, _drug_id, assessment_id)): Path<(Uuid, Uuid, Uuid)>,
	Json(params): Json<ParamsForCreate<RelatednessAssessmentForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<RelatednessAssessment>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, RELATEDNESS_ASSESSMENT_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForCreate { data } = params;
	let mut data = data;
	data.drug_reaction_assessment_id = assessment_id;

	let id = RelatednessAssessmentBmc::create(&ctx, &mm, data).await?;
	let entity = RelatednessAssessmentBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness
pub async fn list_relatedness_assessments(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, _drug_id, assessment_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<RelatednessAssessment>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, RELATEDNESS_ASSESSMENT_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;
	let filter = RelatednessAssessmentFilter {
		drug_reaction_assessment_id: Some(OpValsValue::from(vec![OpValValue::Eq(
			json!(assessment_id.to_string()),
		)])),
		..Default::default()
	};
	let entities = RelatednessAssessmentBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness/{id}
pub async fn get_relatedness_assessment(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, _drug_id, _assessment_id, id)): Path<(Uuid, Uuid, Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<RelatednessAssessment>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, RELATEDNESS_ASSESSMENT_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;
	let entity = RelatednessAssessmentBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness/{id}
pub async fn update_relatedness_assessment(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, _drug_id, _assessment_id, id)): Path<(Uuid, Uuid, Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<RelatednessAssessmentForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<RelatednessAssessment>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, RELATEDNESS_ASSESSMENT_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForUpdate { data } = params;
	RelatednessAssessmentBmc::update(&ctx, &mm, id, data).await?;
	let entity = RelatednessAssessmentBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness/{id}
pub async fn delete_relatedness_assessment(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, _drug_id, _assessment_id, id)): Path<(Uuid, Uuid, Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, RELATEDNESS_ASSESSMENT_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	RelatednessAssessmentBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}
