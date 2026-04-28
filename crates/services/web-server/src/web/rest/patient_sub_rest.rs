// Patient sub-resources REST endpoints (D.7.1.r, D.8.r, D.9, D.10)

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{
	DEATH_CAUSE_CREATE, DEATH_CAUSE_DELETE, DEATH_CAUSE_LIST, DEATH_CAUSE_READ,
	DEATH_CAUSE_UPDATE, MEDICAL_HISTORY_CREATE, MEDICAL_HISTORY_DELETE,
	MEDICAL_HISTORY_LIST, MEDICAL_HISTORY_READ, MEDICAL_HISTORY_UPDATE,
	PARENT_INFORMATION_CREATE, PARENT_INFORMATION_DELETE, PARENT_INFORMATION_LIST,
	PARENT_INFORMATION_READ, PARENT_INFORMATION_UPDATE, PAST_DRUG_CREATE,
	PAST_DRUG_DELETE, PAST_DRUG_LIST, PAST_DRUG_READ, PAST_DRUG_UPDATE,
	PATIENT_DEATH_CREATE, PATIENT_DEATH_DELETE, PATIENT_DEATH_LIST,
	PATIENT_DEATH_READ, PATIENT_DEATH_UPDATE, PATIENT_IDENTIFIER_CREATE,
	PATIENT_IDENTIFIER_DELETE, PATIENT_IDENTIFIER_LIST, PATIENT_IDENTIFIER_READ,
	PATIENT_IDENTIFIER_UPDATE,
};
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
use lib_rest_core::{require_case_write_allowed, require_permission, Result};
use lib_web::middleware::mw_auth::CtxW;
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde_json::json;
use uuid::Uuid;

async fn patient_id_for_case(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Uuid> {
	lib_rest_core::require_case_read_allowed(ctx, mm, case_id).await?;
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

/// POST /api/cases/{case_id}/patient/identifiers
pub async fn create_patient_identifier(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<PatientIdentifierForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<PatientIdentifier>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PATIENT_IDENTIFIER_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let patient_id = patient_id_for_case(&ctx, &mm, case_id).await?;

	let ParamsForCreate { data } = params;
	let mut data = data;
	data.patient_id = patient_id;

	let id = PatientIdentifierBmc::create(&ctx, &mm, data).await?;
	let entity = PatientIdentifierBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/patient/identifiers
pub async fn list_patient_identifiers(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<PatientIdentifier>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PATIENT_IDENTIFIER_LIST)?;
	let patient_id = patient_id_for_case(&ctx, &mm, case_id).await?;

	let filter = PatientIdentifierFilter {
		patient_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			patient_id.to_string()
		))])),
		..Default::default()
	};
	let entities = PatientIdentifierBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/patient/identifiers/{id}
pub async fn get_patient_identifier(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<PatientIdentifier>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PATIENT_IDENTIFIER_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let entity = PatientIdentifierBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/patient/identifiers/{id}
pub async fn update_patient_identifier(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<PatientIdentifierForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<PatientIdentifier>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PATIENT_IDENTIFIER_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;

	let ParamsForUpdate { data } = params;
	PatientIdentifierBmc::update(&ctx, &mm, id, data).await?;
	let entity = PatientIdentifierBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/patient/identifiers/{id}
pub async fn delete_patient_identifier(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<PatientIdentifier>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PATIENT_IDENTIFIER_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;

	let entity = PatientIdentifierBmc::get(&ctx, &mm, id).await?;
	PatientIdentifierBmc::delete(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

// -- Medical History Episodes (D.7.1.r)

/// POST /api/cases/{case_id}/patient/medical-history
pub async fn create_medical_history_episode(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<MedicalHistoryEpisodeForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<MedicalHistoryEpisode>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, MEDICAL_HISTORY_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let patient_id = patient_id_for_case(&ctx, &mm, case_id).await?;

	let ParamsForCreate { data } = params;
	let mut data = data;
	data.patient_id = patient_id;

	let id = MedicalHistoryEpisodeBmc::create(&ctx, &mm, data).await?;
	let entity = MedicalHistoryEpisodeBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/patient/medical-history
pub async fn list_medical_history_episodes(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<MedicalHistoryEpisode>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, MEDICAL_HISTORY_LIST)?;
	let patient_id = patient_id_for_case(&ctx, &mm, case_id).await?;

	let filter = MedicalHistoryEpisodeFilter {
		patient_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			patient_id.to_string()
		))])),
		..Default::default()
	};
	let entities = MedicalHistoryEpisodeBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/patient/medical-history/{id}
pub async fn get_medical_history_episode(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<MedicalHistoryEpisode>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, MEDICAL_HISTORY_READ)?;
	let entity = MedicalHistoryEpisodeBmc::get(&ctx, &mm, id).await?;
	ensure_patient_scope(
		&ctx,
		&mm,
		case_id,
		entity.patient_id,
		id,
		"medical_history_episodes",
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/patient/medical-history/{id}
pub async fn update_medical_history_episode(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<MedicalHistoryEpisodeForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<MedicalHistoryEpisode>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, MEDICAL_HISTORY_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForUpdate { data } = params;
	let entity = MedicalHistoryEpisodeBmc::get(&ctx, &mm, id).await?;
	ensure_patient_scope(
		&ctx,
		&mm,
		case_id,
		entity.patient_id,
		id,
		"medical_history_episodes",
	)
	.await?;
	MedicalHistoryEpisodeBmc::update(&ctx, &mm, id, data).await?;
	let entity = MedicalHistoryEpisodeBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/patient/medical-history/{id}
pub async fn delete_medical_history_episode(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, MEDICAL_HISTORY_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let entity = MedicalHistoryEpisodeBmc::get(&ctx, &mm, id).await?;
	ensure_patient_scope(
		&ctx,
		&mm,
		case_id,
		entity.patient_id,
		id,
		"medical_history_episodes",
	)
	.await?;
	MedicalHistoryEpisodeBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}

// -- Past Drug History (D.8.r)

/// POST /api/cases/{case_id}/patient/past-drugs
pub async fn create_past_drug_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<PastDrugHistoryForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<PastDrugHistory>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PAST_DRUG_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let patient_id = patient_id_for_case(&ctx, &mm, case_id).await?;

	let ParamsForCreate { data } = params;
	let mut data = data;
	data.patient_id = patient_id;

	let id = PastDrugHistoryBmc::create(&ctx, &mm, data).await?;
	let entity = PastDrugHistoryBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/patient/past-drugs
pub async fn list_past_drug_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<PastDrugHistory>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PAST_DRUG_LIST)?;
	let patient_id = patient_id_for_case(&ctx, &mm, case_id).await?;

	let filter = PastDrugHistoryFilter {
		patient_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			patient_id.to_string()
		))])),
		..Default::default()
	};
	let entities = PastDrugHistoryBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/patient/past-drugs/{id}
pub async fn get_past_drug_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<PastDrugHistory>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PAST_DRUG_READ)?;
	let entity = PastDrugHistoryBmc::get(&ctx, &mm, id).await?;
	ensure_patient_scope(
		&ctx,
		&mm,
		case_id,
		entity.patient_id,
		id,
		"past_drug_history",
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/patient/past-drugs/{id}
pub async fn update_past_drug_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<PastDrugHistoryForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<PastDrugHistory>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PAST_DRUG_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForUpdate { data } = params;
	let entity = PastDrugHistoryBmc::get(&ctx, &mm, id).await?;
	ensure_patient_scope(
		&ctx,
		&mm,
		case_id,
		entity.patient_id,
		id,
		"past_drug_history",
	)
	.await?;
	PastDrugHistoryBmc::update(&ctx, &mm, id, data).await?;
	let entity = PastDrugHistoryBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/patient/past-drugs/{id}
pub async fn delete_past_drug_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PAST_DRUG_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let entity = PastDrugHistoryBmc::get(&ctx, &mm, id).await?;
	ensure_patient_scope(
		&ctx,
		&mm,
		case_id,
		entity.patient_id,
		id,
		"past_drug_history",
	)
	.await?;
	PastDrugHistoryBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}

// -- Patient Death Information (D.9)

/// POST /api/cases/{case_id}/patient/death-info
pub async fn create_patient_death_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<PatientDeathInformationForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<PatientDeathInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PATIENT_DEATH_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let patient_id = patient_id_for_case(&ctx, &mm, case_id).await?;

	let ParamsForCreate { data } = params;
	let mut data = data;
	data.patient_id = patient_id;

	let id = PatientDeathInformationBmc::create(&ctx, &mm, data).await?;
	let entity = PatientDeathInformationBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/patient/death-info
pub async fn list_patient_death_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<PatientDeathInformation>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PATIENT_DEATH_LIST)?;
	let patient_id = patient_id_for_case(&ctx, &mm, case_id).await?;

	let filter = PatientDeathInformationFilter {
		patient_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			patient_id.to_string()
		))])),
	};
	let entities = PatientDeathInformationBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/patient/death-info/{id}
pub async fn get_patient_death_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<PatientDeathInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PATIENT_DEATH_READ)?;
	let entity = PatientDeathInformationBmc::get(&ctx, &mm, id).await?;
	ensure_patient_scope(
		&ctx,
		&mm,
		case_id,
		entity.patient_id,
		id,
		"patient_death_information",
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/patient/death-info/{id}
pub async fn update_patient_death_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<PatientDeathInformationForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<PatientDeathInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PATIENT_DEATH_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForUpdate { data } = params;
	let entity = PatientDeathInformationBmc::get(&ctx, &mm, id).await?;
	ensure_patient_scope(
		&ctx,
		&mm,
		case_id,
		entity.patient_id,
		id,
		"patient_death_information",
	)
	.await?;
	PatientDeathInformationBmc::update(&ctx, &mm, id, data).await?;
	let entity = PatientDeathInformationBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/patient/death-info/{id}
pub async fn delete_patient_death_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PATIENT_DEATH_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let entity = PatientDeathInformationBmc::get(&ctx, &mm, id).await?;
	ensure_patient_scope(
		&ctx,
		&mm,
		case_id,
		entity.patient_id,
		id,
		"patient_death_information",
	)
	.await?;
	PatientDeathInformationBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}

// -- Reported Cause of Death (D.9.2.r)

/// POST /api/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes
pub async fn create_reported_cause_of_death(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, death_info_id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForCreate<ReportedCauseOfDeathForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<ReportedCauseOfDeath>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DEATH_CAUSE_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	ensure_death_info_case(&ctx, &mm, case_id, death_info_id).await?;
	let ParamsForCreate { data } = params;
	let mut data = data;
	data.death_info_id = death_info_id;

	let id = ReportedCauseOfDeathBmc::create(&ctx, &mm, data).await?;
	let entity = ReportedCauseOfDeathBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes
pub async fn list_reported_causes_of_death(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, death_info_id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<ReportedCauseOfDeath>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DEATH_CAUSE_LIST)?;
	ensure_death_info_case(&ctx, &mm, case_id, death_info_id).await?;
	let filter = ReportedCauseOfDeathFilter {
		death_info_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			death_info_id.to_string()
		))])),
		..Default::default()
	};
	let entities = ReportedCauseOfDeathBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes/{id}
pub async fn get_reported_cause_of_death(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, death_info_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<ReportedCauseOfDeath>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DEATH_CAUSE_READ)?;
	let entity = ReportedCauseOfDeathBmc::get(&ctx, &mm, id).await?;
	if entity.death_info_id != death_info_id {
		return Err(model::Error::EntityUuidNotFound {
			entity: "reported_causes_of_death",
			id,
		}
		.into());
	}
	ensure_death_info_case(&ctx, &mm, case_id, death_info_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes/{id}
pub async fn update_reported_cause_of_death(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, death_info_id, id)): Path<(Uuid, Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<ReportedCauseOfDeathForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<ReportedCauseOfDeath>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DEATH_CAUSE_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForUpdate { data } = params;
	let entity = ReportedCauseOfDeathBmc::get(&ctx, &mm, id).await?;
	if entity.death_info_id != death_info_id {
		return Err(model::Error::EntityUuidNotFound {
			entity: "reported_causes_of_death",
			id,
		}
		.into());
	}
	ensure_death_info_case(&ctx, &mm, case_id, death_info_id).await?;
	ReportedCauseOfDeathBmc::update(&ctx, &mm, id, data).await?;
	let entity = ReportedCauseOfDeathBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes/{id}
pub async fn delete_reported_cause_of_death(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, death_info_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DEATH_CAUSE_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let entity = ReportedCauseOfDeathBmc::get(&ctx, &mm, id).await?;
	if entity.death_info_id != death_info_id {
		return Err(model::Error::EntityUuidNotFound {
			entity: "reported_causes_of_death",
			id,
		}
		.into());
	}
	ensure_death_info_case(&ctx, &mm, case_id, death_info_id).await?;
	ReportedCauseOfDeathBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}

// -- Autopsy Cause of Death (D.9.4.r)

/// POST /api/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes
pub async fn create_autopsy_cause_of_death(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, death_info_id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForCreate<AutopsyCauseOfDeathForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<AutopsyCauseOfDeath>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DEATH_CAUSE_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	ensure_death_info_case(&ctx, &mm, case_id, death_info_id).await?;
	let ParamsForCreate { data } = params;
	let mut data = data;
	data.death_info_id = death_info_id;

	let id = AutopsyCauseOfDeathBmc::create(&ctx, &mm, data).await?;
	let entity = AutopsyCauseOfDeathBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes
pub async fn list_autopsy_causes_of_death(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, death_info_id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<AutopsyCauseOfDeath>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DEATH_CAUSE_LIST)?;
	ensure_death_info_case(&ctx, &mm, case_id, death_info_id).await?;
	let filter = AutopsyCauseOfDeathFilter {
		death_info_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			death_info_id.to_string()
		))])),
		..Default::default()
	};
	let entities = AutopsyCauseOfDeathBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes/{id}
pub async fn get_autopsy_cause_of_death(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, death_info_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<AutopsyCauseOfDeath>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DEATH_CAUSE_READ)?;
	let entity = AutopsyCauseOfDeathBmc::get(&ctx, &mm, id).await?;
	if entity.death_info_id != death_info_id {
		return Err(model::Error::EntityUuidNotFound {
			entity: "autopsy_causes_of_death",
			id,
		}
		.into());
	}
	ensure_death_info_case(&ctx, &mm, case_id, death_info_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes/{id}
pub async fn update_autopsy_cause_of_death(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, death_info_id, id)): Path<(Uuid, Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<AutopsyCauseOfDeathForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<AutopsyCauseOfDeath>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DEATH_CAUSE_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForUpdate { data } = params;
	let entity = AutopsyCauseOfDeathBmc::get(&ctx, &mm, id).await?;
	if entity.death_info_id != death_info_id {
		return Err(model::Error::EntityUuidNotFound {
			entity: "autopsy_causes_of_death",
			id,
		}
		.into());
	}
	ensure_death_info_case(&ctx, &mm, case_id, death_info_id).await?;
	AutopsyCauseOfDeathBmc::update(&ctx, &mm, id, data).await?;
	let entity = AutopsyCauseOfDeathBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes/{id}
pub async fn delete_autopsy_cause_of_death(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, death_info_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DEATH_CAUSE_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let entity = AutopsyCauseOfDeathBmc::get(&ctx, &mm, id).await?;
	if entity.death_info_id != death_info_id {
		return Err(model::Error::EntityUuidNotFound {
			entity: "autopsy_causes_of_death",
			id,
		}
		.into());
	}
	ensure_death_info_case(&ctx, &mm, case_id, death_info_id).await?;
	AutopsyCauseOfDeathBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}

// -- Parent Information (D.10)

/// POST /api/cases/{case_id}/patient/parents
pub async fn create_parent_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<ParentInformationForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<ParentInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PARENT_INFORMATION_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let patient_id = patient_id_for_case(&ctx, &mm, case_id).await?;

	let ParamsForCreate { data } = params;
	let mut data = data;
	data.patient_id = patient_id;

	let id = ParentInformationBmc::create(&ctx, &mm, data).await?;
	let entity = ParentInformationBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/patient/parents
pub async fn list_parent_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<ParentInformation>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PARENT_INFORMATION_LIST)?;
	let patient_id = patient_id_for_case(&ctx, &mm, case_id).await?;

	let filter = ParentInformationFilter {
		patient_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			patient_id.to_string()
		))])),
	};
	let entities = ParentInformationBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/patient/parents/{id}
pub async fn get_parent_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<ParentInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PARENT_INFORMATION_READ)?;
	let entity = ParentInformationBmc::get(&ctx, &mm, id).await?;
	ensure_patient_scope(
		&ctx,
		&mm,
		case_id,
		entity.patient_id,
		id,
		"parent_information",
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/patient/parents/{id}
pub async fn update_parent_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<ParentInformationForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<ParentInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PARENT_INFORMATION_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForUpdate { data } = params;
	let entity = ParentInformationBmc::get(&ctx, &mm, id).await?;
	ensure_patient_scope(
		&ctx,
		&mm,
		case_id,
		entity.patient_id,
		id,
		"parent_information",
	)
	.await?;
	ParentInformationBmc::update(&ctx, &mm, id, data).await?;
	let entity = ParentInformationBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/patient/parents/{id}
pub async fn delete_parent_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PARENT_INFORMATION_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let entity = ParentInformationBmc::get(&ctx, &mm, id).await?;
	ensure_patient_scope(
		&ctx,
		&mm,
		case_id,
		entity.patient_id,
		id,
		"parent_information",
	)
	.await?;
	ParentInformationBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}
