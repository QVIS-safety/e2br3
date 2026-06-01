// Safety Report sub-resources REST endpoints (C.2.r, C.3.x, C.4.r, C.5)

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{
	LITERATURE_REFERENCE_CREATE, LITERATURE_REFERENCE_DELETE,
	LITERATURE_REFERENCE_LIST, LITERATURE_REFERENCE_READ,
	LITERATURE_REFERENCE_UPDATE, PRIMARY_SOURCE_CREATE, PRIMARY_SOURCE_DELETE,
	PRIMARY_SOURCE_LIST, PRIMARY_SOURCE_READ, PRIMARY_SOURCE_UPDATE,
	SAFETY_REPORT_READ, SAFETY_REPORT_UPDATE, SENDER_INFORMATION_CREATE,
	SENDER_INFORMATION_DELETE, SENDER_INFORMATION_LIST, SENDER_INFORMATION_READ,
	SENDER_INFORMATION_UPDATE, STUDY_INFORMATION_CREATE, STUDY_INFORMATION_DELETE,
	STUDY_INFORMATION_LIST, STUDY_INFORMATION_READ, STUDY_INFORMATION_UPDATE,
	STUDY_REGISTRATION_CREATE, STUDY_REGISTRATION_DELETE, STUDY_REGISTRATION_LIST,
	STUDY_REGISTRATION_READ, STUDY_REGISTRATION_UPDATE,
};
use lib_core::model::case::{CaseBmc, CaseForUpdate};
use lib_core::model::safety_report::{
	DocumentsHeldBySender, DocumentsHeldBySenderBmc, DocumentsHeldBySenderFilter,
	DocumentsHeldBySenderForCreate, DocumentsHeldBySenderForUpdate,
	LiteratureReference, LiteratureReferenceBmc, LiteratureReferenceFilter,
	LiteratureReferenceForCreate, LiteratureReferenceForUpdate, PrimarySource,
	PrimarySourceBmc, PrimarySourceFilter, PrimarySourceForCreate,
	PrimarySourceForUpdate, SenderInformation, SenderInformationBmc,
	SenderInformationFilter, SenderInformationForCreate, SenderInformationForUpdate,
	StudyFdaCrossReportedInd, StudyFdaCrossReportedIndBmc,
	StudyFdaCrossReportedIndFilter, StudyFdaCrossReportedIndForCreate,
	StudyFdaCrossReportedIndForUpdate, StudyInformation, StudyInformationBmc,
	StudyInformationFilter, StudyInformationForCreate, StudyInformationForUpdate,
	StudyRegistrationNumber, StudyRegistrationNumberBmc,
	StudyRegistrationNumberFilter, StudyRegistrationNumberForCreate,
	StudyRegistrationNumberForUpdate,
};
use lib_core::model::{self, ModelManager};
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_case_write_allowed, require_permission, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde_json::json;
use uuid::Uuid;

fn ensure_case_scope(
	case_id: Uuid,
	entity_case_id: Uuid,
	entity_id: Uuid,
	entity: &'static str,
) -> Result<()> {
	if case_id != entity_case_id {
		return Err(model::Error::EntityUuidNotFound {
			entity,
			id: entity_id,
		}
		.into());
	}
	Ok(())
}

async fn ensure_study_case(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	study_id: Uuid,
) -> Result<()> {
	lib_rest_core::require_case_read_allowed(ctx, mm, case_id).await?;
	let study = StudyInformationBmc::get(ctx, mm, study_id).await?;
	ensure_case_scope(case_id, study.case_id, study_id, "study_information")
}

async fn mark_case_dirty_c(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<()> {
	CaseBmc::update(
		ctx,
		mm,
		case_id,
		CaseForUpdate {
			safety_report_id: None,
			dg_prd_key: None,
			status: None,
			review_receivers_json: None,
			workflow_routes_json: None,
			mfds_report_type: None,
			fda_report_type: None,
			report_year: None,
			source_document_name: None,
			source_document_base64: None,
			source_document_media_type: None,
			submitted_by: None,
			submitted_at: None,
			raw_xml: None,
			dirty_c: Some(true),
			dirty_d: None,
			dirty_e: None,
			dirty_f: None,
			dirty_g: None,
			dirty_h: None,
		},
	)
	.await
	.map_err(Into::into)
}

fn normalize_primary_source_regulatory_value(
	value: Option<String>,
) -> Result<Option<String>> {
	let normalized = value.and_then(|raw| {
		let trimmed = raw.trim();
		if trimmed.is_empty() {
			None
		} else {
			Some(trimmed.to_string())
		}
	});
	match normalized.as_deref() {
		None => Ok(None),
		Some("1") => Ok(Some("1".to_string())),
		Some("2") | Some("3") => Ok(Some("2".to_string())),
		Some(other) => Err(Error::BadRequest {
			message: format!("invalid C.2.r.5 value '{other}' (expected: 1 or 2)"),
		}),
	}
}

fn primary_source_flag_update(value: &str) -> PrimarySourceForUpdate {
	PrimarySourceForUpdate {
		source_reporter_presave_id: None,
		reporter_title: None,
		reporter_given_name: None,
		reporter_middle_name: None,
		reporter_family_name: None,
		organization: None,
		department: None,
		street: None,
		city: None,
		state: None,
		postcode: None,
		telephone: None,
		country_code: None,
		email: None,
		qualification: None,
		qualification_kr1: None,
		primary_source_regulatory: Some(value.to_string()),
	}
}

async fn normalize_primary_source_flags(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	preferred_primary_id: Option<Uuid>,
) -> Result<()> {
	let filter = PrimarySourceFilter {
		case_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			case_id.to_string()
		))])),
		..Default::default()
	};
	let mut entities = PrimarySourceBmc::list(
		ctx,
		mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;
	if entities.is_empty() {
		return Ok(());
	}
	entities.sort_by_key(|entity| entity.sequence_number);

	let chosen_primary_id = preferred_primary_id
		.filter(|preferred| entities.iter().any(|entity| entity.id == *preferred))
		.or_else(|| {
			entities.iter().find_map(|entity| {
				(normalize_primary_source_regulatory_value(
					entity.primary_source_regulatory.clone(),
				)
				.ok()
				.flatten()
				.as_deref() == Some("1"))
				.then_some(entity.id)
			})
		})
		.unwrap_or(entities[0].id);

	for entity in entities {
		let desired = if entity.id == chosen_primary_id {
			"1"
		} else {
			"2"
		};
		let current = normalize_primary_source_regulatory_value(
			entity.primary_source_regulatory.clone(),
		)?
		.unwrap_or_else(|| "2".to_string());
		if current != desired {
			PrimarySourceBmc::update(
				ctx,
				mm,
				entity.id,
				primary_source_flag_update(desired),
			)
			.await?;
		}
	}

	Ok(())
}

// -- Sender Information (C.3.x)

/// POST /api/cases/{case_id}/safety-report/senders
pub async fn create_sender_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<SenderInformationForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<SenderInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, SENDER_INFORMATION_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForCreate { data } = params;
	let mut data = data;
	data.case_id = case_id;

	let id = SenderInformationBmc::create(&ctx, &mm, data).await?;
	let entity = SenderInformationBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/safety-report/senders
pub async fn list_sender_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<SenderInformation>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, SENDER_INFORMATION_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;
	let filter = SenderInformationFilter {
		case_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			case_id.to_string()
		))])),
	};
	let entities = SenderInformationBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/safety-report/senders/{id}
pub async fn get_sender_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<SenderInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, SENDER_INFORMATION_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;
	let entity = SenderInformationBmc::get(&ctx, &mm, id).await?;
	ensure_case_scope(case_id, entity.case_id, id, "sender_information")?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/safety-report/senders/{id}
pub async fn update_sender_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<SenderInformationForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<SenderInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, SENDER_INFORMATION_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForUpdate { data } = params;
	let entity = SenderInformationBmc::get(&ctx, &mm, id).await?;
	ensure_case_scope(case_id, entity.case_id, id, "sender_information")?;
	SenderInformationBmc::update(&ctx, &mm, id, data).await?;
	let entity = SenderInformationBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/safety-report/senders/{id}
pub async fn delete_sender_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, SENDER_INFORMATION_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let entity = SenderInformationBmc::get(&ctx, &mm, id).await?;
	ensure_case_scope(case_id, entity.case_id, id, "sender_information")?;
	SenderInformationBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}

// -- Primary Sources (C.2.r)

/// POST /api/cases/{case_id}/safety-report/primary-sources
pub async fn create_primary_source(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<PrimarySourceForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<PrimarySource>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRIMARY_SOURCE_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForCreate { data } = params;
	let mut data = data;
	data.case_id = case_id;
	data.primary_source_regulatory =
		normalize_primary_source_regulatory_value(data.primary_source_regulatory)?;

	let id = PrimarySourceBmc::create(&ctx, &mm, data).await?;
	let preferred_primary_id = PrimarySourceBmc::get(&ctx, &mm, id)
		.await?
		.primary_source_regulatory
		.as_deref()
		.filter(|value| *value == "1")
		.map(|_| id);
	normalize_primary_source_flags(&ctx, &mm, case_id, preferred_primary_id).await?;
	mark_case_dirty_c(&ctx, &mm, case_id).await?;
	let entity = PrimarySourceBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/safety-report/primary-sources
pub async fn list_primary_sources(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<PrimarySource>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRIMARY_SOURCE_LIST)?;
	let filter = PrimarySourceFilter {
		case_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			case_id.to_string()
		))])),
		..Default::default()
	};
	let entities = PrimarySourceBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/safety-report/primary-sources/{id}
pub async fn get_primary_source(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<PrimarySource>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRIMARY_SOURCE_READ)?;
	let entity = PrimarySourceBmc::get(&ctx, &mm, id).await?;
	ensure_case_scope(case_id, entity.case_id, id, "primary_sources")?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/safety-report/primary-sources/{id}
pub async fn update_primary_source(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<PrimarySourceForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<PrimarySource>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRIMARY_SOURCE_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForUpdate { data } = params;
	let mut data = data;
	data.primary_source_regulatory =
		normalize_primary_source_regulatory_value(data.primary_source_regulatory)?;
	let entity = PrimarySourceBmc::get(&ctx, &mm, id).await?;
	ensure_case_scope(case_id, entity.case_id, id, "primary_sources")?;
	PrimarySourceBmc::update(&ctx, &mm, id, data).await?;
	let preferred_primary_id = PrimarySourceBmc::get(&ctx, &mm, id)
		.await?
		.primary_source_regulatory
		.as_deref()
		.filter(|value| *value == "1")
		.map(|_| id);
	normalize_primary_source_flags(&ctx, &mm, case_id, preferred_primary_id).await?;
	mark_case_dirty_c(&ctx, &mm, case_id).await?;
	let entity = PrimarySourceBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/safety-report/primary-sources/{id}
pub async fn delete_primary_source(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRIMARY_SOURCE_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let entity = PrimarySourceBmc::get(&ctx, &mm, id).await?;
	ensure_case_scope(case_id, entity.case_id, id, "primary_sources")?;
	PrimarySourceBmc::delete(&ctx, &mm, id).await?;
	normalize_primary_source_flags(&ctx, &mm, case_id, None).await?;
	mark_case_dirty_c(&ctx, &mm, case_id).await?;
	Ok(StatusCode::NO_CONTENT)
}

// -- Literature References (C.4.r)

/// POST /api/cases/{case_id}/safety-report/documents
pub async fn create_documents_held_by_sender(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<DocumentsHeldBySenderForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<DocumentsHeldBySender>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, SAFETY_REPORT_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForCreate { data } = params;
	let mut data = data;
	data.case_id = case_id;

	let id = DocumentsHeldBySenderBmc::create(&ctx, &mm, data).await?;
	let entity = DocumentsHeldBySenderBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/safety-report/documents
pub async fn list_documents_held_by_sender(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<DocumentsHeldBySender>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, SAFETY_REPORT_READ)?;
	let filter = DocumentsHeldBySenderFilter {
		case_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			case_id.to_string()
		))])),
		..Default::default()
	};
	let entities = DocumentsHeldBySenderBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/safety-report/documents/{id}
pub async fn get_documents_held_by_sender(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<DocumentsHeldBySender>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, SAFETY_REPORT_READ)?;
	let entity = DocumentsHeldBySenderBmc::get(&ctx, &mm, id).await?;
	ensure_case_scope(case_id, entity.case_id, id, "documents_held_by_sender")?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/safety-report/documents/{id}
pub async fn update_documents_held_by_sender(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<DocumentsHeldBySenderForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<DocumentsHeldBySender>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, SAFETY_REPORT_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForUpdate { data } = params;
	let entity = DocumentsHeldBySenderBmc::get(&ctx, &mm, id).await?;
	ensure_case_scope(case_id, entity.case_id, id, "documents_held_by_sender")?;
	DocumentsHeldBySenderBmc::update(&ctx, &mm, id, data).await?;
	let entity = DocumentsHeldBySenderBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/safety-report/documents/{id}
pub async fn delete_documents_held_by_sender(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, SAFETY_REPORT_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let entity = DocumentsHeldBySenderBmc::get(&ctx, &mm, id).await?;
	ensure_case_scope(case_id, entity.case_id, id, "documents_held_by_sender")?;
	DocumentsHeldBySenderBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}

/// POST /api/cases/{case_id}/safety-report/literature
pub async fn create_literature_reference(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<LiteratureReferenceForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<LiteratureReference>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, LITERATURE_REFERENCE_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForCreate { data } = params;
	let mut data = data;
	data.case_id = case_id;

	let id = LiteratureReferenceBmc::create(&ctx, &mm, data).await?;
	let entity = LiteratureReferenceBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/safety-report/literature
pub async fn list_literature_references(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<LiteratureReference>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, LITERATURE_REFERENCE_LIST)?;
	let filter = LiteratureReferenceFilter {
		case_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			case_id.to_string()
		))])),
		..Default::default()
	};
	let entities = LiteratureReferenceBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/safety-report/literature/{id}
pub async fn get_literature_reference(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<LiteratureReference>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, LITERATURE_REFERENCE_READ)?;
	let entity = LiteratureReferenceBmc::get(&ctx, &mm, id).await?;
	ensure_case_scope(case_id, entity.case_id, id, "literature_references")?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/safety-report/literature/{id}
pub async fn update_literature_reference(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<LiteratureReferenceForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<LiteratureReference>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, LITERATURE_REFERENCE_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForUpdate { data } = params;
	let entity = LiteratureReferenceBmc::get(&ctx, &mm, id).await?;
	ensure_case_scope(case_id, entity.case_id, id, "literature_references")?;
	LiteratureReferenceBmc::update(&ctx, &mm, id, data).await?;
	let entity = LiteratureReferenceBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/safety-report/literature/{id}
pub async fn delete_literature_reference(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, LITERATURE_REFERENCE_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let entity = LiteratureReferenceBmc::get(&ctx, &mm, id).await?;
	ensure_case_scope(case_id, entity.case_id, id, "literature_references")?;
	LiteratureReferenceBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}

// -- Study Information (C.5)

/// POST /api/cases/{case_id}/safety-report/studies
pub async fn create_study_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<StudyInformationForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<StudyInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, STUDY_INFORMATION_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForCreate { data } = params;
	let mut data = data;
	data.case_id = case_id;

	let id = StudyInformationBmc::create(&ctx, &mm, data).await?;
	let entity = StudyInformationBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/safety-report/studies
pub async fn list_study_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<StudyInformation>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, STUDY_INFORMATION_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;
	let filter = StudyInformationFilter {
		case_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			case_id.to_string()
		))])),
	};
	let entities = StudyInformationBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/safety-report/studies/{id}
pub async fn get_study_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<StudyInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, STUDY_INFORMATION_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;
	let entity = StudyInformationBmc::get(&ctx, &mm, id).await?;
	ensure_case_scope(case_id, entity.case_id, id, "study_information")?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/safety-report/studies/{id}
pub async fn update_study_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<StudyInformationForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<StudyInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, STUDY_INFORMATION_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForUpdate { data } = params;
	let entity = StudyInformationBmc::get(&ctx, &mm, id).await?;
	ensure_case_scope(case_id, entity.case_id, id, "study_information")?;
	StudyInformationBmc::update(&ctx, &mm, id, data).await?;
	let entity = StudyInformationBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/safety-report/studies/{id}
pub async fn delete_study_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, STUDY_INFORMATION_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let entity = StudyInformationBmc::get(&ctx, &mm, id).await?;
	ensure_case_scope(case_id, entity.case_id, id, "study_information")?;
	StudyInformationBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}

// -- Study Registration Numbers (C.5.1.r)

/// POST /api/cases/{case_id}/safety-report/studies/{study_id}/registrations
pub async fn create_study_registration_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, study_id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForCreate<StudyRegistrationNumberForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<StudyRegistrationNumber>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, STUDY_REGISTRATION_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	ensure_study_case(&ctx, &mm, case_id, study_id).await?;
	let ParamsForCreate { data } = params;
	let mut data = data;
	data.study_information_id = study_id;

	let id = StudyRegistrationNumberBmc::create(&ctx, &mm, data).await?;
	let entity = StudyRegistrationNumberBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/safety-report/studies/{study_id}/registrations
pub async fn list_study_registration_numbers(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, study_id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<StudyRegistrationNumber>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, STUDY_REGISTRATION_LIST)?;
	ensure_study_case(&ctx, &mm, case_id, study_id).await?;
	let filter = StudyRegistrationNumberFilter {
		study_information_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			study_id.to_string()
		))])),
		..Default::default()
	};
	let entities = StudyRegistrationNumberBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/safety-report/studies/{study_id}/registrations/{id}
pub async fn get_study_registration_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, study_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<StudyRegistrationNumber>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, STUDY_REGISTRATION_READ)?;
	let entity = StudyRegistrationNumberBmc::get(&ctx, &mm, id).await?;
	if entity.study_information_id != study_id {
		return Err(model::Error::EntityUuidNotFound {
			entity: "study_registration_numbers",
			id,
		}
		.into());
	}
	ensure_study_case(&ctx, &mm, case_id, study_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/safety-report/studies/{study_id}/registrations/{id}
pub async fn update_study_registration_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, study_id, id)): Path<(Uuid, Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<StudyRegistrationNumberForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<StudyRegistrationNumber>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, STUDY_REGISTRATION_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForUpdate { data } = params;
	let entity = StudyRegistrationNumberBmc::get(&ctx, &mm, id).await?;
	if entity.study_information_id != study_id {
		return Err(model::Error::EntityUuidNotFound {
			entity: "study_registration_numbers",
			id,
		}
		.into());
	}
	ensure_study_case(&ctx, &mm, case_id, study_id).await?;
	StudyRegistrationNumberBmc::update(&ctx, &mm, id, data).await?;
	let entity = StudyRegistrationNumberBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/safety-report/studies/{study_id}/registrations/{id}
pub async fn delete_study_registration_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, study_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, STUDY_REGISTRATION_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let entity = StudyRegistrationNumberBmc::get(&ctx, &mm, id).await?;
	if entity.study_information_id != study_id {
		return Err(model::Error::EntityUuidNotFound {
			entity: "study_registration_numbers",
			id,
		}
		.into());
	}
	ensure_study_case(&ctx, &mm, case_id, study_id).await?;
	StudyRegistrationNumberBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}

// -- Study FDA Cross-Reported INDs (FDA.C.5.6.r)

/// POST /api/cases/{case_id}/safety-report/studies/{study_id}/fda-cross-reported-inds
pub async fn create_study_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, study_id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForCreate<StudyFdaCrossReportedIndForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<StudyFdaCrossReportedInd>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, STUDY_REGISTRATION_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	ensure_study_case(&ctx, &mm, case_id, study_id).await?;
	let ParamsForCreate { data } = params;
	let mut data = data;
	data.study_information_id = study_id;

	let id = StudyFdaCrossReportedIndBmc::create(&ctx, &mm, data).await?;
	let entity = StudyFdaCrossReportedIndBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/safety-report/studies/{study_id}/fda-cross-reported-inds
pub async fn list_study_fda_cross_reported_inds(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, study_id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<StudyFdaCrossReportedInd>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, STUDY_REGISTRATION_LIST)?;
	ensure_study_case(&ctx, &mm, case_id, study_id).await?;
	let filter = StudyFdaCrossReportedIndFilter {
		study_information_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			study_id.to_string()
		))])),
		..Default::default()
	};
	let entities = StudyFdaCrossReportedIndBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/safety-report/studies/{study_id}/fda-cross-reported-inds/{id}
pub async fn get_study_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, study_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<StudyFdaCrossReportedInd>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, STUDY_REGISTRATION_READ)?;
	let entity = StudyFdaCrossReportedIndBmc::get(&ctx, &mm, id).await?;
	if entity.study_information_id != study_id {
		return Err(model::Error::EntityUuidNotFound {
			entity: "study_fda_cross_reported_inds",
			id,
		}
		.into());
	}
	ensure_study_case(&ctx, &mm, case_id, study_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/safety-report/studies/{study_id}/fda-cross-reported-inds/{id}
pub async fn update_study_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, study_id, id)): Path<(Uuid, Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<StudyFdaCrossReportedIndForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<StudyFdaCrossReportedInd>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, STUDY_REGISTRATION_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForUpdate { data } = params;
	let entity = StudyFdaCrossReportedIndBmc::get(&ctx, &mm, id).await?;
	if entity.study_information_id != study_id {
		return Err(model::Error::EntityUuidNotFound {
			entity: "study_fda_cross_reported_inds",
			id,
		}
		.into());
	}
	ensure_study_case(&ctx, &mm, case_id, study_id).await?;
	StudyFdaCrossReportedIndBmc::update(&ctx, &mm, id, data).await?;
	let entity = StudyFdaCrossReportedIndBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/safety-report/studies/{study_id}/fda-cross-reported-inds/{id}
pub async fn delete_study_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, study_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, STUDY_REGISTRATION_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let entity = StudyFdaCrossReportedIndBmc::get(&ctx, &mm, id).await?;
	if entity.study_information_id != study_id {
		return Err(model::Error::EntityUuidNotFound {
			entity: "study_fda_cross_reported_inds",
			id,
		}
		.into());
	}
	ensure_study_case(&ctx, &mm, case_id, study_id).await?;
	StudyFdaCrossReportedIndBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}
