// Patient sub-resources REST endpoints (D.7.1.r, D.8.r, D.9, D.10)

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::patient::{
	AutopsyCauseOfDeath, AutopsyCauseOfDeathBmc, AutopsyCauseOfDeathFilter,
	AutopsyCauseOfDeathForCreate, AutopsyCauseOfDeathForUpdate,
	MedicalHistoryEpisode, MedicalHistoryEpisodeBmc, MedicalHistoryEpisodeFilter,
	MedicalHistoryEpisodeForCreate, MedicalHistoryEpisodeForUpdate,
	ParentInformation, ParentInformationBmc, ParentInformationFilter,
	ParentInformationForCreate, ParentInformationForUpdate, PastDrugHistory,
	PastDrugHistoryBmc, PastDrugHistoryFilter, PastDrugHistoryForCreate,
	PastDrugHistoryForUpdate, PatientDeathInformation, PatientDeathInformationBmc,
	PatientDeathInformationFilter, PatientDeathInformationForCreate,
	PatientDeathInformationForUpdate, PatientIdentifier, PatientIdentifierBmc,
	PatientIdentifierFilter, PatientIdentifierForCreate, PatientIdentifierForUpdate,
	PatientInformationBmc, ReportedCauseOfDeath, ReportedCauseOfDeathBmc,
	ReportedCauseOfDeathFilter, ReportedCauseOfDeathForCreate,
	ReportedCauseOfDeathForUpdate,
};
use lib_core::model::{self, ModelManager};
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::Result;
use lib_web::middleware::mw_auth::CtxW;
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde_json::json;
use uuid::Uuid;

async fn patient_id_for_case(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Uuid> {
	let patient = PatientInformationBmc::get_by_case(ctx, mm, case_id).await?;
	Ok(patient.id)
}

async fn ensure_patient_scope(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	entity_patient_id: Uuid,
	entity_id: Uuid,
	entity: &'static str,
) -> Result<()> {
	let expected_patient_id = patient_id_for_case(ctx, mm, case_id).await?;
	if expected_patient_id != entity_patient_id {
		return Err(model::Error::EntityUuidNotFound {
			entity,
			id: entity_id,
		}
		.into());
	}
	Ok(())
}

async fn ensure_death_info_case(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	death_info_id: Uuid,
) -> Result<()> {
	let death_info = PatientDeathInformationBmc::get(ctx, mm, death_info_id).await?;
	ensure_patient_scope(
		ctx,
		mm,
		case_id,
		death_info.patient_id,
		death_info_id,
		"patient_death_information",
	)
	.await
}

// -- Patient Identifiers (D.1.1.x)

lib_rest_core::generate_patient_child_rest_fns! {
	Bmc: PatientIdentifierBmc,
	Entity: PatientIdentifier,
	ForCreate: PatientIdentifierForCreate,
	ForUpdate: PatientIdentifierForUpdate,
	Filter: PatientIdentifierFilter,
	CreateFn: create_patient_identifier,
	ListFn: list_patient_identifiers,
	GetFn: get_patient_identifier,
	UpdateFn: update_patient_identifier,
	DeleteFn: delete_patient_identifier,
	RestoreFn: restore_patient_identifier,
	ParentField: patient_id,
	ResolveParentFn: patient_id_for_case,
	ScopeFn: ensure_patient_scope,
	EntityName: "patient_identifiers",
	DeleteResult: (StatusCode, Json<DataRestResult<PatientIdentifier>>),
	DeleteResponse: entity
}

// -- Medical History Episodes (D.7.1.r)

lib_rest_core::generate_patient_child_rest_fns! {
	Bmc: MedicalHistoryEpisodeBmc, Entity: MedicalHistoryEpisode,
	ForCreate: MedicalHistoryEpisodeForCreate, ForUpdate: MedicalHistoryEpisodeForUpdate,
	Filter: MedicalHistoryEpisodeFilter,
	CreateFn: create_medical_history_episode, ListFn: list_medical_history_episodes,
	GetFn: get_medical_history_episode, UpdateFn: update_medical_history_episode,
	DeleteFn: delete_medical_history_episode, RestoreFn: restore_medical_history_episode,
	ParentField: patient_id, ResolveParentFn: patient_id_for_case,
	ScopeFn: ensure_patient_scope, EntityName: "medical_history_episodes",
	DeleteResult: StatusCode, DeleteResponse: no_content
}

// -- Past Drug History (D.8.r)

lib_rest_core::generate_patient_child_rest_fns! {
	Bmc: PastDrugHistoryBmc, Entity: PastDrugHistory,
	ForCreate: PastDrugHistoryForCreate, ForUpdate: PastDrugHistoryForUpdate,
	Filter: PastDrugHistoryFilter,
	CreateFn: create_past_drug_history, ListFn: list_past_drug_history,
	GetFn: get_past_drug_history, UpdateFn: update_past_drug_history,
	DeleteFn: delete_past_drug_history, RestoreFn: restore_past_drug_history,
	ParentField: patient_id, ResolveParentFn: patient_id_for_case,
	ScopeFn: ensure_patient_scope, EntityName: "past_drug_history",
	DeleteResult: StatusCode, DeleteResponse: no_content
}

// -- Patient Death Information (D.9)

/// POST /api/cases/{case_id}/patient/death-info
pub async fn create_patient_death_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<PatientDeathInformationForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<PatientDeathInformation>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"patient/death-info",
		move |ctx, mm| {
			Box::pin(async move {
				let patient_id = patient_id_for_case(ctx, mm, case_id).await?;
				let ParamsForCreate { data } = params;
				let mut data = data;
				data.patient_id = patient_id;
				let id = PatientDeathInformationBmc::create(ctx, mm, data).await?;
				let entity = PatientDeathInformationBmc::get(ctx, mm, id).await?;
				Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// GET /api/cases/{case_id}/patient/death-info
pub async fn list_patient_death_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<PatientDeathInformation>>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"patient/death-info",
		move |ctx, mm| {
			Box::pin(async move {
				let patient_id = patient_id_for_case(ctx, mm, case_id).await?;
				let filter = PatientDeathInformationFilter {
					patient_id: Some(OpValsValue::from(vec![OpValValue::Eq(
						json!(patient_id.to_string()),
					)])),
				};
				let entities = PatientDeathInformationBmc::list(
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

/// GET /api/cases/{case_id}/patient/death-info/{id}
pub async fn get_patient_death_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<PatientDeathInformation>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("patient/death-info/{id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let entity = PatientDeathInformationBmc::get(ctx, mm, id).await?;
				ensure_patient_scope(
					ctx,
					mm,
					case_id,
					entity.patient_id,
					id,
					"patient_death_information",
				)
				.await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// PUT /api/cases/{case_id}/patient/death-info/{id}
pub async fn update_patient_death_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<PatientDeathInformationForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<PatientDeathInformation>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("patient/death-info/{id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForUpdate { data } = params;
				let entity = PatientDeathInformationBmc::get(ctx, mm, id).await?;
				ensure_patient_scope(
					ctx,
					mm,
					case_id,
					entity.patient_id,
					id,
					"patient_death_information",
				)
				.await?;
				PatientDeathInformationBmc::update(ctx, mm, id, data).await?;
				let entity = PatientDeathInformationBmc::get(ctx, mm, id).await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// DELETE /api/cases/{case_id}/patient/death-info/{id}
pub async fn delete_patient_death_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("patient/death-info/{id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let entity = PatientDeathInformationBmc::get(ctx, mm, id).await?;
				ensure_patient_scope(
					ctx,
					mm,
					case_id,
					entity.patient_id,
					id,
					"patient_death_information",
				)
				.await?;
				PatientDeathInformationBmc::delete(ctx, mm, id).await?;
				Ok(StatusCode::NO_CONTENT)
			})
		},
	)
	.await
}

// Reported and autopsy causes share a contract, including child-first scope checks.
macro_rules! death_cause_rest_fns {
	(
		Bmc: $bmc:ident, Entity: $entity:ident,
		ForCreate: $for_create:ident, ForUpdate: $for_update:ident,
		Filter: $filter:ident,
		CreateFn: $create_fn:ident, ListFn: $list_fn:ident,
		GetFn: $get_fn:ident, UpdateFn: $update_fn:ident,
		DeleteFn: $delete_fn:ident, RestoreFn: $restore_fn:ident,
		Route: $route:literal, EntityName: $entity_name:literal
	) => {
		pub async fn $create_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			Path((case_id, death_info_id)): Path<(Uuid, Uuid)>,
			Json(params): Json<ParamsForCreate<$for_create>>,
		) -> Result<(StatusCode, Json<DataRestResult<$entity>>)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_mutation(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				format!("patient/death-info/{death_info_id}/{}", $route),
				move |ctx, mm| {
					Box::pin(async move {
						ensure_death_info_case(ctx, mm, case_id, death_info_id).await?;
						let ParamsForCreate { data } = params;
						let mut data = data;
						data.death_info_id = death_info_id;
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
			Path((case_id, death_info_id)): Path<(Uuid, Uuid)>,
		) -> Result<(StatusCode, Json<DataRestResult<Vec<$entity>>>)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_read(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				format!("patient/death-info/{death_info_id}/{}", $route),
				move |ctx, mm| {
					Box::pin(async move {
						ensure_death_info_case(ctx, mm, case_id, death_info_id).await?;
						let filter = $filter {
							death_info_id: Some(OpValsValue::from(vec![OpValValue::Eq(
								json!(death_info_id.to_string()),
							)])),
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
			Path((case_id, death_info_id, id)): Path<(Uuid, Uuid, Uuid)>,
		) -> Result<(StatusCode, Json<DataRestResult<$entity>>)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_read(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				format!("patient/death-info/{death_info_id}/{}/{id}", $route),
				move |ctx, mm| {
					Box::pin(async move {
						let entity = $bmc::get(ctx, mm, id).await?;
						if entity.death_info_id != death_info_id {
							return Err(model::Error::EntityUuidNotFound {
								entity: $entity_name,
								id,
							}
							.into());
						}
						ensure_death_info_case(ctx, mm, case_id, death_info_id).await?;
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
			Path((case_id, death_info_id, id)): Path<(Uuid, Uuid, Uuid)>,
			Json(params): Json<ParamsForUpdate<$for_update>>,
		) -> Result<(StatusCode, Json<DataRestResult<$entity>>)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_mutation(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				format!("patient/death-info/{death_info_id}/{}/{id}", $route),
				move |ctx, mm| {
					Box::pin(async move {
						let ParamsForUpdate { data } = params;
						let entity = $bmc::get(ctx, mm, id).await?;
						if entity.death_info_id != death_info_id {
							return Err(model::Error::EntityUuidNotFound {
								entity: $entity_name,
								id,
							}
							.into());
						}
						ensure_death_info_case(ctx, mm, case_id, death_info_id).await?;
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
			Path((case_id, death_info_id, id)): Path<(Uuid, Uuid, Uuid)>,
		) -> Result<StatusCode> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_mutation(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				format!("patient/death-info/{death_info_id}/{}/{id}", $route),
				move |ctx, mm| {
					Box::pin(async move {
						let entity = $bmc::get(ctx, mm, id).await?;
						if entity.death_info_id != death_info_id {
							return Err(model::Error::EntityUuidNotFound {
								entity: $entity_name,
								id,
							}
							.into());
						}
						ensure_death_info_case(ctx, mm, case_id, death_info_id).await?;
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
			Path((case_id, death_info_id, id)): Path<(Uuid, Uuid, Uuid)>,
		) -> Result<(StatusCode, Json<DataRestResult<$entity>>)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_mutation(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				format!("patient/death-info/{death_info_id}/{}/{id}", $route),
				move |ctx, mm| {
					Box::pin(async move {
						let entity = $bmc::get(ctx, mm, id).await?;
						if entity.death_info_id != death_info_id {
							return Err(model::Error::EntityUuidNotFound {
								entity: $entity_name,
								id,
							}
							.into());
						}
						ensure_death_info_case(ctx, mm, case_id, death_info_id).await?;
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

death_cause_rest_fns! {
	Bmc: ReportedCauseOfDeathBmc, Entity: ReportedCauseOfDeath,
	ForCreate: ReportedCauseOfDeathForCreate, ForUpdate: ReportedCauseOfDeathForUpdate,
	Filter: ReportedCauseOfDeathFilter,
	CreateFn: create_reported_cause_of_death, ListFn: list_reported_causes_of_death,
	GetFn: get_reported_cause_of_death, UpdateFn: update_reported_cause_of_death,
	DeleteFn: delete_reported_cause_of_death, RestoreFn: restore_reported_cause_of_death,
	Route: "reported-causes", EntityName: "reported_causes_of_death"
}

death_cause_rest_fns! {
	Bmc: AutopsyCauseOfDeathBmc, Entity: AutopsyCauseOfDeath,
	ForCreate: AutopsyCauseOfDeathForCreate, ForUpdate: AutopsyCauseOfDeathForUpdate,
	Filter: AutopsyCauseOfDeathFilter,
	CreateFn: create_autopsy_cause_of_death, ListFn: list_autopsy_causes_of_death,
	GetFn: get_autopsy_cause_of_death, UpdateFn: update_autopsy_cause_of_death,
	DeleteFn: delete_autopsy_cause_of_death, RestoreFn: restore_autopsy_cause_of_death,
	Route: "autopsy-causes", EntityName: "autopsy_causes_of_death"
}

// -- Parent Information (D.10)

lib_rest_core::generate_patient_child_rest_fns! {
	Bmc: ParentInformationBmc, Entity: ParentInformation,
	ForCreate: ParentInformationForCreate, ForUpdate: ParentInformationForUpdate,
	Filter: ParentInformationFilter,
	CreateFn: create_parent_information, ListFn: list_parent_information,
	GetFn: get_parent_information, UpdateFn: update_parent_information,
	DeleteFn: delete_parent_information, RestoreFn: restore_parent_information,
	ParentField: patient_id, ResolveParentFn: patient_id_for_case,
	ScopeFn: ensure_patient_scope, EntityName: "parent_information",
	DeleteResult: StatusCode, DeleteResponse: no_content
}
