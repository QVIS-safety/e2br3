// Drug Recurrence Information REST endpoints (G.k.8.r)

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{
	DRUG_RECURRENCE_CREATE, DRUG_RECURRENCE_DELETE, DRUG_RECURRENCE_LIST,
	DRUG_RECURRENCE_READ, DRUG_RECURRENCE_UPDATE,
};
use lib_core::model::drug_recurrence::{
	DrugRecurrenceInformation, DrugRecurrenceInformationBmc,
	DrugRecurrenceInformationFilter, DrugRecurrenceInformationForCreate,
	DrugRecurrenceInformationForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_case_write_allowed, require_permission, Result};
use lib_web::middleware::mw_auth::CtxW;
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde_json::json;
use uuid::Uuid;

/// POST /api/cases/{case_id}/drugs/{drug_id}/recurrences
/// Create recurrence information for a drug
pub async fn create_drug_recurrence(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, drug_id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForCreate<DrugRecurrenceInformationForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<DrugRecurrenceInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DRUG_RECURRENCE_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	tracing::debug!(
		"{:<12} - rest create_drug_recurrence drug_id={}",
		"HANDLER",
		drug_id
	);

	let ParamsForCreate { data } = params;
	let mut data = data;
	data.drug_id = drug_id;

	let id = DrugRecurrenceInformationBmc::create(&ctx, &mm, data).await?;
	let entity = DrugRecurrenceInformationBmc::get(&ctx, &mm, id).await?;

	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/drugs/{drug_id}/recurrences
/// List all recurrence information for a drug
pub async fn list_drug_recurrences(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((_case_id, drug_id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<DrugRecurrenceInformation>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DRUG_RECURRENCE_LIST)?;
	tracing::debug!(
		"{:<12} - rest list_drug_recurrences drug_id={}",
		"HANDLER",
		drug_id
	);

	// Filter by drug_id
	let filter = DrugRecurrenceInformationFilter {
		drug_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			drug_id.to_string()
		))])),
		..Default::default()
	};
	let entities = DrugRecurrenceInformationBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/drugs/{drug_id}/recurrences/{id}
/// Get specific recurrence information
pub async fn get_drug_recurrence(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((_case_id, _drug_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<DrugRecurrenceInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DRUG_RECURRENCE_READ)?;
	tracing::debug!("{:<12} - rest get_drug_recurrence id={}", "HANDLER", id);

	let entity = DrugRecurrenceInformationBmc::get(&ctx, &mm, id).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/drugs/{drug_id}/recurrences/{id}
/// Update recurrence information
pub async fn update_drug_recurrence(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, _drug_id, id)): Path<(Uuid, Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<DrugRecurrenceInformationForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<DrugRecurrenceInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DRUG_RECURRENCE_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	tracing::debug!("{:<12} - rest update_drug_recurrence id={}", "HANDLER", id);

	let ParamsForUpdate { data } = params;
	DrugRecurrenceInformationBmc::update(&ctx, &mm, id, data).await?;
	let entity = DrugRecurrenceInformationBmc::get(&ctx, &mm, id).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/drugs/{drug_id}/recurrences/{id}
/// Delete recurrence information
pub async fn delete_drug_recurrence(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, _drug_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DRUG_RECURRENCE_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	tracing::debug!("{:<12} - rest delete_drug_recurrence id={}", "HANDLER", id);

	DrugRecurrenceInformationBmc::delete(&ctx, &mm, id).await?;

	Ok(StatusCode::NO_CONTENT)
}
