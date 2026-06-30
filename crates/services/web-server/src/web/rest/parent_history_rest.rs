// Parent History REST endpoints (D.10.7 and D.10.8)

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{
	PARENT_MEDICAL_HISTORY_CREATE, PARENT_MEDICAL_HISTORY_DELETE,
	PARENT_MEDICAL_HISTORY_LIST, PARENT_MEDICAL_HISTORY_READ,
	PARENT_MEDICAL_HISTORY_UPDATE, PARENT_PAST_DRUG_CREATE, PARENT_PAST_DRUG_DELETE,
	PARENT_PAST_DRUG_LIST, PARENT_PAST_DRUG_READ, PARENT_PAST_DRUG_UPDATE,
};
use lib_core::model::parent_history::{
	ParentMedicalHistory, ParentMedicalHistoryBmc, ParentMedicalHistoryFilter,
	ParentMedicalHistoryForCreate, ParentMedicalHistoryForUpdate,
	ParentPastDrugHistory, ParentPastDrugHistoryBmc, ParentPastDrugHistoryFilter,
	ParentPastDrugHistoryForCreate, ParentPastDrugHistoryForUpdate,
};
use lib_core::model::patient::{ParentInformationBmc, PatientInformationBmc};
use lib_core::model::{self, ModelManager};
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_case_write_allowed, require_permission, Result};
use lib_web::middleware::mw_auth::CtxW;
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde_json::json;
use uuid::Uuid;

async fn ensure_parent_case(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	parent_id: Uuid,
) -> Result<()> {
	let parent = ParentInformationBmc::get(ctx, mm, parent_id).await?;
	let patient = PatientInformationBmc::get(ctx, mm, parent.patient_id).await?;
	if patient.case_id != case_id {
		return Err(model::Error::EntityUuidNotFound {
			entity: "parent_information",
			id: parent_id,
		}
		.into());
	}
	Ok(())
}

fn ensure_parent_scope(
	path_parent_id: Uuid,
	entity_parent_id: Uuid,
	entity_id: Uuid,
	entity: &'static str,
) -> Result<()> {
	if path_parent_id != entity_parent_id {
		return Err(model::Error::EntityUuidNotFound {
			entity,
			id: entity_id,
		}
		.into());
	}
	Ok(())
}

// -- Parent Medical History (D.10.7.1.r)

/// POST /api/cases/{case_id}/patient/parent/{parent_id}/medical-history
pub async fn create_parent_medical_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, parent_id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForCreate<ParentMedicalHistoryForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<ParentMedicalHistory>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PARENT_MEDICAL_HISTORY_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	ensure_parent_case(&ctx, &mm, case_id, parent_id).await?;
	tracing::debug!(
		"{:<12} - rest create_parent_medical_history parent_id={}",
		"HANDLER",
		parent_id
	);

	let ParamsForCreate { data } = params;
	let mut data = data;
	data.parent_id = parent_id;

	let id = ParentMedicalHistoryBmc::create(&ctx, &mm, data).await?;
	let entity = ParentMedicalHistoryBmc::get(&ctx, &mm, id).await?;

	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/patient/parent/{parent_id}/medical-history
pub async fn list_parent_medical_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, parent_id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<ParentMedicalHistory>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PARENT_MEDICAL_HISTORY_LIST)?;
	ensure_parent_case(&ctx, &mm, case_id, parent_id).await?;
	tracing::debug!(
		"{:<12} - rest list_parent_medical_history parent_id={}",
		"HANDLER",
		parent_id
	);

	let filter = ParentMedicalHistoryFilter {
		parent_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			parent_id.to_string()
		))])),
		..Default::default()
	};
	let entities = ParentMedicalHistoryBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/patient/parent/{parent_id}/medical-history/{id}
pub async fn get_parent_medical_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, parent_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<ParentMedicalHistory>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PARENT_MEDICAL_HISTORY_READ)?;
	tracing::debug!(
		"{:<12} - rest get_parent_medical_history id={}",
		"HANDLER",
		id
	);

	let entity = ParentMedicalHistoryBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(parent_id, entity.parent_id, id, "parent_medical_history")?;
	ensure_parent_case(&ctx, &mm, case_id, parent_id).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/patient/parent/{parent_id}/medical-history/{id}
pub async fn update_parent_medical_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, parent_id, id)): Path<(Uuid, Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<ParentMedicalHistoryForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<ParentMedicalHistory>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PARENT_MEDICAL_HISTORY_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	tracing::debug!(
		"{:<12} - rest update_parent_medical_history id={}",
		"HANDLER",
		id
	);

	let ParamsForUpdate { data } = params;
	let entity = ParentMedicalHistoryBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(parent_id, entity.parent_id, id, "parent_medical_history")?;
	ensure_parent_case(&ctx, &mm, case_id, parent_id).await?;
	ParentMedicalHistoryBmc::update(&ctx, &mm, id, data).await?;
	let entity = ParentMedicalHistoryBmc::get(&ctx, &mm, id).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/patient/parent/{parent_id}/medical-history/{id}
pub async fn delete_parent_medical_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, parent_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PARENT_MEDICAL_HISTORY_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	tracing::debug!(
		"{:<12} - rest delete_parent_medical_history id={}",
		"HANDLER",
		id
	);

	let entity = ParentMedicalHistoryBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(parent_id, entity.parent_id, id, "parent_medical_history")?;
	ensure_parent_case(&ctx, &mm, case_id, parent_id).await?;
	ParentMedicalHistoryBmc::delete(&ctx, &mm, id).await?;

	Ok(StatusCode::NO_CONTENT)
}

/// POST /api/cases/{case_id}/patient/parent/{parent_id}/medical-history/{id}/restore
pub async fn restore_parent_medical_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, parent_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<ParentMedicalHistory>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PARENT_MEDICAL_HISTORY_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let entity = ParentMedicalHistoryBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(parent_id, entity.parent_id, id, "parent_medical_history")?;
	ensure_parent_case(&ctx, &mm, case_id, parent_id).await?;
	ParentMedicalHistoryBmc::restore(&ctx, &mm, id).await?;
	let entity = ParentMedicalHistoryBmc::get(&ctx, &mm, id).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

// -- Parent Past Drug History (D.10.8.r)

/// POST /api/cases/{case_id}/patient/parent/{parent_id}/past-drugs
pub async fn create_parent_past_drug_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, parent_id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForCreate<ParentPastDrugHistoryForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<ParentPastDrugHistory>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PARENT_PAST_DRUG_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	ensure_parent_case(&ctx, &mm, case_id, parent_id).await?;
	tracing::debug!(
		"{:<12} - rest create_parent_past_drug_history parent_id={}",
		"HANDLER",
		parent_id
	);

	let ParamsForCreate { data } = params;
	let mut data = data;
	data.parent_id = parent_id;

	let id = ParentPastDrugHistoryBmc::create(&ctx, &mm, data).await?;
	let entity = ParentPastDrugHistoryBmc::get(&ctx, &mm, id).await?;

	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/patient/parent/{parent_id}/past-drugs
pub async fn list_parent_past_drug_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, parent_id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<ParentPastDrugHistory>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PARENT_PAST_DRUG_LIST)?;
	ensure_parent_case(&ctx, &mm, case_id, parent_id).await?;
	tracing::debug!(
		"{:<12} - rest list_parent_past_drug_history parent_id={}",
		"HANDLER",
		parent_id
	);

	let filter = ParentPastDrugHistoryFilter {
		parent_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			parent_id.to_string()
		))])),
		..Default::default()
	};
	let entities = ParentPastDrugHistoryBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/patient/parent/{parent_id}/past-drugs/{id}
pub async fn get_parent_past_drug_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, parent_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<ParentPastDrugHistory>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PARENT_PAST_DRUG_READ)?;
	tracing::debug!(
		"{:<12} - rest get_parent_past_drug_history id={}",
		"HANDLER",
		id
	);

	let entity = ParentPastDrugHistoryBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		parent_id,
		entity.parent_id,
		id,
		"parent_past_drug_history",
	)?;
	ensure_parent_case(&ctx, &mm, case_id, parent_id).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/patient/parent/{parent_id}/past-drugs/{id}
pub async fn update_parent_past_drug_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, parent_id, id)): Path<(Uuid, Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<ParentPastDrugHistoryForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<ParentPastDrugHistory>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PARENT_PAST_DRUG_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	tracing::debug!(
		"{:<12} - rest update_parent_past_drug_history id={}",
		"HANDLER",
		id
	);

	let ParamsForUpdate { data } = params;
	let entity = ParentPastDrugHistoryBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		parent_id,
		entity.parent_id,
		id,
		"parent_past_drug_history",
	)?;
	ensure_parent_case(&ctx, &mm, case_id, parent_id).await?;
	ParentPastDrugHistoryBmc::update(&ctx, &mm, id, data).await?;
	let entity = ParentPastDrugHistoryBmc::get(&ctx, &mm, id).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/patient/parent/{parent_id}/past-drugs/{id}
pub async fn delete_parent_past_drug_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, parent_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PARENT_PAST_DRUG_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	tracing::debug!(
		"{:<12} - rest delete_parent_past_drug_history id={}",
		"HANDLER",
		id
	);

	let entity = ParentPastDrugHistoryBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		parent_id,
		entity.parent_id,
		id,
		"parent_past_drug_history",
	)?;
	ensure_parent_case(&ctx, &mm, case_id, parent_id).await?;
	ParentPastDrugHistoryBmc::delete(&ctx, &mm, id).await?;

	Ok(StatusCode::NO_CONTENT)
}

/// POST /api/cases/{case_id}/patient/parent/{parent_id}/past-drugs/{id}/restore
pub async fn restore_parent_past_drug_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, parent_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<ParentPastDrugHistory>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PARENT_PAST_DRUG_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let entity = ParentPastDrugHistoryBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		parent_id,
		entity.parent_id,
		id,
		"parent_past_drug_history",
	)?;
	ensure_parent_case(&ctx, &mm, case_id, parent_id).await?;
	ParentPastDrugHistoryBmc::restore(&ctx, &mm, id).await?;
	let entity = ParentPastDrugHistoryBmc::get(&ctx, &mm, id).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}
