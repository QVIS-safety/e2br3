// Drug-Reaction Assessment REST endpoints (G.k.9.i)

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::drug::DrugInformationBmc;
use lib_core::model::drug_reaction_assessment::{
	DrugReactionAssessment, DrugReactionAssessmentBmc,
	DrugReactionAssessmentForCreate, DrugReactionAssessmentForUpdate,
};
use lib_core::model::reaction::ReactionBmc;
use lib_core::model::{self, ModelManager};
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::Result;
use lib_web::middleware::mw_auth::CtxW;
use uuid::Uuid;

async fn ensure_drug_in_case(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	drug_id: Uuid,
) -> Result<()> {
	DrugInformationBmc::get_in_case(ctx, mm, case_id, drug_id).await?;
	Ok(())
}

fn ensure_assessment_in_drug(
	drug_id: Uuid,
	entity: &DrugReactionAssessment,
) -> Result<()> {
	if entity.drug_id != drug_id {
		return Err(model::Error::EntityUuidNotFound {
			entity: "drug_reaction_assessments",
			id: entity.id,
		}
		.into());
	}
	Ok(())
}

pub async fn create_drug_reaction_assessment(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, drug_id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForCreate<DrugReactionAssessmentForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<DrugReactionAssessment>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("drug-reaction-assessment:new:drug:{drug_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				ensure_drug_in_case(ctx, mm, case_id, drug_id).await?;
				let ParamsForCreate { data } = params;
				let mut data = data;
				ReactionBmc::get_in_case(ctx, mm, case_id, data.reaction_id).await?;
				data.drug_id = drug_id;
				let id = DrugReactionAssessmentBmc::create(ctx, mm, data).await?;
				let entity = DrugReactionAssessmentBmc::get(ctx, mm, id).await?;
				Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

pub async fn list_drug_reaction_assessments(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, drug_id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<DrugReactionAssessment>>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("drug-reaction-assessment:list:drug:{drug_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				ensure_drug_in_case(ctx, mm, case_id, drug_id).await?;
				let entities =
					DrugReactionAssessmentBmc::list_by_drug(ctx, mm, drug_id)
						.await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
			})
		},
	)
	.await
}

pub async fn get_drug_reaction_assessment(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, drug_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<DrugReactionAssessment>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("drug-reaction-assessment:{id}:drug:{drug_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				ensure_drug_in_case(ctx, mm, case_id, drug_id).await?;
				let entity = DrugReactionAssessmentBmc::get(ctx, mm, id).await?;
				ensure_assessment_in_drug(drug_id, &entity)?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

pub async fn update_drug_reaction_assessment(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, drug_id, id)): Path<(Uuid, Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<DrugReactionAssessmentForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<DrugReactionAssessment>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("drug-reaction-assessment:{id}:drug:{drug_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				ensure_drug_in_case(ctx, mm, case_id, drug_id).await?;
				let entity = DrugReactionAssessmentBmc::get(ctx, mm, id).await?;
				ensure_assessment_in_drug(drug_id, &entity)?;
				let ParamsForUpdate { data } = params;
				if let Some(reaction_id) = data.reaction_id {
					ReactionBmc::get_in_case(ctx, mm, case_id, reaction_id).await?;
				}
				DrugReactionAssessmentBmc::update(ctx, mm, id, data).await?;
				let entity = DrugReactionAssessmentBmc::get(ctx, mm, id).await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

pub async fn delete_drug_reaction_assessment(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, drug_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("drug-reaction-assessment:{id}:drug:{drug_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				ensure_drug_in_case(ctx, mm, case_id, drug_id).await?;
				let entity = DrugReactionAssessmentBmc::get(ctx, mm, id).await?;
				ensure_assessment_in_drug(drug_id, &entity)?;
				DrugReactionAssessmentBmc::delete(ctx, mm, id).await?;
				Ok(StatusCode::NO_CONTENT)
			})
		},
	)
	.await
}
