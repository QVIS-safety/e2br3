// Parent History REST endpoints (D.10.7 and D.10.8)

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
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
use lib_rest_core::Result;
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

// Both parent-history resources share the same route and authorization contract.
macro_rules! parent_history_rest_fns {
	(
		Bmc: $bmc:ident, Entity: $entity:ident,
		ForCreate: $for_create:ident, ForUpdate: $for_update:ident,
		Filter: $filter:ident,
		CreateFn: $create_fn:ident, ListFn: $list_fn:ident,
		GetFn: $get_fn:ident, UpdateFn: $update_fn:ident,
		DeleteFn: $delete_fn:ident, RestoreFn: $restore_fn:ident,
		Fingerprint: $fingerprint:literal, EntityName: $entity_name:literal
	) => {
		pub async fn $create_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			Path((case_id, parent_id)): Path<(Uuid, Uuid)>,
			Json(params): Json<ParamsForCreate<$for_create>>,
		) -> Result<(StatusCode, Json<DataRestResult<$entity>>)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_mutation(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				format!("{}:new:parent:{parent_id}", $fingerprint),
				move |ctx, mm| {
					Box::pin(async move {
						ensure_parent_case(ctx, mm, case_id, parent_id).await?;
						let ParamsForCreate { data } = params;
						let mut data = data;
						data.parent_id = parent_id;
						let id = $bmc::create(ctx, mm, data).await?;
						let entity = $bmc::get(ctx, mm, id).await?;
						Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
					})
				},
			)
			.await
		}

		pub async fn $list_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			Path((case_id, parent_id)): Path<(Uuid, Uuid)>,
		) -> Result<(StatusCode, Json<DataRestResult<Vec<$entity>>>)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_read(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				format!("{}:list:parent:{parent_id}", $fingerprint),
				move |ctx, mm| {
					Box::pin(async move {
						ensure_parent_case(ctx, mm, case_id, parent_id).await?;
						let filter = $filter {
							parent_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
								parent_id.to_string()
							))])),
							..Default::default()
						};
						let entities = $bmc::list(
							ctx,
							mm,
							Some(vec![filter]),
							Some(ListOptions::default()),
						)
						.await?;
						Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
					})
				},
			)
			.await
		}

		pub async fn $get_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			Path((case_id, parent_id, id)): Path<(Uuid, Uuid, Uuid)>,
		) -> Result<(StatusCode, Json<DataRestResult<$entity>>)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_read(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				format!("{}:{id}:parent:{parent_id}", $fingerprint),
				move |ctx, mm| {
					Box::pin(async move {
						ensure_parent_case(ctx, mm, case_id, parent_id).await?;
						let entity = $bmc::get(ctx, mm, id).await?;
						ensure_parent_scope(
							parent_id,
							entity.parent_id,
							id,
							$entity_name,
						)?;
						Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
					})
				},
			)
			.await
		}

		pub async fn $update_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			Path((case_id, parent_id, id)): Path<(Uuid, Uuid, Uuid)>,
			Json(params): Json<ParamsForUpdate<$for_update>>,
		) -> Result<(StatusCode, Json<DataRestResult<$entity>>)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_mutation(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				format!("{}:{id}:parent:{parent_id}", $fingerprint),
				move |ctx, mm| {
					Box::pin(async move {
						ensure_parent_case(ctx, mm, case_id, parent_id).await?;
						let entity = $bmc::get(ctx, mm, id).await?;
						ensure_parent_scope(
							parent_id,
							entity.parent_id,
							id,
							$entity_name,
						)?;
						let ParamsForUpdate { data } = params;
						$bmc::update(ctx, mm, id, data).await?;
						let entity = $bmc::get(ctx, mm, id).await?;
						Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
					})
				},
			)
			.await
		}

		pub async fn $delete_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			Path((case_id, parent_id, id)): Path<(Uuid, Uuid, Uuid)>,
		) -> Result<StatusCode> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_mutation(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				format!("{}:{id}:parent:{parent_id}", $fingerprint),
				move |ctx, mm| {
					Box::pin(async move {
						ensure_parent_case(ctx, mm, case_id, parent_id).await?;
						let entity = $bmc::get(ctx, mm, id).await?;
						ensure_parent_scope(
							parent_id,
							entity.parent_id,
							id,
							$entity_name,
						)?;
						$bmc::delete(ctx, mm, id).await?;
						Ok(StatusCode::NO_CONTENT)
					})
				},
			)
			.await
		}

		pub async fn $restore_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			Path((case_id, parent_id, id)): Path<(Uuid, Uuid, Uuid)>,
		) -> Result<(StatusCode, Json<DataRestResult<$entity>>)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_mutation(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				format!("{}:{id}:parent:{parent_id}", $fingerprint),
				move |ctx, mm| {
					Box::pin(async move {
						ensure_parent_case(ctx, mm, case_id, parent_id).await?;
						let entity = $bmc::get(ctx, mm, id).await?;
						ensure_parent_scope(
							parent_id,
							entity.parent_id,
							id,
							$entity_name,
						)?;
						$bmc::restore(ctx, mm, id).await?;
						let entity = $bmc::get(ctx, mm, id).await?;
						Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
					})
				},
			)
			.await
		}
	};
}

parent_history_rest_fns! {
	Bmc: ParentMedicalHistoryBmc, Entity: ParentMedicalHistory,
	ForCreate: ParentMedicalHistoryForCreate, ForUpdate: ParentMedicalHistoryForUpdate,
	Filter: ParentMedicalHistoryFilter,
	CreateFn: create_parent_medical_history, ListFn: list_parent_medical_history,
	GetFn: get_parent_medical_history, UpdateFn: update_parent_medical_history,
	DeleteFn: delete_parent_medical_history, RestoreFn: restore_parent_medical_history,
	Fingerprint: "parent-medical-history", EntityName: "parent_medical_history"
}

parent_history_rest_fns! {
	Bmc: ParentPastDrugHistoryBmc, Entity: ParentPastDrugHistory,
	ForCreate: ParentPastDrugHistoryForCreate, ForUpdate: ParentPastDrugHistoryForUpdate,
	Filter: ParentPastDrugHistoryFilter,
	CreateFn: create_parent_past_drug_history, ListFn: list_parent_past_drug_history,
	GetFn: get_parent_past_drug_history, UpdateFn: update_parent_past_drug_history,
	DeleteFn: delete_parent_past_drug_history, RestoreFn: restore_parent_past_drug_history,
	Fingerprint: "parent-past-drug-history", EntityName: "parent_past_drug_history"
}
