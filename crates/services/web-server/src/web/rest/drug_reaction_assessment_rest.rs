// Drug-Reaction Assessment REST endpoints (G.k.9.i)

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{
	DRUG_REACTION_ASSESSMENT_CREATE, DRUG_REACTION_ASSESSMENT_DELETE,
	DRUG_REACTION_ASSESSMENT_LIST, DRUG_REACTION_ASSESSMENT_READ,
	DRUG_REACTION_ASSESSMENT_UPDATE,
};
use lib_core::model::drug_reaction_assessment::{
	DrugReactionAssessment, DrugReactionAssessmentBmc,
	DrugReactionAssessmentForCreate, DrugReactionAssessmentForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_case_write_allowed, require_permission, Result};
use lib_web::middleware::mw_auth::CtxW;
use uuid::Uuid;

/// POST /api/cases/{case_id}/drugs/{drug_id}/reaction-assessments
/// Create a drug-reaction assessment linking a drug to a reaction
pub async fn create_drug_reaction_assessment(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, drug_id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForCreate<DrugReactionAssessmentForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<DrugReactionAssessment>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DRUG_REACTION_ASSESSMENT_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	tracing::debug!(
		"{:<12} - rest create_drug_reaction_assessment drug_id={}",
		"HANDLER",
		drug_id
	);

	let ParamsForCreate { data } = params;
	let mut data = data;
	data.drug_id = drug_id;

	let id = DrugReactionAssessmentBmc::create(&ctx, &mm, data).await?;
	let entity = DrugReactionAssessmentBmc::get(&ctx, &mm, id).await?;

	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/drugs/{drug_id}/reaction-assessments
/// List all reaction assessments for a drug
pub async fn list_drug_reaction_assessments(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((_case_id, drug_id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<DrugReactionAssessment>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DRUG_REACTION_ASSESSMENT_LIST)?;
	tracing::debug!(
		"{:<12} - rest list_drug_reaction_assessments drug_id={}",
		"HANDLER",
		drug_id
	);

	let entities =
		DrugReactionAssessmentBmc::list_by_drug(&ctx, &mm, drug_id).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{id}
/// Get a specific drug-reaction assessment
pub async fn get_drug_reaction_assessment(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((_case_id, _drug_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<DrugReactionAssessment>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DRUG_REACTION_ASSESSMENT_READ)?;
	tracing::debug!(
		"{:<12} - rest get_drug_reaction_assessment id={}",
		"HANDLER",
		id
	);

	let entity = DrugReactionAssessmentBmc::get(&ctx, &mm, id).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{id}
/// Update a drug-reaction assessment
pub async fn update_drug_reaction_assessment(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, _drug_id, id)): Path<(Uuid, Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<DrugReactionAssessmentForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<DrugReactionAssessment>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DRUG_REACTION_ASSESSMENT_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	tracing::debug!(
		"{:<12} - rest update_drug_reaction_assessment id={}",
		"HANDLER",
		id
	);

	let ParamsForUpdate { data } = params;
	DrugReactionAssessmentBmc::update(&ctx, &mm, id, data).await?;
	let entity = DrugReactionAssessmentBmc::get(&ctx, &mm, id).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{id}
/// Delete a drug-reaction assessment
pub async fn delete_drug_reaction_assessment(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, _drug_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DRUG_REACTION_ASSESSMENT_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	tracing::debug!(
		"{:<12} - rest delete_drug_reaction_assessment id={}",
		"HANDLER",
		id
	);

	DrugReactionAssessmentBmc::delete(&ctx, &mm, id).await?;

	Ok(StatusCode::NO_CONTENT)
}
