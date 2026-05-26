use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{
	PRESAVE_TEMPLATE_CREATE, PRESAVE_TEMPLATE_DELETE, PRESAVE_TEMPLATE_LIST,
	PRESAVE_TEMPLATE_READ, PRESAVE_TEMPLATE_UPDATE,
};
use lib_core::model::presave::{
	NarrativePresave, NarrativePresaveBmc, NarrativePresaveCaseSummary,
	NarrativePresaveCaseSummaryBmc, NarrativePresaveCaseSummaryForCreate,
	NarrativePresaveCaseSummaryForUpdate, NarrativePresaveForCreate,
	NarrativePresaveForUpdate, NarrativePresaveSenderDiagnosis,
	NarrativePresaveSenderDiagnosisBmc, NarrativePresaveSenderDiagnosisForCreate,
	NarrativePresaveSenderDiagnosisForUpdate, ProductPresave, ProductPresaveBmc,
	StudyPresave, StudyPresaveBmc, StudyPresaveFdaCrossReportedInd,
	StudyPresaveFdaCrossReportedIndBmc, StudyPresaveFdaCrossReportedIndForCreate,
	StudyPresaveFdaCrossReportedIndForUpdate, StudyPresaveForCreate,
	StudyPresaveForUpdate, StudyPresaveRegistrationNumber,
	StudyPresaveRegistrationNumberBmc, StudyPresaveRegistrationNumberForCreate,
	StudyPresaveRegistrationNumberForUpdate,
};
use lib_core::model::{self, ModelManager};
use lib_core::regulatory::RegulatoryAuthority;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_permission, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct PresaveAuthorityQuery {
	pub authority: Option<RegulatoryAuthority>,
}

pub async fn list_product_presaves(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Query(query): Query<PresaveAuthorityQuery>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<ProductPresave>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let mut entities = ProductPresaveBmc::list(&ctx, &mm, None).await?;
	if let Some(authority) = query.authority {
		entities.retain(|entity| entity.authority == authority);
	}
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn get_product_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<ProductPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = ProductPresaveBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

#[derive(Debug, Deserialize)]
pub struct StudyRegistrationNumberForRestCreate {
	pub sequence_number: i32,
	pub registration_number: Option<String>,
	pub country_code: Option<String>,
	pub deleted: Option<bool>,
}

impl StudyRegistrationNumberForRestCreate {
	fn into_core(
		self,
		study_presave_id: Uuid,
	) -> StudyPresaveRegistrationNumberForCreate {
		StudyPresaveRegistrationNumberForCreate {
			study_presave_id,
			sequence_number: self.sequence_number,
			registration_number: self.registration_number,
			country_code: self.country_code,
			deleted: self.deleted,
		}
	}
}

#[derive(Debug, Deserialize)]
pub struct StudyFdaCrossReportedIndForRestCreate {
	pub sequence_number: i32,
	pub ind_number: Option<String>,
	pub deleted: Option<bool>,
}

impl StudyFdaCrossReportedIndForRestCreate {
	fn into_core(
		self,
		study_presave_id: Uuid,
	) -> StudyPresaveFdaCrossReportedIndForCreate {
		StudyPresaveFdaCrossReportedIndForCreate {
			study_presave_id,
			sequence_number: self.sequence_number,
			ind_number: self.ind_number,
			deleted: self.deleted,
		}
	}
}

#[derive(Debug, Deserialize)]
pub struct NarrativeSenderDiagnosisForRestCreate {
	pub sequence_number: i32,
	pub diagnosis_meddra_version: Option<String>,
	pub diagnosis_meddra_code: Option<String>,
	pub deleted: Option<bool>,
}

impl NarrativeSenderDiagnosisForRestCreate {
	fn into_core(
		self,
		narrative_presave_id: Uuid,
	) -> NarrativePresaveSenderDiagnosisForCreate {
		NarrativePresaveSenderDiagnosisForCreate {
			narrative_presave_id,
			sequence_number: self.sequence_number,
			diagnosis_meddra_version: self.diagnosis_meddra_version,
			diagnosis_meddra_code: self.diagnosis_meddra_code,
			deleted: self.deleted,
		}
	}
}

#[derive(Debug, Deserialize)]
pub struct NarrativeCaseSummaryForRestCreate {
	pub sequence_number: i32,
	pub summary_type: Option<String>,
	pub language_code: Option<String>,
	pub summary_text: Option<String>,
	pub deleted: Option<bool>,
}

impl NarrativeCaseSummaryForRestCreate {
	fn into_core(
		self,
		narrative_presave_id: Uuid,
	) -> NarrativePresaveCaseSummaryForCreate {
		NarrativePresaveCaseSummaryForCreate {
			narrative_presave_id,
			sequence_number: self.sequence_number,
			summary_type: self.summary_type,
			language_code: self.language_code,
			summary_text: self.summary_text,
			deleted: self.deleted,
		}
	}
}

fn text_present(value: Option<&str>) -> bool {
	value.is_some_and(|value| !value.trim().is_empty())
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

fn validate_study_rest_required(data: &StudyPresaveForCreate) -> Result<()> {
	if data.product_presave_id.is_none() {
		return Err(Error::BadRequest {
			message:
				"study presave requires product_presave_id at REST save boundary"
					.to_string(),
		});
	}
	Ok(())
}

fn validate_study_update_rest_required(
	current: &StudyPresave,
	data: &StudyPresaveForUpdate,
) -> Result<()> {
	if current
		.product_presave_id
		.or(data.product_presave_id)
		.is_none()
	{
		return Err(Error::BadRequest {
			message:
				"study presave requires product_presave_id at REST save boundary"
					.to_string(),
		});
	}
	Ok(())
}

fn validate_narrative_rest_required(data: &NarrativePresaveForCreate) -> Result<()> {
	if !text_present(data.case_narrative.as_deref()) {
		return Err(Error::BadRequest {
			message:
				"narrative presave requires case_narrative at REST save boundary"
					.to_string(),
		});
	}
	Ok(())
}

fn validate_narrative_update_rest_required(
	current: &NarrativePresave,
	data: &NarrativePresaveForUpdate,
) -> Result<()> {
	if data
		.case_narrative
		.as_deref()
		.is_some_and(|value| value.trim().is_empty())
	{
		return Err(Error::BadRequest {
			message:
				"narrative presave requires case_narrative at REST save boundary"
					.to_string(),
		});
	}
	if data.case_narrative.is_none()
		&& !text_present(current.case_narrative.as_deref())
	{
		return Err(Error::BadRequest {
			message:
				"narrative presave requires case_narrative at REST save boundary"
					.to_string(),
		});
	}
	Ok(())
}

pub async fn create_study_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForCreate<StudyPresaveForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<StudyPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	let ParamsForCreate { data } = params;
	validate_study_rest_required(&data)?;
	let id = StudyPresaveBmc::create(&ctx, &mm, data).await?;
	let entity = StudyPresaveBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn list_study_presaves(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Query(query): Query<PresaveAuthorityQuery>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<StudyPresave>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let mut entities = StudyPresaveBmc::list(&ctx, &mm, None).await?;
	if let Some(authority) = query.authority {
		entities.retain(|entity| entity.authority == authority);
	}
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn get_study_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<StudyPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = StudyPresaveBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn update_study_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<StudyPresaveForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<StudyPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let ParamsForUpdate { data } = params;
	let current = StudyPresaveBmc::get(&ctx, &mm, id).await?;
	validate_study_update_rest_required(&current, &data)?;
	StudyPresaveBmc::update(&ctx, &mm, id, data).await?;
	let entity = StudyPresaveBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_study_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	StudyPresaveBmc::update(
		&ctx,
		&mm,
		id,
		StudyPresaveForUpdate {
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}

pub async fn create_study_registration_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(study_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<StudyRegistrationNumberForRestCreate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<StudyPresaveRegistrationNumber>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	let ParamsForCreate { data } = params;
	let data = data.into_core(study_id);
	let id = StudyPresaveRegistrationNumberBmc::create(&ctx, &mm, data).await?;
	let entity = StudyPresaveRegistrationNumberBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn list_study_registration_numbers(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(study_id): Path<Uuid>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<StudyPresaveRegistrationNumber>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let entities =
		StudyPresaveRegistrationNumberBmc::list_by_parent(&ctx, &mm, study_id)
			.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn get_study_registration_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((study_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<StudyPresaveRegistrationNumber>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = StudyPresaveRegistrationNumberBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		study_id,
		entity.study_presave_id,
		id,
		"study_presave_registration_numbers",
	)?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn update_study_registration_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((study_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<StudyPresaveRegistrationNumberForUpdate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<StudyPresaveRegistrationNumber>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let entity = StudyPresaveRegistrationNumberBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		study_id,
		entity.study_presave_id,
		id,
		"study_presave_registration_numbers",
	)?;
	let ParamsForUpdate { data } = params;
	StudyPresaveRegistrationNumberBmc::update(&ctx, &mm, id, data).await?;
	let entity = StudyPresaveRegistrationNumberBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_study_registration_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((study_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	let entity = StudyPresaveRegistrationNumberBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		study_id,
		entity.study_presave_id,
		id,
		"study_presave_registration_numbers",
	)?;
	StudyPresaveRegistrationNumberBmc::update(
		&ctx,
		&mm,
		id,
		StudyPresaveRegistrationNumberForUpdate {
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}

pub async fn create_study_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(study_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<StudyFdaCrossReportedIndForRestCreate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<StudyPresaveFdaCrossReportedInd>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	let ParamsForCreate { data } = params;
	let data = data.into_core(study_id);
	let id = StudyPresaveFdaCrossReportedIndBmc::create(&ctx, &mm, data).await?;
	let entity = StudyPresaveFdaCrossReportedIndBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn list_study_fda_cross_reported_inds(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(study_id): Path<Uuid>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<StudyPresaveFdaCrossReportedInd>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let entities =
		StudyPresaveFdaCrossReportedIndBmc::list_by_parent(&ctx, &mm, study_id)
			.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn get_study_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((study_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<StudyPresaveFdaCrossReportedInd>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = StudyPresaveFdaCrossReportedIndBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		study_id,
		entity.study_presave_id,
		id,
		"study_presave_fda_cross_reported_inds",
	)?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn update_study_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((study_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<StudyPresaveFdaCrossReportedIndForUpdate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<StudyPresaveFdaCrossReportedInd>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let entity = StudyPresaveFdaCrossReportedIndBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		study_id,
		entity.study_presave_id,
		id,
		"study_presave_fda_cross_reported_inds",
	)?;
	let ParamsForUpdate { data } = params;
	StudyPresaveFdaCrossReportedIndBmc::update(&ctx, &mm, id, data).await?;
	let entity = StudyPresaveFdaCrossReportedIndBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_study_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((study_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	let entity = StudyPresaveFdaCrossReportedIndBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		study_id,
		entity.study_presave_id,
		id,
		"study_presave_fda_cross_reported_inds",
	)?;
	StudyPresaveFdaCrossReportedIndBmc::update(
		&ctx,
		&mm,
		id,
		StudyPresaveFdaCrossReportedIndForUpdate {
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}

pub async fn create_narrative_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForCreate<NarrativePresaveForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<NarrativePresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	let ParamsForCreate { data } = params;
	validate_narrative_rest_required(&data)?;
	let id = NarrativePresaveBmc::create(&ctx, &mm, data).await?;
	let entity = NarrativePresaveBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn list_narrative_presaves(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Query(query): Query<PresaveAuthorityQuery>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<NarrativePresave>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let mut entities = NarrativePresaveBmc::list(&ctx, &mm, None).await?;
	if let Some(authority) = query.authority {
		entities.retain(|entity| entity.authority == authority);
	}
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn get_narrative_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<NarrativePresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = NarrativePresaveBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn update_narrative_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<NarrativePresaveForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<NarrativePresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let ParamsForUpdate { data } = params;
	let current = NarrativePresaveBmc::get(&ctx, &mm, id).await?;
	validate_narrative_update_rest_required(&current, &data)?;
	NarrativePresaveBmc::update(&ctx, &mm, id, data).await?;
	let entity = NarrativePresaveBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_narrative_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	NarrativePresaveBmc::update(
		&ctx,
		&mm,
		id,
		NarrativePresaveForUpdate {
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}

pub async fn create_narrative_sender_diagnosis(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(narrative_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<NarrativeSenderDiagnosisForRestCreate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<NarrativePresaveSenderDiagnosis>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	let ParamsForCreate { data } = params;
	let data = data.into_core(narrative_id);
	let id = NarrativePresaveSenderDiagnosisBmc::create(&ctx, &mm, data).await?;
	let entity = NarrativePresaveSenderDiagnosisBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn list_narrative_sender_diagnoses(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(narrative_id): Path<Uuid>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<NarrativePresaveSenderDiagnosis>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let entities =
		NarrativePresaveSenderDiagnosisBmc::list_by_parent(&ctx, &mm, narrative_id)
			.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn get_narrative_sender_diagnosis(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((narrative_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<NarrativePresaveSenderDiagnosis>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = NarrativePresaveSenderDiagnosisBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		narrative_id,
		entity.narrative_presave_id,
		id,
		"narrative_presave_sender_diagnoses",
	)?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn update_narrative_sender_diagnosis(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((narrative_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<NarrativePresaveSenderDiagnosisForUpdate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<NarrativePresaveSenderDiagnosis>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let entity = NarrativePresaveSenderDiagnosisBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		narrative_id,
		entity.narrative_presave_id,
		id,
		"narrative_presave_sender_diagnoses",
	)?;
	let ParamsForUpdate { data } = params;
	NarrativePresaveSenderDiagnosisBmc::update(&ctx, &mm, id, data).await?;
	let entity = NarrativePresaveSenderDiagnosisBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_narrative_sender_diagnosis(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((narrative_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	let entity = NarrativePresaveSenderDiagnosisBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		narrative_id,
		entity.narrative_presave_id,
		id,
		"narrative_presave_sender_diagnoses",
	)?;
	NarrativePresaveSenderDiagnosisBmc::update(
		&ctx,
		&mm,
		id,
		NarrativePresaveSenderDiagnosisForUpdate {
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}

pub async fn create_narrative_case_summary(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(narrative_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<NarrativeCaseSummaryForRestCreate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<NarrativePresaveCaseSummary>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	let ParamsForCreate { data } = params;
	let data = data.into_core(narrative_id);
	let id = NarrativePresaveCaseSummaryBmc::create(&ctx, &mm, data).await?;
	let entity = NarrativePresaveCaseSummaryBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn list_narrative_case_summaries(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(narrative_id): Path<Uuid>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<NarrativePresaveCaseSummary>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let entities =
		NarrativePresaveCaseSummaryBmc::list_by_parent(&ctx, &mm, narrative_id)
			.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn get_narrative_case_summary(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((narrative_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<NarrativePresaveCaseSummary>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = NarrativePresaveCaseSummaryBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		narrative_id,
		entity.narrative_presave_id,
		id,
		"narrative_presave_case_summaries",
	)?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn update_narrative_case_summary(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((narrative_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<NarrativePresaveCaseSummaryForUpdate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<NarrativePresaveCaseSummary>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let entity = NarrativePresaveCaseSummaryBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		narrative_id,
		entity.narrative_presave_id,
		id,
		"narrative_presave_case_summaries",
	)?;
	let ParamsForUpdate { data } = params;
	NarrativePresaveCaseSummaryBmc::update(&ctx, &mm, id, data).await?;
	let entity = NarrativePresaveCaseSummaryBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_narrative_case_summary(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((narrative_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	let entity = NarrativePresaveCaseSummaryBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		narrative_id,
		entity.narrative_presave_id,
		id,
		"narrative_presave_case_summaries",
	)?;
	NarrativePresaveCaseSummaryBmc::update(
		&ctx,
		&mm,
		id,
		NarrativePresaveCaseSummaryForUpdate {
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}
